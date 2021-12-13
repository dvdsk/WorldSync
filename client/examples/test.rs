use std::thread;

use client::gui;
use iced::Application;
use tokio::sync::mpsc;

pub async fn test_server(port: u16) {
    use server::db::user::UserDb;

    let db = server::db::test_db();
    let host_state = server::host::Host::new();
    let world = server::World::from(db.clone(), host_state.clone()).await;
    let mut userdb = UserDb::from(db);

    use protocol::User;
    userdb
        .add_user(User::test_user(0), User::test_password(0))
        .await
        .expect("could not add test users");

    let events = server::events_channel();
    let sessions = server::Sessions::default();

    let (host_req, host_req_recv) = mpsc::channel(100);
    let monitor = server::host::monitor(host_state, events.clone(), host_req_recv);
    let host = server::host(sessions, userdb, world, port, events, host_req);
    tokio::join!(monitor, host);
}

fn main() {
    shared::setup_tracing();

    thread::spawn(||{
        use tokio::runtime::Runtime;
        let rt  = Runtime::new().unwrap();
        rt.block_on(async {
            test_server(8080).await;
        });
    });

    gui::State::run(iced::Settings::default()).unwrap()
}
