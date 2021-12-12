use std::hash::Hash;

use crate::gui::parts::ClearError;
pub use crate::Event as Msg;
use crate::{world_dl, mc};
use protocol::HostId;
use iced::{Align, Button, Column, Command, Element, HorizontalAlignment, Length, Row, Space, Text, button};

use super::RpcConn;
use super::parts::{ErrorBar, Loading};

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Could not connect to WorldSync server")]
    NoMetaConn,
    #[error("Error downloading world: {0}")]
    Sync(#[from] world_dl::Error),
    #[error("Could not start minecraft server: {0}")]
    ServerStart(#[from] wrapper::Error),
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
    DlStarting{num_obj: usize},
    Loading(u8),
    WorldUpdated,
    Mc(Result<wrapper::parser::Line,wrapper::Error>),
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
    host_id: Option<HostId>,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, event: Event, rpc: RpcConn) -> Command<Msg> {
        match event {
            Event::Error(e) => self.errorbar.add(e),
            Event::ClearError(e) => self.errorbar.clear(e),
            Event::WantToHost => return self.request_to_host(rpc),
            Event::ObjToSync{left} => self.downloading.set_progress(left as f32),
            Event::DlStarting{num_obj} => self.downloading.start(num_obj as f32, 0.0),
            Event::WorldUpdated => {
                self.downloading.finished();
                self.loading_server.start(100.0, 0.0);
            }
            Event::Loading(p) => self.loading_server.set_progress(p as f32),
            Event::Mc(event) => match event {
                Ok(line) => return mc::send_line(line, rpc),
                Err(e) => self.errorbar.add(e.into()),
            }
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let bottom_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .align_items(Align::Center)
            .width(Length::FillPortion(8))
            .push(top_spacer)
            .push(title())
            .push(host_button(&mut self.host))
            .push(self.downloading.view())
            .push(self.loading_server.view())
            .push(bottom_spacer);

        let ui = Row::new()
            .push(left_spacer)
            .push(center_column)
            .push(sidebar);

        let errorbar = self.errorbar.view().map(Msg::HostPage);
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
    Button::new(state, Text::new("Host").horizontal_alignment(HorizontalAlignment::Center))
        .on_press(Msg::HostPage(Event::WantToHost))
}
