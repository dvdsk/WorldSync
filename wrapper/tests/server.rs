use std::path::Path;

use wrapper::parser::{Line, Message};
use wrapper::{Config, Error, Instance};

#[tokio::test]
async fn fail_to_start() {
    shared::setup_tracing();
    std::fs::create_dir_all("tests/data/fail_to_start").unwrap();
    std::fs::write("tests/data/fail_to_start/eula.txt", "eula=true").unwrap();
    let (mut instance, _handle) = Instance::start("tests/data/fail_to_start", 2)
        .await
        .unwrap();
    loop {
        match instance.next_event().await {
            Ok(_) => continue,
            Err(Error::NoJar(_)) => return,
            Err(e) => panic!("wrong error: {:?}", e),
        }
    }
}

async fn setup_server(dir: &Path, port: u16) {
    const URL: &str = "https://launcher.mojang.com/v1/objects\
    /3cf24a8694aca6267883b17d934efacc5e44440d/server.jar";
    dbg!(URL);
    let response = reqwest::get(URL).await.unwrap();

    tokio::fs::create_dir_all(dir).await.unwrap();
    let mut jar_path = dir.to_owned();
    jar_path.push("server.jar");
    let mut eula_path = dir.to_owned();
    eula_path.push("eula.txt");
    let bytes = response.bytes().await.unwrap();
    tokio::fs::write(jar_path, bytes).await.unwrap();
    tokio::fs::write(eula_path, "eula=true").await.unwrap();
    Config::default().with_port(port).write(dir).await.unwrap();
}

#[tokio::test]
#[ignore] // Ignore unless specifically requested (use -- --incude-ignored)
async fn start_fresh_server() {
    shared::setup_tracing();

    let server_path = Path::new("tests/data/start_fresh");
    setup_server(server_path, 12387).await;

    let (mut instance, _handle) = Instance::start(server_path, 1).await.unwrap();
    loop {
        match instance.next_event().await {
            Ok(Line {
                msg: Message::DoneLoading(_),
                ..
            }) => return,
            Ok(_) => continue,
            Err(e) => panic!("error during server start: {:?}", e),
        }
    }
}

#[tokio::test]
#[ignore] // Ignore unless specifically requested (use -- --incude-ignored)
async fn saving() {
    shared::setup_tracing();

    let server_path = Path::new("tests/data/saving");
    setup_server(server_path, 34867).await;

    let (mut instance, mut handle) = Instance::start(server_path, 1).await.unwrap();
    loop {
        match instance.next_event().await {
            Ok(Line {
                msg: Message::DoneLoading(_),
                ..
            }) => break,
            Ok(line) => {
                dbg!(line);
                continue;
            }
            Err(e) => panic!("error during server start: {:?}", e),
        }
    }

    handle
        .save()
        .await
        .expect("could not instruct server to save");
    loop {
        match dbg!(instance.next_event().await) {
            Ok(Line {
                msg: Message::Saved,
                ..
            }) => break,
            Ok(_) => continue,
            Err(e) => panic!("error after server save: {:?}", e),
        }
    }
}
