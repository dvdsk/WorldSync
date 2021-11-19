use crate::gui::{RpcConn, login, host};
use protocol::{Host, ServiceClient, Uuid};

#[derive(Debug, Clone)]
pub enum Event {
    LoggedIn(RpcConn, Option<Host>),
    HostPage(host::Event),
    LoginPage(login::Event),
    Error,
}
