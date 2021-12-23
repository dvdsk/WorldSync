use protocol::{Error, SessionId, User};
use shared::tarpc::context;

use protocol::Uuid;

mod util;
use util::{free_port, spawn_test_server, test_conn};

async fn try_log_in(
    user: impl Into<String>,
    pass: impl Into<String>,
    port: u16,
) -> Result<Uuid, Error> {
    let client = test_conn(port).await;
    client
        .log_in(context::current(), user.into(), pass.into())
        .await
        .unwrap()
}

#[tokio::test]
async fn wrong_user_and_pass() {
    let port = free_port();
    spawn_test_server(port).await;
    let result = try_log_in("wrong user", "wrong pass", port).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn correct_user_and_pass() {
    let port = free_port();
    spawn_test_server(port).await;
    let username = User::test_username(0);
    let password = User::test_password(0);
    let result = try_log_in(username, password, port).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn correct_user_wrong_pass() {
    let port = free_port();
    spawn_test_server(port).await;
    let username = User::test_username(0);
    let result = try_log_in(username, "wrong pass", port).await;
    assert!(result.is_err());
}

async fn log_in_and_change_pass(client: &protocol::ServiceClient) -> SessionId {
    let session_id = client
        .log_in(
            context::current(),
            User::test_username(0),
            User::test_password(0),
        )
        .await
        .expect("rpc failure")
        .unwrap();
    client
        .update_password(
            context::current(),
            session_id,
            "changed_password".to_owned(),
        )
        .await
        .expect("rpc failure")
        .unwrap();
    session_id
}

async fn test_change_password(port: u16) {
    let client = test_conn(port).await;
    log_in_and_change_pass(&client).await;
    client
        .log_in(
            context::current(),
            User::test_username(0),
            "changed_password".to_owned(),
        )
        .await
        .expect("rpc failure")
        .unwrap();
}

#[tokio::test]
async fn change_password() {
    let port = free_port();
    spawn_test_server(port).await;
    test_change_password(port).await;
}

async fn test_session_expired_after_pass_change(port: u16) {
    let client = test_conn(port).await;
    let session_id = log_in_and_change_pass(&client).await;
    let res = client
        .get_account(context::current(), session_id)
        .await
        .expect("rpc failure");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), Error::SessionExpired);
}

#[tokio::test]
async fn session_expired_after_pass_change() {
    let port = free_port();
    spawn_test_server(port).await;
    test_session_expired_after_pass_change(port).await;
}
