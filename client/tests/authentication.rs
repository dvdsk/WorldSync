use protocol::{Error, User};

use client::context;
use protocol::{Credentials, Uuid};

mod util;
use util::{free_port, test_conn, test_server};

async fn try_log_in(user: &str, pass: &str, port: u16) -> Result<Uuid, Error> {
    let client = test_conn(port).await;
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
        r = try_log_in("wrong user", "wrong pass", port) => r,
    };

    assert!(result.is_err());
}

#[tokio::test]
async fn correct_user_and_pass() {
    let port = free_port();
    let server = test_server(port);
    let correct = User::test_credentials(0);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = try_log_in(&correct.username, &correct.password, port) => r,
    };
    assert!(result.is_ok());
}

#[tokio::test]
async fn correct_user_wrong_pass() {
    let port = free_port();
    let server = test_server(port);
    let correct = User::test_credentials(0);
    let result = tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = try_log_in(&correct.username, "wrong pass", port) => r,
    };
    assert!(result.is_err());
}
