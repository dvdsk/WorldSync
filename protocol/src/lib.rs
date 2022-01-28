use std::fmt;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;
use sync::{DirContent, DirUpdate, ObjectId, UpdateList};
use wrapper::parser::Line;

use serde::{Deserialize, Serialize};
use shared::tarpc;
pub use time;
pub use uuid::Uuid;

mod connect;
pub use connect::{connect, connect_local};

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum Error {
    #[error("wrong username or password")]
    IncorrectLogin,
    #[error("internal server error, please ask admin for help")]
    Internal,
    #[error("user already exists")]
    AlreadyExists,
    #[error("user changed while modifying")]
    UserChanged(User),
    #[error("user was removed while modifying")]
    UserRemoved,
    #[error("no user in database with given id")]
    UserNotInDb,
    #[error("access restricted")]
    Unauthorized,
    #[error("session expired or did not exist")]
    SessionExpired,
    #[error("missed to many server events, need to log in again")]
    Lagging,
    #[error("a sessions backlog should not be accessed concurrently")]
    BackLogLocked,
    #[error("could not dump save, target directory does not exist")]
    DirDoesNotExist,
    #[error("could not load save, someone is currently hosting")]
    SaveInUse,
    #[error("could not dump save, folder is not empty")]
    NotEmpty,
    #[error("session is not currently hosting")]
    NotHost,
    #[error("files with this path may not be changed by a client: {0}")]
    ForbiddenPath(PathBuf),
    #[error("not creating a new save")]
    NotSaving,
}

// governs the maximum time between events, is used to detect connection
// lost
pub const AWAIT_EVENT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    #[cfg(not(feature = "deployed"))]
    TestHB(usize),
    AwaitTimeout,
    NewHost(HostDetails),
    HostLoading(u8),
    HostLoaded,
    HostShuttingDown,
    HostShutdown,
    HostDropped,
    HostUnreachable,
    HostRestored,
    HostCanceld,
}

pub type UserId = u64;
pub type HostId = Uuid;
pub type SessionId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Addr {
    Domain(String),
    Ip(IpAddr),
}

impl Addr {
    pub fn is_loopback(&self) -> bool {
        match self {
            Self::Ip(addr) if addr.is_loopback() => true,
            _ => false,
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Self::Domain(s) => s.clone(),
            Self::Ip(ip) => ip.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostDetails {
    pub name: String,
    pub addr: Addr,
    pub port: u16,
    pub id: HostId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostState {
    NoHost,
    Loading(HostDetails),
    Up(HostDetails),
    Unreachable(HostDetails),
    ShuttingDown(HostDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct User {
    pub username: String,
}

impl User {
    pub fn test_user(num: u8) -> Self {
        Self {
            username: Self::test_username(num),
        }
    }
    pub fn test_username(num: u8) -> String {
        format!("TestUser_{}", num)
    }
    pub fn test_password(num: u8) -> String {
        format!("testpass{}", num)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum Platform {
    Windows,
    Linux,
}

impl Platform {
    pub fn current() -> Result<Self, ()> {
        if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else if cfg!(target_os = "windows") {
            Ok(Self::Windows)
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct Version {
    semver: String,
    commit: String,
    branch: String,
    build_date: String,
    features: String,
    profile: String,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  version:")?;
        writeln!(f, "     - semver: {}", self.semver)?;
        writeln!(f, "     - commit: {}", self.commit)?;
        writeln!(f, "     - branch: {}", self.branch)?;
        writeln!(f, "  build_date: {}", self.build_date)?;
        writeln!(f, "  features: {}", self.features)?;
        writeln!(f, "  profile: {}", self.profile)?;
        Ok(())
    }
}

pub fn current_version() -> Version {
    Version {
        semver: env!("VERGEN_BUILD_SEMVER").to_string(),
        commit: env!("VERGEN_GIT_SHA_SHORT").to_string(),
        branch: env!("VERGEN_GIT_BRANCH").to_string(),
        build_date: env!("VERGEN_BUILD_DATE").to_string(),
        features: env!("VERGEN_CARGO_FEATURES").to_string(),
        profile: env!("VERGEN_CARGO_PROFILE").to_string(),
    }
}

use bincode::config::Options;
pub fn bincode_opts() -> impl bincode::Options + Copy {
    bincode::options().with_no_limit()
}

/// This is the service definition. It looks a lot like a trait definition.
/// It defines one RPC, hello, which takes one arg, name, and returns a String.
#[tarpc::service]
pub trait Service {
    async fn version() -> Version;
    async fn log_in(username: String, password: String) -> Result<SessionId, Error>;
    async fn get_account(id: SessionId) -> Result<User, Error>;
    async fn update_account(id: SessionId, new: User) -> Result<(), Error>;
    async fn update_password(id: SessionId, new: String) -> Result<(), Error>;
    async fn close_account(id: SessionId) -> Result<(), Error>;
    async fn await_event(id: SessionId) -> Result<Event, Error>;
    async fn host(id: SessionId) -> Result<HostState, Error>;
    async fn request_to_host(id: SessionId, host_id: HostId) -> Result<(), Error>;
    async fn dir_update(id: SessionId, dir: DirContent, platform: Platform) -> Result<DirUpdate, Error>;
    async fn new_save(id: SessionId, host_id: HostId, dir: DirContent)
        -> Result<UpdateList, Error>;
    async fn register_save(id: SessionId, host_id: HostId) -> Result<(), Error>;
    async fn get_object(id: SessionId, object: ObjectId) -> Result<Vec<u8>, Error>;
    async fn put_object(
        id: SessionId,
        host_id: HostId,
        object: ObjectId,
        path: PathBuf,
        bytes: Vec<u8>,
    ) -> Result<(), Error>;
    async fn pub_mc_line(id: HostId, line: Line) -> Result<(), Error>;

    async fn add_user(user: User, password: String) -> Result<(), Error>;
    async fn list_users() -> Result<Vec<(UserId, User)>, Error>;
    async fn override_account(id: UserId, old: User, new: User) -> Result<(), Error>;
    async fn override_password(id: UserId, new: String) -> Result<(), Error>;
    async fn remove_account(id: UserId) -> Result<(), Error>;
    async fn dump_server(dir: PathBuf, platform: Platform) -> Result<(), Error>;
    async fn dump_save(dir: PathBuf) -> Result<(), Error>;
    async fn set_save(dir: PathBuf) -> Result<(), Error>;
}
