use iced::Command;
use tracing::info;

use crate::gui::hosting;

use super::{Msg, Page};

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
}
