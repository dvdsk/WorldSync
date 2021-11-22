use core::hash::Hash;
use std::collections::HashMap;
use std::fmt::Display;

pub use crate::Event as Msg;
use iced::widget::Column;
use iced::{button, Button, Length, Row};
use iced::{Align, Element, HorizontalAlignment, Text};

pub trait ClearError : Clone {
    type Error: Eq + Hash + Display;
    fn clear(e: Self::Error) -> Self;
}

pub struct ErrorBar<Err>(HashMap<Err, button::State>);

impl<E> Default for ErrorBar<E> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<'a, Err: Clone + Eq + Hash + Display> ErrorBar<Err> {
    pub fn add(&mut self, err: Err) {
        self.0.insert(err, button::State::new());
    }
    pub fn clear(&mut self, err: Err) {
        self.0.remove(&err);
    }
    pub fn view<Ev: 'a + ClearError<Error = Err>>(&'a mut self) -> Element<Ev> {
        let mut column = Column::new();
        for (err, button_state) in self.0.iter_mut() {
            let button =
                Button::new(button_state, Text::new('x')).on_press(Ev::clear(err.clone()));
            let text = Text::new(err.to_string()).horizontal_alignment(HorizontalAlignment::Left);
            column = column.push(
                Row::new()
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(button)
                    .push(text),
            );
        }
        column.into()
    }
}
