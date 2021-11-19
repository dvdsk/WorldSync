use std::net::SocketAddr;
use std::time::Instant;

use serde::{Deserialize, Serialize};
pub use tarpc;
pub use uuid::Uuid;

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

pub type UserId = u64;
pub type SessionId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub addr: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Version {
    pub protocol: String,
    pub server: String,
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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
    async fn host(id: SessionId) -> Result<Option<Host>, Error>;

    async fn add_user(user: User, password: String) -> Result<(), Error>;
    async fn list_users() -> Result<Vec<(UserId, User)>, Error>;
    async fn override_account(id: UserId, old: User, new: User) -> Result<(), Error>;
    async fn override_password(id: UserId, new: String) -> Result<(), Error>;
    async fn remove_account(id: UserId) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
