use std::ops::RangeInclusive;

use crate::gui::parts::ClearError;
pub use crate::Event as Msg;
use crate::world_dl;
use iced::{Button, Column, Command, Element, HorizontalAlignment, Length, ProgressBar, Row, Space, Text, button};

use super::RpcConn;
use super::parts::{ErrorBar, Loading};

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Could not connect to WorldSync server")]
    NoMetaConn,
    #[error("Error downloading world: {0}")]
    Sync(#[from] world_dl::Error),
}

impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        unimplemented!("should not run into {:?} on login page", e)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Error(Error),
    ClearError(Error),
    WantToHost,
    ObjToSync{left: usize},
    DlStarting,
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
    host: button::State,
    downloading: Loading,
    loading_server: Loading,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, event: Event, rpc: &mut Option<RpcConn>) -> Command<Msg> {
        match dbg!(event) {
            Event::Error(e) => self.errorbar.add(e),
            Event::ClearError(e) => self.errorbar.clear(e),
            Event::WantToHost => return self.request_to_host(rpc),
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .push(top_spacer)
            .push(title())
            .push(host_button(&mut self.host))
            .push(self.downloading.view())
            .push(self.loading_server.view());

        let ui = Row::new()
            .push(left_spacer)
            .push(center_column)
            .push(sidebar);

        let errorbar = self.errorbar.view().map(move |e| Msg::HostPage(e));
        Column::new()
            .width(Length::Fill)
            .push(errorbar)
            .push(ui)
            .into()
    }
}

fn title() -> Text {
    Text::new("You will be hosting")
        .width(Length::FillPortion(1))
        .horizontal_alignment(HorizontalAlignment::Center)
}

fn host_button(state: &mut button::State) -> Button<Msg> {
    Button::new(state, Text::new("Login"))
        .on_press(Msg::StartHosting)
}
