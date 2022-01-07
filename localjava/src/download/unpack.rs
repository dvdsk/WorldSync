use bytes::Bytes;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::sync::mpsc;

use super::util;

use zip::result::ZipError;
#[derive(Debug)]
pub enum ArchiveErr {
    Tar(std::io::Error),
    Zip(ZipError),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
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
) -> Result<(), Error> {
    use flate2::read::GzDecoder;
    use ArchiveErr::Tar;
    use Error::*;

    let stream = util::ChannelRead::from(byte_rx);
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
) -> Result<(), Error> {
    use ArchiveErr::Zip;
    use Error::*;

    let mut buf = Vec::new();
    while let Some(bytes) = stream.blocking_recv() {
        buf.extend_from_slice(&bytes);
    }
    let reader = Cursor::new(buf);

    let mut zip = zip::ZipArchive::new(reader)
        .map_err(Zip)
        .map_err(ListEntries)?;

    let mut unzipped = 0;
    for i in 0..zip.len() {
        let mut zipped = zip.by_index(i).map_err(Zip).map_err(AccessEntry)?;
        let unsafe_path = PathBuf::from(zipped.name());
        let path = zipped
            .enclosed_name()
            .ok_or(Error::PathLeft(Some(unsafe_path)))?;
        let path = dir.join(path);
        if zipped.is_dir() {
            std::fs::create_dir(path).map_err(Create)?;
            continue;
        }

        let mut file = std::fs::File::create(path).map_err(Create)?;
        std::io::copy(&mut zipped, &mut file).map_err(Write)?;
        unzipped += zipped.compressed_size();
        progress.send(unzipped).unwrap();
    }

    Ok(())
}
