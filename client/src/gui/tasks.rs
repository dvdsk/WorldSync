use iced::Command;
use tracing::info;

use crate::Event;

use super::{RpcConn, State};

#[derive(Default, Clone)]
pub struct SubStatus {
    id: usize,
    active: bool,
}

impl SubStatus {
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        assert!(self.active, "subscription was not active");
        self.active = false;
    }
    pub fn start(&mut self) {
        assert!(!self.active, "subscription was already active");
        self.active = true;
        self.id += 1;
    }
    pub fn active(&self) -> Option<usize> {
        match self.active {
            true => Some(self.id),
            false => None,
        }
    }
}

impl State {
    pub fn unwrap_rpc(&self) -> RpcConn {
        self.rpc.as_ref().unwrap().clone()
    }

    pub fn handle_server_event(&mut self, event: protocol::Event) -> Command<Event> {
        use super::{host, join, Page};
        use protocol::Event::*;

        match event {
            HostLoading(p) if self.page == Page::Host => {
                return self
                    .can_host
                    .update(host::Event::Loading(p), self.unwrap_rpc())
            }
            HostLoaded if self.page == Page::Host => self.page = Page::Hosting,
            NewHost(host) => match self.can_host.is_us(&host) {
                true => {
                    info!("attempting to host");
                    self.downloading_world.start();
                }
                false => {
                    info!("got new host: {:?}", host);
                    self.can_join = Some(join::Page::from(host, join::HostState::Loading(0)));
                    self.page = Page::Join;
                }
            },
            HostDropped | HostCanceld | HostShutdown => self.page = Page::Host,
            TestHB(n) => info!("recieved hb {}", n),
            _e => if let Some(p) = self.can_join.as_mut() {
                p.host_state = _e.into()
            },
        }
        Command::none()
    }
}
