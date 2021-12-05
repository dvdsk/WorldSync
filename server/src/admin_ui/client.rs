use shared::tarpc::context;
use protocol::{User, UserId};

use crate::admin_ui::tui::Tui;

type Done = bool;
impl Tui {
    pub async fn override_account(&self, id: UserId, user: User, new_user: &mut User) -> Done {
        if user == *new_user {
            return true;
        }

        match self
            .client
            .override_account(context::current(), id, user.clone(), new_user.clone())
            .await
            .expect("rpc failure")
        {
            Ok(_) => true,
            Err(protocol::Error::UserChanged(curr_user)) => {
                println!("user changed on server! please edit again");
                *new_user = curr_user;
                false
            }
            Err(e) => panic!("unexpected error: {}", e),
        }
    }

    pub async fn override_password(&self, id: UserId, password: Option<String>) {
        if password.is_none() {
            return;
        }

        match self
            .client
            .override_password(context::current(), id, password.unwrap())
            .await
            .expect("rpc failure")
        {
            Ok(_) => (),
            Err(protocol::Error::UserRemoved) => println!("failed: user was removed"),
            Err(e) => panic!("unexpected error: {}", e),
        }
    }
}
