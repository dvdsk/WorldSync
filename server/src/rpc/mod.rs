use crate::db::user::UserDb;
use crate::{Sessions, World};
use protocol::{SessionId, UserId};
use std::net::SocketAddr;

mod calls;

#[derive(Clone)]
pub struct ConnState {
    pub peer_addr: SocketAddr,
    pub sessions: Sessions,
    pub userdb: UserDb,
    pub world: World,
}

impl ConnState {
    pub fn get_user_id(&self, id: SessionId) -> Option<UserId> {
        self.sessions.0.read().unwrap().get(&id).map(|s| s.user_id)
    }
    pub fn clear_sessions(&self, id: UserId) {
        self.sessions
            .0
            .write()
            .unwrap()
            .retain(|_, v| v.user_id != id)
    }
}
