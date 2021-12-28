use crate::db::user::UserDb;
use crate::host::HostEvent;
use crate::{Sessions, World};
use protocol::{Error, Event, HostId, SessionId, UserId, Addr};
use sync::DirContent;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

mod calls;

#[derive(Clone)]
pub struct ConnState {
    pub peer_addr: Option<Addr>,
    pub sessions: Sessions,
    pub events: Arc<broadcast::Sender<Event>>,
    pub userdb: UserDb,
    pub world: World,
    pub host_req: mpsc::Sender<HostEvent>,
}

fn allowed_paths() -> HashSet<&'static Path> {
    HashSet::from([
        Path::new("world"),
        Path::new("logs"),
    ])
}

impl ConnState {
    pub fn peer_addr(&self) -> Addr {
        self.peer_addr.clone().unwrap()
    }
    pub fn get_user_id(&self, id: SessionId) -> Option<UserId> {
        self.sessions.get_user_id(id)
    }
    pub fn clear_user_sessions(&self, id: UserId) {
        self.sessions.clear_user(id)
    }
    pub fn add_session(&mut self, id: UserId) -> SessionId {
        let backlog = self.events.subscribe();
        self.sessions.add(id, backlog)
    }
    pub async fn is_host(&self, id: HostId) -> Result<(), Error> {
        self.world.is_host(id).await.map_err(|_| Error::NotHost)
    }
    pub fn verify_content_safe(content: &DirContent) -> Result<(), Error> {
        let allowed = allowed_paths();
        for file in &content.0 {
            if !allowed.contains(file.path.as_path()) {
                return Err(Error::ForbiddenPath(file.path.clone()))
            }
        }
        Ok(())
    }
    pub fn path_safe(path: &Path) -> Result<(), Error> {
        let allowed = allowed_paths();
        if !allowed.contains(path) {
            return Err(Error::ForbiddenPath(path.into()));
        }
        Ok(())
    }
}
