use crate::gui::{style, RpcConn};
use protocol::HostState;
pub use protocol::ServiceClient;
use serde::{Deserialize, Serialize};
use shared::tarpc;
pub use tarpc::context;
use tracing::instrument;

use super::{Error, Event, Msg, Page};
use futures::future;
use iced::Command;

fn parse_server_str(server_str: &str) -> Result<(String, u16), Error> {
    let (domain, port) = server_str.split_once(':').ok_or(Error::InvalidFormat)?;
    let port = port.parse().map_err(|_| Error::NotANumber)?;
    Ok((domain.to_owned(), port))
}

#[derive(Default, Serialize, Deserialize)]
pub struct LoginFields {
    server: String,
    username: String,
    password: String,
}

impl LoginFields {
    pub fn load(db: &sled::Db) -> Option<Self> {
        db.get("logins")
            .unwrap()
            .map(|bytes| bincode::deserialize(&bytes).unwrap())
    }
    pub fn store(&self, db: &sled::Db) {
        let bytes = bincode::serialize(self).unwrap();
        db.insert("logins", bytes).unwrap();
    }
}

impl super::Inputs {
    pub fn load(db: &sled::Db) -> Option<Self> {
        use super::Input;
        LoginFields::load(db).map(|fields| Self {
            server: Input {
                value: fields.server,
                ..Input::default()
            },
            username: Input {
                value: fields.username,
                ..Input::default()
            },
            password: Input {
                value: fields.password,
                ..Input::default()
            },
        })
    }
    pub fn store(&self, db: &sled::Db) {
        let fields = LoginFields {
            server: self.server.value.clone(),
            username: self.username.value.clone(),
            password: self.password.value.clone(),
        };
        fields.store(db);
    }
}

#[instrument(err)]
pub async fn login(
    domain: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(RpcConn, HostState), Error> {
    let client = protocol::connect(&domain, port)
        .await
        .map_err(|_| Error::NoMetaConn)?;
    let server_version = client
        .version(context::current())
        .await
        .map_err(|_| Error::NoMetaConn)?;

    if server_version != protocol::current_version() {
        return Err(Error::VersionMismatch {
            our: protocol::current_version(),
            server: server_version,
        });
    }

    let session = client
        .log_in(context::current(), username, password)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    let host = client
        .host(context::current(), session)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    Ok((RpcConn { client, session }, host))
}

impl Page {
    pub fn on_submit(&mut self) -> Command<Msg> {
        self.logging_in = true;
        match parse_server_str(&self.inputs.server.value) {
            Ok((domain, port)) => {
                let task = login(
                    domain,
                    port,
                    self.inputs.username.value.clone(),
                    self.inputs.password.value.clone(),
                );
                Command::perform(task, move |res| match res {
                    Ok((rpc, host)) => Msg::LoggedIn(rpc, host),
                    Err(err) => Msg::LoginPage(Event::Error(err)),
                })
            }
            Err(e) => {
                tracing::warn!("{}", e);
                Command::perform(future::ready(e), move |e| Msg::LoginPage(Event::Error(e)))
            }
        }
    }

    pub fn handle_err(&mut self, e: Error) -> Command<Msg> {
        match e {
            Error::NoMetaConn | Error::NotANumber | Error::InvalidFormat => {
                self.inputs.server.style = style::Input::Err
            }
            Error::IncorrectLogin => {
                self.inputs.username.style = style::Input::Err;
                self.inputs.password.style = style::Input::Err;
            }
            _ => (),
        }
        self.errorbar.add(e);
        Command::none()
    }
}
