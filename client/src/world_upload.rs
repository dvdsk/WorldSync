use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::gui::hosting::Event as hEvent;
use futures::stream::{self, BoxStream};
use protocol::HostId;
use shared::tarpc::client::RpcError;
use shared::tarpc::context;
use sync::{DirContent, ObjectId, UpdateList};
use tokio::fs;
use tracing::{error, instrument, debug};

use crate::gui::{hosting, RpcConn};
use crate::{server_path, Event};

pub fn sub(conn: RpcConn, count: usize, host_id: HostId) -> iced::Subscription<Event> {
    iced::Subscription::from_recipe(WorldUpload {
        conn: Cell::new(Some(conn)),
        host_id: Some(host_id),
        count,
    })
}

pub struct WorldUpload {
    conn: Cell<Option<RpcConn>>,
    host_id: Option<HostId>,
    count: usize,
}

#[derive(Debug)]
enum Phase {
    Started,
    Uploading,
    SetSave,
    End,
}

#[derive(Debug)]
struct State {
    conn: RpcConn,
    phase: Phase,
    host_id: HostId,
    object_list: Option<UpdateList>,
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("Lost connection to worldsync server: {0:?}")]
    NoMetaConn(#[from] RpcError),
    #[error("Could not access local file system, is folder read only or hard drive full?")]
    Fs,
    #[error("{0}")]
    Protocol(#[from] protocol::Error),
    #[error("Could not sync save")]
    SyncError,
}

impl From<sync::Error> for Error {
    fn from(_: sync::Error) -> Self {
        Error::Fs
    }
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::Fs
    }
}

impl<H, I> iced_native::subscription::Recipe<H, I> for WorldUpload
where
    H: Hasher,
{
    type Output = Event;

    fn hash(&self, state: &mut H) {
        struct Marker;
        self.count.hash(state);
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(
        mut self: Box<Self>,
        _input: BoxStream<'static, I>,
    ) -> BoxStream<'static, Self::Output> {
        Box::pin(stream::unfold(
            State {
                conn: self.conn.replace(None).unwrap(),
                phase: Phase::Started,
                host_id: self.host_id.take().unwrap(),
                object_list: None,
            },
            move |state| async move {
                match state.phase {
                    Phase::Started => Some(state.build_updatelist().await),
                    Phase::Uploading => Some(state.upload_objects().await),
                    Phase::SetSave => Some(state.register_save().await),
                    Phase::End => None,
                }
            },
        ))
    }
}

impl State {
    #[instrument(err)]
    async fn do_build_updatelist(&mut self) -> Result<UpdateList, Error> {
        let dir = DirContent::from_dir(server_path().into())
            .await
            .map_err(|_| Error::SyncError)?;
        assert_ne!(dir.len(), 0, "dircontent should never be empty");

        let to_upload = self
            .conn
            .client
            .new_save(context::current(), self.conn.session, self.host_id, dir)
            .await
            .expect("rpc error")?;
        Ok(to_upload)
    }

    async fn build_updatelist(mut self) -> (Event, Self) {
        let event = match self.do_build_updatelist().await {
            Ok(list) => {
                let num_obj = list.0.len();
                self.object_list = Some(list);
                self.phase = Phase::Uploading;
                hosting::Event::UploadStarting(num_obj)
            }
            Err(e) => {
                self.phase = Phase::End;
                hosting::Event::Error(e.into())
            }
        };
        (Event::HostingPage(event), self)
    }

    async fn upload_objects(mut self) -> (Event, Self) {
        let item = self.object_list.as_mut().unwrap().0.pop();
        let event = match item {
            Some((id, path)) => match self.upload_obj(id, &path).await {
                Ok(_) => {
                    let list = self.object_list.as_mut().unwrap();
                    let obj_left = list.0.len();
                    self.phase = Phase::Uploading;
                    hEvent::Uploading(obj_left)
                }
                Err(e) => {
                    self.phase = Phase::End;
                    hEvent::Error(e.into())
                }
            },
            None => {
                self.phase = Phase::SetSave;
                hEvent::UploadDone
            }
        };
        (Event::HostingPage(event), self)
    }

    #[instrument(err)]
    async fn upload_obj(&mut self, obj_id: ObjectId, path: &Path) -> Result<(), Error> {
        let bytes = fs::read(local_path(path)).await?;
        let bytes = self
            .conn
            .client
            .put_object(
                shared::context(2 * 60),
                self.conn.session,
                self.host_id,
                obj_id,
                path.to_owned(),
                bytes,
            )
            .await??;
        Ok(bytes)
    }

    #[instrument(err)]
    async fn do_register_save(&mut self) -> Result<(), Error> {
        debug!("{:?}", self);
        self.conn
            .client
            .register_save(
                context::current(),
                self.conn.session,
                self.host_id,
            )
            .await??;
        Ok(())
    }

    async fn register_save(mut self) -> (Event, Self) {
        let event = match self.do_register_save().await {
            Ok(_) => hEvent::SaveRegisterd,
            Err(e) => hEvent::Error(e.into()),
        };
        self.phase = Phase::End;

        (Event::HostingPage(event), self)
    }
}

fn local_path(remote_path: &Path) -> PathBuf {
    Path::new(server_path()).join(remote_path)
}
