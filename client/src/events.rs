use crate::gui::login;
use protocol::{Host, ServiceClient, Uuid};

#[derive(Debug, Clone)]
pub enum Event {
    LoggedIn(ServiceClient, Uuid, Option<Host>),
    LoginPage(login::Event),
    Error,
}
