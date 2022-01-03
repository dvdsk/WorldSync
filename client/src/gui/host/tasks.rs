use crate::gui::RpcConn;
pub use protocol::{HostId, ServiceClient};
use shared::tarpc;
pub use tarpc::context;
use tracing::instrument;

use super::{Error, Event, Msg, Page};
use iced::Command;

#[instrument(err)]
async fn request_to_host(rpc: RpcConn, host_id: HostId) -> Result<(), Error> {
    rpc.client
        .request_to_host(context::current(), rpc.session, host_id)
        .await?
        .map_err(|e| e.into())
}

impl Page {
    pub fn request_to_host(&mut self) -> Command<Msg> {
        let host_id = HostId::new_v4();
        self.host_id = Some(host_id);
        let task = request_to_host(self.rpc.clone(), host_id);

        Command::perform(task, move |res| match res {
            // if we became host we will get the msg via
            // the server event subscription
            Ok(_) => Msg::Empty,
            Err(err) => Msg::HostPage(Event::Error(err)),
        })
    }

    pub fn is_us(&self, host: &protocol::HostDetails) -> bool {
        self.host_id == Some(host.id)
    }
}
