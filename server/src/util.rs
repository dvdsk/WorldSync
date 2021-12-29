use std::time::Duration;

use tokio::sync::mpsc;

use super::{db, events_channel, host, Sessions, World};

pub async fn spawn_test_server(port: u16) {
    use crate::db::user::UserDb;

    let db = db::test_db();
    let domain = "".to_string();
    let host_state = host::Host::new();
    let world = World::from(db.clone(), host_state.clone()).await;
    let mut userdb = UserDb::from(db);

    use protocol::User;
    for i in 0..10 {
        match userdb
            .add_user(User::test_user(i), User::test_password(i))
            .await
        {
            Ok(_) => continue,
            Err(db::user::Error::AlreadyExists) => continue,
            Err(e) => panic!("{}", e),
        }
    }

    let events = events_channel();
    let sessions = Sessions::default();

    let (host_req, host_req_recv) = mpsc::channel(100);
    let monitor = host::monitor(host_state, events.clone(), host_req_recv);
    let host = host(sessions, userdb, world, port, domain, events, host_req);
    tokio::spawn(async move {
        tokio::join!(monitor, host);
    });
    // extra time to ensure server reachable by the time we exit this function
    tokio::time::sleep(Duration::from_millis(50)).await;
}
