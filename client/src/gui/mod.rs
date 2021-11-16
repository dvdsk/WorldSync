use crate::Event;
use iced::{executor, Application, Clipboard, Command, Element, Settings, Text};

pub mod login;
mod style;

pub struct State {
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
        use Event::*;
        match message {
            LoginPage(event) => return self.login.update(event),
            LoggedIn(client, uuid) => eprintln!("logged_in"),
            Error => eprintln!("tmp error remove when error handling in place"),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Event> {
        match self.page {
            Page::Login => self.login.view(),
        }
    }
}
