use std::thread;

use client::gui;
use iced::Application;
use server::util::spawn_test_server;

fn main() {
    shared::setup_test_tracing();

    thread::spawn(|| {
        use tokio::runtime::Runtime;
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            spawn_test_server(8080).await;
        });
    });

    gui::State::run(iced::Settings::default()).unwrap()
}
