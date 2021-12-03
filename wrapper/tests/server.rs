use wrapper::parser::{Line, Message};
use wrapper::{Error, Instance};

#[tokio::test]
async fn fail_to_start() {
    std::fs::create_dir_all("tests/data/fail_to_start").unwrap();
    let (mut instance, _handle) = Instance::start("tests/data/fail_to_start", 2).await.unwrap();
    loop {
        match instance.next_event().await {
            Ok(_) => continue,
            Err(Error::NoJar(_)) => return,
            Err(e) => panic!("wrong error: {:?}", e),
        }
    }
}

#[tokio::test]
#[ignore] // Ignore unless specifically requested
async fn start_fresh_server() {
    use std::io::Cursor;

    const URL: &str = "https://launcher.mojang.com/v1/objects\
    /3cf24a8694aca6267883b17d934efacc5e44440d/server.jar";
    dbg!(URL);
    let response = reqwest::get(URL).await.unwrap();

    std::fs::create_dir_all("tests/data/start_fresh_server").unwrap();
    let mut file = std::fs::File::create("tests/data/start_fresh_server/server.jar").unwrap();
    let mut content =  Cursor::new(response.bytes().await.unwrap());
    std::io::copy(&mut content, &mut file).unwrap();

    let (mut instance, _handle) = Instance::start("tests/data/start_fresh_server", 2).await.unwrap();
    loop {
        match instance.next_event().await {
            Ok(Line{ msg: Message::DoneLoading(_), ..}) => return,
            Ok(_) => continue,
            Err(e) => panic!("error during server start: {:?}", e),
        }
    }
}
