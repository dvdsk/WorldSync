use bytes::Bytes;
use reqwest::Response;
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::trace;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not request latest java version from oracle.com")]
    RequestFailed(reqwest::Error),
    #[error("Could not get bytes from oracle.com response")]
    InvalidResponse(reqwest::Error),
    #[error("Ran into an io error during unpacking the local java install: {0:?}")]
    Unpacking(UnpackErr),
    #[error("Directory could not be accessed, io error: {0:?}")]
    Inaccessible(std::io::ErrorKind),
    #[error("Directory needs to be empty to download java into it")]
    NotEmpty,
}

fn download_url() -> String {
    //https://www.oracle.com/java/technologies/jdk-script-friendly-urls/
    const OS: &str = "linux";
    const ARCH: &str = "x64";
    const PACK: &str = "tar.gz";
    format!(
        "https://download.oracle.com/java/17/latest/jdk-17_{}-{}_bin.{}",
        OS, ARCH, PACK
    )
}

#[derive(Debug, thiserror::Error)]
pub enum UnpackErr {
    #[error("Io error listing the files: {0:?}")]
    ListEntries(std::io::Error),
    #[error("Io error accessing a file listing: {0:?}")]
    AccessEntry(std::io::Error),
    #[error("Io error while unpacking a file: {0:?}")]
    Unpack(std::io::Error),
    #[error("A path in the tar went out of the target dir")]
    PathLeft(Option<PathBuf>),
}

pub fn unpack(
    stream: impl Read,
    dir: &Path,
    progress: mpsc::UnboundedSender<u64>,
) -> Result<(), UnpackErr> {
    use flate2::read::GzDecoder;
    use UnpackErr::*;

    let tar = GzDecoder::new(stream);
    let mut ar = tar::Archive::new(tar);

    for file in ar.entries().map_err(ListEntries)? {
        let mut file = file.map_err(AccessEntry)?;
        let contained_path = file.unpack_in(dir).map_err(Unpack)?;
        if !contained_path {
            let path = file.path().ok().map(|cow| cow.into_owned());
            return Err(PathLeft(path));
        }
        progress.send(file.size()).unwrap();
    }
    Ok(())
}

async fn dir_empty(dir: &Path) -> Result<bool, Error> {
    let content = fs::read_dir(dir)
        .await
        .map_err(|e| e.kind())
        .map_err(Error::Inaccessible)?
        .next_entry()
        .await
        .map_err(|e| e.kind())
        .map_err(Error::Inaccessible)?;
    match content {
        Some(_) => Ok(false),
        None => Ok(true),
    }
}

#[derive(Clone)]
pub struct Progress {
    pub total: u64,
    pub downloaded: u64,
    pub decoded: u64,
}

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unpacking = self.decoded as f32 / self.total as f32 * 100.0;
        let downloading = self.downloaded as f32 / self.total as f32 * 100.0;

        write!(
            f,
            "Downloading ({:.1}%) and Unpacking ({:.1}%)",
            downloading, unpacking
        )
    }
}

impl Progress {
    fn from(resp: &Response) -> Self {
        Self {
            total: resp.content_length().unwrap(),
            decoded: 0,
            downloaded: 0,
        }
    }
}

struct Download<S: Stream<Item = Result<Bytes, reqwest::Error>>> {
    decode_task: Option<JoinHandle<Result<(), UnpackErr>>>,
    bytes_decoded: mpsc::UnboundedReceiver<u64>,
    progress: Progress,
    phase: Phase,
    stream: S,
    dir: PathBuf,
    tx: mpsc::UnboundedSender<Bytes>,
}

async fn download(dir: PathBuf) -> Result<(), Error> {
    use futures::stream::TryStreamExt;
    let stream = test_stream(dir).await?;
    // this is needed as try_next needs Pin<TryStream> an TryStream is
    // not implemented for Pin<TryStream> this is due to trait aliasses
    // not yet being stable, and will not be a problem in the future.
    // this line of code can be removed when trait aliasses are stabalized
    let mut stream = stream.into_stream().boxed();
    while let Some(progress) = stream.try_next().await? {
        // print!("\rprogress: {}", progress);
        println!("progress: {}", progress);
    }

    Ok(())
}

use futures::{stream, Stream, StreamExt, TryStream};
async fn test_stream(dir: PathBuf) -> Result<impl TryStream<Ok = Progress, Error = Error>, Error> {
    let (task, response, tx, rx) = init_download(&dir).await?;
    let init = Download {
        decode_task: Some(task),
        bytes_decoded: rx,
        phase: Phase::Running,
        progress: Progress::from(&response),
        stream: response.bytes_stream(),
        dir,
        tx,
    };

    Ok(stream::try_unfold(init, state_machine))
}

enum Phase {
    Running,
    DlDone,
}

async fn state_machine<S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin>(
    mut state: Download<S>,
) -> Result<Option<(Progress, Download<S>)>, Error> {
    match state.phase {
        Phase::Running => {
            let done = state.download_and_unpack().await?;
            if done {
                state.phase = Phase::DlDone;
            }
            Ok(Some((state.progress.clone(), state)))
        }
        Phase::DlDone => {
            let done = state.unpack().await?;
            match done {
                false => Ok(Some((state.progress.clone(), state))),
                true => Ok(None),
            }
        }
    }
}

impl<S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin> Download<S> {
    async fn unpack(&mut self) -> Result<bool, Error> {
        match self.bytes_decoded.recv().await {
            Some(bytes) => {
                self.progress.decoded += bytes;
                Ok(false)
            }
            None => {
                dbg!();
                self
                    .decode_task
                    .take()
                    .expect("should always have ownership over the task")
                    .await
                    .expect("io error occured joining with the task")
                    .map_err(|e| Error::Unpacking(e))?;
                Ok(true)
            }
        }
    }

    async fn download_and_unpack(&mut self) -> Result<bool, Error> {
        use Error::*;

        let stream_next = tokio::select! {
            item = self.stream.next() => item,
            decoded = self.bytes_decoded.recv() => {
                self.progress.decoded += decoded.unwrap();
                return Ok(false)
            }
        };

        let item = match stream_next {
            Some(item) => item,
            None => return Ok(true)
        };

        let bytes = item.map_err(InvalidResponse)?;

        self.progress.downloaded += bytes.len() as u64;
        match self.tx.send(bytes) {
            Ok(_) => Ok(false),
            Err(_) => {
                let err = self
                    .decode_task
                    .take()
                    .expect("should always have ownership over the task")
                    .await
                    .expect("io error occured joining with the task")
                    .expect_err("task should return an error if sending failed");
                return Err(Error::Unpacking(err));
            }
        }
    }
}

async fn init_download(
    dir: &PathBuf,
) -> Result<
    (
        JoinHandle<Result<(), UnpackErr>>,
        Response,
        mpsc::UnboundedSender<Bytes>,
        mpsc::UnboundedReceiver<u64>,
    ),
    Error,
> {
    use Error::*;

    // only if dir is empty now can we safely remove all its contents
    // in case of error
    if !dir_empty(&dir).await? {
        return Err(NotEmpty);
    }

    let url = download_url();
    trace!("downloading: {}", url);
    let response = reqwest::get(url)
        .await
        .map_err(RequestFailed)?
        .error_for_status()
        .map_err(RequestFailed)?;

    let dir_clone = dir.clone();
    let (byte_tx, byte_rx) = mpsc::unbounded_channel();
    let (size_tx, size_rx) = mpsc::unbounded_channel();
    let task = tokio::task::spawn_blocking(move || {
        let stream = ChannelRead::from(byte_rx);
        unpack(stream, &dir_clone, size_tx)
    });

    Ok((task, response, byte_tx, size_rx))
}

use std::io::{Cursor, Read};
// Wrap a channel into something that impls `io::Read`
struct ChannelRead {
    rx: mpsc::UnboundedReceiver<Bytes>,
    current: Cursor<Vec<u8>>,
}

impl ChannelRead {
    fn from(rx: mpsc::UnboundedReceiver<Bytes>) -> ChannelRead {
        ChannelRead {
            rx,
            current: Cursor::new(Vec::new()),
        }
    }
}

impl Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.current.position() == self.current.get_ref().len() as u64 {
            if let Some(bytes) = self.rx.blocking_recv() {
                self.current = Cursor::new(bytes.to_vec());
            }
            // If recv() "fails", it means the sender closed its part of
            // the channel, which means EOF. Propagate EOF by allowing
            // a read from the exhausted cursor.
        }
        self.current.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_download() {
        let test_dir = Path::new("test_download");
        if !test_dir.is_dir() {
            fs::create_dir(test_dir).await.unwrap();
        }
        download(test_dir.into()).await.unwrap();
        // fs::remove_dir_all(test_dir).await.unwrap();
    }
}
