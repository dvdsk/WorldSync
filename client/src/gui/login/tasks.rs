use super::{Page, Msg};
use iced::Command;

fn parse_server_str(server_str: &str) -> Result<(String, u16), &'static str> {
    let (domain, port) = server_str
        .split_once(':')
        .ok_or("Not a server, please check the address")?;
    let port = port
        .parse()
        .map_err(|_| "Port is not a number, please check the address")?;
    return Ok((domain.to_owned(), port));
}

impl Page {
    pub fn on_submit(&mut self) -> Command<Msg> {
        self.logging_in = true;
        match parse_server_str(&self.inputs.server.value) {
            Ok((domain, port)) => {
                let task = crate::connect_and_login(
                    domain,
                    port,
                    self.inputs.username.value.clone(),
                    self.inputs.password.value.clone(),
                );
                Command::perform(task, move |msg| msg);
            }
            Err(error_str) => {
                tracing::warn!(error_str);
                self.error = Some(error_str);
            }
        }
        Command::none()
    }
}
