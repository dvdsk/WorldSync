use std::fs::{DirBuilder, File};
use std::io::Write;
use std::path::PathBuf;

use sync::{DirContent, FileStatus};

#[tokio::test]
async fn empty_dir() {
    shared::setup_tracing();

    DirBuilder::new()
        .recursive(true)
        .create("test_data/empty_dir")
        .unwrap();

    let dir_status = DirContent::from_dir(PathBuf::from("test_data/empty_dir"))
        .await
        .unwrap();
    assert_eq!(dir_status, DirContent(Vec::new()))
}

#[tokio::test]
async fn dir_with_files() {
    shared::setup_tracing();

    DirBuilder::new()
        .recursive(true)
        .create("test_data/dir_with_files/subdir")
        .unwrap();

    let test_paths = [
        "test_data/dir_with_files/subdir/applesaus",
        "test_data/dir_with_files/foo.txt",
        "test_data/dir_with_files/world1_mca.mca",
    ];
    for path in test_paths {
        let mut file = File::create(path).unwrap();
        file.write_all(b"Hello, world!").unwrap();
    }

    let mut dir_status = DirContent::from_dir(PathBuf::from("test_data/dir_with_files"))
        .await
        .unwrap();

    let mut correct = DirContent(vec![
        FileStatus {
            path: PathBuf::from("subdir/applesaus"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("foo.txt"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("world1_mca.mca"),
            hash: 469007863229145464,
        },
    ]);
    correct.0.sort_by_key(|k| k.path.clone());
    dir_status.0.sort_by_key(|k| k.path.clone());
    assert_eq!(dir_status, correct)
}
