mod util;
use client::context;
use util::{free_port, test_server};
use std::time::Duration;
use tokio::time::sleep;

async fn version(port: u16) -> protocol::Version{
    sleep(Duration::from_millis(700)).await;
    let client = client::connect(port).await;
    client
        .version(context::current())
        .await
        .unwrap()
}

#[tokio::test]
async fn version_matches() {
    let correct = protocol::Version {
        protocol: protocol::version().to_owned(),
        server: server::version().to_owned(),
    };

    let port = free_port();
    let server = test_server(port);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = version(port) => r,
    };
    assert_eq!(correct, result);
}
