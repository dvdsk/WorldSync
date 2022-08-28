use core::fmt;
use core::hash::Hash;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::RangeInclusive;

pub use crate::Event as Msg;
use iced::widget::Column;
use iced::{button, Button, Length, ProgressBar, Row, Space};
use iced::{Element, Text};
use iced_native::{alignment::Horizontal, Alignment}; 

pub trait ClearError: Clone {
    type Error: Eq + Hash + Display;
    fn clear(e: Self::Error) -> Self;
}

#[derive(Debug)]
pub struct ErrorBar<Err: fmt::Debug>(HashMap<Err, button::State>);

impl<E: fmt::Debug> Default for ErrorBar<E> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<'a, Err: fmt::Debug + Clone + Eq + Hash + Display> ErrorBar<Err> {
    pub fn add(&mut self, err: Err) {
        self.0.insert(err, button::State::new());
    }
    pub fn clear(&mut self, err: Err) {
        self.0.remove(&err);
    }
    pub fn view<Ev: 'a + ClearError<Error = Err>>(&'a mut self) -> Element<Ev> {
        let mut column = Column::new();
        for (err, button_state) in self.0.iter_mut() {
            let button = Button::new(button_state, Text::new('x')).on_press(Ev::clear(err.clone()));
            let text = Text::new(err.to_string()).horizontal_alignment(Horizontal::Left);
            column = column.push(
                Row::new()
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
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
    },
}

impl Default for Loading {
    fn default() -> Self {
        Loading::NotStarted
    }
}

impl Loading {
    pub fn set_progress(&mut self, val: f32) {
        if let Loading::InProgress { value, .. } = self {
            *value = val;
        }
    }
    pub fn finished(&mut self) {
        match self {
            Loading::InProgress { value, range } => *value = *range.end(),
            Loading::NotStarted => panic!("was not in progress"),
        }
    }
    pub fn start(&mut self, complete: f32, val: f32) {
        *self = Loading::InProgress {
            range: 0.0..=complete + 1.0,
            value: val,
        }
    }
    pub fn stop(&mut self) {
        *self = Loading::NotStarted
    }

    pub fn view(&self) -> Element<Msg> {
        use Loading::*;
        match self {
            NotStarted => Space::with_height(Length::FillPortion(1)).into(),
            InProgress { range, value } => ProgressBar::new(range.clone(), *value).into(),
        }
    }
}
