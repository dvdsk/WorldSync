mod error;
mod events;
pub mod gui;
pub mod mc;
mod world_download;
mod world_upload;
use std::path::Path;

pub use error::Error;
pub use events::Event;
pub use world_download::WorldDl;

pub fn server_path() -> &'static Path {
    Path::new("worldsync/mc_server")
}
pub fn java_path() -> &'static Path {
    Path::new("worldsync/mc_server")
}
pub fn db_path() -> &'static Path {
    Path::new("worldsync/db")
}
pub fn log_path() -> &'static Path {
    Path::new("worldsync/logs")
}
