use std::path::Path;

use server::Sessions;
use server::{db::user::UserDb, World};
use structopt::StructOpt;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use typed_sled::sled;
mod admin_ui;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "8080")]
    port: u16,
    /// do not start a server but run the included admin control
    /// text ui tool
    #[structopt(long)]
    admin_ui: bool,
    /// domain that should be send to clients as host when the
    /// current host connects from the local network
    #[structopt(long)]
    domain: String,
    /// Verbosity of the logging, options: TRACE, DEBUG, INFO, WARN or ERROR
    #[structopt(name = "log", default_value = "INFO")]
    log_level: tracing::Level,
}

fn main() {
    let opt = Opt::from_args();
    let _log_guard =
        shared::setup_tracing(Path::new("logs"), "worldsync_server.log", opt.log_level);
    println!("{}", protocol::current_version());

    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .max_blocking_threads(20)
        .build()
        .unwrap();

    rt.block_on(async {
        if opt.admin_ui {
            admin_ui::run(opt.port).await;
            return;
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

        server::host(
            sessions, user_db, world, opt.port, opt.domain, events, host_req,
        )
        .await;
    });
}
