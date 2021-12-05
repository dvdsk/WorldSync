use std::path::PathBuf;

use crate::db::world::WorldDb;
use sync::{DirContent, DirUpdate, ObjectId};

use super::ConnState;
use shared::tarpc;
use protocol::{Host, HostId, Service, SessionId, User, UserId};
use protocol::{Error, Event};
use tarpc::context;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, warn};

#[tarpc::server]
impl Service for ConnState {
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
                let uuid = self.add_session(user_id);
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
            Err(e) => Err(e.into()),
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
        self.clear_user_sessions(user_id);
        info!("user ({}) changed password", user_id);
        Ok(())
    }

    async fn close_account(mut self, _: context::Context, id: SessionId) -> Result<(), Error> {
        let user_id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let name = self.userdb.remove_user(user_id).await?;
        info!("user ({})({}) removed itself", name, user_id);
        Ok(())
    }

    async fn host(
        self,
        _: context::Context,
        id: SessionId,
    ) -> Result<Option<protocol::Host>, Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        Ok(self.world.host())
    }

    async fn request_to_host(
        self,
        _: context::Context,
        id: SessionId,
        host_id: HostId,
    ) -> Result<(), Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let host_changed = self.world.set_host(self.peer_addr, id);
        if host_changed {
            let host = Host {
                addr: self.peer_addr,
                id: host_id,
            };
            let _irrelevant = self.events.send(Event::NewHost(host));
        }
        Ok(())
    }
    async fn await_event(self, _: context::Context, id: SessionId) -> Result<Event, Error> {
        let backlog = {
            let sessions = self.sessions.by_id.read().unwrap();
            let session = sessions.get(&id).ok_or(Error::SessionExpired)?;
            session.backlog.clone()
        };

        let mut backlog = backlog.try_lock_owned().map_err(|_| Error::BackLogLocked)?;

        match backlog.recv().await {
            Err(RecvError::Closed) => panic!("events queue got closed"),
            Err(RecvError::Lagged(_)) => Err(Error::Lagging),
            Ok(event) => Ok(event),
        }
    }
    async fn dir_update(
        self,
        _: context::Context,
        id: SessionId,
        dir: DirContent,
    ) -> Result<DirUpdate, Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        Ok(self.world.get_update(dir))
    }
    async fn get_object(
        self,
        _: context::Context,
        id: SessionId,
        object: ObjectId,
    ) -> Result<Vec<u8>, Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        Ok(WorldDb::get_object(object).await?)
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
        self.clear_user_sessions(user_id);
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

    async fn dump_save(self, _: context::Context, dir: PathBuf) -> Result<(), Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }
        
        if !dir.exists() {
            return Err(Error::DirDoesNotExist);
        }

        self.world.dump_save(dir).await;
        Ok(())
    }

    async fn set_save(self, _: context::Context, dir: PathBuf) -> Result<(), Error> {
        if !self.peer_addr.ip().is_loopback() {
            return Err(Error::Unauthorized);
        }
        
        if !dir.exists() {
            return Err(Error::DirDoesNotExist);
        }

        self.world.load_save(dir.clone()).await?;
        info!("set save to whatever was in: {:?}", dir);
        Ok(())
    }
}
