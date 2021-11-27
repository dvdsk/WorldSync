use crate::gui::RpcConn;
use protocol::tarpc;
pub use protocol::ServiceClient;
pub use tarpc::context;

use super::{Error, Event, Msg, Page};
use iced::Command;

async fn request_to_host(rpc: RpcConn) -> Result<(), Error> {
    rpc.client
        .request_to_host(context::current(), rpc.session)
        .await
        .map_err(|_| Error::NoMetaConn)?
        .map_err(|e| e.into())
}

impl Page {
    pub fn request_to_host(&self, rpc: &mut Option<RpcConn>) -> Command<Msg> {
        let rpc = rpc
            .as_mut()
            .expect("rpc connection should exist on host page")
            .clone();
        let task = request_to_host(rpc);

        Command::perform(task, move |res| match res {
            // if we became host we will get the msg via 
            // the server event subscription
            Ok(()) => Msg::None,
            Err(err) => Msg::HostPage(Event::Error(err)),
        })
    }
}
