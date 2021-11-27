use iced::{Command, Element, Text};
pub use crate::Event as Msg;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
}

impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        unimplemented!("should not run into {:?} on login page", e)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
}

#[derive(Default)]
pub struct Page {
    pub host: Option<protocol::Host>,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        match dbg!(event) {
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        Element::new(Text::new("unimplemented"))
    }
}
