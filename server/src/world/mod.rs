use std::path::PathBuf;
use protocol::HostState;
use sync::{DirContent, DirUpdate, UpdateList};
use tracing::{info, instrument};
use typed_sled::sled;

use crate::db::world::WorldDb;

#[derive(Clone, Debug)]
pub struct World {
    db: WorldDb,
    pub host: crate::host::Host,
}

impl World {
    pub async fn from(
        db: sled::Db,
        host: crate::host::Host,
    ) -> Self {
        Self {
            db: WorldDb::from(db).await,
            host,
        }
    }

    pub fn get_update(&self, dir: DirContent) -> DirUpdate {
        self.db.get_update_list(dir)
    }

    #[instrument(err)]
    pub async fn dump_save(&self, target: PathBuf) -> Result<(), protocol::Error> {
        let is_empty = tokio::fs::read_dir(&target)
            .await
            .expect("could not check save dump content")
            .next_entry()
            .await
            .unwrap()
            .is_none();

        if !is_empty {
            return Err(protocol::Error::NotEmpty);
        }

        let save = self.db.last_save();
        for sync::Object { org_path, id, .. } in save.objects() {
            let mut source = WorldDb::obj_path().to_owned();
            source.push(id.0.to_string());
            let mut target = target.clone();
            target.push(org_path);
            tokio::fs::copy(source, target).await.unwrap();
        }
        info!("dumped save to: {:?}", target);
        Ok(())
    }

    pub async fn set_save(&self, source: PathBuf) -> Result<(), protocol::Error> {
        match *self.host.state.read().await {
            HostState::NoHost => (),
            _ => return Err(protocol::Error::SaveInUse),
        }

        let content = DirContent::from_dir(source.clone()).await.unwrap();
        let (new_save, update_list) = UpdateList::for_new_save(&self.db, content);
        for (object_id, path) in update_list.0 {
            let full_path = source.join(path);
            let bytes = tokio::fs::read(full_path).await.unwrap();
            WorldDb::add_obj(object_id, &bytes).await?;
        }
        self.db.push_save(new_save);
        info!("loaded and set save from: {:?}", source);

        Ok(())
    }
}
