use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use protocol::SessionId;
use sync::{DirContent, DirUpdate, UpdateList};
use tracing::info;
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
    pub fn set_host(&self, addr: SocketAddr, session_id: SessionId) -> bool {
        let mut state = self.state.write().unwrap();
        state.set_host(addr, session_id)
    }

    pub async fn from(db: sled::Db) -> Self {
        Self {
            state: Arc::new(RwLock::new(State { host: None })),
            db: WorldDb::from(db).await, 
        }
    }

    pub fn get_update(&self, dir: DirContent) -> DirUpdate {
        self.db.get_update_list(dir)
    }

    pub async fn dump_save(&self, target: PathBuf) {
        let save = self.db.last_save();
        for sync::Object{org_path, id, ..} in save.objects() {
            let mut source = WorldDb::obj_path().to_owned();
            source.push(id.0.to_string());
            let mut target = target.clone();
            target.push(org_path);
            tokio::fs::copy(source, target).await.unwrap();
        }
        info!("dumped save to: {:?}", target);
    }

    pub async fn load_save(&self, source: PathBuf) -> Result<(), protocol::Error> {
        if self.host().is_some() {
            return Err(protocol::Error::SaveInUse);
        }

        let content = DirContent::from_path(source).await.unwrap();
        let (new_save, update_list) = UpdateList::for_new_save(&self.db, content);
        for (object_id, path) in update_list.0 {
            let bytes = tokio::fs::read(path).await.unwrap();
            WorldDb::add_obj(object_id, &bytes).await?;
        }
        self.db.push_save(new_save);

        Ok(())
    }
}

#[derive(Clone)]
pub struct Host {
    last_hb: Instant,
    addr: SocketAddr,
    session_id: SessionId,
}

pub struct State {
    host: Option<Host>,
}

impl State {
    pub fn host(&self) -> Option<protocol::Host> {
        self.host.as_ref().map(|h| protocol::Host { addr: h.addr, id: h.session_id })
    }
    pub fn set_host(&mut self, addr: SocketAddr, session_id: SessionId) -> bool {
        if self.host.is_some() {
            return false;
        }

        self.host = Some(Host {
            last_hb: Instant::now(),
            session_id,
            addr,
        });

        true
    }
}
