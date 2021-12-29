use std::thread;

use client::gui;
use iced::Application;
use server::util::spawn_test_server;
use tokio::sync::Mutex;
use std::sync::{Arc, Barrier};

fn main() {
    shared::setup_test_tracing();
    let server_up = Arc::new(Barrier::new(2));

    let ui_closed = Arc::new(Mutex::new(()));
    let ui_guard = ui_closed.blocking_lock();

    let server_up_clone = server_up.clone();
    let ui_closed_clone = ui_closed.clone();
    thread::spawn(move || {
        use tokio::runtime::Runtime;
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            spawn_test_server(8080).await;
            server_up_clone.wait();
            ui_closed_clone.lock().await;
        });
    });

    server_up.wait();
    gui::State::run(iced::Settings::default()).unwrap();
    drop(ui_guard);
}
