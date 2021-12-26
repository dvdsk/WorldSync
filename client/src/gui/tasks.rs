use iced::Command;
use tracing::{info, warn};
use std::fs;

use crate::Event;

use super::{RpcConn, State};

pub fn open_settings() -> sled::Db {
    use crate::db_path;
    let dir = db_path().parent().unwrap();
    fs::create_dir_all(dir).unwrap();
    match sled::open(db_path()) {
        Ok(db) => db,
        Err(e) => {
            warn!("error opening db: {:?}", e);
            fs::remove_dir(db_path()).expect("could not remove corrupt db");
            sled::open(db_path()).expect("could not open new db")
        }
    }
}

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
        if self.active {
            return;
        }
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
            HostLoaded if self.page == Page::Host => {
                self.page = Page::Hosting;
            }
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
            HostDropped | HostCanceld | HostShutdown => self.page = dbg!(Page::Host),
            #[cfg(not(feature = "deployed"))]
            TestHB(n) => info!("recieved hb {}", n),
            _e => if let Some(p) = self.can_join.as_mut() {
                p.host_state = _e.into()
            },
        }
        Command::none()
    }
}
