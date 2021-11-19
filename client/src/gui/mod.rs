use crate::Event;
use protocol::{ServiceClient, Uuid};
use iced::{executor, Application, Clipboard, Command, Element};
use tracing::info;

pub mod login;
pub mod parts;
pub mod host;
pub mod hosting;
pub mod join;
mod style;

#[derive(Clone, Debug)]
pub struct RpcConn {
    client: ServiceClient,
    session: Uuid,
}

pub struct State {
    login: login::Page,
    hosting: hosting::Page,
    can_host: host::Page,
    can_join: join::Page,
    page: Page,

    rpc: Option<RpcConn>,
}

impl State {
    fn new() -> Self {
        Self {
            login: login::Page::new(),
            hosting: hosting::Page::new(),
            can_host: host::Page::new(),
            can_join: join::Page::new(),
            page: Page::Login,

            rpc: None
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
            HostPage(event) => return self.can_host.update(event),
            LoggedIn(rpc, Some(host)) => {
                info!("logged in, can join {:?}", host);
                self.rpc = Some(rpc);
                self.page = Page::Join;
            }
            LoggedIn(rpc, None) => {
                info!("logged in, no one is hosting");
                self.rpc = Some(rpc);
                self.page = Page::Host;
            }
            Error => eprintln!("tmp error remove when error handling in place"),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Event> {
        match self.page {
            Page::Login => self.login.view(),
            Page::Join => self.can_join.view(),
            Page::Host => self.can_host.view(),
            Page::Hosting => self.hosting.view(),
        }
    }
}
