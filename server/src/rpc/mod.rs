use crate::db::user::UserDb;
use crate::{Sessions, World};
use protocol::{Event, SessionId, UserId};
use std::net::SocketAddr;
use tokio::sync::broadcast;

mod calls;

#[derive(Clone)]
pub struct ConnState {
    pub peer_addr: SocketAddr,
    pub sessions: Sessions,
    pub events: broadcast::Sender<Event>,
    pub userdb: UserDb,
    pub world: World,
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
