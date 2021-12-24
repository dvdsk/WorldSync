use super::parts::{ClearError, ErrorBar, Loading};
use super::tasks::SubStatus;
use super::{RpcConn, SubsList};
pub use crate::Event as Msg;
use crate::{mc, world_upload};
use iced::{Column, Command, Element, HorizontalAlignment, Length, Row, Space, Text};
use protocol::HostId;
use std::sync::Arc;
use std::time::Instant;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Ran into problem running minecraft server: {0}")]
    Mc(#[from] wrapper::Error),
    #[error("No longer the host")]
    NotHost,
    #[error(
        "Minecraft could not save the world, is folder read only or is there no more storage left?"
    )]
    McSaveErr(#[from] wrapper::HandleError),
    #[error("Could not upload save: {0}")]
    Upload(#[from] crate::world_upload::Error),
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
    PeriodicSave,
    Error(Error),
    UploadStarting(usize),
    Uploading(usize),
    UploadDone,
    SaveRegisterd,
}

impl ClearError for Event {
    type Error = Error;
    fn clear(e: Error) -> Self {
        Self::ClearError(e)
    }
}

pub struct Page {
    errorbar: ErrorBar<Error>,
    mc_handle: wrapper::Handle,
    pub host_id: HostId,
    uploading: Loading,
    last_save: Option<Instant>,
    uploading_sub: SubStatus,
    rpc: RpcConn,
}

impl Page {
    pub fn from(server: Arc<wrapper::Handle>, host_id: HostId, rpc: RpcConn) -> Self {
        Self {
            errorbar: ErrorBar::default(),
            mc_handle: Arc::try_unwrap(server)
                .expect("server handle should only have one reference"),
            host_id,
            uploading: Loading::default(),
            last_save: None,
            uploading_sub: SubStatus::default(),
            rpc,
        }
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        match event {
            Event::Error(e) => self.errorbar.add(e),
            Event::ClearError(e) => self.errorbar.clear(e),
            Event::PeriodicSave => {
                if self.uploading_sub.active().is_none() {
                    return self.save_world();
                }
            }
            Event::Mc(event) => match dbg!(event) {
                Ok(line) => return self.handle_server_line(line, self.rpc.clone()),
                Err(e) => self.errorbar.add(e.into()),
            },
            Event::UploadStarting(num_obj) => self.uploading.start(num_obj as f32, 0.0),
            Event::Uploading(p) => self.uploading.set_progress(p as f32),
            Event::UploadDone => self.uploading.finished(),
            Event::SaveRegisterd => {
                self.last_save = Some(Instant::now());
                self.uploading_sub.stop();
            }
        }
        Command::none()
    }

    pub fn add_subs(&self, subs: &mut SubsList) {
        if let Some(id) = self.uploading_sub.active() {
            let host_id = self.host_id;
            subs.push(world_upload::sub(self.rpc.clone(), id, host_id))
        }
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let bottom_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .push(top_spacer)
            .push(title())
            .push(last_save(self.last_save))
            .push(self.uploading.view())
            .push(bottom_spacer);

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

fn last_save(at: Option<Instant>) -> Text {
    let text = match at {
        Some(instant) => {
            let minutes = instant.elapsed() / 60;
            format!("last save {:?} minutes ago", minutes)
        }
        None => "no save made yet".to_string(),
    };
    Text::new(text)
        .width(Length::FillPortion(1))
        .horizontal_alignment(HorizontalAlignment::Center)
}
