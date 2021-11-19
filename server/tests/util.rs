use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tokio::time::sleep;

static FREE_PORT: AtomicU16 = AtomicU16::new(8080);
pub fn free_port() -> u16 {
    FREE_PORT.fetch_add(1, Ordering::Relaxed)
}

fn setup_tracing() {
    use tracing_subscriber::{filter, prelude::*};

    let filter_modules = filter::filter_fn(|metadata| {
        if let Some(module) = metadata.module_path() {
            !module.contains("tarp")
        } else {
            true
        }
    });
    let fmt = tracing_subscriber::fmt::layer()
        .pretty()
        .with_test_writer();

    let _ignore_err = tracing_subscriber::registry()
        .with(fmt)
        .with(filter::LevelFilter::INFO)
        .with(filter_modules)
        .try_init();
}

pub async fn test_server(port: u16) {
    use server::db::user::UserDb;

    setup_tracing();
    let db = server::db::test_db();
    let sessions = server::Sessions::new();
    let mut userdb = UserDb::open(db);

    use protocol::User;
    for i in 0..10 {
        userdb
            .add_user(User::test_user(i), User::test_password(i))
            .await
            .expect("could not add test users");
    }

    let server = server::host(sessions, userdb, port);
    server.await
}

pub async fn test_conn(port: u16) -> protocol::ServiceClient {
    loop {
        sleep(Duration::from_millis(50)).await;
        match client::connect("127.0.0.1", port).await {
            Ok(client) => break client,
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => continue,
            Err(e) => panic!("could not connect to server: {:?}", e),
        }
    }
}
