use iced::Command;
use tracing::{info, instrument};
use wrapper::Line;

use crate::gui::{hosting, RpcConn};

use super::{elapsed, Msg, Page};

#[instrument(err)]
pub async fn notify_conn_lost(
    mut handle: wrapper::Handle,
    last_save: Option<std::time::Instant>,
) -> Result<(), wrapper::HandleError> {
    let msg = match last_save {
        Some(instant) => format!(
            "Host lost connection to WorldSync, any work since the last save {} will not be saved",
            elapsed(instant)
        ),
        None => "Host lost connection to WorldSync, no save has been made yet, all progress since the host started is lost".into(),
    };
    handle.say(msg).await
}

#[instrument(err)]
pub async fn save(mut handle: wrapper::Handle) -> Result<(), wrapper::HandleError> {
    handle.save().await
}

impl Page {
    pub fn notify_conn_lost(&mut self) -> Command<Msg> {
        info!("saving world on schedual");
        let last_save = self.last_save;
        let handle = self.mc_handle.clone();
        Command::perform(notify_conn_lost(handle, last_save), |e| match e {
            Ok(_) => Msg::Empty,
            Err(e) => Msg::HostingPage(hosting::Event::Error(e.into())),
        })
    }

    pub fn save_world(&mut self) -> Command<Msg> {
        info!("saving world on schedual");
        let handle = self.mc_handle.clone();
        Command::perform(save(handle), |e| match e {
            Ok(_) => Msg::Empty,
            Err(e) => Msg::HostingPage(hosting::Event::Error(e.into())),
        })
    }

    pub fn handle_server_line(&mut self, line: Line, rpc: RpcConn) -> Command<Msg> {
        match line {
            Line {
                msg: wrapper::Message::Saved,
                ..
            } => {
                self.uploading_sub.start();
                Command::none()
            }
            _ => super::mc::send_line(line, rpc, self.host_id),
        }
    }
}
