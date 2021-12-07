use std::sync::Arc;

use iced::{Column, Command, Element, HorizontalAlignment, Length, Row, Space, Text};
pub use crate::Event as Msg;
use super::RpcConn;
use super::parts::{ClearError, ErrorBar};

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Ran into problem running minecraft server: {0}")]
    Mc(wrapper::Error),
    #[error("No longer the host")]
    NotHost,
}

impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        unimplemented!("should not run into {:?} on login page", e)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    ClearError(Error),
    Handle(Arc<wrapper::Handle>),
    Mc(wrapper::parser::Line),
    Error(Error),
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
    server: Option<wrapper::Handle>,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, event: Event, rpc: RpcConn) -> Command<Msg> {
        match event {
            Event::Error(e) => self.errorbar.add(e),
            Event::ClearError(e) => self.errorbar.clear(e),
            Event::Handle(h) => {
                let h = Arc::try_unwrap(h).expect("could not get ownership over server handle");
                self.server = Some(h);
            }
            Event::Mc(line) => return Self::send_line(line, rpc),
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .push(top_spacer)
            .push(title());

        let ui = Row::new()
            .push(left_spacer)
            .push(center_column)
            .push(sidebar);

        let errorbar = self.errorbar.view().map(move |e| Msg::HostingPage(e));
        Column::new()
            .width(Length::Fill)
            .push(errorbar)
            .push(ui)
            .into()
    }
}

fn title() -> Text {
    Text::new("Hosting")
        .width(Length::FillPortion(1))
        .horizontal_alignment(HorizontalAlignment::Center)
}
