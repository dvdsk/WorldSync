use protocol::{tarpc, User, UserId};
use tarpc::context;

use super::WorldClient;
use dialoguer::{Confirm, Input, Password, Select};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("server side error: {0}")]
    Protocol(#[from] protocol::Error),
    #[error("userlist is empty")]
    NoUsers,
    #[error("canceled")]
    Canceld,
}

pub struct Tui {
    pub client: WorldClient,
}

pub async fn main_menu(client: WorldClient) {
    let mut ui = Tui { client };
    loop {
        let selection = Select::new()
            .item("Add user")
            .item("Modify user")
            .item("Remove user")
            .item("Exit")
            .interact()
            .unwrap();

        match selection {
            0 => ui.add_user().await,
            1 => ui.modify_user().await,
            2 => ui.remove_user().await,
            3 => return,
            _ => panic!("impossible"),
        }
    }
}

impl Tui {
    async fn add_user(&mut self) {
        let validate_username = |input: &String| {
            if input.len() > 2 {
                Ok(())
            } else {
                Err("usernames with less then three characters are not allowed")
            }
        };

        let username: String = Input::new()
            .with_prompt("username")
            .validate_with(validate_username)
            .interact()
            .unwrap();
        let password: String = Password::new()
            .with_prompt("password")
            .with_confirmation("Repeat password", "Error the passwords dont match")
            .interact()
            .unwrap();

        let user = User { username };
        if let Err(e) = self
            .client
            .add_user(context::current(), user, password)
            .await
            .expect("rpc failure")
        {
            println!("could not add user: {}", e)
        }
    }

    async fn modify_user(&mut self) {
        let (id, user) = match self.pick_user().await {
            Err(Error::Canceld) => return,
            Err(Error::NoUsers) => {
                println!("no users to list");
                return;
            }
            Err(e) => {
                println!("could not load user list: {}", e);
                return;
            }
            Ok(id_user) => id_user,
        };

        let mut new_user = user.clone();
        let mut password = None;
        loop {
            println!("id: {}", id);
            println!("username: {}", new_user.username);
            println!(
                "password: {}",
                password.clone().unwrap_or("[unchanged]".to_owned())
            );

            let selection = Select::new()
                .item("change username")
                .item("change password")
                .item("abort")
                .item("save and exit")
                .interact()
                .unwrap();

            match selection {
                0 => change_username(&mut new_user),
                1 => change_password(&mut password),
                2 => return,
                3 => {
                    self.override_password(id, password.clone()).await;
                    let done = self.override_account(id, user.clone(), &mut new_user).await;
                    if done {
                        return;
                    };
                }
                _i => unimplemented!("{}", _i),
            }
        }
    }

    async fn remove_user(&mut self) {
        let (id, user) = match self.pick_user().await {
            Err(Error::Canceld) => return,
            Err(Error::NoUsers) => {
                println!("no users to list");
                return;
            }
            Err(e) => {
                println!("could not load user list: {}", e);
                return;
            }
            Ok(id_user) => id_user,
        };

        let prompt = format!("delete user: '{}'", user.username);
        if Confirm::new().with_prompt(prompt).interact().unwrap() {
            self.client
                .remove_account(context::current(), id)
                .await
                .expect("rpc failure")
                .unwrap();
        } else {
            println!("canceld removal");
        }
    }

    async fn pick_user(&mut self) -> Result<(UserId, User), Error> {
        let mut list = self
            .client
            .list_users(context::current())
            .await
            .expect("rpc failure")?;

        if list.is_empty() {
            return Err(Error::NoUsers);
        }

        let names: Vec<String> = list
            .iter()
            .map(|u| format!("\"{}\"", u.1.username))
            .collect();
        let selection = Select::new()
            .with_prompt("select user to modify")
            .items(&names)
            .item("cancel")
            .interact()
            .unwrap();

        if selection == list.len() {
            return Err(Error::Canceld);
        }

        Ok(list.remove(selection))
    }
}

fn change_username(user: &mut User) {
    let validate_username = |input: &String| {
        if input.len() > 2 || input.is_empty() {
            Ok(())
        } else {
            Err("usernames with less then three characters are not allowed")
        }
    };

    let new_name: String = Input::new()
        .with_prompt("change username")
        .with_initial_text(&user.username)
        .validate_with(validate_username)
        .allow_empty(true)
        .interact()
        .unwrap();

    if new_name.is_empty() {
        println!("canceling")
    } else {
        user.username = new_name
    }
}

fn change_password(pass: &mut Option<String>) {
    let validate_password = |input: &String| {
        if input.len() > 10 || input.is_empty() {
            Ok(())
        } else {
            Err("password with less then 10 characters are not allowed")
        }
    };

    let new_pass: String = Input::new()
        .with_prompt("change password")
        .validate_with(validate_password)
        .allow_empty(true)
        .interact()
        .unwrap();

    if new_pass.is_empty() {
        println!("canceling")
    } else {
        *pass = Some(new_pass)
    }
}
