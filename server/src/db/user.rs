use std::collections::HashMap;
use std::time::Duration;

use protocol::{Credentials, User, UserId};
use serde::{Deserialize, Serialize};
use tokio::task;
use tokio::time::sleep;
use typed_sled::CompareAndSwapError;
use typed_sled::{sled, Tree};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserEntry {
    id: UserId,
    user: User,
    passhash: String,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("entry already exists")]
    AlreadyExists,
    #[error("io error accessing database")]
    Db(#[from] sled::Error),
    #[error("user entry changed since start of modify")]
    Changed(UserEntry),
    #[error("user has been removed")]
    UserRemoved,
    #[error("user does not exist")]
    DoesNotExist,
    #[error("incorrect password")]
    IncorrectPass,
    #[error("incorrect password")]
    IncorrectName,
}

impl From<Error> for protocol::Error {
    fn from(e: Error) -> Self {
        use Error::*;
        match e {
            AlreadyExists => protocol::Error::AlreadyExists,
            Db(_) => protocol::Error::Internal,
            Changed(entry) => protocol::Error::UserChanged(entry.user),
            UserRemoved => protocol::Error::UserRemoved,
            DoesNotExist => protocol::Error::UserNotInDb,
            IncorrectPass => protocol::Error::Unauthorized,
            IncorrectName => unimplemented!(
                "should not be auto converted but explicitly handled
                 to prevent accidentily leaking users in database"
            ),
        }
    }
}

type DbResult<T> = core::result::Result<T, Error>;

#[derive(Clone)]
pub struct UserDb {
    index: HashMap<String, UserId>,
    tree: Tree<UserId, UserEntry>,
    db: sled::Db,
}

impl UserDb {
    pub fn open(db: sled::Db) -> Self {
        let tree = Tree::init(&db, "userdb");
        let index = tree
            .iter()
            .values()
            .map(|e: Result<UserEntry, _>| {
                e.expect(
                    "unexpected error 
                    reading value from database, 
                    has the database format or serialized type changed?",
                )
            })
            .map(|e| (e.user.username, e.id))
            .collect();
        UserDb { index, tree, db }
    }

    fn get_entry(&self, user_id: UserId) -> DbResult<Option<UserEntry>> {
        self.tree.get(&user_id).map_err(Error::Db)
    }

    pub fn get_user(&self, user_id: UserId) -> DbResult<Option<User>> {
        self.get_entry(user_id).map(|o| o.map(|e| e.user))
    }

    pub fn get_userlist(&self) -> DbResult<Vec<(UserId, User)>> {
        let res: Result<_, sled::Error> = self
            .tree
            .iter()
            .values()
            .map(|v| v.map(|e| (e.id, e.user)))
            .collect();
        Ok(res?)
    }

    async fn add_unique_entry(&mut self, entry: UserEntry) -> DbResult<()> {
        self.tree
            .compare_and_swap(&entry.id, None, Some(&entry))?
            .map_err(|_| Error::AlreadyExists)?;
        self.tree.flush_async().await?;
        Ok(())
    }

    pub async fn override_user(&mut self, id: UserId, new: User) -> DbResult<()> {
        let mut current = self.get_entry(id)?.ok_or(Error::UserRemoved)?;
        loop {
            let new_entry = UserEntry {
                user: new.clone(),
                ..current.clone()
            };

            let old_username = current.user.username.clone();
            match self.update_userentry(id, current, new_entry).await {
                Ok(_) => {
                    self.index.remove(&old_username);
                    self.index.insert(new.username, id);
                    return Ok(());
                }
                Err(Error::Changed(curr)) => current = curr,
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn update_user(&mut self, id: UserId, old: User, new: User) -> DbResult<()> {
        let current = self.get_entry(id)?.ok_or(Error::UserRemoved)?;
        let new_entry = UserEntry {
            user: new.clone(),
            ..current.clone()
        };
        let expected = UserEntry {
            user: old.clone(),
            ..current
        };

        self.update_userentry(id, expected, new_entry).await?;
        self.index.remove(&old.username);
        self.index.insert(new.username, id);
        Ok(())
    }

    async fn update_userentry(
        &mut self,
        id: UserId,
        mut expected: UserEntry,
        new: UserEntry,
    ) -> DbResult<()> {
        loop {
            // check if something else then the password changed
            let res = self
                .tree
                .compare_and_swap(&id, Some(&expected), Some(&new))?;

            let new_hash = match res {
                Ok(_) => break,
                Err(CompareAndSwapError { current: None, .. }) => Err(Error::UserRemoved)?,
                Err(CompareAndSwapError {
                    current: Some(curr),
                    ..
                }) => {
                    if curr.user != expected.user {
                        Err(Error::Changed(curr))?
                    } else {
                        curr.passhash
                    }
                }
            };
            expected.passhash = new_hash;
        }
        self.tree.flush_async().await?;
        Ok(())
    }

    pub async fn remove_user(&mut self, id: UserId) -> DbResult<String> {
        let entry = self.tree.remove(&id)?.ok_or(Error::DoesNotExist)?;
        self.tree.flush_async().await?;
        self.index.remove(&entry.user.username);
        Ok(entry.user.username)
    }

    // SECURITY this function can leak the usernames in the database through timing attack
    fn validate_credentials_blocking(&self, credentials: Credentials) -> DbResult<UserId> {
        let id = self.index.get(&credentials.username).copied();
        if id.is_none() {
            return Err(Error::IncorrectName);
        }
        let id = id.unwrap();

        if let Some(entry) = self.get_entry(id)? {
            let correct =
                argon2::verify_encoded(&entry.passhash, credentials.password.as_bytes()).unwrap();
            if correct {
                return Ok(entry.id);
            }
        }
        Err(Error::IncorrectPass)
    }

    // SECURITY sleep compensates for possible timing attack that could leak usernames
    pub async fn validate_credentials(&self, credentials: Credentials) -> DbResult<UserId> {
        let userdb = self.clone();
        let validate =
            task::spawn_blocking(move || userdb.validate_credentials_blocking(credentials));
        let (res, _) = tokio::join!(validate, sleep(Duration::from_millis(100)));
        res.expect("could not rejoin thread")
    }

    pub async fn add_user(&mut self, user: User, password: impl Into<String>) -> DbResult<()> {
        let id = self.db.generate_id()?;
        let passhash = encode_pass(dbg!(password.into())).await;
        let entry = UserEntry { id, user: user.clone(), passhash };
        self.add_unique_entry(entry).await?;
        self.index.insert(user.username, id); 
        Ok(())
    }
}

pub async fn encode_pass(password: String) -> String {
    task::spawn_blocking(move || {
        use argon2::Config;
        use rand::{distributions::Alphanumeric, Rng};
        let salt: Vec<u8> = rand::thread_rng()
            .sample_iter(Alphanumeric)
            .take(32)
            .collect();
        argon2::hash_encoded(password.as_bytes(), &salt, &Config::default()).unwrap()
    })
    .await
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_w_existing_user() {
        let db = super::super::test_db();
        let mut userdb = UserDb::open(db);
        let testuser = User {
            username: "test".to_owned(),
        };
        userdb.add_user(testuser.clone(), "1234").await.unwrap();
        let res = userdb.add_user(testuser, "1234").await;
        assert!(matches!(res, Err(Error::AlreadyExists)));
    }
}
