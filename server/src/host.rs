use std::net::SocketAddr;
use std::sync::Arc;

use protocol::{Event, HostDetails};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::sleep;
use tracing::{error, info, warn};

async fn unreachable(addr: SocketAddr) {
    use async_minecraft_ping::ConnectionConfig;
    async fn inner(addr: SocketAddr) -> Result<(), ()> {
        let mut conn = ConnectionConfig::build(addr.ip().to_string())
            .with_port(addr.port())
            .connect()
            .await
            .map_err(|_| ())?;
        loop {
            sleep(Duration::from_secs(5 * 60)).await;
            conn.status().await.map_err(|_| ())?;
        }
    }

    let _ignore_res = inner(addr).await;
}

async fn reachable(addr: SocketAddr) {
    use async_minecraft_ping::ConnectionConfig;
    while let Err(_) = ConnectionConfig::build(addr.ip().to_string())
        .with_port(addr.port())
        .connect()
        .await
    {
        sleep(Duration::from_secs(5 * 60)).await;
    }
}

pub enum HostEvent {
    Loading,
    Loaded,
    RequestToHost(HostDetails),
    ShuttingDown,
    ShutDown(HostId),
}

pub struct Host {
    state: Arc<RwLock<HostState>>,
    request: mpsm::Sender<HostEvent>,
}

impl Host {
    pub async fn get_state(&self) -> HostState {
        self.state.read().await.clone()
    }
    pub async fn set_state(&self, new: HostState) {
        *self.state.write().await = new;
    }
}

async fn new_host(sender: BroadCast, events: &mut Reciever) -> HostState {
    loop {
        match events.recv().await {
            HostEvent::RequestToHost(host) => return HostState::Loading(host),
            _e => error("should not recieve: {:?} in state NoHost", _e),
        }
    }
}

async fn loaded_or_timeout(
    host: HostDetails,
    sender: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    let mut deadline = Instant::now() + Durations::from_secs(5 * 60);
    while let Ok(event) = time::timeout_at(deadline, events.recv()) {
        match event {
            HostEvent::Loading => deadline = Instant::now() + Durations::from_secs(5 * 60),
            HostEvent::Loaded => return HostState::Up(host),
            _e => error("should not recieve: {:?} in state Loading", _e),
        }
    }
}

async fn got_shutdown_msg(host: HostDetails, events: &mut Reciever) {
    loop {
        if let HostEvent::ShuttingDown = events.recv().await {
            break;
        }
    }
}

async fn shutdown_or_unreachable(
    host: HostDetails,
    sender: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    tokio::select! {
        _ = unreachable(host.addr) => HostState::Unreachable(host),
        _ = got_shutdown_msg(host, events) => HostState::ShuttingDown(host),
    }
}

async fn up_or_timeout(
    host: HostDetails,
    sender: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
}

async fn done_or_timeout(
    host: HostDetails,
    sender: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    let res = time::timeout(Duration::from_secs(5 * 60), reachable(host.addr)).await;
    match res {
        Ok(_) => HostState::Up(host),
        Err(_) => {
            warn!("host went down");
            HostState::NoHost(host)
        }
    }
}

async fn shut_down_or_timeout(
    host: HostDetails,
    sender: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    async fn got_shutdown(events: &mut Reciever) {
        loop {
            match events.recv().await {
                HostState::ShutDown(id) => return,
                _ => error("should not recieve: {:?} in state Loading", _e),
            }
        }
    }

    match time::timeout(Duration::from_secs(5 * 60), got_shutdown(events)) {
        Ok(_) => HostState::NoHost,
        Err(_) => HostState::NoHost, //TODO annotate broken save
    }
}

type BroadCast = broadcast::Sender<protocol::Event>;
type Reciever = mpsc::Receiver<HostEvent>;
pub async fn monitor(host: Host, broadcast: BroadCast, events: Receiver) {
    loop {
        // host state may only be changed here
        let current = host.get_state().await;
        let new = match current {
            HostState::NoHost => new_host(broadcast, events).await,
            HostState::Loading(host) => loaded_or_timeout(host, &mut broadcast, &mut events).await,
            HostState::Up(host) => shutdown_or_unreachable(host, &mut broadcast, &mut events).await,
            HostState::Unreachable(host) => up_or_timeout(host, &mut broadcast, &mut events).await,
            HostState::ShuttingDown(host) => {
                shut_down_or_timeout(host, &mut broadcast, &mut events).await
            }
        };

        host.set_state(new).await;
    }
}
