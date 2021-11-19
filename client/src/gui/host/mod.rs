use iced::{Column, Command, Element, Length, Text};
pub use crate::Event as Msg;
use crate::gui::style;
use crate::gui::parts::ClearError;

use super::parts::ErrorBar;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
}

impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        todo!();
        unimplemented!("should not run into {:?} on login page", e)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    ClearError(Error),
}

impl ClearError for Event {
    type Error = Error;
    fn clear(e: Error) -> Self {
        Self::ClearError(e)
    }
}

#[derive(Default)]
pub struct Page {
    errorbar: ErrorBar<Error>,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        match dbg!(event) {
            Event::ClearError(e) => self.errorbar.clear(e),
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let main_ui = Text::new("unimplemented");
        let errorbar = self.errorbar.view().map(move |e| Msg::HostPage(e));
        Column::new()
            .width(Length::Fill)
            .push(errorbar)
            .push(main_ui)
            .into()
    }
}
