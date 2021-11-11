use futures::future;
use futures::StreamExt;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

use protocol::{tarpc, User, World, UserId};
use tarpc::server::{incoming::Incoming, Channel};
use tarpc::tokio_serde::formats::Json;
use uuid::Uuid;

pub mod admin_ui;
pub mod db;
use db::user::UserDb;
mod rpc;
use rpc::ConnState;

type SessionId = Uuid;
pub struct Session {
    user_id: UserId,
    last_active: Instant,
}

#[derive(Clone)]
pub struct Sessions(Arc<RwLock<HashMap<SessionId, Session>>>);
impl Sessions {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    fn add(&mut self, user_id: UserId) -> SessionId {
        let uuid = Uuid::new_v4();
        let mut sessions = self.0.write().unwrap();
        let session = Session {
            user_id,
            last_active: Instant::now(),
        };
        sessions.insert(uuid, session);
        uuid
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}


pub async fn host(sessions: Sessions, userdb: UserDb, port: u16) {
    let server_addr = (IpAddr::V4(Ipv4Addr::LOCALHOST), port);

    // JSON transport is provided by the json_transport tarpc module. It makes it easy
    // to start up a serde-powered json serialization strategy over TCP.
    let mut listener = tarpc::serde_transport::tcp::listen(&server_addr, Json::default)
        .await
        .unwrap();
    listener.config_mut().max_frame_length(usize::MAX);
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        .map(tarpc::server::BaseChannel::with_defaults)
        // Limit channels to 1 per IP.
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        // serve is generated by the service attribute. It takes as input any type implementing
        // the generated World trait.
        .map(|channel| {
            let peer_addr = channel.transport().peer_addr().unwrap();
            let server = ConnState {
                peer_addr,
                sessions: sessions.clone(),
                userdb: userdb.clone(),
            };
            channel.execute(server.serve())
        })
        // Max 10 channels.
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;
}
