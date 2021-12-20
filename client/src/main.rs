use client::gui;
use iced::Application;
#[cfg(not(feature = "deployed"))]
use tracing::warn;

pub fn main() -> iced::Result {
    shared::setup_tracing();
    #[cfg(not(feature = "deployed"))]
    warn!("Running without deployed feature, can not connect to deployed servers");

    println!("{}", protocol::current_version());

    gui::State::run(iced::Settings::default())
}
