use std::sync::{Arc, RwLock};

use server::Sessions;
use server::{World, db::user::UserDb};
use tokio::sync::broadcast;
use typed_sled::sled;
use structopt::StructOpt;
mod admin_ui;
mod host;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "8080")]
    port: u16,
    #[structopt(long)]
    admin_ui: bool
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    shared::setup_tracing();

    if opt.admin_ui {
        admin_ui::run(opt.port).await;
        return 
    }

    let db = sled::open("db").unwrap();
    let sessions = Sessions::default();
    let user_db = UserDb::from(db.clone());
    let events = server::events_channel();
    let world = World::from(db, events).await;
    let host_state = Arc::new(RwLock::new(host::Host));

    host::monitor(host_state).await;

    server::host(sessions, user_db, world, opt.port, events).await;
}
