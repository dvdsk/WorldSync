mod util;
use client::context;
use protocol::User;
use util::{free_port, test_conn, test_server};

async fn version(port: u16) -> protocol::Version {
    let client = test_conn(port).await;
    client.version(context::current()).await.unwrap()
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

async fn update(port: u16) {
    let client = test_conn(port).await;
    let session = client
        .log_in(context::current(), User::test_credentials(0))
        .await
        .expect("rpc failure")
        .unwrap();
    let updated = User {
        username: "updated user".to_string(),
        ..User::test_user(0)
    };
    client
        .update_account(context::current(), session, updated.clone())
        .await
        .expect("rpc failure")
        .unwrap();
    let curr = client
        .get_account(context::current(), session)
        .await
        .expect("rpc failure")
        .unwrap();
    assert_eq!(curr, updated);
}

#[tokio::test]
async fn update_user() {
    let port = free_port();
    let server = test_server(port);

    tokio::select! {
        _ = server => panic!("server crashed during client test"),
        r = update(port) => r,
    };
}
