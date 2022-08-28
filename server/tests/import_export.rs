use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

mod util;
use shared::Platform;
use shared::tarpc::context;
use util::{free_port, spawn_test_server, test_conn};

fn dir() -> &'static Path {
    Path::new("test_data/import_export")
}

fn clear_dir() {
    match fs::remove_dir_all(dir()) {
        Ok(_) => (),
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(e) => panic!("{e}"),
    }
}

fn set_start_condition() {
    clear_dir();
    fs::create_dir_all(dir().join("data/world")).unwrap();
    fs::write(dir().join("data/world/01.mca"), "minecraft world data").unwrap();

    for p in ["windows", "linux"] {
        let subdir = dir().join("platform").join(p);
        fs::create_dir_all(subdir.join("java")).unwrap();
        fs::write(subdir.join("minecraft.jar"), "lorem ipsum").unwrap();
        fs::write(subdir.join("java.bin"), format!("java bin for: {p}")).unwrap();
    }
}

fn verify_end_conditions() {
    let mut expected: HashMap<PathBuf, &'static str> = [
        (dir().join("minecraft.jar"), "lorem ipsum"),
        (dir().join("java.bin"), "java bin for: linux"),
        (dir().join("world/01.mca"), "minecraft world data"),
    ]
    .into_iter()
    .collect();

    for entry in walkdir::WalkDir::new(dir()) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let correct = expected
            .remove(path)
            .expect(&format!("unexpected file in dir: {path:?}"));
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(&content, correct);
    }

    assert!(
        expected.len() == 0,
        "not all files found, missing: {expected:?}"
    )
}

#[tokio::test]
async fn main() {
    let port = free_port();
    spawn_test_server(port).await;

    set_start_condition();
    let client = test_conn(port).await;
    client
        .set_save(context::current(), dir().into())
        .await
        .unwrap()
        .unwrap();

    clear_dir();
    fs::create_dir(dir()).unwrap();

    client
        .dump_server(context::current(), dir().into(), Platform::Linux)
        .await
        .unwrap()
        .unwrap();

    verify_end_conditions();
}
