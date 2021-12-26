use std::sync::Arc;

use protocol::{Addr, Event, HostDetails, HostState};
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{self, sleep, Duration, Instant};
use tracing::{error, info};

async fn unreachable(addr: &Addr, port: u16) {
    use async_minecraft_ping::ConnectionConfig;
    async fn inner(addr: &Addr, port: u16) -> Result<(), ()> {
        loop {
            sleep(Duration::from_secs(5)).await;
            let _status = ConnectionConfig::build(addr.to_string())
                .with_port(port)
                .connect()
                .await
                .map_err(|_| ())?
                .status()
                .await;
        }
    }

    let _ignore_res = inner(addr, port).await;
}

async fn reachable(addr: &Addr, port: u16) {
    use async_minecraft_ping::ConnectionConfig;
    while let Err(_) = ConnectionConfig::build(addr.to_string())
        .with_port(port)
        .connect()
        .await
    {
        sleep(Duration::from_secs(5)).await;
    }
}

/// except RequestToHost all of these events should come from the current
/// host. This must be checked on the sender side!
#[derive(Debug)]
pub enum HostEvent {
    Loading(u8),
    Loaded,
    RequestToHost(HostDetails),
    ShuttingDown,
    ShutDown,
}

use wrapper::parser::Line;
impl TryFrom<Line> for HostEvent {
    type Error = Line;
    fn try_from(line: Line) -> Result<Self, Self::Error> {
        use wrapper::parser::Message::*;
        match line.msg {
            Loading(p) => Ok(HostEvent::Loading(p)),
            DoneLoading(_) => Ok(HostEvent::Loaded),
            Stopping => Ok(HostEvent::ShuttingDown),
            _ => Err(line),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Host {
    pub state: Arc<RwLock<HostState>>,
}

impl Host {
    pub fn new() -> Host {
        Self {
            state: Arc::new(RwLock::new(HostState::NoHost)),
        }
    }
}

impl Host {
    pub async fn get_state(&self) -> HostState {
        self.state.read().await.clone()
    }
    pub async fn set_state(&self, new: HostState) {
        *self.state.write().await = new;
    }
}

async fn new_host(broadcast: &mut BroadCast, events: &mut Reciever) -> HostState {
    loop {
        match events.recv().await {
            Some(HostEvent::RequestToHost(host)) => {
                info!("new host: {:?}", host);
                let _irrelevant = broadcast.send(Event::NewHost(host.clone()));
                return HostState::Loading(host);
            }
            _e => error!("should not recieve: {:?} in state NoHost", _e),
        }
    }
}

async fn loaded_or_timeout(
    host: HostDetails,
    broadcast: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    let mut deadline = Instant::now() + Duration::from_secs(5 * 60);
    while let Ok(event) = time::timeout_at(deadline, events.recv()).await {
        match event.unwrap() {
            HostEvent::Loading(p) => {
                let _irrelevant = broadcast.send(Event::HostLoading(p));
                deadline = Instant::now() + Duration::from_secs(5 * 60);
            }
            HostEvent::Loaded => {
                let _irrelevant = broadcast.send(Event::HostLoaded);
                info!("host done loading: {:?}", host);
                return HostState::Up(host);
            }
            _e => error!("should not recieve: {:?} in state Loading", _e),
        }
    }

    let _irrelevant = broadcast.send(Event::HostCanceld);
    info!("host canceld loading, host: {:?}", host);
    HostState::NoHost
}

async fn got_shutdown_msg(events: &mut Reciever) {
    loop {
        if let Some(HostEvent::ShuttingDown) = events.recv().await {
            break;
        }
    }
}

async fn shutdown_or_unreachable(
    host: HostDetails,
    broadcast: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    tokio::select! {
        _ = unreachable(&host.addr, host.port) => {
            let _irrelevant = broadcast.send(Event::HostUnreachable);
            info!("host unreachable, host: {:?}", host);
            HostState::Unreachable(host)
        }
        _ = got_shutdown_msg(events) => {
            let _irrelevant = broadcast.send(Event::HostShuttingDown);
            info!("host shutting down, host: {:?}", host);
            HostState::ShuttingDown(host)
        }
    }
}

async fn up_or_timeout(host: HostDetails, broadcast: &mut BroadCast) -> HostState {
    match time::timeout(
        Duration::from_secs(5 * 60),
        reachable(&host.addr, host.port),
    )
    .await
    {
        Ok(_) => {
            let _irrelevant = broadcast.send(Event::HostRestored);
            info!("unreachable host restored contact, host: {:?}", host);
            HostState::Up(host)
        }
        Err(_) => {
            let _irrelevant = broadcast.send(Event::HostDropped);
            info!(
                "host unreachable for 5 minutes, dropping host, host: {:?}",
                host
            );
            HostState::NoHost //TODO annotate broken save
        }
    }
}

async fn shut_down_or_timeout(
    host: HostDetails,
    broadcast: &mut BroadCast,
    events: &mut Reciever,
) -> HostState {
    async fn got_shutdown(events: &mut Reciever) {
        loop {
            match events.recv().await {
                Some(HostEvent::ShutDown) => return,
                _e => error!("should not recieve: {:?} in state Loading", _e),
            }
        }
    }

    match time::timeout(Duration::from_secs(5 * 60), got_shutdown(events)).await {
        Ok(_) => {
            let _irrelevant = broadcast.send(Event::HostShutdown);
            info!("host shut down okay, host: {:?}", host);
            HostState::NoHost
        }
        Err(_) => {
            let _irrelevant = broadcast.send(Event::HostShutdown);
            info!("host shutdown timed out, host: {:?}", host);
            HostState::NoHost //TODO annotate broken save
        }
    }
}

type BroadCast = Arc<broadcast::Sender<protocol::Event>>;
type Reciever = mpsc::Receiver<HostEvent>;
pub async fn monitor(host: Host, mut broadcast: BroadCast, mut events: Reciever) {
    loop {
        // host state may only be changed here
        let current = host.get_state().await;
        let new = match current {
            HostState::NoHost => new_host(&mut broadcast, &mut events).await,
            HostState::Loading(host) => loaded_or_timeout(host, &mut broadcast, &mut events).await,
            HostState::Up(host) => shutdown_or_unreachable(host, &mut broadcast, &mut events).await,
            HostState::Unreachable(host) => up_or_timeout(host, &mut broadcast).await,
            HostState::ShuttingDown(host) => {
                shut_down_or_timeout(host, &mut broadcast, &mut events).await
            }
        };

        host.set_state(new).await;
    }
}
