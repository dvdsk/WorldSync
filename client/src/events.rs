use crate::gui::login;
use protocol::{Uuid, WorldClient};

#[derive(Debug, Clone)]
pub enum Event {
    LoggedIn(WorldClient, Uuid),
    LoginPage(login::Event),
    Error,
}
