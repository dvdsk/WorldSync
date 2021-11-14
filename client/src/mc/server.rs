use mc_server_wrapper_lib::{
    communication::{ServerCommand, ServerEvent},
    McServerConfig, McServerManager, McServerStartError,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Notify,
};
use tracing::{info, log::error};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No server binary at the path")]
    BinMissing,
    #[error("Could not auto accept Eula (io-error): {0}")]
    CouldNotWriteEula(std::io::Error),
    #[error("Could not start server (io-error): {0}")]
    CouldNotStart(std::io::Error),
    #[error("Server crashed, (io-error): {0}")]
    Crash(std::io::Error),
    #[error("Unknown server error: {0}")]
    Unknown(String),
}

pub struct Instance {
    _manager: Arc<McServerManager>,
    event: Receiver<ServerEvent>,
    started: Arc<Notify>,
    save_done: Arc<Notify>,
}

pub struct Handle {
    config: McServerConfig,
    cmd: Sender<ServerCommand>,
    started: Arc<Notify>,
    save_done: Arc<Notify>,
}

impl Instance {
    pub fn new(server_path: PathBuf) -> Result<(Self, Handle), Error> {
        let (_manager, cmd, event) = McServerManager::new();
        let config = McServerConfig::new(server_path, 1024, None, false);
        config.validate().map_err(|_| Error::BinMissing)?;
        let started = Arc::new(Notify::new());
        let save_done = Arc::new(Notify::new());
        let server = Self {
            _manager,
            event,
            started: started.clone(),
            save_done: save_done.clone(),
        };
        let handle = Handle {
            config,
            cmd,
            started,
            save_done,
        };
        Ok((server, handle))
    }
    pub async fn maintain(mut self) -> Result<(), Error> {
        loop {
            use ServerEvent::*;
            let event = loop {
                if let Some(event) = self.event.recv().await {
                    break event;
                }
            };

            // dbg!(&event);
            let e = match event {
                StartServerResult(Ok(_)) => {
                    dbg!("server started");
                    self.started.notify_one();
                    continue;
                }
                StartServerResult(Err(McServerStartError::IoError(e))) => Error::CouldNotStart(e),
                StartServerResult(Err(e)) => panic!("server start error: {:?}", e),
                AgreeToEulaResult(Ok(_)) => continue,
                AgreeToEulaResult(Err(e)) => Error::CouldNotWriteEula(e),
                ServerStopped(Err(e), _) => Error::Crash(e),
                ServerStopped(Ok(_), _) => return Ok(()),
                StderrLine(s) => Error::Unknown(s),
                StdoutLine(s) => {
                    // dbg!(s);
                    continue;
                }
                ConsoleEvent(msg, None) => {
                    // dbg!(msg);
                    continue;
                }
                ConsoleEvent(_, Some(event)) => {
                    // info!("server event: {:?}", event);
                    continue;
                }
            };
            return Err(e);
        }
    }
}

/// errors are handled by the Instance pair coupled to this Handle stopping
/// and returning the error.
impl Handle {
    pub async fn start(&mut self) {
        let _ignore_err = self.cmd
            .send(ServerCommand::StartServer {
                config: Some(self.config.clone()),
            })
            .await;

        self.started.notified().await;
    }
    pub async fn save(&self) {
        let _ignore_err = self.cmd
            .send(ServerCommand::WriteCommandToStdin("save".to_owned()))
            .await;
        self.save_done.notified().await;
    }
    pub async fn stop(&self) {
        let _ignore_err = self.cmd
            .send(ServerCommand::WriteCommandToStdin("save".to_owned()))
            .await;
        self.cmd.closed().await;
    }
}
