use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::mpsc;
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

async fn cleanup(dir: &Path) -> Result<(), Error> {
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

#[tracing::instrument]
pub async fn download_java(dir: PathBuf) -> Result<(), Error> {
    use futures_util::StreamExt;
    use Error::*;

    // only if dir is empty now can we safely remove all its contents
    // in case of error
    if !dir_empty(&dir).await? {
        return Err(NotEmpty);
    }

    let url = download_url();
    trace!("downloading: {}", url);
    let mut stream = reqwest::get(url)
        .await
        .map_err(RequestFailed)?
        .error_for_status()
        .map_err(RequestFailed)?
        .bytes_stream();

    let dir_clone = dir.clone();
    let (tx, rx) = mpsc::unbounded_channel();
    let task = tokio::task::spawn_blocking(move || {
        let stream = ChannelRead::from(rx);
        unpack(stream, &dir_clone)
    });

    let res = loop {
        let item = match stream.next().await {
            Some(item) => item,
            None => break Ok(()),
        };

        let bytes = match item {
            Ok(bytes) => bytes,
            Err(e) => break Err(InvalidResponse(e)),
        };

        match tx.send(bytes) {
            Ok(_) => (),
            Err(_) => {
                let unpack_err = task.await.unwrap().unwrap_err();
                break Err(unpack_err);
            }
        }
    };

    if let Err(org_err) = res {
        cleanup(&dir).await.map_err(|e| CleanUp {
            org: Box::new(org_err),
            during_cleanup: Box::new(e),
        })?;
    }

    Ok(())
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
