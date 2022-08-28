use super::{db, events_channel, host, Sessions, World};
use shared::Platform;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;

/// util function meant for testing only, panics if anything goes wrong
pub async fn spawn_test_server(port: u16) {
    spawn(port, false).await;
}

/// also populates the object store
pub async fn spawn_full_test_server(port: u16) {
    spawn(port, true).await;
}

async fn setup_import_dir() -> &'static Path {
    use std::io::ErrorKind::AlreadyExists;
    let new_dir = |dir: &Path| match std::fs::create_dir_all(dir) {
        Ok(_) => (),
        Err(e) if e.kind() == AlreadyExists => (),
        Err(e) => panic!("{:?}", e),
    };

    let path = Path::new("save_dump");
    new_dir(&path.join("data"));
    for platform in &[Platform::Linux, Platform::Windows] {
        new_dir(&path.join("platform").join(platform.to_string()));
    }
    path
}

/// util function meant for testing only, panics if anything goes wrong
async fn setup_host_files(world: &mut World) {
    info!("setting up host files");
    let path = setup_import_dir().await;
    todo!("get platform specific stuff from worldsync server");
    info!("importing host files to object store");
    world.set_save(path.to_owned()).await.unwrap();
}

async fn spawn(port: u16, with_host_files: bool) {
    use crate::db::user::UserDb;

    let db = db::test_db();
    let domain = "".to_string();
    let host_state = host::Host::new();
    let mut world = World::from(db.clone(), host_state.clone()).await;
    let mut userdb = UserDb::from(db);

    use protocol::User;
    for i in 0..10 {
        match userdb
            .add_user(User::test_user(i), User::test_password(i))
            .await
        {
            Ok(_) => continue,
            Err(db::user::Error::AlreadyExists) => continue,
            Err(e) => panic!("{}", e),
        }
    }

    if with_host_files {
        setup_host_files(&mut world).await;
    }

    let events = events_channel();
    let sessions = Sessions::default();

    let (host_req, host_req_recv) = mpsc::channel(100);
    let monitor = host::monitor(host_state, events.clone(), host_req_recv);
    let host = host(sessions, userdb, world, port, domain, events, host_req);
    tokio::spawn(async move {
        tokio::join!(monitor, host);
    });
    // extra time to ensure server reachable by the time we exit this function
    tokio::time::sleep(Duration::from_millis(50)).await;
}
