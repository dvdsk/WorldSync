use std::path::{Path, PathBuf};
use std::process::Command;

mod download;

#[derive(Debug, PartialEq)]
pub struct Version(semver::Version);

#[derive(thiserror::Error, Debug)]
pub enum VersionError {
    #[error("no quoted bit to parse for semver in java --version output")]
    NoQuoted,
    #[error("Could not parse quoted version segment: {0}")]
    Parse(semver::Error),
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self(semver::Version::new(major, minor, patch))
    }
}

impl TryFrom<&str> for Version {
    type Error = VersionError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let version = s.split('"').nth(1).ok_or(Self::Error::NoQuoted)?;
        let version = semver::Version::parse(version).map_err(Self::Error::Parse)?;
        Ok(Version(version))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not a java executable at the java bin path")]
    NotJava,
    #[error("error while running java to determine its version: {0}")]
    RunError(String),
    #[error("no file at the java bin path")]
    NotFile,
    #[error("local java path is not a dir")]
    NotDir,
    #[error("java executable could not be run: {0:?}")]
    CouldNotRun(std::io::ErrorKind),
    #[error("error while downloading fresh java install: {0}")]
    Download(#[from] download::Error),
    #[error("could not parse version: {0}")]
    ParseVersion(#[from] VersionError),
}

#[cfg(target_os = "windows")]
fn java_bin(dir: &Path) -> PathBuf {
    dir.join("bin/java.exe")
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn java_bin(dir: &Path) -> PathBuf {
    dir.join("bin/java")
}

use download::Error as DlError;
use download::Progress;
use futures::TryStream;
#[cfg(target_os = "linux")]
pub async fn download_stream(
    dir: PathBuf,
) -> Result<impl TryStream<Ok = Progress, Error = DlError>, Error> {
    let url = download::build_url("linux", "tar.gz");
    Ok(download::unpack_stream(dir, url, download::unpack_tar_gz).await?)
}
#[cfg(target_os = "macos")]
pub async fn download_stream(
    dir: PathBuf,
) -> Result<impl TryStream<Ok = Progress, Error = DlError>, Error> {
    let url = download::build_url("macos", "tar.gz");
    Ok(download::unpack_stream(dir, url, download::unpack_tar_gz).await?)
}
#[cfg(target_os = "windows")]
pub async fn download_stream(
    dir: PathBuf,
) -> Result<impl TryStream<Ok = Progress, Error = DlError>, Error> {
    let url = download::build_url("windows", "zip");
    Ok(download::unpack_stream(dir, url, download::unpack_zip).await?)
}

// TODO improve so this still works when jdk version changes
fn jdk_dir(dir: &Path) -> PathBuf {
    dir.join("jdk-17.0.1")
}

/// given a path in which the local java install should exist
/// return its version.
pub fn version(dir: impl AsRef<Path>) -> Result<Version, Error> {
    let dir = dir.as_ref();
    dir.is_dir().then(|| ()).ok_or(Error::NotDir)?;
    let dir = jdk_dir(dir);
    let java_bin = java_bin(&dir);
    java_bin.is_file().then(|| ()).ok_or(Error::NotFile)?;

    let output = Command::new(java_bin)
        .arg("-version")
        .output()
        .map_err(|e| e.kind())
        .map_err(Error::CouldNotRun)?;
    output.status.success().then(|| ()).ok_or(Error::NotJava)?;
    let output = String::from_utf8(output.stderr).expect("cmd output is not valid utf8");
    Ok(Version::try_from(output.as_str())?)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn path_does_not_exist() {
        let res = version("non/existing/path");
        if let Err(Error::NotDir) = res {
            return;
        }
        panic!("result should be err notdir, is: {:?}", res);
    }

    #[test]
    fn parse_version() {
        let input = "java version \"17.0.1\" 2021-10-19 LTS\nJava(TM) SE Runtime Environment (build 1
7.0.1+12-LTS-39)\nJava HotSpot(TM) 64-Bit Server VM (build 17.0.1+12-LTS-39, mixed mode, sharing)\n";
        let res = Version::try_from(input).unwrap();
        assert_eq!(res, Version::new(17, 0, 1));
    }
}
