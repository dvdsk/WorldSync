use client::gui;
use iced::Application;

#[cfg(not(feature = "deployed"))]
pub fn main() -> iced::Result {
    shared::setup_tracing();
    gui::State::run(iced::Settings::default())
}

#[cfg(feature = "deployed")]
pub fn main() -> iced::Result {
    gui::State::run(iced::Settings::default())
}
