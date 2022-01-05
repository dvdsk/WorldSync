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

#[derive(Copy, Clone)]
enum Phase {
    Running,
    Done,
}

type Tomato = Result<Bytes, reqwest::Error>;
struct Download<S: Stream<Item = Tomato>> {
    phase: Phase,
    decode_task: Option<JoinHandle<Result<(), Error>>>,
    stream: S,
    dir: PathBuf,
    tx: mpsc::UnboundedSender<Bytes>,
}

use futures::{stream, Stream};
async fn test_stream(dir: PathBuf) -> impl Stream<Item = Result<(), Error>> {
    let init = match init_download(&dir).await {
        Ok((task, response, tx)) => Download {
            phase: Phase::Running,
            decode_task: Some(task),
            stream: response.bytes_stream(),
            dir,
            tx,
        },
        Err(e) => todo!(),
        // Err(e) => return stream::once(async { Err(e) }),
    };

    use Phase::*;
    stream::unfold(init, |mut state| async move {
        match state.phase {
            Running => {
                let res = state.advance().await;
                match res {
                    Ok(_) => Some((res, state)),
                    Err(e) => {
                        state.phase = Done;
                        let e = cleanup(e, &state.dir).await;
                        Some((Err(e), state))
                    }
                }
            }
            Done => None,
        }
    })
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

impl<S: Stream<Item = Tomato> + Unpin> Download<S> {
    async fn advance(&mut self) -> Result<(), Error> {
        use futures_util::StreamExt;
        use Error::*;

        let item = match self.stream.next().await {
            Some(item) => item,
            None => return Ok(()),
        };

        let bytes = match item {
            Ok(bytes) => bytes,
            Err(e) => return Err(InvalidResponse(e)),
        };

        match self.tx.send(bytes) {
            Ok(_) => Ok(()),
            Err(_) => {
                let unpack_err = self.decode_task.take().unwrap().await.unwrap().unwrap_err();
                return Err(unpack_err);
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
        download_java(test_dir.into()).await.unwrap();
        // fs::remove_dir_all(test_dir).await.unwrap();
    }
}
