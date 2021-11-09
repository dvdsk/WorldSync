use protocol::{tarpc, Error, User, UserId};
use tarpc::context;

use super::WorldClient;
use dialoguer::{Input, Password, Select};

struct Tui {
    client: WorldClient,
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
            Err(e) => {
                println!("could not load user list: {}", e);
                return;
            }
            Ok(user) => user,
        };

        let mut new_user = user.clone();
        loop {
            println!("username: {}", new_user.username);

            let selection = Select::new()
                .item("change username")
                .item("abort")
                .item("save and exit")
                .interact()
                .unwrap();

            match selection {
                0 => change_username(&mut new_user),
                1 => {
                    match self
                        .client
                        .update_user(context::current(), id, user.clone(), new_user.clone())
                        .await
                        .expect("rpc failure")
                    {
                        Ok(_) => return,
                        Err(Error::UserChanged(curr_user)) => {
                            println!("user changed on server! please edit again");
                            new_user = curr_user;
                        }
                        Err(e) => panic!("unexpected error: {}", e),
                    }
                }
                2 => {
                    return;
                }
                _i => unimplemented!("{}", _i),
            }
        }
    }

    async fn pick_user(&mut self) -> Result<(UserId, User), Error> {
        let mut list = self
            .client
            .list_users(context::current())
            .await
            .expect("rpc failure")?;

        let names: Vec<String> = list.iter().map(|u| u.1.username.clone()).collect();
        let selection = Select::new()
            .with_prompt("select user to modify")
            .items(&names)
            .interact()
            .unwrap();
        Ok(list.remove(selection))
    }

    async fn remove_user(&mut self) {}
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
