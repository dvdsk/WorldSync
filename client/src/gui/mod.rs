use crate::{events, mc, world_dl, Event};
use derivative::Derivative;
use iced::{executor, Application, Clipboard, Command, Element, Subscription};
use protocol::{HostState, ServiceClient, Uuid};
use tracing::info;

pub mod host;
pub mod hosting;
pub mod join;
pub mod login;
pub mod parts;
mod style;
mod tasks;

use tasks::SubStatus;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct RpcConn {
    #[derivative(Debug = "ignore")]
    pub client: ServiceClient,
    pub session: Uuid,
}

pub struct State {
    login: login::Page,
    hosting: hosting::Page,
    can_host: host::Page,
    can_join: Option<join::Page>,
    page: Page,

    rpc: Option<RpcConn>,
    server_events: bool,
    downloading_world: SubStatus,
    mc_server: SubStatus,
}

impl State {
    fn new() -> Self {
        Self {
            login: login::Page::new(),
            hosting: hosting::Page::new(),
            can_host: host::Page::new(),
            can_join: None,
            page: Page::Login,

            rpc: None,
            server_events: false,
            downloading_world: SubStatus::default(),
            mc_server: SubStatus::default(),
        }
    }
}

#[derive(Debug, PartialEq)]
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
        clipboard: &mut Clipboard,
    ) -> Command<Self::Message> {
        use Event::*;

        match message {
            LoginPage(event) => return self.login.update(event),
            HostPage(event) => return self.can_host.update(event, self.unwrap_rpc()),
            HostingPage(event) => return self.hosting.update(event, self.unwrap_rpc()),
            LoggedIn(rpc, host_state) => {
                use HostState::*;
                self.server_events = true;
                match host_state.clone() {
                    NoHost => {
                        info!("logged in, no one is hosting");
                        self.rpc = Some(rpc);
                        self.page = Page::Host;
                    }
                    Loading(details)
                    | Up(details)
                    | Unreachable(details)
                    | ShuttingDown(details) => {
                        info!("logged in, can join {:?}", host_state);
                        self.rpc = Some(rpc);
                        self.can_join = Some(join::Page::from(details, host_state.into()));
                        self.page = Page::Join;
                    }
                }
            }
            ClipHost => clipboard.write(self.can_join.as_ref().unwrap().host.addr.ip().to_string()),
            WorldUpdated => {
                self.mc_server.start();
                return self.can_host
                    .update(host::Event::WorldUpdated, self.unwrap_rpc());
            }
            Mc(event) => match self.page {
                Page::Host => return self.can_host.update(host::Event::Mc(event), self.unwrap_rpc()),
                Page::Hosting => return self.hosting.update(hosting::Event::Mc(event), self.unwrap_rpc()),
                _ => panic!("should not recieve server events on other page"),
            }
            Server(event) => return self.handle_server_event(event),
            Error(e) => panic!("tmp error remove {:?}", e),
            Empty => (),
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Event> {
        let mut subs = Vec::new();
        if self.server_events {
            let rpc = self.rpc.as_ref().unwrap().clone();
            subs.push(events::sub_to_server(rpc))
        }
        if let Some(id) = self.downloading_world.active() {
            let rpc = self.rpc.as_ref().unwrap().clone();
            subs.push(world_dl::sub(rpc, id))
        }
        if let Some(_id) = self.mc_server.active() {
            subs.push(mc::sub())
        }

        Subscription::batch(subs)
    }

    fn view(&mut self) -> Element<Event> {
        match self.page {
            Page::Login => self.login.view(),
            Page::Join => self.can_join.as_mut().unwrap().view(),
            Page::Host => self.can_host.view(),
            Page::Hosting => self.hosting.view(),
        }
    }
}
