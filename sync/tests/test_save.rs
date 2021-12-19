use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Mutex;

use sync::{
    DirContent, DirUpdate, FileStatus, ObjectId, ObjectStore, StoreKey, SyncAction, UpdateList,
};

#[derive(Default)]
pub struct Objects {
    free_id: AtomicU64,
    map: Mutex<HashMap<StoreKey, ObjectId>>,
}

use async_trait::async_trait;
#[async_trait]
impl ObjectStore for Objects {
    type Error = ();
    fn new_obj_id(&self) -> ObjectId {
        let id = self.free_id.fetch_add(1, Relaxed);
        ObjectId(id)
    }
    fn contains(&self, key: &StoreKey) -> Option<ObjectId> {
        self.map.lock().unwrap().get(key).cloned()
    }
    async fn store_obj(
        &self,
        id: ObjectId,
        path: PathBuf,
        bytes: &[u8],
    ) -> Result<(), Self::Error> {
        let key = dbg!(StoreKey::calc_from(path, bytes));
        match self.map.lock().unwrap().insert(key, id) {
            Some(_) => Err(()),
            None => Ok(()),
        }
    }
    fn store_path() -> &'static Path {
        Path::new("placeholder")
    }
    async fn retrieve_obj(_id: ObjectId) -> Result<Vec<u8>, Self::Error> {
        unimplemented!()
    }
}

fn remote_a() -> DirContent {
    DirContent(vec![
        FileStatus {
            path: PathBuf::from("none_existing_dir/applesaus"),
            hash: 2725998475414856250,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/foo.txt"),
            hash: 42,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/world1_mca.mca"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/missing_in_b.mca"),
            hash: 4242,
        },
    ])
}

// matches hashes for fake_files
fn remote_b() -> DirContent {
    DirContent(vec![
        FileStatus {
            path: PathBuf::from("none_existing_dir/applesaus"),
            hash: 2725998475414856250,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/foo.txt"),
            hash: 10940344417258880963,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/world1_mca.mca"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/extra_file.mca"),
            hash: 4242,
        },
    ])
}

fn fake_files() -> Vec<(&'static str, [u8; 2])> {
    vec![
        ("none_existing_dir/applesaus", [9u8, 1u8]),
        ("none_existing_dir/foo.txt", [34u8, 2u8]),
        ("none_existing_dir/world1_mca.mca", [8u8, 3u8]),
        ("none_existing_dir/extra_file.mca", [45u8, 72u8]),
    ]
}

mod partial_store {
    use super::*;

    #[tokio::test]
    async fn save() {
        let store = Objects::default();

        for (path, bytes) in fake_files().iter().take(2) {
            let id = store.new_obj_id();
            let path = PathBuf::from(path);
            store.store_obj(id, path, bytes).await.unwrap();
        }

        let (_new_save, update_list) = UpdateList::for_new_save(&store, remote_b());
        assert_eq!(update_list.0.len(), 2);
    }
}

mod empty_store {
    use super::*;

    #[test]
    fn load_and_save() {
        let store = Objects::default();

        let (new_save, update_list) = UpdateList::for_new_save(&store, remote_a());
        assert_eq!(update_list.0.len(), 4);

        let update = new_save.needed_update(remote_b());
        assert_eq!(
            update,
            DirUpdate(vec![
                SyncAction::Replace(PathBuf::from("none_existing_dir/foo.txt"), ObjectId(1)),
                SyncAction::Add(
                    PathBuf::from("none_existing_dir/missing_in_b.mca"),
                    ObjectId(3)
                ),
                SyncAction::Remove(PathBuf::from("none_existing_dir/extra_file.mca"))
            ])
        )
    }
}
