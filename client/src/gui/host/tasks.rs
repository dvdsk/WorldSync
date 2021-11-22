use crate::gui::RpcConn;
pub use protocol::ServiceClient;
use protocol::tarpc;
pub use tarpc::context;

use super::{Error, Event, Msg, Page};
use iced::Command;

async fn request_to_host(rpc: RpcConn) -> Result<bool, Error> {
    let became_host = rpc.client
        .request_to_host(context::current(), rpc.session)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    Ok(became_host)
}

impl Page {
    pub fn request_to_host(&self, rpc: &mut Option<RpcConn>) -> Command<Msg> {
        let rpc = rpc
            .as_mut()
            .expect("rpc connection should exist on host page")
            .clone();
        let task = request_to_host(rpc);

        Command::perform(task, move |res| match res {
            Ok(true) => Msg::HostAssigned,
            Ok(false) => Msg::None, // update subscription will update state soon enough
            Err(err) => Msg::HostPage(Event::Error(err)),
        })
    }
}
