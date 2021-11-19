use std::collections::HashMap;

use crate::gui::style;
use crate::gui::parts::ErrorBar;
pub use crate::Event as Msg;
use iced::widget::Column;
use iced::{button, text_input, Button, Checkbox, Command, Length, Row, Space, TextInput};
use iced::{Align, Element, HorizontalAlignment, Text};

use super::parts::ClearError;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Not a server, please check the address")]
    InvalidFormat,
    #[error("Port is not a number, please check the address")]
    NotANumber,
    #[error("Could not connect to WorldSync server")]
    NoMetaConn,
    #[error("Invalid username or password")]
    IncorrectLogin,
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
struct Inputs {
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


#[derive(Default)]
pub struct Page {
    inputs: Inputs,
    errorbar: ErrorBar<Error>,
    submit: button::State,
    remember: bool,
    logging_in: bool,
}

impl Page {
    pub fn new() -> Self {
        Self {
            #[cfg(not(deployed))]
            inputs: Inputs {
                server: Input {
                    value: "127.0.0.1:8080".to_owned(),
                    ..Input::default()
                },
                username: Input {
                    value: "TestUser_0".to_owned(),
                    ..Input::default()
                },
                password: Input {
                    value: "testpass0".to_owned(),
                    ..Input::default()
                },
            },
            ..Self::default()
        }
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        let inputs = &mut self.inputs;
        match dbg!(event) {
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
            Event::RememberToggle(value) => self.remember = value,
            Event::Submit => return self.on_submit(),
            Event::Error(e) => return self.handle_err(e),
            Event::ClearError(e) => self.errorbar.clear(e),
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let inputs = self.inputs.view().map(move |e| Msg::LoginPage(e));
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
        let errorbar = self.errorbar.view().map(move |e| Msg::LoginPage(e));
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
