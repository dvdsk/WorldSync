use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sync::{DirContent, DirUpdate, ObjectId, ObjectStore, Save};
use tokio::fs;
use tracing::instrument;
use typed_sled::{sled, Tree};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Coud not read obj: {1}, ran into error: {0:?}")]
    CantReadObj(io::ErrorKind, PathBuf),
    #[error("Coud not write obj: {1}, ran into error: {0:?}")]
    CantWriteObj(io::ErrorKind, PathBuf),
}

impl From<Error> for protocol::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::CantReadObj(_, _) => protocol::Error::Internal,
            Error::CantWriteObj(_, _) => protocol::Error::Internal,
        }
    }
}

#[derive(Clone)]
pub struct WorldDb {
    db: sled::Db,
    objects: Tree<(PathBuf, u64), ObjectId>,
    saves: sled::Tree, // save by a id (saveId)
}

impl ObjectStore for WorldDb {
    fn new_obj_id(&self) -> ObjectId {
        let id = self.db.generate_id().unwrap();
        ObjectId(id)
    }
    fn contains(&self, file: &Path, hash: u64) -> Option<ObjectId> {
        let key = (file.to_owned(), hash);
        self.objects.get(&key).unwrap()
    }
}

impl WorldDb {
    pub async fn from(db: sled::Db) -> Self {
        let objects = Tree::open(&db, "objects");
        let saves = db.open_tree("saves").unwrap();
        if !Self::obj_path().exists() {
            fs::create_dir(Self::obj_path()).await.unwrap();
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

    pub fn obj_path() -> &'static Path {
        Path::new("object_store")
    }

    #[instrument(err)]
    pub async fn get_object(id: ObjectId) -> Result<Vec<u8>, Error> {
        let mut path = Self::obj_path().to_owned();
        path.push(id.0.to_string());

        fs::read(&path)
            .await
            .map_err(|e| Error::CantReadObj(e.kind(), path))
    }

    #[instrument(err)]
    pub async fn add_obj(id: ObjectId, bytes: &[u8]) -> Result<(), Error> {
        let mut path = Self::obj_path().to_owned();
        path.push(id.0.to_string());
        dbg!(bytes.len());

        fs::write(&path, bytes)
            .await
            .map_err(|e| Error::CantWriteObj(e.kind(), path))
    }

    pub fn get_update_list(&self, dir: DirContent) -> DirUpdate {
        self.last_save().needed_update(dir)
    }
}
