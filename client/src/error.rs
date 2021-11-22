#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("Error in the minecraft server")]
    McServer(wrapper::Error),
    #[error("Lost connection to worldsync server")]
    NoMetaConn,
    #[error("internal server error, please ask admin for help")]
    Internal,
    #[error("session expired or did not exist")]
    SessionExpired,
}

impl From<protocol::Error> for Error {
    fn from(err: protocol::Error) -> Self {
        match err {
            Internal => Self::Internal,
            SessionExpired => Self::SessionExpired,
            _ => panic!("unexpected server error: {:?}", err),
        }
    }
}
