use core::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sync::{DirContent, DirUpdate, ObjectId, ObjectStore, Save, StoreKey};
use tokio::fs;
use tracing::instrument;
use typed_sled::{sled, Tree};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Coud not read obj: {1}, ran into error: {0:?}")]
    CantReadObj(io::ErrorKind, PathBuf),
    #[error("Coud not write obj: {1}, ran into error: {0:?}")]
    CantWriteObj(io::ErrorKind, PathBuf),
    #[error("Object was already present: {0:?}")]
    ObjectAlreadyPresent(#[from] typed_sled::CompareAndSwapError<ObjectId>),
}

impl From<Error> for protocol::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::CantReadObj(_, _) => protocol::Error::Internal,
            Error::CantWriteObj(_, _) => protocol::Error::Internal,
            Error::ObjectAlreadyPresent(_) => protocol::Error::Internal,
        }
    }
}

#[derive(Clone)]
pub struct WorldDb {
    db: sled::Db,
    objects: Tree<StoreKey, ObjectId>,
    saves: sled::Tree, // save by a id (saveId)
}

impl fmt::Debug for WorldDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldDb").finish()
    }
}

use async_trait::async_trait;
#[async_trait]
impl ObjectStore for WorldDb {
    type Error = Error;
    fn new_obj_id(&self) -> ObjectId {
        let id = self.db.generate_id().unwrap();
        ObjectId(id)
    }
    fn contains(&self, key: &StoreKey) -> Option<ObjectId> {
        self.objects.get(key).unwrap()
    }
    async fn store_obj(
        &self,
        id: ObjectId,
        path: PathBuf,
        bytes: &[u8],
    ) -> Result<(), Self::Error> {
        let obj_path = Self::obj_path(id);
        fs::write(&obj_path, bytes)
            .await
            .map_err(|e| Error::CantWriteObj(e.kind(), obj_path))?;

        let key = StoreKey::calc_from(path, bytes);
        self.objects
            .compare_and_swap(&key, None, Some(&id))
            .unwrap()?;
        self.objects.flush_async().await.unwrap();
        Ok(())
    }
    fn store_path() -> &'static Path {
        Path::new("object-store")
    }
    async fn retrieve_obj(id: ObjectId) -> Result<Vec<u8>, Self::Error> {
        let path = Self::obj_path(id);
        fs::read(&path)
            .await
            .map_err(|e| Error::CantReadObj(e.kind(), path))
    }
}

impl WorldDb {
    pub async fn from(db: sled::Db) -> Self {
        let objects = Tree::open(&db, "objects");
        let saves = db.open_tree("saves").unwrap();
        if !Self::store_path().exists() {
            fs::create_dir(Self::store_path()).await.unwrap();
        }

        WorldDb { objects, db, saves }
    }

    pub fn last_save(&self) -> Save {
        match self.saves.last().unwrap() {
            Some((_, v)) => bincode::deserialize(&v).unwrap(),
            None => Save::new_empty(),
        }
    }

    pub fn push_save(&self, save: Save) {
        // TODO on what key do we insert saves?
        // for now use time
        let unix_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let key = unix_timestamp.as_secs().to_be_bytes();
        let bytes = bincode::serialize(&save).unwrap();
        self.saves.insert(key, bytes).unwrap();
    }

    #[instrument(err)]
    pub async fn get_object(id: ObjectId) -> Result<Vec<u8>, Error> {
        Self::retrieve_obj(id).await
    }

    #[instrument(err)]
    pub async fn add_obj(&self, id: ObjectId, path: PathBuf, bytes: &[u8]) -> Result<(), Error> {
        self.store_obj(id, path, bytes).await
    }

    pub fn get_update_list(&self, dir: DirContent) -> DirUpdate {
        self.last_save().needed_update(dir)
    }
}
