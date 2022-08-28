pub use crate::Event as Msg;
use iced::{
    button, Button, Column, Element, Length, Row, Space, Text,
};
use iced_native::{alignment::Horizontal, Alignment}; 
use protocol::HostDetails;

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
}

#[derive(Debug, Clone)]
pub enum HostState {
    Loading(u8),
    Running,
    ShuttingDown,
    Unreachable,
}

impl From<protocol::HostState> for HostState {
    fn from(state: protocol::HostState) -> Self {
        use protocol::HostState::*;
        match state {
            NoHost => panic!("need not"),
            Loading(_) => Self::Loading(0),
            Up(_) => Self::Running,
            Unreachable(_) => Self::Unreachable,
            ShuttingDown(_) => Self::ShuttingDown,
        }
    }
}

impl From<protocol::Event> for HostState {
    fn from(e: protocol::Event) -> Self {
        use protocol::Event::*;
        match e {
            HostLoading(progress) => Self::Loading(progress),
            HostLoaded => Self::Running,
            HostShuttingDown => Self::ShuttingDown,
            HostUnreachable => Self::Unreachable,
            _e => panic!("event should not be handled here: {:?}", _e),
        }
    }
}

pub struct Page {
    pub host: HostDetails,
    pub host_state: HostState,
    loading: Loading,
    copy: button::State,
}

impl Page {
    pub fn from(host: HostDetails, host_state: HostState) -> Self {
        Self {
            host,
            host_state,
            loading: Loading::default(),
            copy: button::State::default(),
        }
    }

    pub fn view(&mut self) -> Element<Msg> {
        let sidebar = Space::with_width(Length::FillPortion(4));
        let left_spacer = Space::with_width(Length::FillPortion(1));
        let top_spacer = Space::with_height(Length::FillPortion(1));
        let bottom_spacer = Space::with_height(Length::FillPortion(1));
        let center_column = Column::new()
            .align_items(Alignment::Center)
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
        use HostState::*;
        let label = match self.host_state {
            Loading(_) => format!("{} started hosting", self.host.name),
            Running => format!("{} is hosting", self.host.name),
            ShuttingDown => format!("{} was hosting (shutting down)", self.host.name),
            Unreachable => format!("{} is hosting and having issues", self.host.name),
        };

        Text::new(label)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Center)
    }
}

fn copy_button(state: &mut button::State) -> Button<Msg> {
    Button::new(
        state,
        Text::new("Copy Ip").horizontal_alignment(Horizontal::Center),
    )
    .on_press(Msg::ClipHost)
}
