use std::net::SocketAddr;
use protocol::Error;
use protocol::{tarpc, Credentials, User, UserId, World, SessionId};
use tarpc::context;
use crate::db::user::UserDb;
use crate::Sessions;

#[derive(Clone)]
pub struct ConnState {
    pub peer_addr: SocketAddr,
    pub sessions: Sessions,
    pub userdb: UserDb,
}

#[tarpc::server]
impl World for ConnState {
    async fn version(self, _: context::Context) -> protocol::Version {
        protocol::Version {
            protocol: protocol::version().to_owned(),
            server: crate::version().to_owned(),
        }
    }
    async fn log_in(mut self, _: context::Context, credentials: Credentials) -> Result<SessionId, Error> {
        if let Some(user) = self.userdb.validate_credentials(credentials).await? {
            let uuid = self.sessions.add(user);
            Ok(uuid)
        } else {
            Err(Error::IncorrectLogin)
        }
    }
    async fn add_user(mut self, _: context::Context, user: User, password: String) -> Result<(),Error> {
        Ok(self.userdb.add_user(user, password).await?)
    }
    async fn list_users(self, _: context::Context) -> Result<Vec<(UserId, User)>, Error> {
        Ok(self.userdb.get_userlist()?)
    }
    async fn update_user(mut self, _: context::Context, id: UserId, old: User, new: User) -> Result<(), Error> {
        Ok(self.userdb.update_user(id, old, new).await?)
    }
}
