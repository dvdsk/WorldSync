use server::Sessions;
use server::{World, db::user::UserDb};
use typed_sled::sled;
use structopt::StructOpt;
mod admin_ui;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "8080")]
    port: u16,
    #[structopt(long)]
    admin_ui: bool
}

fn setup_tracing() {
    use tracing_subscriber::{filter, prelude::*};

    let filter_modules = filter::filter_fn(|metadata| {
        // levels are orderd ERROR < WARN < INFO < DEBUG < TRACE
        if metadata.level() < &tracing::Level::INFO {
            return true; // return true to report this span
        }

        let is_ignored = |m: &str| {
            m.contains("tarp") 
            || m.contains("wgpu")
            || m.contains("naga")
            || m.contains("gfx_backend")
            || m.contains("winit::platform_impl")
        };

        match metadata.module_path() {
            Some(module) if is_ignored(module) => false,
            _ => true,
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

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    setup_tracing();

    if opt.admin_ui {
        admin_ui::run(opt.port).await;
        return 
    }

    let db = sled::open("db").unwrap();
    let sessions = Sessions::default();
    let user_db = UserDb::from(db.clone());
    let world = World::from(db).await;
    let events = server::events_channel();
    server::host(sessions, user_db, world, opt.port, events).await;
}
