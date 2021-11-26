use crate::{events, Event};
use iced::{executor, Application, Clipboard, Command, Element, Subscription};
use protocol::{ServiceClient, Uuid};
use tracing::info;

pub mod host;
pub mod hosting;
pub mod join;
pub mod login;
pub mod parts;
mod style;
mod tasks;

#[derive(Clone, Debug)]
pub struct RpcConn {
    pub client: ServiceClient,
    pub session: Uuid,
}

pub struct State {
    login: login::Page,
    hosting: hosting::Page,
    can_host: host::Page,
    can_join: join::Page,
    page: Page,

    server_events: Option<events::ServerSub>,
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

            server_events: None,
            rpc: None,
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
        match message {
            Event::LoginPage(event) => return self.login.update(event),
            Event::HostPage(event) => return self.can_host.update(event, &mut self.rpc),
            Event::LoggedIn(rpc, Some(host)) => {
                info!("logged in, can join {:?}", host);
                self.rpc = Some(rpc);
                self.page = Page::Join;
                // TODO add updater
            }
            Event::LoggedIn(rpc, Option::None) => {
                info!("logged in, no one is hosting");
                self.rpc = Some(rpc);
                self.page = Page::Host;
                // TODO add updater
            }

            Event::Server(protocol::Event::TestHB(n)) => info!("recieved hb {}", n),
            Event::Error(e) => panic!("tmp error remove {:?}", e),
            _e => todo!("handle: {:?}", _e),
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Event> {
        match &self.rpc {
            None => Subscription::none(),
            Some(rpc) => events::sub_to_server(rpc.clone()),
        }
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
