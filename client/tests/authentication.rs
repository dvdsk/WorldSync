use std::time::Duration;

use client::context;
use protocol::{Credentials, Uuid};
use tokio::time::sleep;

mod util;
use util::{free_port, test_server};

async fn try_log_in(user: &str, pass: &str, port: u16) -> Result<Uuid, ()> {
    sleep(Duration::from_millis(700)).await;
    let client = client::connect(port).await;
    let credentials = Credentials {
        username: user.to_owned(),
        password: pass.to_owned(),
    };
    client
        .log_in(context::current(), credentials)
        .await
        .unwrap()
}


#[tokio::test]
async fn wrong_user_and_pass() {
    let port = free_port();
    let server = test_server(port);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = try_log_in("user", "1234", port) => r,
    };

    assert!(result.is_err());
}

#[tokio::test]
async fn correct_user_and_pass() {
    let port = free_port();
    let server = test_server(port);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = try_log_in("existing user", "5678", port) => r,
    };
    assert!(result.is_ok());
}

#[tokio::test]
async fn correct_user_wrong_pass() {
    let port = free_port();
    let server = test_server(port);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = try_log_in("existing user", "56789", port) => r,
    };
    assert!(result.is_err());
}
