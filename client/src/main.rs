use client::{gui, log_path};
use iced::Application;
#[cfg(not(feature = "deployed"))]
use tracing::warn;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "worldsync client")]
struct Opt {
    /// Verbosity of the logging, options: TRACE, DEBUG, INFO, WARN or ERROR
    #[structopt(short, long, default_value = "INFO")]
    log_level: shared::LogLevel,
}

pub fn main() -> iced::Result {
    let opt = Opt::from_args();
    let _log_guard = shared::setup_tracing(log_path(), "worldsync.log", opt.log_level);

    #[cfg(not(feature = "deployed"))]
    warn!("Running without deployed feature, can not connect to deployed servers");

    println!("{}", protocol::current_version());

    let mut settings = iced::Settings::default();
    settings.window.size = (500, 400);
    gui::State::run(settings)
}
