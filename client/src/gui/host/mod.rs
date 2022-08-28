use std::hash::Hash;

use crate::gui::parts::ClearError;
use crate::mc;
pub use crate::Event as Msg;
use iced::{
    button, Button, Column, Command, Element, Length, Row, Space, Text,
};
use iced_native::{Alignment, alignment::Horizontal};
use protocol::HostId;
use shared::tarpc::client::RpcError;

use super::parts::{ErrorBar, Loading};
use super::tasks::SubStatus;
use super::{RpcConn, SubsList};

mod tasks;
use crate::world_download;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Lost connection to worldsync server: {0:?}")]
    NoMetaConn(#[from] RpcError),
    #[error("Error downloading world: {0}")]
    Sync(#[from] world_download::Error),
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
    ObjToSync { left: usize },
    DlStarting { num_obj: usize },
    Loading(u8),
    WorldUpdated,
    Mc(Result<wrapper::parser::Line, wrapper::Error>),
}

impl ClearError for Event {
    type Error = Error;
    fn clear(e: Error) -> Self {
        Self::ClearError(e)
    }
}

pub struct Page {
    errorbar: ErrorBar<Error>,
    host: button::State,
    java_progress: Loading,
    java_setup: SubStatus,
    download_progress: Loading,
    download: SubStatus,
    loading_server: Loading,
    rpc: RpcConn,
    pub host_id: Option<HostId>,
}

impl Page {
    pub fn from(rpc: RpcConn) -> Self {
        Self {
            errorbar: Default::default(),
            host: Default::default(),
            java_progress: Default::default(),
            java_setup: Default::default(),
            download_progress: Default::default(),
            download: Default::default(),
            loading_server: Default::default(),
            rpc,
            host_id: None,
        }
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        match event {
            Event::Error(e) => self.errorbar.add(e),
            Event::ClearError(e) => self.errorbar.clear(e),
            Event::WantToHost => return self.request_to_host(),
            Event::ObjToSync { left } => self.download_progress.set_progress(left as f32),
            Event::DlStarting { num_obj } => self.download_progress.start(num_obj as f32, 0.0),
            Event::WorldUpdated => {
                self.download_progress.finished();
                todo!("if java setup done");
                self.loading_server.start(100.0, 0.0);
            }
            Event::Loading(p) => self.loading_server.set_progress(p as f32),
            Event::Mc(event) => match event {
                Ok(line) => return mc::send_line(line, self.rpc.clone(), self.host_id.unwrap()),
                Err(e) => self.errorbar.add(e.into()),
            },
        }
        Command::none()
    }

    pub fn add_subs(&self, subs: &mut SubsList) {
        if let Some(id) = self.download.active() {
            let host_id = self.host_id;
            subs.push(world_download::sub(self.rpc.clone(), id))
        }
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let bottom_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .align_items(Alignment::Center)
            .width(Length::FillPortion(8))
            .push(top_spacer)
            .push(title())
            .push(host_button(&mut self.host))
            .push(self.java_progress.view())
            .push(self.download_progress.view())
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
        .horizontal_alignment(Horizontal::Center)
}

fn host_button(state: &mut button::State) -> Button<Msg> {
    Button::new(
        state,
        Text::new("Host").horizontal_alignment(Horizontal::Center),
    )
    .on_press(Msg::HostPage(Event::WantToHost))
}
