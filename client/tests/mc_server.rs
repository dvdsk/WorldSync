use std::time::Duration;

use client::mc;
use tokio::time::sleep;

async fn control_server(mut handle: mc::server::Handle) {
    handle.start().await;
    dbg!();
    handle.save().await;
    dbg!();
    sleep(Duration::from_secs(2)).await;

    dbg!();
    handle.stop().await;
    dbg!();
}

#[tokio::test]
async fn start_and_save() {
    let (server, handle) =
        mc::server::Instance::new("tests/data/server.jar".into()).expect("could not create server");

    dbg!();
    let res = tokio::select! {
        _ = control_server(handle) => unreachable!(), 
        res = server.maintain() => res,
    };
    assert!(res.is_ok());
}
