use iced::Application;
use client::gui;

pub fn main() -> iced::Result {
    gui::State::run(iced::Settings::default())
}

