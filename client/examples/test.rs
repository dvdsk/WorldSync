use std::thread;

use client::gui;
use iced::Application;

fn setup_tracing() {
    use tracing_subscriber::{filter, prelude::*};

    let filter_modules = filter::filter_fn(|metadata| {
        if let Some(module) = metadata.module_path() {
            !module.contains("tarp") 
            && !module.contains("wgpu")
            && !module.contains("naga")
            && !module.contains("gfx_backend")
            && !module.contains("winit::platform_impl")
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

    let db = server::db::test_db();
    let world = server::World::from(db.clone()).await;
    let mut userdb = UserDb::from(db);

    use protocol::User;
    userdb
        .add_user(User::test_user(0), User::test_password(0))
        .await
        .expect("could not add test users");

    let events = server::events_channel();
    let sessions = server::Sessions::default();

    let send_hb = server::send_test_hb(events.clone());
    let server = server::host(sessions, userdb, world, port, events);
    tokio::join!(send_hb, server);
}

fn main() {
    setup_tracing();

    thread::spawn(||{
        use tokio::runtime::Runtime;
        let rt  = Runtime::new().unwrap();
        rt.block_on(async {
            test_server(8080).await;
        });
    });

    gui::State::run(iced::Settings::default()).unwrap()
}
