use crate::Event as Msg;
use iced::widget::Column;
use iced::{button, text_input, Button, Checkbox, Command, Length, Row, Space, TextInput};
use iced::{Align, Element, HorizontalAlignment, Text};

#[derive(Default)]
struct InputState {
    button: text_input::State,
    value: String,
}

#[derive(Default)]
struct Inputs {
    server: InputState,
    username: InputState,
    password: InputState,
}

impl Inputs {
    fn view(&mut self) -> Element<Event> {
        let texts = Column::new()
            .push(right_text("Server:"))
            .push(right_text("Username:"))
            .push(right_text("Password:"));
        let text_inputs = Column::new()
            .width(Length::FillPortion(1))
            .push(TextInput::new(
                &mut self.server.button,
                &self.server.value,
                &self.server.value,
                Event::Servername,
            ))
            .push(TextInput::new(
                &mut self.username.button,
                &self.username.value,
                &self.username.value,
                Event::Username,
            ))
            .push(
                TextInput::new(
                    &mut self.password.button,
                    &self.password.value,
                    &self.password.value,
                    Event::Password,
                )
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
    submit: button::State,
    remember: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Servername(String),
    Username(String),
    Password(String),
    RememberToggle(bool),
    Submit,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        let inputs = &mut self.inputs;
        match event {
            Event::Servername(s) => inputs.server.value = s,
            Event::Username(s) => inputs.username.value = s,
            Event::Password(s) => inputs.password.value = s,
            Event::RememberToggle(value) => self.remember = value,
            Event::Submit => {
                dbg!(
                    &inputs.server.value,
                    &inputs.username.value,
                    &inputs.password.value
                );
            }
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let inputs = self.inputs.view().map(move |e| Msg::LoginEvent(e));
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
            .push(login_button(&mut self.submit))
            .push(remember_me(self.remember))
            .push(Space::with_height(Length::FillPortion(2)));

        Row::new()
            .push(Space::with_width(Length::FillPortion(3)))
            .push(column)
            .push(Space::with_width(Length::FillPortion(3)))
            .into()
    }
}

fn remember_me(is_checked: bool) -> Checkbox<Msg> {
    Checkbox::new(is_checked, "Keep me logged in", |b| {
        Msg::LoginEvent(Event::RememberToggle(b))
    })
}

fn right_text(text: &str) -> Text {
    Text::new(text).horizontal_alignment(HorizontalAlignment::Right)
}

fn login_button(state: &mut button::State) -> Button<Msg> {
    Button::new(state, Text::new("Login")).on_press(Msg::LoginEvent(Event::Submit))
}
