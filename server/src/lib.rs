use protocol::Event;
use shared::tarpc::server::BaseChannel;
use std::collections::HashMap;
use std::net::Ipv6Addr;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::info;

use protocol::{Service, UserId};
use shared::tarpc;
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use uuid::Uuid;

pub mod admin_ui;
pub mod db;
pub mod host;
mod world;
use db::user::UserDb;
pub use world::World;
mod rpc;
use rpc::ConnState;

type SessionId = Uuid;
pub struct Session {
    user_id: UserId,
    backlog: Arc<Mutex<broadcast::Receiver<Event>>>,
}

#[derive(Clone, Default)]
pub struct Sessions {
    by_id: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl Sessions {
    fn add(&mut self, user_id: UserId, backlog: broadcast::Receiver<Event>) -> SessionId {
        let uuid = Uuid::new_v4();
        let mut sessions = self.by_id.write().unwrap();
        let session = Session {
            user_id,
            backlog: Arc::new(Mutex::new(backlog)),
        };
        sessions.insert(uuid, session);
        uuid
    }
    pub fn get_user_id(&self, id: SessionId) -> Option<UserId> {
        self.by_id.read().unwrap().get(&id).map(|s| s.user_id)
    }
    pub fn clear_user(&self, id: UserId) {
        self.by_id.write().unwrap().retain(|_, v| v.user_id != id)
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(not(feature = "deployed"))]
pub async fn send_test_hb(event_sender: Arc<broadcast::Sender<Event>>) {
    use tokio::time;

    let mut number = 0;
    loop {
        let _ignore_error = event_sender.send(Event::TestHB(number));
        time::sleep(Duration::from_secs(5)).await;
        number += 1;
    }
}

pub fn events_channel() -> Arc<broadcast::Sender<Event>> {
    Arc::new(broadcast::channel(50).0)
}

pub async fn extract_peer_addr(conn: &mut TcpStream) -> IpAddr {
    let mut buf = [0u8; 512]; // 108 bytes enough for v1
    let fut = conn.peek(&mut buf);
    if let Err(_) = timeout(Duration::from_millis(250), fut).await {
        return conn.peer_addr().unwrap().ip();
    }

    use ppp::model::Addresses;
    use ppp::error::ParseError;
    let (left, header) = match ppp::parse_header(&buf) {
        Ok(res) => res,
        Err(ParseError::Failure) => return conn.peer_addr().unwrap().ip(),
        Err(ParseError::Incomplete) => panic!("not enough bytes to parse proxy protocol header"),
    };

    let mut buf = [0u8; 512];
    conn.read(&mut buf[..512 - left.len()])
        .await
        .expect("could not remove proxy protocol header"); // remove proxy protocol header from conn

    match header.addresses {
        Addresses::IPv4 { source_address, .. } => IpAddr::V4(Ipv4Addr::from(source_address)),
        Addresses::IPv6 { source_address, .. } => IpAddr::V6(Ipv6Addr::from(source_address)),
        _ => panic!("unsupported adress type"),
    }
}

pub async fn host(
    sessions: Sessions,
    userdb: UserDb,
    world: World,
    port: u16,
    events: Arc<broadcast::Sender<Event>>,
    host_req: mpsc::Sender<host::HostEvent>,
) {
    let server_addr = (IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    info!("starting listener on port {}", port);

    let listener = TcpListener::bind(server_addr).await.unwrap();
    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let codec_builder = LengthDelimitedCodec::builder();
    loop {
        let (mut conn, _addr) = listener.accept().await.unwrap();
        let peer_addr = extract_peer_addr(&mut conn).await;
        let framed = codec_builder.new_framed(conn);

        use tarpc::serde_transport as transport;
        let transport = transport::new(framed, Bincode::default());

        let fut = BaseChannel::with_defaults(transport).execute(
            ConnState {
                peer_addr,
                events: events.clone(),
                sessions: sessions.clone(),
                userdb: userdb.clone(),
                world: world.clone(),
                host_req: host_req.clone(),
            }
            .serve(),
        );
        tokio::spawn(fut);
    }
}
