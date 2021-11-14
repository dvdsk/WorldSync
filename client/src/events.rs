use crate::gui::login;

#[derive(Debug, Clone)]
pub enum Event {
    LoginEvent(login::Event),
}
