use std::net::SocketAddr;
use protocol::Error;
use protocol::{tarpc, Credentials, User, World};
use tarpc::context;
use crate::db::user::UserDb;
use uuid::Uuid;
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
    async fn log_in(mut self, _: context::Context, credentials: Credentials) -> Result<Uuid, Error> {
        if let Some(user) = self.userdb.validate_credentials(credentials).await? {
            let uuid = self.sessions.add(user);
            Ok(uuid)
        } else {
            Err(Error::IncorrectLogin)
        }
    }
    async fn add_user(mut self, _: context::Context, user: User, password: String) -> Result<(),Error> {
        Ok(self.userdb.store(user, password).await?)
    }
    async fn list_users(self, _: context::Context) -> Result<Vec<User>, Error> {
        Ok(self.userdb.get_userlist()?)
    }
    async fn update_user(self, _: context::Context, old: User, new: User) -> Result<(), Error> {
        Ok(self.userdb.update_user(old, new).await?)
    }
}
