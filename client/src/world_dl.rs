use std::cell::Cell;
use std::hash::{Hash, Hasher};

use crate::gui::host::Event as hEvent;
use futures::stream::{self, BoxStream};
use protocol::tarpc::client::RpcError;
use protocol::tarpc::context;
use sync::{DirContent, DirUpdate, ObjectId, SyncAction};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::error;

use crate::gui::RpcConn;
use crate::Event;

pub struct WorldDl {
    conn: Cell<Option<RpcConn>>,
}

impl WorldDl {
    pub fn with_rpc(conn: RpcConn) -> Self {
        Self {
            conn: Cell::new(Some(conn)),
        }
    }
}

enum Phase {
    Started,
    Updating(DirUpdate),
    Done,
    End,
}

struct State {
    conn: RpcConn,
    phase: Phase,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Lost connection to meta conn")]
    NoMetaConn,
    #[error("Could not access local file system, is folder read only or hard drive full?")]
    FsError,
    #[error("{0}")]
    Protocol(#[from] protocol::Error),
}

impl From<RpcError> for Error {
    fn from(_: RpcError) -> Self {
        Error::NoMetaConn
    }
}

impl From<sync::Error> for Error {
    fn from(_: sync::Error) -> Self {
        Error::FsError
    }
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::FsError
    }
}

impl<H, I> iced_native::subscription::Recipe<H, I> for WorldDl
where
    H: Hasher,
{
    type Output = Event;

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        Box::pin(stream::unfold(
            State {
                conn: self.conn.replace(None).unwrap(),
                phase: Phase::Started,
            },
            move |mut state| async move {
                match state.phase {
                    Phase::Started => Some(state.await_dir_update().await),
                    Phase::Updating(update_list) => Some(state.apply(update_list).await),
                    Phase::Done => Some((Event::WorldUpdated, state)),
                    Phase::End => None,
                }
            },
        ))
    }
}

use crate::gui::host;
impl State {
    async fn get_dir_update(&mut self) -> Result<DirUpdate, Error> {
        let dir_content = DirContent::from_path("").await?;
        let dir_update = self
            .conn
            .client
            .dir_update(context::current(), self.conn.session, dir_content)
            .await??;
        Ok(dir_update)
    }

    async fn await_dir_update(mut self) -> (Event, Self) {
        let event = match self.get_dir_update().await {
            Ok(update_list) => {
                self.phase = Phase::Updating(update_list);
                Event::HostPage(host::Event::DlStarting)
            }
            Err(e) => Event::HostPage(host::Event::Error(e.into())),
        };

        (event, self)
    }

    async fn download_obj(&mut self, id: ObjectId) -> Result<Vec<u8>, Error> {
        let bytes = self
            .conn
            .client
            .get_object(context::current(), self.conn.session, id)
            .await??;
        Ok(bytes)
    }

    // TODO improve, spawn a few tasks and run request concurrently
    async fn apply_action(&mut self, action: SyncAction) -> Result<(), Error> {
        match action {
            SyncAction::Remove(path) => fs::remove_file(path).await?,
            SyncAction::Replace(path, id) => {
                let bytes = self.download_obj(id).await?;
                let mut file = fs::OpenOptions::new()
                    .truncate(true)
                    .write(true)
                    .open(path)
                    .await?;
                file.write_all(&bytes).await?;
            }
            SyncAction::Add(path, id) => {
                let bytes = self.download_obj(id).await?;
                let mut file = fs::OpenOptions::new().create_new(true).open(path).await?;
                file.write_all(&bytes).await?;
            }
        }
        Ok(())
    }
    async fn apply(mut self, mut update_list: DirUpdate) -> (Event, Self) {
        match update_list.0.pop() {
            Some(action) => match self.apply_action(action).await {
                Ok(_) => {
                    let left = update_list.0.len();
                    let progress = hEvent::ObjToSync { left };
                    (Event::HostPage(progress), self)
                }
                Err(e) => {
                    let event = hEvent::Error(e.into());
                    self.phase = Phase::End;
                    (Event::HostPage(event), self)

                }
            },
            None => {
                self.phase = Phase::End;
                (Event::WorldUpdated, self)
            }
        }
    }
}
