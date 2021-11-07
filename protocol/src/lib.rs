pub use tarpc;
use serde::{Serialize, Deserialize};
pub use uuid::Uuid;

#[derive(thiserror::Error, Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    #[error("wrong username or password")]
    IncorrectLogin,
    #[error("internal server error, please ask admin for help")]
    Internal,
    #[error("user already exists")]
    AlreadyExists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Version {
    pub protocol: String,
    pub server: String,
}

pub fn version() -> &'static str{ 
    env!("CARGO_PKG_VERSION")
}

/// This is the service definition. It looks a lot like a trait definition.
/// It defines one RPC, hello, which takes one arg, name, and returns a String.
#[tarpc::service]
pub trait World {
    /// Returns a greeting for name.
    async fn version() -> Version;
    async fn log_in(credentials: Credentials) -> Result<Uuid, Error>;

    async fn add_user(user: User, password: String) -> Result<(),Error>;
    async fn list_users() -> Result<Vec<User>, Error>;
    async fn update_user(old: User, new: User) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
