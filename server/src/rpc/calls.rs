use std::path::PathBuf;

use crate::db::world::WorldDb;
use crate::host::HostEvent;
use sync::{DirContent, DirUpdate, ObjectId,UpdateList};
use wrapper::parser::Line;

use super::ConnState;
use protocol::{Error, Event, AWAIT_EVENT_TIMEOUT};
use protocol::{HostDetails, HostId, HostState, Service, SessionId, User, UserId};
use shared::tarpc;
use tarpc::context;
use tokio::sync::broadcast::error::RecvError;
use tracing::{info, instrument, warn};

#[tarpc::server]
impl Service for ConnState {
    async fn version(self, _: context::Context) -> protocol::Version {
        protocol::current_version()
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
                    "Incorrect password for user: '{}' from {:?}",
                    username, self.peer_addr
                );
                Err(Error::IncorrectLogin)
            }
            Err(DbError::IncorrectName) => {
                warn!(
                    "Incorrect username ({}) from {:?}",
                    username, self.peer_addr
                );
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

    async fn host(self, _: context::Context, id: SessionId) -> Result<HostState, Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        Ok(self.world.host.state.read().await.clone())
    }

    #[instrument(err, skip(self))]
    async fn request_to_host(
        self,
        _: context::Context,
        id: SessionId,
        host_id: HostId,
    ) -> Result<(), Error> {
        let user_id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let name = self.userdb.get_name(user_id)?.unwrap();
        let details = HostDetails {
            name,
            addr: self.peer_addr(),
            port: 25565,
            id: host_id,
        };
        self.host_req
            .send(HostEvent::RequestToHost(details))
            .await
            .unwrap();
        Ok(())
    }
    async fn await_event(self, _: context::Context, id: SessionId) -> Result<Event, Error> {
        let backlog = {
            let sessions = self.sessions.by_id.read().unwrap();
            let session = sessions.get(&id).ok_or(Error::SessionExpired)?;
            session.backlog.clone()
        };

        let mut backlog = backlog.try_lock_owned().map_err(|_| Error::BackLogLocked)?;

        let timeout_res = tokio::time::timeout(AWAIT_EVENT_TIMEOUT, backlog.recv()).await;
        match timeout_res {
            Err(_elapsed) => Ok(Event::AwaitTimeout),
            Ok(res) => match res {
                Err(RecvError::Closed) => panic!("events queue got closed"),
                Err(RecvError::Lagged(_)) => Err(Error::Lagging),
                Ok(event) => Ok(event),
            },
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
    #[instrument(err, skip(self, dir))]
    async fn new_save(
        mut self,
        _: context::Context,
        id: SessionId,
        host_id: HostId,
        dir: DirContent,
    ) -> Result<UpdateList, Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let _ = self.is_host(host_id).await?;
        let list = self.world.new_save(dir);
        Ok(list)
    }

    #[instrument(err, skip(self))]
    async fn register_save(
        mut self,
        _: context::Context,
        id: SessionId,
        host_id: HostId,
    ) -> Result<(), Error> {
        let id = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let _ = self.is_host(host_id).await?;
        self.world.flush_save()?;
        info!("user: {}, finished saving", id);

        Ok(())
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

    async fn put_object(
        self,
        _: context::Context,
        id: SessionId,
        host_id: HostId,
        object: ObjectId,
        path: PathBuf,
        bytes: Vec<u8>,
    ) -> Result<(), Error> {
        let _ = self.get_user_id(id).ok_or(Error::SessionExpired)?;
        let _ = self.is_host(host_id).await?;
        let _ = Self::path_safe(&path)?;
        Ok(self.world.add_obj(object, path, &bytes).await?)
    }

    #[instrument(err, skip(self))]
    async fn pub_mc_line(
        self,
        _: context::Context,
        host_id: HostId,
        line: Line,
    ) -> Result<(), Error> {
        let _ = self.is_host(host_id).await?;
        match HostEvent::try_from(line) {
            Ok(event) => self.host_req.send(event).await.unwrap(),
            Err(_) => (), // unprocessed line
        }
        Ok(())
    }

    async fn add_user(
        mut self,
        _: context::Context,
        user: User,
        password: String,
    ) -> Result<(), Error> {
        if !self.peer_addr().is_loopback() {
            return Err(Error::Unauthorized);
        }
        self.userdb.add_user(user.clone(), password).await?;
        info!("added user: {}", user.username);
        Ok(())
    }

    async fn list_users(self, _: context::Context) -> Result<Vec<(UserId, User)>, Error> {
        if !self.peer_addr().is_loopback() {
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
        if !self.peer_addr().is_loopback() {
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
        if !self.peer_addr().is_loopback() {
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
        if !self.peer_addr().is_loopback() {
            return Err(Error::Unauthorized);
        }
        let name = self.userdb.remove_user(id).await?;
        info!("removed user: {}", name);
        Ok(())
    }

    async fn dump_save(self, _: context::Context, dir: PathBuf) -> Result<(), Error> {
        if !self.peer_addr().is_loopback() {
            return Err(Error::Unauthorized);
        }

        if !dir.exists() {
            return Err(Error::DirDoesNotExist);
        }

        self.world.dump_save(dir.clone()).await?;
        info!("dumped last save into: {:?}", dir);
        Ok(())
    }

    async fn set_save(self, _: context::Context, dir: PathBuf) -> Result<(), Error> {
        if !self.peer_addr().is_loopback() {
            return Err(Error::Unauthorized);
        }

        if !dir.exists() {
            return Err(Error::DirDoesNotExist);
        }

        self.world.set_save(dir.clone()).await?;
        info!("set save to whatever was in: {:?}", dir);
        Ok(())
    }
}
