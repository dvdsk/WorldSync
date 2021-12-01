use crate::{events, world_dl, mc, Event};
use derivative::Derivative;
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
    can_join: join::Page,
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
            can_join: join::Page::new(),
            page: Page::Login,

            rpc: None,
            server_events: false,
            downloading_world: SubStatus::default(),
            mc_server: SubStatus::default(),
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
        use protocol::Event::*;
        use Event::*;

        match dbg!(message) {
            LoginPage(event) => return self.login.update(event),
            HostPage(event) => return self.can_host.update(event, &mut self.rpc),
            LoggedIn(rpc, host) => {
                self.server_events = true;
                match host {
                    Some(host) => {
                        info!("logged in, can join {:?}", host);
                        self.rpc = Some(rpc);
                        self.page = Page::Join;
                    }
                    None => {
                        info!("logged in, no one is hosting");
                        self.rpc = Some(rpc);
                        self.page = Page::Host;
                    }
                }
            }
            WorldUpdated => {
                self.mc_server.start();
                self.can_host.update(host::Event::WorldUpdated, &mut self.rpc);
            }
            ServerStarted => {
                self.page = Page::Hosting;
            }
            Server(NewHost(host)) if self.can_host.is_us(&host) => {
                info!("attempting to host");
                self.downloading_world.start();
            }
            Server(NewHost(host)) => {
                info!("got new host: {:?}", host);
                self.can_join.host = Some(host);
                self.page = Page::Join;
            }
            Server(TestHB(n)) => info!("recieved hb {}", n),
            Error(e) => panic!("tmp error remove {:?}", e),
            Empty => (),
            _e => todo!("handle: {:?}", _e),
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
            Page::Join => self.can_join.view(),
            Page::Host => self.can_host.view(),
            Page::Hosting => self.hosting.view(),
        }
    }
}
