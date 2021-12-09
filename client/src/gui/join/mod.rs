pub use crate::Event as Msg;
use iced::{
    button, Align, Button, Column, Command, Element, HorizontalAlignment, Length, Row, Space, Text,
};

use super::parts::Loading;

mod tasks;

#[derive(thiserror::Error, Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {}

impl From<protocol::Error> for Error {
    fn from(e: protocol::Error) -> Self {
        unimplemented!("should not run into {:?} on login page", e)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    HostLoading(u8),
    HostLoaded,
}

pub struct Page {
    pub host: protocol::Host,
    loading: Loading,
    copy: button::State,
}

impl Page {
    pub fn from(host: protocol::Host) -> Self {
        Self {
            host,
            loading: Loading::default(),
            copy: button::State::default(),
        }
    }

    pub fn update(&mut self, event: Event) -> Command<Msg> {
        use Event::*;
        match event {
            HostLoading(p) => self
                .loading
                .start(100.0, p as f32),
            HostLoaded => self.loading.stop(),
        }
        Command::none()
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let bottom_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .align_items(Align::Center)
            .width(Length::FillPortion(8))
            .push(top_spacer)
            .push(self.title())
            .push(copy_button(&mut self.copy))
            .push(self.loading.view())
            .push(bottom_spacer);

        let ui = Row::new()
            .push(left_spacer)
            .push(center_column)
            .push(sidebar);

        Column::new().width(Length::Fill).push(ui).into()
    }
}

impl Page {
    fn title(&self) -> Text {
        let label = match self.host.loading {
            true => format!("{} is hosting", self.host.name),
            false => format!("{} started hosting", self.host.name),
        };
        Text::new(label)
            .width(Length::FillPortion(1))
            .horizontal_alignment(HorizontalAlignment::Center)
    }
}

fn copy_button(state: &mut button::State) -> Button<Msg> {
    Button::new(
        state,
        Text::new("Copy Ip").horizontal_alignment(HorizontalAlignment::Center),
    )
    .on_press(Msg::ClipHost)
}
