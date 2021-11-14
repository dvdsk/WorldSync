
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error in the minecraft server")]
    McServer(wrapper::Error),
}
