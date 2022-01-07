use bytes::Bytes;
use std::io::{Cursor, Read};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use tokio::fs;
use tokio::sync::mpsc;

// Wrap a channel into something that impls `io::Read`
pub struct ChannelRead {
    rx: mpsc::UnboundedReceiver<Bytes>,
    current: Cursor<Vec<u8>>,
    read: Rc<AtomicU64>,
}

impl ChannelRead {
    pub fn from(rx: mpsc::UnboundedReceiver<Bytes>) -> ChannelRead {
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

/// os either: linux, macos or windows
/// pack tar.gz for linux and macos and zip for windows
pub fn download_url(os: &str, pack: &str) -> String {
    //https://www.oracle.com/java/technologies/jdk-script-friendly-urls/
    const ARCH: &str = "x64";
    format!(
        "https://download.oracle.com/java/17/latest/jdk-17_{}-{}_bin.{}",
        os, ARCH, pack
    )
}

use super::Error;
pub async fn dir_empty(dir: &Path) -> Result<bool, Error> {
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
