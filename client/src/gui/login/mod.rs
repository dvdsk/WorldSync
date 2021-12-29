use crate::gui::parts::ErrorBar;
use crate::gui::style;
pub use crate::Event as Msg;
use iced::widget::Column;
use iced::{button, text_input, Button, Checkbox, Command, Length, Row, Space, TextInput};
use iced::{Align, Element, HorizontalAlignment, Text};
use shared::tarpc::client::RpcError;

use super::parts::ClearError;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Not a server, please check the address")]
    InvalidFormat,
    #[error("Port is not a number, please check the address")]
    NotANumber,
    #[error("Lost connection to worldsync server: {0:?}")]
    NoMetaConn(#[from] RpcError),
    #[error("Could not connect to worldsync server: {0:?}")]
    CouldNotConnect(std::io::ErrorKind),
    #[error("Invalid username or password")]
    IncorrectLogin,
    #[error("Version incorrect, try updating")]
    VersionMismatch {
        our: protocol::Version,
        server: protocol::Version,
    },
}

impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        if let protocol::Error::IncorrectLogin = e {
            Error::IncorrectLogin
        } else {
            panic!("should not run into {:?} on login page", e)
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Servername(String),
    Username(String),
    Password(String),
    RememberToggle(bool),
    Submit,
    Error(Error),
    ClearError(Error),
}

impl ClearError for Event {
    type Error = Error;
    fn clear(e: Error) -> Self {
        Self::ClearError(e)
    }
}

#[derive(Default)]
struct Input {
    button: text_input::State,
    value: String,
    style: style::Input,
}

impl Input {
    fn view<F: 'static + Fn(String) -> Event>(&mut self, event: F) -> TextInput<Event> {
        TextInput::new(&mut self.button, &self.value, &self.value, event).style(self.style)
    }
}

#[derive(Default)]
pub struct Inputs {
    server: Input,
    username: Input,
    password: Input,
}

impl Inputs {
    fn view(&mut self) -> Element<Event> {
        let texts = Column::new()
            .push(right_text("Server:"))
            .push(right_text("Username:"))
            .push(right_text("Password:"));
        let text_inputs = Column::new()
            .width(Length::FillPortion(1))
            .push(self.server.view(Event::Servername))
            .push(self.username.view(Event::Username))
            .push(
                self.password
                    .view(Event::Password)
                    .password()
                    .on_submit(Event::Submit),
            );
        Row::new()
            .push(texts)
            .push(Space::with_width(Length::FillPortion(1)))
            .push(text_inputs)
            .into()
    }
}

pub struct Page {
    db: sled::Db,
    inputs: Inputs,
    errorbar: ErrorBar<Error>,
    submit: button::State,
    remember: bool,
    logging_in: bool,
}

impl Page {
    pub fn new(db: sled::Db) -> Self {
        let inputs = Inputs::load(&db);
        let remember = inputs.is_some();
        #[cfg(feature = "deployed")]
        let inputs = inputs.unwrap_or(Inputs::default());
        #[cfg(not(feature = "deployed"))]
        let inputs = {
            let mut inputs = inputs.unwrap_or(Inputs::default());
            inputs.server.value= "127.0.0.1:8080".into();
            inputs.username.value = "TestUser_0".into();
            inputs.password.value = "testpass0".into();
            inputs
        };

        Self {
            inputs,
            remember,
            db,
            errorbar: ErrorBar::default(),
            submit: button::State::default(),
            logging_in: false,
        }
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        let inputs = &mut self.inputs;
        match event {
            Event::Servername(s) => {
                inputs.server.value = s;
                inputs.server.style = style::Input::Ok;
            }
            Event::Username(s) => {
                inputs.username.value = s;
                inputs.server.style = style::Input::Ok;
            }
            Event::Password(s) => {
                inputs.password.value = s;
                inputs.server.style = style::Input::Ok;
            }
            Event::RememberToggle(value) => {
                self.remember = value;
                inputs.store(&self.db);
            }
            Event::Submit => return self.on_submit(),
            Event::Error(e) => return self.handle_err(e),
            Event::ClearError(e) => self.errorbar.clear(e),
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let inputs = self.inputs.view().map(Msg::LoginPage);
        let title = Text::new("WorldSync")
            .width(Length::FillPortion(1))
            .horizontal_alignment(HorizontalAlignment::Center);

        let column = Column::new()
            .align_items(Align::Center)
            .width(Length::FillPortion(8))
            .push(Space::with_height(Length::FillPortion(4)))
            .push(title)
            .push(Space::with_height(Length::FillPortion(1)))
            .push(inputs)
            .push(Space::with_height(Length::FillPortion(1)))
            .push(login_button(&mut self.submit, self.logging_in))
            .push(remember_me(self.remember))
            .push(Space::with_height(Length::FillPortion(2)));

        let main_ui = Row::new()
            .push(Space::with_width(Length::FillPortion(3)))
            .push(column)
            .push(Space::with_width(Length::FillPortion(3)));
        let errorbar = self.errorbar.view().map(Msg::LoginPage);
        Column::new()
            .width(Length::Fill)
            .push(errorbar)
            .push(main_ui)
            .into()
    }
}

fn remember_me(is_checked: bool) -> Checkbox<Msg> {
    Checkbox::new(is_checked, "Keep me logged in", |b| {
        Msg::LoginPage(Event::RememberToggle(b))
    })
}

fn right_text(text: &str) -> Text {
    Text::new(text).horizontal_alignment(HorizontalAlignment::Right)
}

fn login_button(state: &mut button::State, logging_in: bool) -> Button<Msg> {
    let button_style = if logging_in {
        style::Button::Blocked
    } else {
        style::Button::Clickable
    };
    Button::new(state, Text::new("Login"))
        .on_press(Msg::LoginPage(Event::Submit))
        .style(button_style)
}
