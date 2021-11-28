use std::io;
use std::path::{Path, PathBuf};
use sync::{DirContent, DirUpdate, ObjectId, ObjectStore, Save};
use tokio::fs;
use tracing::instrument;
use typed_sled::{sled, Tree};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Coud not read obj: {1}, ran into error: {0:?}")]
    CantReadObj(io::ErrorKind, PathBuf),
}

impl From<Error> for protocol::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::CantReadObj(_, _) => protocol::Error::Internal,
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
    pub fn from(db: sled::Db) -> Self {
        let objects = Tree::open(&db, "objects");
        let saves = db.open_tree("saves").unwrap();
        WorldDb { objects, db, saves }
    }

    fn last_save(&self) -> Save {
        match self.saves.last().unwrap() {
            Some((_, v)) => bincode::deserialize(&v).unwrap(),
            None => Save::new_empty(),
        }
    }

    #[instrument(err)]
    pub async fn get_object(id: ObjectId) -> Result<Vec<u8>, Error> {
        let mut path = PathBuf::from("object_store");
        path.push(id.0.to_string());

        fs::read(&path)
            .await
            .map_err(|e| Error::CantReadObj(e.kind(), path))
    }

    pub fn get_update_list(&self, dir: DirContent) -> DirUpdate {
        self.last_save().needed_update(dir)
    }
}
