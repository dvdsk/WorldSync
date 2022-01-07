use std::path::{Path, PathBuf};
use std::process::Command;

mod download;

#[derive(Debug, PartialEq)]
pub struct Version;

impl TryFrom<String> for Version {
    type Error = Error;
    fn try_from(output: String) -> Result<Self, Self::Error> {
        dbg!(output);
        todo!()
    }
}

#[derive(Debug, PartialEq)]
pub enum Error {
    NotJava,
    RunError(String),
    NotFile,
    NotDir,
    CouldNotRun(std::io::ErrorKind),
}

#[cfg(target_os = "windows")]
fn java_bin(dir: &Path) -> PathBuf {
    dir.join("bin/java.exe")
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn java_bin(dir: &Path) -> PathBuf {
    dir.join("bin/java")
}

/// given a path in which the local java install should exist
/// return its version.
pub fn local_version(dir: impl AsRef<Path>) -> Result<Version, Error> {
    let dir = dir.as_ref();
    dir.is_dir().then(|| ()).ok_or(Error::NotDir)?;
    let java_bin = java_bin(dir);
    java_bin.is_file().then(|| ()).ok_or(Error::NotFile)?;

    let output = Command::new(java_bin)
        .arg("-version")
        .output()
        .map_err(|e| e.kind())
        .map_err(Error::CouldNotRun)?;
    output.status.success().then(|| ()).ok_or(Error::NotJava)?;
    if output.stderr.iter().count() > 0 {
        let err = String::from_utf8(output.stderr).expect("cmd output is not valid utf8");
        Err(Error::RunError(err))?;
    }
    let output = String::from_utf8(output.stdout).expect("cmd output is not valid utf8");
    Ok(Version::try_from(output)?)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn path_does_not_exist() {
        let res = local_version("non/existing/path");
        assert_eq!(res, Err(Error::NotDir));
    }

    // #[test]
    // fn download_fresh() {
    //     let 


    // }
}
