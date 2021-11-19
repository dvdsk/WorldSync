use crate::gui::style;
pub use protocol::ServiceClient;
use protocol::{Host, Uuid, tarpc};
pub use tarpc::context;

use super::{Error, Event, Msg, Page};
use futures::future;
use iced::Command;

fn parse_server_str(server_str: &str) -> Result<(String, u16), Error> {
    let (domain, port) = server_str.split_once(':').ok_or(Error::InvalidFormat)?;
    let port = port.parse().map_err(|_| Error::NotANumber)?;
    return Ok((domain.to_owned(), port));
}

pub async fn login(
    domain: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(ServiceClient, Uuid, Option<Host>), Error> {
    let client = crate::connect(&domain, port)
        .await
        .map_err(|_| Error::NoMetaConn)?;
    let session_id = client
        .log_in(context::current(), username, password)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    let host = client
        .host(context::current(), session_id)
        .await
        .map_err(|_| Error::NoMetaConn)??;
    Ok((client, session_id, host))
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
                    Ok((client, uuid, host)) => Msg::LoggedIn(client, uuid, host),
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
        }
        self.errorbar.add(e);
        Command::none()
    }
}
