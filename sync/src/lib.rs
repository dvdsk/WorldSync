use futures::future::join_all;
pub use seahash::hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::task;
use tracing::instrument;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub org_path: PathBuf,
    pub hash: u64,
    pub id: ObjectId,
    pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Ran into an error while walking through save dir: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("Could not open file in save: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapShot(Vec<Object>);

impl SnapShot {
    pub fn append(&mut self, other: Self) {
        self.0.extend(other.into_iter())
    }

    pub fn size(&self) -> u64 {
        self.0.iter().map(|obj| obj.size).sum()
    }
    pub fn into_iter(self) -> impl Iterator<Item = Object> {
        self.0.into_iter()
    }

    pub fn new_empty() -> Self {
        Self(Vec::new())
    }
    pub fn objects(&self) -> &Vec<Object> {
        &self.0
    }
    /// given a remote directorys content return the changes needed to
    /// turn the remote into this save
    pub fn needed_update(&self, remote: DirContent) -> DirUpdate {
        use SyncAction::*;

        let mut update = Vec::new();
        let mut remote: HashMap<PathBuf, u64> =
            remote.0.into_iter().map(|t| (t.path, t.hash)).collect();
        for obj in &self.0 {
            match remote.remove_entry(&obj.org_path) {
                None => update.push(Add(obj.org_path.clone(), obj.id)),
                Some((_, hash)) if hash == obj.hash => continue,
                Some((path, _)) => update.push(Replace(path, obj.id)),
            }
        }

        for (path, _) in remote.into_iter() {
            update.push(Remove(path));
        }

        DirUpdate(update)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreKey {
    pub path: PathBuf,
    pub hash: u64,
}
impl StoreKey {
    pub fn from(path: PathBuf, hash: u64) -> Self {
        Self { path, hash }
    }
    pub fn calc_from(path: PathBuf, bytes: &[u8]) -> Self {
        let hash = hash(bytes);
        Self::from(path, hash)
    }
}

use async_trait::async_trait;
#[async_trait]
pub trait ObjectStore {
    type Error;
    fn new_obj_id(&self) -> ObjectId;
    fn contains(&self, key: &StoreKey) -> Option<ObjectId>;
    async fn store_obj(&self, id: ObjectId, path: PathBuf, bytes: &[u8])
        -> Result<(), Self::Error>;
    async fn retrieve_obj(id: ObjectId) -> Result<Vec<u8>, Self::Error>;
    fn store_path() -> &'static Path;
    fn obj_path(id: ObjectId) -> PathBuf {
        let mut path = Self::store_path().to_owned();
        path.push(id.0.to_string());
        path
    }
}

impl UpdateList {
    pub fn into_iter(self) -> impl Iterator<Item = (ObjectId, PathBuf)> {
        self.0.into_iter()
    }

    /// return the Save and determine the objects we need to add to be able
    /// to load the save later
    pub fn for_new_save(store: &impl ObjectStore, remote: DirContent) -> (SnapShot, UpdateList) {
        let mut new_objects = Vec::new();
        let mut new_save = Vec::new();
        for file in remote.0 {
            let key = StoreKey::from(file.path.clone(), file.hash);
            let obj_id = match store.contains(&key) {
                Some(obj_id) => obj_id,
                None => {
                    let id = store.new_obj_id();
                    new_objects.push((id, file.path.clone()));
                    id
                }
            };

            new_save.push(Object {
                org_path: file.path,
                hash: file.hash,
                id: obj_id,
                size: file.size,
            })
        }
        (SnapShot(new_save), UpdateList(new_objects))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum SyncAction {
    Replace(PathBuf, ObjectId),
    Remove(PathBuf),
    Add(PathBuf, ObjectId),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjectId(pub u64);
/// list of (relative) paths on the remote that need to be uploaded
/// and the objectid they should be assigned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateList(pub Vec<(ObjectId, PathBuf)>);

/// list of actions needed to get a local directory
/// up to date with the central server.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DirUpdate(pub Vec<SyncAction>);
/// list of paths with hashes that a central server can compare
/// to a known save and calculate the diffrences
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DirContent(pub Vec<FileStatus>);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileStatus {
    /// relative path
    pub path: PathBuf,
    pub hash: u64,
    pub size: u64,
}

// maximum number of concurrent file operations, keeping a file in memory
// might take a lot of memory and the operation (such as hashing) can
// take quite some cpu. This limits such cases (needed for pi3)
//
// an alternative would be to set the tokio max_blocking_threads lower (def 512)
// use tokio::sync::Semaphore;
// static CONCURRENT_FILE_OPS: Semaphore = Semaphore::const_new(1);

impl FileStatus {
    #[instrument(err)]
    async fn new(path: PathBuf, base: PathBuf) -> Result<FileStatus, Error> {
        // limit the number of reads and hashes to save on memory
        // let _permit = CONCURRENT_FILE_OPS.acquire().await.unwrap();
        let mut file = File::open(&path).await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        let size = bytes.len() as u64;

        let hash = task::spawn_blocking(move || seahash::hash(&bytes));

        let path = path
            .strip_prefix(base)
            .map(|p| p.to_owned())
            .unwrap_or(path);

        Ok(FileStatus {
            hash: hash.await.expect("error joining hash task"),
            path,
            size,
        })
    }
}

impl DirContent {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn build_file_list(dir: &Path) -> Result<Vec<PathBuf>, Error> {
        let mut paths = Vec::new();
        for res in WalkDir::new(dir) {
            let entry = res.unwrap(); //?;
            if entry.file_type().is_dir() {
                continue;
            }
            let path = entry.into_path();
            paths.push(path);
        }
        Ok(paths)
    }

    #[instrument(err)]
    pub async fn from_dir(dir: PathBuf) -> Result<Self, Error> {
        let dir_clone = dir.clone();
        let paths = task::spawn_blocking(move || Self::build_file_list(&dir_clone))
            .await
            .expect("error joining dirwalker task");

        DirContent::from_file_list(paths?, &dir).await
    }

    #[instrument(err)]
    pub async fn from_file_list(paths: Vec<PathBuf>, base: &Path) -> Result<Self, Error> {
        let make_filecheck = paths
            .into_iter()
            .map(|p| FileStatus::new(p, base.to_owned()));
        let results = join_all(make_filecheck).await;
        let results: Result<_, _> = results.into_iter().collect();
        let checks = results?;

        Ok(DirContent(checks))
    }
}

pub trait PathCheck {
    fn is_safe(&self, path: impl AsRef<Path>) -> bool;
}


/// replace any path in self that is not on the allowed list with one
/// of a known safe Save or remove the path if it is not in a previous
/// Save or on the allowed list.
pub fn secure_snapshot(
    unchecked: (SnapShot, UpdateList),
    reference: SnapShot,
    check: impl PathCheck, //HashSet<&Path>,
) -> (SnapShot, UpdateList) {
    let mut safe_obj: HashMap<PathBuf, Object> = reference
        .into_iter()
        .map(|o| (o.org_path.clone(), o))
        .collect();
    let checked_save = unchecked
        .0
        .into_iter()
        .filter_map(|obj| match check.is_safe(&obj.org_path) {
            true => Some(obj),
            false => safe_obj.remove(&obj.org_path),
        })
        .collect();
    let checked_list = unchecked
        .1
        .into_iter()
        .filter(|(_, path)| check.is_safe(path))
        .collect();

    (SnapShot(checked_save), UpdateList(checked_list))
}
