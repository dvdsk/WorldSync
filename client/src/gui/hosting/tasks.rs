use iced::Command;
use tracing::{info, instrument};
use wrapper::Line;

use crate::gui::{hosting, RpcConn};

use super::{Msg, Page};

#[instrument(err)]
pub async fn save(mut handle: wrapper::Handle) -> Result<(), wrapper::HandleError> {
    handle.save().await
}

impl Page {
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
