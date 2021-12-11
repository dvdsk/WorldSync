use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use protocol::SessionId;
use sync::{DirContent, DirUpdate, UpdateList};
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task;
use tokio::time::sleep;
use tracing::{info, instrument};
use typed_sled::sled;

use crate::db::world::WorldDb;

#[derive(Clone, Debug)]
pub struct World {
    state: Arc<RwLock<State>>,
    db: WorldDb,
}

impl World {
    pub fn host(&self) -> Option<protocol::Host> {
        let state = self.state.read().unwrap();
        state.host()
    }
    pub fn set_host(&self, addr: SocketAddr, session_id: SessionId, name: String) -> bool {
        let mut state = self.state.write().unwrap();
        state.set_host(addr, session_id, name)
    }

    pub async fn from(db: sled::Db, sender: Arc<Sender<protocol::Event>>) -> Self {
        Self {
            state: Arc::new(RwLock::new(State {
                events: sender.subscribe(),
            })),
            db: WorldDb::from(db).await,
        }
    }

    pub fn get_update(&self, dir: DirContent) -> DirUpdate {
        self.db.get_update_list(dir)
    }

    #[instrument(err)]
    pub async fn dump_save(&self, target: PathBuf) -> Result<(), protocol::Error> {
        let is_empty = tokio::fs::read_dir(&target)
            .await
            .expect("could not check save dump content")
            .next_entry()
            .await
            .unwrap()
            .is_none();

        if !is_empty {
            return Err(protocol::Error::NotEmpty);
        }

        let save = self.db.last_save();
        for sync::Object { org_path, id, .. } in save.objects() {
            let mut source = WorldDb::obj_path().to_owned();
            source.push(id.0.to_string());
            let mut target = target.clone();
            target.push(org_path);
            tokio::fs::copy(source, target).await.unwrap();
        }
        info!("dumped save to: {:?}", target);
        Ok(())
    }

    pub async fn set_save(&self, source: PathBuf) -> Result<(), protocol::Error> {
        if self.host().is_some() {
            return Err(protocol::Error::SaveInUse);
        }

        let content = DirContent::from_dir(source.clone()).await.unwrap();
        let (new_save, update_list) = UpdateList::for_new_save(&self.db, content);
        for (object_id, path) in update_list.0 {
            let full_path = source.join(path);
            let bytes = tokio::fs::read(full_path).await.unwrap();
            WorldDb::add_obj(object_id, &bytes).await?;
        }
        self.db.push_save(new_save);
        info!("loaded and set save from: {:?}", source);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Host {
    name: String,
    last_hb: Instant,
    addr: SocketAddr,
    session_id: SessionId,
}

#[derive(Debug)]
pub struct State {
    events: Receiver<protocol::Event>,
}

impl State {
    pub fn host(&self) -> Option<protocol::Host> {
        todo!();
        // self.host.as_ref().map(|h| protocol::Host {
        //     loading: true,
        //     reachable: true,
        //     name: h.name.clone(),
        //     addr: h.addr,
        //     id: h.session_id,
        // })
    }
    pub fn set_host(&mut self, addr: SocketAddr, session_id: SessionId, name: String) -> bool {
        todo!();
        // if self.host.is_some() {
        //     return false;
        // }

        // self.host = Some(Host {
        //     name,
        //     last_hb: Instant::now(),
        //     session_id,
        //     addr,
        // });

        // true
    }
}
