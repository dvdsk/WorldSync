use protocol::{HostId, HostState, Platform};
use serde::{Deserialize, Serialize};
use shared::dir_empty;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use sync::{DirContent, DirUpdate, ObjectId, ObjectStore, SnapShot, UpdateList};
use tracing::{debug, info, instrument};
use typed_sled::sled;

use crate::db::world::WorldDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platforms {
    windows: SnapShot,
    linux: SnapShot,
}

impl Platforms {
    pub fn platform(self, platform: Platform) -> SnapShot {
        match platform {
            Platform::Linux => self.linux,
            Platform::Windows => self.windows,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Save {
    /// user modifiable data (saves, databases)
    pub data: SnapShot,
    /// executables, may be platform specific
    runtime: Platforms,
}

impl Save {
    pub fn new_empty() -> Self {
        Save {
            data: SnapShot::new_empty(),
            runtime: Platforms {
                linux: SnapShot::new_empty(),
                windows: SnapShot::new_empty(),
            },
        }
    }

    pub fn objects(self, platform: Platform) -> impl Iterator<Item = sync::Object> {
        let data = self.data.into_iter();
        let runtime = match platform {
            Platform::Linux => self.runtime.linux.into_iter(),
            Platform::Windows => self.runtime.windows.into_iter(),
        };
        data.chain(runtime)
    }

    pub fn needed_update(self, remote: DirContent, platform: Platform) -> DirUpdate {
        use sync::SyncAction::*;

        let mut update = Vec::new();
        let mut remote: HashMap<PathBuf, u64> =
            remote.0.into_iter().map(|t| (t.path, t.hash)).collect();
        for obj in self.data.into_iter() {
            match remote.remove_entry(&obj.org_path) {
                None => update.push(Add(obj.org_path.clone(), obj.id)),
                Some((_, hash)) if hash == obj.hash => continue,
                Some((path, _)) => update.push(Replace(path, obj.id)),
            }
        }
        for obj in self.runtime.platform(platform).into_iter() {
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

#[derive(Clone, Debug)]
pub struct World {
    db: WorldDb,
    new_save: Arc<Mutex<Option<Save>>>,
    pub host: crate::host::Host,
}

impl World {
    pub async fn from(db: sled::Db, host: crate::host::Host) -> Self {
        Self {
            db: WorldDb::from(db).await,
            new_save: Arc::new(Mutex::new(None)),
            host,
        }
    }

    pub fn get_update(&self, dir: DirContent, platform: Platform) -> DirUpdate {
        self.db.get_update_list(dir, platform)
    }

    pub async fn dump_snapshot(dir: &Path, snap: SnapShot) {
        for sync::Object { org_path, id, .. } in snap.into_iter() {
            let source = WorldDb::obj_path(id);
            let target = dir.join(&org_path);
            match target.parent() {
                Some(path) if !path.is_dir() => fs::create_dir_all(path).unwrap(),
                _ => (),
            }
            tokio::fs::copy(source, target).await.unwrap();
        }
    }

    pub async fn dump_server(
        &self,
        target: &Path,
        platform: Platform,
    ) -> Result<(), protocol::Error> {
        if !dir_empty(&target).await {
            return Err(protocol::Error::NotEmpty);
        }

        let save = self.db.last_save();
        Self::dump_snapshot(target, save.data.clone()).await;
        Self::dump_snapshot(target, save.runtime.platform(platform)).await;
        info!("dumped server to: {:?}", target);
        Ok(())
    }

    #[instrument(err)]
    pub async fn dump_save(&self, target: &Path) -> Result<(), protocol::Error> {
        if !dir_empty(target).await {
            return Err(protocol::Error::NotEmpty);
        }

        let save = self.db.last_save();
        Self::dump_snapshot(&target.join("data"), save.data).await;
        Self::dump_snapshot(&target.join("platform/windows"), save.runtime.windows).await;
        Self::dump_snapshot(&target.join("platform/linux"), save.runtime.linux).await;

        info!("dumped save to: {:?}", target);
        Ok(())
    }

    pub async fn is_host(&self, id: HostId) -> Result<(), ()> {
        match &*self.host.state.read().await {
            HostState::Up(host) | HostState::Loading(host) | HostState::Unreachable(host) => {
                if host.id != id {
                    Err(())
                } else {
                    Ok(())
                }
            }
            _ => Err(()),
        }
    }

    pub fn new_save(&mut self, content: DirContent) -> UpdateList {
        let unchecked = UpdateList::for_new_save(&self.db, content);
        let (snapshot, list) = self.db.secure_snapshot(unchecked);
        let mut save = self.db.last_save();
        save.data = snapshot;
        *self.new_save.lock().unwrap() = Some(save);
        list
    }

    pub fn flush_save(&mut self) -> Result<(), protocol::Error> {
        let save = self
            .new_save
            .lock()
            .unwrap()
            .take()
            .ok_or(protocol::Error::NotSaving)?;
        self.db.push_save(save);
        Ok(())
    }

    async fn add_snapshot(&self, path: PathBuf) -> Result<SnapShot, protocol::Error> {
        let content = DirContent::from_dir(path.clone()).await.unwrap();
        let (snapshot, update_list) = UpdateList::for_new_save(&self.db, content);
        for (object_id, local_path) in update_list.0 {
            let full_path = path.join(&local_path);
            let bytes = tokio::fs::read(full_path).await.unwrap();
            self.add_obj(object_id, local_path.clone(), &bytes).await?;
            debug!("added object: {:?}", local_path);
        }
        Ok(snapshot)
    }

    pub async fn set_save(&self, source: PathBuf) -> Result<(), protocol::Error> {
        match *self.host.state.read().await {
            HostState::NoHost => (),
            _ => return Err(protocol::Error::SaveInUse),
        }

        let save = Save {
            data: self.add_snapshot(source.join("data")).await?,
            runtime: Platforms {
                windows: self.add_snapshot(source.join("platform/windows")).await?,
                linux: self.add_snapshot(source.join("platform/linux")).await?,
            },
        };
        self.db.push_save(save);
        info!("loaded and set save from: {:?}", source);

        Ok(())
    }

    pub async fn add_obj(
        &self,
        id: ObjectId,
        path: PathBuf,
        bytes: &[u8],
    ) -> Result<(), protocol::Error> {
        Ok(self.db.add_obj(id, path, bytes).await?)
    }
}
