use core::hash::Hash;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::RangeInclusive;

pub use crate::Event as Msg;
use iced::widget::Column;
use iced::{Button, Length, ProgressBar, Row, Space, button};
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

pub enum Loading {
    NotStarted,
    InProgress {
        range: RangeInclusive<f32>,
        value: f32,
    }
}

impl Default for Loading {
    fn default() -> Self {
        Loading::NotStarted
    }
}

impl Loading {
    pub fn view(&self) -> Element<Msg> {
        use Loading::*;
        match self {
            NotStarted => Space::with_height(Length::FillPortion(1)).into(),
            InProgress{range, value} => ProgressBar::new(range.clone(), *value).into(),
        }
    }
}
