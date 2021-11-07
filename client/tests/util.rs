use std::sync::atomic::{AtomicU16, Ordering};

static FREE_PORT: AtomicU16 = AtomicU16::new(8080);
pub fn free_port() -> u16 {
    dbg!(FREE_PORT.fetch_add(1, Ordering::Relaxed))
}

pub async fn test_server(port: u16) {
    use server::db::user::UserDb;
    let db = server::db::test_db();
    let sessions = server::Sessions::new();
    let mut userdb = UserDb::open(&db);

    use server::db::user::User;
    let test_user = User {
        username: "existing user".to_owned(),
    };
    userdb.store(test_user, "5678".to_owned()).await;
    let server = server::host(sessions, userdb, port);
    server.await
}
