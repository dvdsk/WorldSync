use std::sync::Arc;

use super::parts::{ClearError, ErrorBar};
use super::RpcConn;
use crate::mc;
pub use crate::Event as Msg;
use iced::{Column, Command, Element, HorizontalAlignment, Length, Row, Space, Text};
use protocol::HostId;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Ran into problem running minecraft server: {0}")]
    Mc(#[from] wrapper::Error),
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
    Mc(Result<wrapper::parser::Line, wrapper::Error>),
    Error(Error),
}

impl ClearError for Event {
    type Error = Error;
    fn clear(e: Error) -> Self {
        Self::ClearError(e)
    }
}

pub struct Page {
    errorbar: ErrorBar<Error>,
    server: wrapper::Handle,
    host_id: HostId,
}

impl Page {
    pub fn from(server: Arc<wrapper::Handle>, host_id: HostId) -> Self {
        Self {
            errorbar: ErrorBar::default(),
            server: Arc::try_unwrap(server).expect("server handle should only have one reference"),
            host_id,
        }
    }

    pub fn update(&mut self, event: Event, rpc: RpcConn) -> Command<Msg> {
        match event {
            Event::Error(e) => self.errorbar.add(e),
            Event::ClearError(e) => self.errorbar.clear(e),
            Event::Mc(event) => match event {
                Ok(line) => return mc::send_line(line, rpc, self.host_id),
                Err(e) => self.errorbar.add(e.into()),
            },
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new().push(top_spacer).push(title());

        let ui = Row::new()
            .push(left_spacer)
            .push(center_column)
            .push(sidebar);

        let errorbar = self.errorbar.view().map(Msg::HostingPage);
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
