use client::gui;
use iced::Application;
#[cfg(not(feature = "deployed"))]
use tracing::warn;

pub fn main() -> iced::Result {
    shared::setup_tracing();
    #[cfg(not(feature = "deployed"))]
    warn!("Running without deployed feature, can not connect to deployed servers");

    println!("{}", protocol::current_version());

    let mut settings = iced::Settings::default();
    settings.window.size = (500, 400);
    gui::State::run(settings)
}
