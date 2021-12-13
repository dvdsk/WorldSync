use client::gui;
use iced::Application;

pub fn main() -> iced::Result {
    shared::setup_tracing();
    gui::State::run(iced::Settings::default())
}
