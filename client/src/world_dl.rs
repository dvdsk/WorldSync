use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::gui::host::Event as hEvent;
use futures::stream::{self, BoxStream};
use protocol::tarpc::client::RpcError;
use protocol::tarpc::context;
use sync::{DirContent, DirUpdate, ObjectId, SyncAction};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, instrument};

use crate::gui::RpcConn;
use crate::Event;

pub fn sub(conn: RpcConn, count: usize) -> iced::Subscription<Event> {
    iced::Subscription::from_recipe(WorldDl {
        conn: Cell::new(Some(conn)),
        count,
    })
}

pub struct WorldDl {
    conn: Cell<Option<RpcConn>>,
    count: usize,
}

#[derive(Debug)]
enum Phase {
    Started,
    Updating,
    End,
}

#[derive(Debug)]
struct State {
    conn: RpcConn,
    phase: Phase,
    updates: Option<DirUpdate>,
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq, Hash)]
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
        self.count.hash(state);
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        Box::pin(stream::unfold(
            State {
                conn: self.conn.replace(None).unwrap(),
                phase: Phase::Started,
                updates: None,
            },
            move |state| async move {
                match &state.phase {
                    Phase::Started => Some(state.await_dir_update().await),
                    Phase::Updating => Some(state.apply_updates().await),
                    Phase::End => None,
                }
            },
        ))
    }
}

use crate::gui::host;
impl State {
    #[instrument(err)]
    async fn get_dir_update(&mut self) -> Result<DirUpdate, Error> {
        if !Path::new("server").is_dir() {
            let full_path = fs::canonicalize("server").await.unwrap();
            info!("created directory for server in: {:?}", full_path);
            fs::create_dir("server").await.unwrap();
        }
        let dir_content = DirContent::from_path("server".into()).await?;
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
                let num_obj = update_list.0.len();
                self.phase = Phase::Updating;
                self.updates = Some(update_list);
                Event::HostPage(host::Event::DlStarting { num_obj })
            }
            Err(e) => {
                self.phase = Phase::End;
                Event::HostPage(host::Event::Error(e.into()))
            }
        };

        (event, self)
    }

    // TODO improve, spawn a few tasks and run request concurrently
    async fn apply_updates(self) -> (Event, Self) {
        let Self {
            mut conn,
            mut phase,
            mut updates,
        } = self;
        let list = updates.as_mut().unwrap();
        match list.0.pop() {
            Some(action) => match apply_action(&mut conn, action).await {
                Ok(_) => {
                    let left = list.0.len();
                    let progress = hEvent::ObjToSync { left };
                    let state = Self {
                        conn,
                        phase,
                        updates,
                    };
                    (Event::HostPage(progress), state)
                }
                Err(e) => {
                    let event = hEvent::Error(e.into());
                    phase = Phase::End;
                    let state = Self {
                        conn,
                        phase,
                        updates,
                    };
                    (Event::HostPage(event), state)
                }
            },
            None => {
                let state = Self {
                    conn,
                    phase: Phase::End,
                    updates,
                };
                (Event::WorldUpdated, state)
            }
        }
    }
}

async fn apply_action(conn: &mut RpcConn, action: SyncAction) -> Result<(), Error> {
    match action {
        SyncAction::Remove(path) => fs::remove_file(path).await?,
        SyncAction::Replace(path, id) => {
            let bytes = download_obj(conn, id).await?;
            let mut file = fs::OpenOptions::new()
                .truncate(true)
                .write(true)
                .open(path)
                .await?;
            file.write_all(&bytes).await?;
        }
        SyncAction::Add(path, id) => {
            let bytes = download_obj(conn, id).await?;
            let mut file = fs::OpenOptions::new().create_new(true).open(path).await?;
            file.write_all(&bytes).await?;
        }
    }
    Ok(())
}

async fn download_obj(conn: &mut RpcConn, id: ObjectId) -> Result<Vec<u8>, Error> {
    let bytes = conn
        .client
        .get_object(context::current(), conn.session, id)
        .await??;
    Ok(bytes)
}
