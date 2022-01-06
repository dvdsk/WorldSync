use bytes::Bytes;
use reqwest::Response;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
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

/// os either: linux, macos or windows
/// pack tar.gz for linux and macos and zip for windows
fn download_url(os: &str, pack: &str) -> String {
    //https://www.oracle.com/java/technologies/jdk-script-friendly-urls/
    const ARCH: &str = "x64";
    format!(
        "https://download.oracle.com/java/17/latest/jdk-17_{}-{}_bin.{}",
        os, ARCH, pack
    )
}

use zip::result::ZipError;
#[derive(Debug)]
pub enum ArchiveErr {
    Tar(std::io::Error),
    Zip(ZipError),
}

#[derive(Debug, thiserror::Error)]
pub enum UnpackErr {
    #[error("Io error listing the files: {0:?}")]
    ListEntries(ArchiveErr),
    #[error("Io error accessing a file listing: {0:?}")]
    AccessEntry(ArchiveErr),
    #[error("Io error while unpacking a file: {0:?}")]
    Unpack(ArchiveErr),
    #[error("Io error while creating new file: {0:?}")]
    Create(std::io::Error),
    #[error("Io error while writing data to a file: {0:?}")]
    Write(std::io::Error),
    #[error("A path in the archive tried to escape the target dir")]
    PathLeft(Option<PathBuf>),
}

pub fn unpack_tar_gz(
    byte_rx: mpsc::UnboundedReceiver<Bytes>,
    dir: PathBuf,
    progress: mpsc::UnboundedSender<u64>,
) -> Result<(), UnpackErr> {
    use flate2::read::GzDecoder;
    use ArchiveErr::Tar;
    use UnpackErr::*;

    let stream = ChannelRead::from(byte_rx);
    let read_observer = stream.get_observer();
    let tar = GzDecoder::new(stream);
    let mut ar = tar::Archive::new(tar);

    for file in ar.entries().map_err(Tar).map_err(ListEntries)? {
        let mut file = file.map_err(Tar).map_err(AccessEntry)?;
        let contained_path = file.unpack_in(&dir).map_err(Tar).map_err(Unpack)?;
        if !contained_path {
            let path = file.path().ok().map(|cow| cow.into_owned());
            return Err(PathLeft(path));
        }
        progress
            .send(read_observer.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap();
    }
    Ok(())
}

pub fn unpack_zip(
    mut stream: mpsc::UnboundedReceiver<Bytes>,
    dir: PathBuf,
    progress: mpsc::UnboundedSender<u64>,
) -> Result<(), UnpackErr> {
    use ArchiveErr::Zip;
    use UnpackErr::*;

    let mut buf = Vec::new();
    while let Some(bytes) = stream.blocking_recv() {
        buf.extend_from_slice(&bytes);
    }
    let reader = Cursor::new(buf);

    let mut zip = zip::ZipArchive::new(reader)
        .map_err(Zip)
        .map_err(ListEntries)?;
    for i in 0..zip.len() {
        let mut zipped = zip.by_index(i).map_err(Zip).map_err(AccessEntry)?;
        let unsafe_path = PathBuf::from(zipped.name());
        let path = zipped
            .enclosed_name()
            .ok_or(UnpackErr::PathLeft(Some(unsafe_path)))?;
        let path = dir.join(path);
        let mut file = std::fs::File::create(path).map_err(Create)?;
        std::io::copy(&mut zipped, &mut file).map_err(Write)?;
        progress.send(zipped.compressed_size()).unwrap();
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


use futures::stream::TryStreamExt;
async fn download_targz(dir: PathBuf, url: String) -> Result<(), Error> {
    let mut stream = unpack_stream(dir, url, unpack_tar_gz).await?;
    while let Some(progress) = stream.try_next().await? {
        print!("\rprogress: {}", progress);
    }

    Ok(())
}

async fn download_zip(dir: PathBuf, url: String) -> Result<(), Error> {{
    let mut stream = unpack_stream(dir, url, unpack_zip).await?;
    while let Some(progress) = stream.try_next().await? {
        print!("\rprogress: {}", progress);
    }

    Ok(())
}
}

use futures::{stream, Stream, StreamExt, TryStream};
async fn unpack_stream<F>(
    dir: PathBuf,
    url: String,
    unpack: F,
) -> Result<impl TryStream<Ok = Progress, Error = Error>, Error>
where
    F: Fn(
            mpsc::UnboundedReceiver<Bytes>,
            PathBuf,
            mpsc::UnboundedSender<u64>,
        ) -> Result<(), UnpackErr>
        + Send
        + 'static,
{
    let (task, response, tx, rx) = init_download(&dir, url, unpack).await?;
    let init = Download {
        decode_task: Some(task),
        bytes_decoded: rx,
        phase: Phase::Running,
        progress: Progress::from(&response),
        stream: response.bytes_stream(),
        dir,
        tx,
    };

    let stream = stream::try_unfold(init, state_machine);
    // this is needed as try_next needs Pin<TryStream> an TryStream is
    // not implemented for Pin<TryStream> this is due to trait aliasses
    // not yet being stable, and will not be a problem in the future.
    // this line of code can be removed when trait aliasses are stabalized
    // let mut stream = stream.into_stream().boxed();
    Ok(stream.into_stream().boxed())
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
                self.progress.decoded = bytes;
                Ok(false)
            }
            None => {
                self.decode_task
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
                self.progress.decoded = decoded.unwrap();
                return Ok(false)
            }
        };

        let item = match stream_next {
            Some(item) => item,
            None => return Ok(true),
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

async fn init_download<F>(
    dir: &PathBuf,
    url: String,
    unpack: F,
) -> Result<
    (
        JoinHandle<Result<(), UnpackErr>>,
        Response,
        mpsc::UnboundedSender<Bytes>,
        mpsc::UnboundedReceiver<u64>,
    ),
    Error,
>
where
    F: Fn(
            mpsc::UnboundedReceiver<Bytes>,
            PathBuf,
            mpsc::UnboundedSender<u64>,
        ) -> Result<(), UnpackErr>
        + Send
        + 'static,
{
    use Error::*;

    // only if dir is empty now can we safely remove all its contents
    // in case of error
    if !dir_empty(&dir).await? {
        return Err(NotEmpty);
    }

    trace!("downloading: {}", url);
    let response = reqwest::get(url)
        .await
        .map_err(RequestFailed)?
        .error_for_status()
        .map_err(RequestFailed)?;

    let dir_clone = dir.clone();
    let (byte_tx, byte_rx) = mpsc::unbounded_channel();
    let (size_tx, size_rx) = mpsc::unbounded_channel();
    let task = tokio::task::spawn_blocking(move || unpack(byte_rx, dir_clone, size_tx));

    Ok((task, response, byte_tx, size_rx))
}

use std::io::{Cursor, Read};
// Wrap a channel into something that impls `io::Read`
pub struct ChannelRead {
    rx: mpsc::UnboundedReceiver<Bytes>,
    current: Cursor<Vec<u8>>,
    read: Rc<AtomicU64>,
}

impl ChannelRead {
    fn from(rx: mpsc::UnboundedReceiver<Bytes>) -> ChannelRead {
        ChannelRead {
            rx,
            current: Cursor::new(Vec::new()),
            read: Rc::new(Default::default()),
        }
    }

    pub fn get_observer(&self) -> Rc<AtomicU64> {
        self.read.clone()
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
        let n = self.current.read(buf)?;
        self.read
            .fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_download_linux() {
        let test_dir = Path::new("test_download_linux");
        if !test_dir.is_dir() {
            fs::create_dir(test_dir).await.unwrap();
        }

        let url = download_url("linux", "tar.gz");
        download_targz(test_dir.into(), url).await.unwrap();
        fs::remove_dir_all(test_dir).await.unwrap();
    }
    #[tokio::test]
    async fn test_download_windows() {
        let test_dir = Path::new("test_download_windows");
        if !test_dir.is_dir() {
            fs::create_dir(test_dir).await.unwrap();
        }

        let url = download_url("windows", "zip");
        download_targz(test_dir.into(), url).await.unwrap();
        fs::remove_dir_all(test_dir).await.unwrap();
    }
}
