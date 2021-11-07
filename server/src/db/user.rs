use std::time::Duration;

use protocol::{Credentials, User};
use serde::{Deserialize, Serialize};
use tokio::task;
use tokio::time::sleep;
use typed_sled::{sled, Tree};

#[derive(Clone, Serialize, Deserialize)]
struct UserEntry {
    user: User,
    passhash: String,
}

impl UserEntry {
    pub async fn from_user(user: User, password: String) -> Self {
        let passhash = task::spawn_blocking(move || {
            use argon2::Config;
            use rand::{distributions::Alphanumeric, Rng};
            let salt: Vec<u8> = rand::thread_rng()
                .sample_iter(Alphanumeric)
                .take(32)
                .collect();
            argon2::hash_encoded(password.as_bytes(), &salt, &Config::default()).unwrap()
        })
        .await
        .unwrap();
        UserEntry { user, passhash }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("entry already exists")]
    AlreadyExists,
    #[error("io error accessing database")]
    Db(#[from] sled::Error),
}

impl From<Error> for protocol::Error {
    fn from(e: Error) -> Self {
        use Error::*;
        match e {
            AlreadyExists => protocol::Error::AlreadyExists,
            Db(_) => protocol::Error::Internal,
        }
    }
}

type DbResult<T> = core::result::Result<T, Error>;

#[derive(Clone)]
pub struct UserDb(Tree<String, UserEntry>);

impl UserDb {
    pub fn open(db: &sled::Db) -> Self {
        let tree: Tree<String, UserEntry> = Tree::init(db, "userdb");
        UserDb(tree)
    }

    fn get_entry(&self, username: String) -> DbResult<Option<UserEntry>> {
        self.0.get(&username).map_err(Error::Db)
    }

    pub fn get_userlist(&self) -> DbResult<Vec<User>> {
        let res: Result<Vec<User>, sled::Error> = self.0.iter()
            .values()
            .map(|v| v.map(|e| e.user))
            .collect();

        Ok(res?)
    }

    async fn add_unique_entry(&mut self, entry: UserEntry) -> DbResult<()> {
        self.0
            .compare_and_swap(&entry.user.username, None, Some(&entry))?
            .map_err(|_| Error::AlreadyExists)?;
        self.0.flush_async().await?;
        Ok(())
    }

    pub async fn update_user(&mut self, old: User, new: User) -> DbResult<()> {
    }

    // SECURITY this function can leak the usernames in the database through timing attack
    fn validate_credentials_blocking(&self, credentials: Credentials) -> DbResult<Option<User>> {
        if let Some(entry) = self.get_entry(credentials.username)? {
            let correct =
                argon2::verify_encoded(&entry.passhash, credentials.password.as_bytes()).unwrap();
            if correct {
                return Ok(Some(entry.user));
            }
        }
        Ok(None)
    }

    // SECURITY sleep compensates for possible timing attack that could leak usernames
    pub async fn validate_credentials(&self, credentials: Credentials) -> DbResult<Option<User>> {
        let userdb = self.clone();
        let validate =
            task::spawn_blocking(move || userdb.validate_credentials_blocking(credentials));
        let (res, _) = tokio::join!(validate, sleep(Duration::from_millis(100)));
        res.expect("could not rejoin thread")
    }

    pub async fn store(&mut self, user: User, password: impl Into<String>) -> DbResult<()> {
        let entry = UserEntry::from_user(user, password.into()).await;
        self.add_unique_entry(entry).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_w_existing_user() {
        let db = super::super::test_db();
        let mut userdb = UserDb::open(&db);
        let testuser = User {
            username: "test".to_owned(),
        };
        userdb.store(testuser, "1234").await.unwrap();
        let res = userdb.store(testuser, "1234").await;
        assert!(matches!(res, Err(Error::AlreadyExists)));
    }
}
