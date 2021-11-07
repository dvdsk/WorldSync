use std::time::Duration;

use protocol::Credentials;
use serde::{Deserialize, Serialize};
use tokio::task;
use tokio::time::sleep;
use typed_sled::{sled, Tree};

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
}

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
        }).await.unwrap();
        UserEntry { user, passhash }
    }
}

#[derive(Clone)]
pub struct UserDb(Tree<String, UserEntry>);

impl UserDb {
    pub fn open(db: &sled::Db) -> Self {
        let tree: Tree<String, UserEntry> = Tree::init(db, "userdb");
        UserDb(tree)
    }

    fn get_entry(&self, username: String) -> Option<UserEntry> {
        self.0.get(&username).unwrap()
    }

    async fn add_entry(&mut self, entry: UserEntry) -> Result<(), ()> {
        self.0
            .compare_and_swap(&entry.user.username, None, Some(&entry))
            .unwrap()
            .map_err(|_| ())?;
        self.0.flush_async().await.unwrap();
        Ok(())
    }

    // SECURITY this function can leak the usernames in the database through timing attack
    fn validate_credentials_blocking(&self, credentials: Credentials) -> Result<User, ()> {
        if let Some(entry) = self.get_entry(credentials.username) {
            let correct =
                argon2::verify_encoded(&entry.passhash, credentials.password.as_bytes()).unwrap();
            if correct {
                return Ok(entry.user);
            }
        }
        Err(())
    }

    // SECURITY sleep compensates for possible timing attack that could leak usernames
    pub async fn validate_credentials(&self, credentials: Credentials) -> Result<User, ()> {
        let userdb = self.clone();
        let validate =
            task::spawn_blocking(move || userdb.validate_credentials_blocking(credentials));
        let (res, _) = tokio::join!(validate, sleep(Duration::from_millis(100)));
        res.unwrap()
    }

    pub async fn store(&mut self, user: User, password: String) {
        let entry = UserEntry::from_user(user, password).await;
        self.add_entry(entry).await.expect("entry already exists");
    }
}
