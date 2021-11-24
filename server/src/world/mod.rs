use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use typed_sled::sled;

use crate::db::world::WorldDb;

#[derive(Clone)]
pub struct World {
    state: Arc<RwLock<State>>,
    db: WorldDb,
}

impl World {
    pub fn host(&self) -> Option<protocol::Host> {
        let state = self.state.read().unwrap();
        state.host()
    }
    pub fn set_host(&self, addr: SocketAddr) -> bool {
        let mut state = self.state.write().unwrap();
        state.set_host(addr)
    }

    pub fn from(db: sled::Db) -> Self {
        Self {
            state: Arc::new(RwLock::new(State { host: None })),
            db: WorldDb::from(db), 
        }
    }
}

#[derive(Clone)]
pub struct Host {
    last_hb: Instant,
    addr: SocketAddr,
}

pub struct State {
    host: Option<Host>,
}

impl State {
    pub fn host(&self) -> Option<protocol::Host> {
        self.host.as_ref().map(|h| protocol::Host { addr: h.addr })
    }
    pub fn set_host(&mut self, addr: SocketAddr) -> bool {
        if self.host.is_some() {
            return false;
        }

        self.host = Some(Host {
            last_hb: Instant::now(),
            addr,
        });

        true
    }
}
