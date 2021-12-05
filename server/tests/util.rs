use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use protocol::ServiceClient;
use shared::tarpc;
use shared::tarpc::client::Config;
use shared::tarpc::tokio_serde::formats::Json;
use tokio::net::TcpStream;
use tokio::time::sleep;

static FREE_PORT: AtomicU16 = AtomicU16::new(34879);
pub fn free_port() -> u16 {
    FREE_PORT.fetch_add(1, Ordering::Relaxed)
}

async fn connect_tcp(_domain: &str, port: u16) -> Result<TcpStream, std::io::Error> {
    TcpStream::connect(format!("127.0.0.1:{}", port)).await
}

pub async fn connect(domain: &str, port: u16) -> Result<ServiceClient, std::io::Error> {
    let stream = connect_tcp(domain, port).await?;
    let transport = tarpc::serde_transport::Transport::from((stream, Json::default()));
    let client = ServiceClient::new(Config::default(), transport).spawn();
    Ok(client)
}

pub async fn test_server(port: u16) {
    use server::db::user::UserDb;

    let db = server::db::test_db();
    let world = server::World::from(db.clone()).await;
    let mut userdb = UserDb::from(db);

    use protocol::User;
    for i in 0..10 {
    userdb
        .add_user(User::test_user(i), User::test_password(i))
        .await
        .expect("could not add test users");
    }

    let events = server::events_channel();
    let sessions = server::Sessions::default();
    server::host(sessions, userdb, world, port, events).await;
}

pub async fn test_conn(port: u16) -> protocol::ServiceClient {
    loop {
        sleep(Duration::from_millis(50)).await;
        match connect("127.0.0.1", port).await {
            Ok(client) => break client,
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => continue,
            Err(e) => panic!("could not connect to server: {:?}", e),
        }
    }
}
