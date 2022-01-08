use bytes::Bytes;
use futures::stream::{Stream, TryStreamExt};
use reqwest::Response;
use std::fmt;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

mod stream;
mod unpack;
mod util;

pub use unpack::{unpack_zip, unpack_tar_gz};
pub use util::build_url;
pub use stream::unpack_stream;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not request latest java version from oracle.com")]
    RequestFailed(reqwest::Error),
    #[error("Could not get bytes from oracle.com response")]
    InvalidResponse(reqwest::Error),
    #[error("Ran into an io error during unpacking the local java install: {0:?}")]
    Unpacking(unpack::Error),
    #[error("Directory could not be accessed, io error: {0:?}")]
    Inaccessible(std::io::ErrorKind),
    #[error("Directory needs to be empty to download java into it")]
    NotEmpty,
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
    decode_task: Option<JoinHandle<Result<(), unpack::Error>>>,
    bytes_decoded: mpsc::UnboundedReceiver<u64>,
    progress: Progress,
    phase: stream::Phase,
    stream: S,
    // option so we can drop it when the download is done
    // signaling to the reader we are done
    byte_tx: Option<mpsc::UnboundedSender<Bytes>>,
}

async fn download_targz(dir: PathBuf, url: String) -> Result<(), Error> {
    let mut stream = stream::unpack_stream(dir, url, unpack::unpack_tar_gz).await?;
    while let Some(progress) = stream.try_next().await? {
        print!("\rprogress: {}", progress);
    }
    Ok(())
}

async fn download_zip(dir: PathBuf, url: String) -> Result<(), Error> {
    let mut stream = stream::unpack_stream(dir, url, unpack::unpack_zip).await?;
    while let Some(progress) = stream.try_next().await? {
        print!("\rprogress: {}", progress);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn download_linux() {
        let test_dir = Path::new("test_download_linux");
        if !test_dir.is_dir() {
            fs::create_dir(test_dir).await.unwrap();
        }

        let url = util::build_url("linux", "tar.gz");
        download_targz(test_dir.into(), url).await.unwrap();
        fs::remove_dir_all(test_dir).await.unwrap();
    }
    #[tokio::test]
    async fn download_windows() {
        let test_dir = Path::new("test_download_windows");
        if !test_dir.is_dir() {
            fs::create_dir(test_dir).await.unwrap();
        }

        let url = util::build_url("windows", "zip");
        download_zip(test_dir.into(), url).await.unwrap();
        fs::remove_dir_all(test_dir).await.unwrap();
    }
    #[tokio::test]
    async fn fail_to_unpack() {
        let test_dir = Path::new("test_fail_to_unpack");
        if !test_dir.is_dir() {
            fs::create_dir(test_dir).await.unwrap();
        }

        let url = util::build_url("windows", "zip");
        let res = download_targz(test_dir.into(), url).await;
        use Error::Unpacking;
        use unpack::Error::AccessEntry;
        use unpack::ArchiveErr::Tar;
        match res {
            Err(Unpacking(AccessEntry(Tar(_)))) => (),
            _ => panic!("should error trying to open a zip as tar"),
        }
        fs::remove_dir_all(test_dir).await.unwrap();
    }
}
