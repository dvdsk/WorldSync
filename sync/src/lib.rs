use futures::future::join_all;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::task;
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub org_path: PathBuf,
    hash: u64,
    pub id: ObjectId,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Ran into an error while walking through save dir: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("Could not open file in save: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Save(Vec<Object>);

impl Save {
    pub fn new_empty() -> Self {
        Self(Vec::new())
    }
    pub fn objects(&self) -> &Vec<Object> {
        &self.0
    }
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

pub trait ObjectStore {
    fn contains(&self, file: &Path, hash: u64) -> Option<ObjectId>;
    fn new_obj_id(&self) -> ObjectId;
}

impl UpdateList {
    pub fn for_new_save(store: &impl ObjectStore, remote: DirContent) -> (Save, UpdateList) {
        let mut new_objects = Vec::new();
        let mut new_save = Vec::new();
        for file in remote.0 {
            let obj_id = match store.contains(&file.path, file.hash) {
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
            })
        }
        (Save(new_save), UpdateList(new_objects))
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
/// list of the paths on the remote that need to be uploaded
/// and the objectid they should be assigned
#[derive(Debug, Clone)]
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
    pub path: PathBuf,
    pub hash: u64,
}

impl FileStatus {
    async fn new(path: PathBuf) -> Result<FileStatus, Error> {
        let mut file = File::open(&path).await?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;

        let hash = task::spawn_blocking(move || seahash::hash(&bytes));
        Ok(FileStatus {
            hash: hash.await.expect("error joining hash task"),
            path,
        })
    }
}

impl DirContent {
    #[instrument(err)]
    fn build_file_list(path: &Path) -> Result<Vec<PathBuf>, Error> {
        let mut paths = Vec::new();
        for res in WalkDir::new(path) {
            let entry = res?;
            if entry.file_type().is_dir() {
                continue;
            }

            paths.push(entry.into_path());
        }
        Ok(paths)
    }

    #[instrument(err)]
    pub async fn from_path(path: PathBuf) -> Result<Self, Error> {
        let paths = task::spawn_blocking(move || Self::build_file_list(&path))
            .await
            .expect("error joining dirwalker task");

        DirContent::from_file_list(paths?).await
    }

    pub async fn from_file_list(paths: Vec<PathBuf>) -> Result<Self, Error> {
        let make_filecheck = paths.into_iter().map(FileStatus::new);
        let results = join_all(make_filecheck).await;
        let results: Result<_, _> = results.into_iter().collect();
        let checks = results?;

        Ok(DirContent(checks))
    }
}
