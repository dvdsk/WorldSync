use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;

use sync::{DirStatus, DirUpdate, FileStatus, ObjectId, ObjectStore, SyncAction, UpdateList};

#[derive(Default)]
pub struct Objects {
    free_id: AtomicU64,
    map: HashMap<PathBuf, (ObjectId, u64)>,
}

impl ObjectStore for Objects {
    fn new_obj_id(&self) -> ObjectId {
        let id = self.free_id.fetch_add(1, Relaxed);
        ObjectId(id)
    }
    fn contains(&self, file: &Path, hash: u64) -> Option<ObjectId> {
        match self.map.get(file) {
            Some((id, stored_hash)) if *stored_hash == hash => Some(*id),
            Some(_) => None,
            None => None,
        }
    }
}

#[test]
fn load_and_save() {
    let store = Objects::default();

    let remote_a = DirStatus(vec![
        FileStatus {
            path: PathBuf::from("none_existing_dir/applesaus"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/foo.txt"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/world1_mca.mca"),
            hash: 469007863229145464,
        },
        FileStatus {
            path: PathBuf::from("none_existing_dir/missing_in_b.mca"),
            hash: 4242,
        },
    ]);
    let (new_save, update_list) = UpdateList::for_new_save(&store, remote_a);
    assert!(update_list.0.len() == 4);

    let remote_b = DirStatus(vec![
        FileStatus {
            path: PathBuf::from("none_existing_dir/applesaus"),
            hash: 469007863229145464,
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
            path: PathBuf::from("none_existing_dir/extra_file.mca"),
            hash: 4242,
        },
    ]);
    let update = new_save.needed_update(remote_b);
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
