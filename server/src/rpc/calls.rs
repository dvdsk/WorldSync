use crate::db::user::UserDb;
use crate::Sessions;
use protocol::Error;
use protocol::{tarpc, SessionId, User, UserId, World};
use std::net::SocketAddr;
use tarpc::context;
use tracing::{info, warn};

#[derive(Clone)]
pub struct ConnState {
    pub peer_addr: SocketAddr,
    pub sessions: Sessions,
    pub userdb: UserDb,
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

#[tarpc::server]
impl World for ConnState {
    async fn version(self, _: context::Context) -> protocol::Version {
        protocol::Version {
            protocol: protocol::version().to_owned(),
            server: crate::version().to_owned(),
        }
    }
    async fn log_in(
        mut self,
        _: context::Context,
        username: String,
        password: String,
    ) -> Result<SessionId, Error> {
        use crate::db::user::Error as DbError;
        let user_id = self
            .userdb
            .get_user_id(&username)
            .ok_or(Error::IncorrectLogin)?;
        match self.userdb.validate_credentials(user_id, password).await {
            Ok(user_id) => {
                let uuid = self.sessions.add(user_id);
                Ok(uuid)
            }
            Err(DbError::IncorrectPass) => {
                warn!(
                    "Incorrect password for user: '{}' from {}",
                    username, self.peer_addr
                );
                Err(Error::IncorrectLogin)
            }
            Err(DbError::IncorrectName) => {
                warn!("Incorrect username ({}) from {}", username, self.peer_addr);
                Err(Error::IncorrectLogin)
            }
            Err(e) => Err(e)?,
        }
    }

    async fn get_account(self, _: context::Context, id: SessionId) -> Result<User, Error> {
        let user_id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        self.userdb.get_user(user_id)?.ok_or(Error::Internal)
    }

    async fn update_account(
        mut self,
        _: context::Context,
        id: SessionId,
        new: User,
    ) -> Result<(), Error> {
        let user_id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        self.userdb.override_user(user_id, new).await?;
        info!("user ({}) updated account details", user_id);
        Ok(())
    }

    async fn update_password(
        self,
        _: context::Context,
        id: SessionId,
        new: String,
    ) -> Result<(), Error> {
        let user_id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        self.userdb.change_password(user_id, new).await?;
        self.clear_sessions(user_id);
        info!("user ({}) changed password", user_id);
        Ok(())
    }

    async fn close_account(mut self, _: context::Context, id: SessionId) -> Result<(), Error> {
        let user_id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let name = self.userdb.remove_user(user_id).await?;
        info!("user ({})({}) removed itself", name, user_id);
        Ok(())
    }

    async fn add_user(
        mut self,
        _: context::Context,
        user: User,
        password: String,
    ) -> Result<(), Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }
        self.userdb.add_user(user.clone(), password).await?;
        info!("added user: {}", user.username);
        Ok(())
    }

    async fn list_users(self, _: context::Context) -> Result<Vec<(UserId, User)>, Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }
        Ok(self.userdb.get_userlist()?)
    }

    async fn override_account(
        mut self,
        _: context::Context,
        id: UserId,
        old: User,
        new: User,
    ) -> Result<(), Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }
        self.userdb.update_user(id, old.clone(), new).await?;
        info!("updated user: {}", old.username);
        Ok(())
    }

    async fn override_password(
        self,
        _: context::Context,
        user_id: UserId,
        new_password: String,
    ) -> Result<(), Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }

        self.userdb.change_password(user_id, new_password).await?;
        self.clear_sessions(user_id);
        if let Ok(Some(user)) = self.userdb.get_user(user_id) {
            info!("overrode password for user: {}", user.username);
        }
        Ok(())
    }

    async fn remove_account(mut self, _: context::Context, id: UserId) -> Result<(), Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }
        let name = self.userdb.remove_user(id).await?;
        info!("removed user: {}", name);
        Ok(())
    }
}
