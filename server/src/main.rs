use server::{Sessions, db::user::UserDb};
use typed_sled::sled;

#[tokio::main]
async fn main() {
    let db = sled::open("db").unwrap();
    let sessions = Sessions::new();
    let user_db = UserDb::open(&db);
    server::host(sessions, user_db, 8080).await;
}
