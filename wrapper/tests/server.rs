use std::path::Path;

use wrapper::parser::{Line, Message};
use wrapper::{Config, Error, Instance};

#[tokio::test]
async fn fail_to_start() {
    shared::setup_test_tracing();
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

async fn await_loaded(instance: &mut Instance) {
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
async fn start_fresh_server() {
    shared::setup_test_tracing();

    let server_path = Path::new("tests/data/start_fresh");
    setup_server(server_path, 12387).await;

    let (mut instance, _handle) = Instance::start(server_path, 1).await.unwrap();
    await_loaded(&mut instance).await;
}

#[tokio::test]
#[ignore] // Ignore unless specifically requested (use -- --incude-ignored)
async fn saving() {
    shared::setup_test_tracing();

    let server_path = Path::new("tests/data/saving");
    setup_server(server_path, 34867).await;

    let (mut instance, mut handle) = Instance::start(server_path, 1).await.unwrap();
    await_loaded(&mut instance).await;

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
            Ok(e) => {dbg!(e); continue;}
            Err(e) => panic!("error after server save: {:?}", e),
        }
    }
}

fn random_string(length: usize) -> String {
    use rand::{distributions::Alphanumeric, Rng};
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[tokio::test]
#[ignore] // Ignore unless specifically requested (use -- --incude-ignored)
async fn say() {
    shared::setup_test_tracing();

    let server_path = Path::new("tests/data/say");
    setup_server(server_path, 34867).await;

    let (mut instance, mut handle) = Instance::start(server_path, 1).await.unwrap();
    await_loaded(&mut instance).await;

    let mut test_cases: Vec<_> = (1..2000)
        .into_iter()
        .step_by(10)
        .map(|len| random_string(len))
        .collect();
    test_cases.push(String::from("💖")); // unicode fun;
    test_cases.push(String::from("Οὐχὶ ταὐτὰ")); // some greek fun;

    for test_string in test_cases {
        handle
            .say(&test_string)
            .await
            .expect("could not instruct server to say message");
        let line = instance.next_event().await.unwrap();
        let correct = Message::Chat {
            from: "Server".into(),
            msg: test_string,
        };
        assert_eq!(line.msg, correct);
    }
}
