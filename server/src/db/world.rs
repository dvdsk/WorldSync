use std::io;
use std::path::{Path, PathBuf};
use protocol::UserId;
use sync::{ObjectId, ObjectStore, Save};
use tokio::fs;
use tracing::instrument;
use typed_sled::{sled, Tree};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Coud not read obj: {1}, ran into error: {0:?}")]
    CantReadObj(io::ErrorKind, PathBuf),
}

impl From<Error> for protocol::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::CantReadObj(_,_) => protocol::Error::Internal,
        }
    }
}

type SaveId = (UserId, u8);

#[derive(Clone)]
pub struct WorldDb {
    db: sled::Db,
    objects: Tree<(PathBuf, u64), ObjectId>,
    saves: Tree<SaveId, Save>,
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
        let saves = Tree::open(&db, "saves");
        WorldDb { objects, db, saves }
    }

    #[instrument(err)]
    pub async fn get_object(id: ObjectId) -> Result<Vec<u8>, Error> {
        let mut path = PathBuf::from("object_store");
        path.push(id.0.to_string());

        fs::read(&path)
            .await
            .map_err(|e| Error::CantReadObj(e.kind(), path))
    }
}
