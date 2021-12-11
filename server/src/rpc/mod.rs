use crate::db::user::UserDb;
use crate::host::HostEvent;
use crate::{Sessions, World};
use protocol::{Event, SessionId, UserId};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

mod calls;

#[derive(Clone)]
pub struct ConnState {
    pub peer_addr: SocketAddr,
    pub sessions: Sessions,
    pub events: Arc<broadcast::Sender<Event>>,
    pub userdb: UserDb,
    pub world: World,
    pub host_req: mpsc::Sender<HostEvent>,
}

impl ConnState {
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
}
