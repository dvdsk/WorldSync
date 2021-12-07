use crate::gui::RpcConn;

use super::{Msg, Page};
use iced::Command;
use shared::tarpc::context::Context;
use wrapper::parser::Line;

async fn send(line: Line, rpc: RpcConn) -> Msg {
    use crate::gui::hosting::Event as hEvent;
    use crate::gui::hosting::Error as hError;
    let res = rpc
        .client
        .pub_mc_line(Context::current(), rpc.session, line)
        .await
        .expect("rpc failure");
    match res {
        Ok(_) => Msg::Empty,
        Err(protocol::Error::NotHost) => Msg::HostingPage(hEvent::Error(hError::NotHost)),
        e_ => panic!("unexpected error: {:?}", e_),
    }
}

impl Page {
    pub fn send_line(line: Line, rpc: RpcConn) -> Command<Msg> {
        let send = send(line, rpc);
        Command::perform(send, |msg| msg)
    }
}
