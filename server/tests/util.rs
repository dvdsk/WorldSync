use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

static FREE_PORT: AtomicU16 = AtomicU16::new(34879);
pub fn free_port() -> u16 {
    FREE_PORT.fetch_add(1, Ordering::Relaxed)
}

pub async fn test_server(port: u16) {
    use server::db::user::UserDb;

    let db = server::db::test_db();
    let host_state = server::host::Host::new();
    let world = server::World::from(db.clone(), host_state.clone()).await;
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

    let (host_req, host_req_recv) = mpsc::channel(100);
    let monitor = server::host::monitor(host_state, events.clone(), host_req_recv);
    let host = server::host(sessions, userdb, world, port, events, host_req);
    tokio::spawn(async move {
        tokio::join!(monitor, host);
    }).await.unwrap();
}

pub async fn test_conn(port: u16) -> protocol::ServiceClient {
    loop {
        sleep(Duration::from_millis(50)).await;
        match protocol::connect("127.0.0.1", port).await {
            Ok(client) => break client,
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => continue,
            Err(e) => panic!("could not connect to server: {:?}", e),
        }
    }
}
