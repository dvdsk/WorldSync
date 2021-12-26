use server::Sessions;
use server::{World, db::user::UserDb};
use tokio::sync::mpsc;
use typed_sled::sled;
use structopt::StructOpt;
mod admin_ui;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "8080")]
    port: u16,
    #[structopt(long)]
    admin_ui: bool,
    #[structopt(long)]
    domain: String,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    shared::setup_tracing();
    println!("{}", protocol::current_version());

    if opt.admin_ui {
        admin_ui::run(opt.port).await;
        return 
    }

    let db = sled::open("db").unwrap();
    let sessions = Sessions::default();
    let user_db = UserDb::from(db.clone());
    let events = server::events_channel();
    let host_state = server::host::Host::new();
    let world = World::from(db, host_state.clone()).await;

    let (host_req, host_req_recv) = mpsc::channel(100);
    let events_clone = events.clone();
    tokio::spawn(async move {
        server::host::monitor(host_state, events_clone, host_req_recv).await;
    });

    server::host(sessions, user_db, world, opt.port, opt.domain, events, host_req).await;
}
