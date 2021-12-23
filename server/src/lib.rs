use protocol::Event;
use shared::tarpc::server::BaseChannel;
use std::collections::HashMap;
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
    let mut buf = [0u8; 256]; // 108 bytes enough for v1
    let fut = conn.peek(&mut buf);
    if let Err(_) = timeout(Duration::from_millis(10), fut).await {
        dbg!("timeout");
        // return conn.peer_addr().unwrap().ip();
    }
    dbg!("no_timeout");

    use ppp::{HeaderResult, PartialResult};
    let (len, addr) = match HeaderResult::parse(&buf) {
        HeaderResult::V1(Ok(header)) => {
            use ppp::v1::Addresses::*;
            let len = header.header.len();
            match header.addresses {
                Tcp4(addr) => (len, IpAddr::V4(addr.source_address)),
                Tcp6(addr) => (len, IpAddr::V6(addr.source_address)),
                _ => unreachable!(),
            }
        }
        HeaderResult::V2(Ok(header)) => {
            use ppp::v2::Addresses::*;
            let len = header.len();
            match header.addresses {
                IPv4(addr) => (len, IpAddr::V4(addr.source_address)),
                IPv6(addr) => (len, IpAddr::V6(addr.source_address)),
                _ => unreachable!(),
            }
        }
        HeaderResult::V1(Err(e)) if e.is_incomplete() => {
            panic!("header incomplete need more bytes")
        }
        HeaderResult::V2(Err(e)) if e.is_incomplete() => {
            panic!("header incomplete need more bytes")
        }
        _ => return conn.peer_addr().unwrap().ip(),
    };

    dbg!(len, &addr);
    let mut buf = [0u8; 512];
    conn.read(&mut buf[..len])
        .await
        .expect("could not remove proxy protocol header"); // remove proxy protocol header from conn
    addr
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

    let base_state = ConnState {
        peer_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        events,
        sessions,
        userdb,
        world,
        host_req,
    };

    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let codec_builder = LengthDelimitedCodec::builder();
    let listener = TcpListener::bind(server_addr).await.unwrap();
    loop {
        dbg!();
        let (mut conn, _) = listener.accept().await.unwrap();
        let mut conn_state = base_state.clone();

        tokio::spawn(async move {
            conn_state.peer_addr = extract_peer_addr(&mut conn).await;
            let framed = codec_builder.new_framed(conn);

            use tarpc::serde_transport as transport;
            let transport = transport::new(framed, Bincode::default());

            BaseChannel::with_defaults(transport)
                .execute(conn_state.serve())
                .await;
        });
    }
}
