use crate::Event;
use iced::{executor, Application, Clipboard, Command, Element};

pub mod login;
mod host;
mod hosting;
mod join;
mod style;

pub struct State {
    login: login::Page,
    hosting: hosting::Page,
    host: host::Page,
    join: join::Page,
    page: Page,
}

impl State {
    fn new() -> Self {
        Self {
            login: login::Page::new(),
            hosting: hosting::Page::new(),
            host: host::Page::new(),
            join: join::Page::new(),
            page: Page::Login,
        }
    }
}

enum Page {
    Login,
    Hosting,
    Host,
    Join,
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
            LoggedIn(client, uuid, Some(host)) => {
                eprintln!("logged in, someone is hosting");
            }
            LoggedIn(client, uuid, None) => {
                eprintln!("logged_in");
                self.page = Page::Host;
            }
            Error => eprintln!("tmp error remove when error handling in place"),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Event> {
        match self.page {
            Page::Login => self.login.view(),
            Page::Join => self.join.view(),
            Page::Host => self.host.view(),
            Page::Hosting => self.hosting.view(),
        }
    }
}
