use protocol::{Addr, Event};
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
use tracing::{debug, info, warn};

use protocol::{Service, UserId};
use shared::tarpc;
use tarpc::server::Channel;
use tarpc::tokio_serde::formats::Bincode;
use uuid::Uuid;

pub mod admin_ui;
pub mod db;
pub mod host;
#[cfg(feature = "util")]
pub mod util;
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid header: {0:?}")]
    InvalidAddressType(ppp::v2::Addresses),
    #[error("could not determine address from connection: {0:?}")]
    AddrExtraction(std::io::ErrorKind),
}

pub async fn extract_peer_addr(conn: &mut TcpStream) -> Result<IpAddr, Error> {
    let mut buf = [0u8; 256]; // 108 bytes enough for v1
    let fut = conn.peek(&mut buf);
    if let Err(_) = timeout(Duration::from_millis(10), fut).await {
        warn!("timeout peeking proxy protocol header");
    }

    use ppp::{HeaderResult, PartialResult};
    let (len, addr) = match HeaderResult::parse(&buf) {
        HeaderResult::V1(_) => {
            debug!("proxy protocol header is not supported, assuming none is present");
            return Ok(conn
                .peer_addr()
                .map_err(|e| e.kind())
                .map_err(Error::AddrExtraction)?
                .ip());
        }
        HeaderResult::V2(Ok(header)) => {
            use ppp::v2::Addresses::*;
            let len = header.len();
            match header.addresses {
                IPv4(addr) => (len, IpAddr::V4(addr.source_address)),
                IPv6(addr) => (len, IpAddr::V6(addr.source_address)),
                _addr => return Err(Error::InvalidAddressType(_addr)),
            }
        }
        HeaderResult::V2(Err(e)) if e.is_incomplete() => {
            panic!("header incomplete need more bytes")
        }
        _ => {
            return Ok(conn
                .peer_addr()
                .map_err(|e| e.kind())
                .map_err(Error::AddrExtraction)?
                .ip())
        }
    };

    let mut buf = [0u8; 512];
    conn.read(&mut buf[..len])
        .await
        .expect("could not remove proxy protocol header"); // remove proxy protocol header from conn
    Ok(addr)
}

/// Temp impl until ip.is_global() is stabalized, this is a bad
/// solution it will probably break on a lot of networks but it works
/// on mine.
fn is_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => {
            let oct = addr.octets();
            match (oct[0], oct[1]) {
                (10, _) => true,
                (192, 168) => true,
                (172, _n) => unimplemented!(),
                _ => false,
            }
        }
        _ => unimplemented!(),
    }
}

pub async fn host(
    sessions: Sessions,
    userdb: UserDb,
    world: World,
    port: u16,
    domain: String,
    events: Arc<broadcast::Sender<Event>>,
    host_req: mpsc::Sender<host::HostEvent>,
) {
    let server_addr = (IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    info!("starting listener on port {}", port);

    let base_state = ConnState {
        peer_addr: None,
        events,
        sessions,
        userdb,
        world,
        host_req,
    };
    use shared::tarpc::tokio_util;
    use tokio_util::codec::length_delimited::LengthDelimitedCodec;
    let mut codec_builder = LengthDelimitedCodec::builder();
    let listener = TcpListener::bind(server_addr).await.unwrap();
    loop {
        let (mut conn, _) = listener.accept().await.unwrap();
        let mut conn_state = base_state.clone();
        let domain_clone = domain.clone();

        tokio::spawn(async move {
            conn_state.peer_addr = match extract_peer_addr(&mut conn).await {
                // Ok(addr) if !addr.is_private() => domain,
                Ok(addr) if is_private(addr) => Some(Addr::Domain(domain_clone)),
                Ok(ip) => Some(Addr::Ip(ip)),
                Err(e) => {
                    debug!("could not extract peer address: {}, dropping connection", e);
                    return;
                }
            };

            let framed = codec_builder
                .max_frame_length(100 * 1024 * 1024)
                .new_framed(conn);

            use tarpc::serde_transport as transport;
            let transport = transport::new(framed, Bincode::default());

            BaseChannel::with_defaults(transport)
                .execute(conn_state.serve())
                .await;
        });
    }
}
