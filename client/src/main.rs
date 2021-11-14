use iced::{executor, Application, Clipboard, Command, Element, Settings, Text};
mod gui;
mod events;
use gui::login;
pub use events::Event;

pub fn main() -> iced::Result {
    State::run(Settings::default())
}

struct State {
    login: login::Page,
    page: Page,
}

impl State {
    fn new() -> Self {
        Self {
            login: login::Page::new(),
            page: Page::Login,
        }
    }
}

enum Page {
    Login,
}

impl Application for State {
    type Executor = executor::Default;
    type Message = Event;
    type Flags = ();

    fn new(_flags: ()) -> (State, Command<Event>) {
        (State::new(), Command::none())
    }

    fn title(&self) -> String {
        String::from("A cool application")
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut Clipboard,
    ) -> Command<Self::Message> {
        match message {
            Event::LoginEvent(event) => self.login.update(event),
        }
    }

    fn view(&mut self) -> Element<Event> {
        match self.page {
            Page::Login => self.login.view(),
        }
    }
}
