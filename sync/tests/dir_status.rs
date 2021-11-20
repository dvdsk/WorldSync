use std::fs::{DirBuilder, File};
use std::io::Write;
use std::path::PathBuf;

use sync::{DirStatus, FileStatus};

#[tokio::test]
async fn empty_dir() {
    DirBuilder::new()
        .recursive(true)
        .create("test_data/empty_dir")
        .unwrap();

    let dir_status = DirStatus::from_path(PathBuf::from("test_data/empty_dir"))
        .await
        .unwrap();
    assert_eq!(dir_status, DirStatus(Vec::new()))
}

#[tokio::test]
async fn dir_with_files() {
    DirBuilder::new()
        .recursive(true)
        .create("test_data/dir_with_files")
        .unwrap();

    let test_paths = [
        "test_data/dir_with_files/world1_mca.mca",
        "test_data/dir_with_files/foo.txt",
        "test_data/dir_with_files/applesaus",
    ];
    for path in test_paths {
        let mut file = File::create(path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
    }

    let dir_status = DirStatus::from_path(PathBuf::from("test_data/dir_with_files"))
        .await
        .unwrap();

    let correct = DirStatus(vec![
        FileStatus {
            path: PathBuf::from("test_data/dir_with_files/applesaus"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("test_data/dir_with_files/foo.txt"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("test_data/dir_with_files/world1_mca.mca"),
            hash: 469007863229145464,
        },
    ]);
    assert_eq!(dir_status, correct)
}
