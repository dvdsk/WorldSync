use bytes::Bytes;
use reqwest::Response;
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
    #[error("While cleaning up after error: {org} a second error occured: {during_cleanup}")]
    CleanUp {
        org: Box<Error>,
        during_cleanup: Box<Error>,
    },
    #[error("Ran into an io error during unpacking the local java install: {0:?}")]
    Unpacking(std::io::ErrorKind),
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

pub fn unpack(stream: impl Read, dir: &Path) -> Result<(), Error> {
    use flate2::read::GzDecoder;
    let tar = GzDecoder::new(stream);
    let mut archive = tar::Archive::new(tar);
    archive
        .unpack(dir)
        .map_err(|e| e.kind())
        .map_err(Error::Unpacking)
}

async fn do_cleanup(dir: &Path) -> Result<(), Error> {
    assert!(dir.is_dir());
    fs::remove_dir_all(dir)
        .await
        .map_err(|e| e.kind())
        .map_err(Error::Inaccessible)
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

struct Download<S: Stream<Item = Result<Bytes, reqwest::Error>>> {
    decode_task: Option<JoinHandle<Result<(), Error>>>,
    stream: S,
    dir: PathBuf,
    tx: mpsc::UnboundedSender<Bytes>,
}

async fn download(dir: PathBuf) -> Result<(), Error>{
    use futures::stream::TryStreamExt;
    let stream = test_stream(dir).await?;
    // this is needed as try_next needs Pin<TryStream> an TryStream is 
    // not implemented for Pin<TryStream> this is due to trait aliasses
    // not yet being stable, and will not be a problem in the future.
    // this line of code can be removed when trait aliasses are stabalized
    let mut stream = stream.into_stream().boxed();
    while let Some(_n) = stream.try_next().await? {
        dbg!("progress!");
    }

    Ok(())
}

use futures::{stream, Stream, TryStream, StreamExt};
async fn test_stream(dir: PathBuf) -> Result<impl TryStream<Ok = usize, Error = Error>, Error> {
    let (task, response, tx) = init_download(&dir).await?;
    let init = Download {
        decode_task: Some(task),
        stream: response.bytes_stream(),
        dir,
        tx,
    };

    Ok(stream::try_unfold(init, |mut state| async move {
        let res = state.advance().await;
        let yielded = 1;
        match res {
            State::Downloading => Ok(Some((yielded, state))),
            State::Done => Ok(None),
            State::Error(e) => {
                let e = cleanup(e, &state.dir).await;
                return Err(e);
            }
        }
    }))
}

async fn cleanup(org_err: Error, dir: &Path) -> Error {
    match do_cleanup(&dir).await {
        Ok(_) => org_err,
        Err(e) => Error::CleanUp {
            org: Box::new(org_err),
            during_cleanup: Box::new(e),
        },
    }
}

enum State {
    Downloading,
    Done,
    Error(Error),
}

impl<S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin> Download<S> {
    async fn advance(&mut self) -> State {
        use Error::*;

        let item = match self.stream.next().await {
            Some(item) => item,
            None => return State::Done,
        };

        let bytes = match item {
            Ok(bytes) => bytes,
            Err(e) => return State::Error(InvalidResponse(e)),
        };

        match self.tx.send(bytes) {
            Ok(_) => State::Downloading,
            Err(_) => {
                let unpack_err = self.decode_task.take().unwrap().await.unwrap().unwrap_err();
                return State::Error(unpack_err);
            }
        }
    }
}

async fn init_download(
    dir: &PathBuf,
) -> Result<
    (
        JoinHandle<Result<(), Error>>,
        Response,
        mpsc::UnboundedSender<Bytes>,
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
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tokio::task::spawn_blocking(move || {
        let stream = ChannelRead::from(rx);
        unpack(stream, &dir_clone)
    });

    Ok((task, response, tx))
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
