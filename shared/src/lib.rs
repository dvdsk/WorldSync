use std::path::Path;
use std::time::{Duration, SystemTime};

pub use tarpc;
use tarpc::context::Context;
use tracing::Metadata;

pub fn context(seconds: u64) -> Context {
    let mut context = Context::current();
    context.deadline = SystemTime::now() + Duration::from_secs(seconds);
    context
}

fn filter(metadata: &Metadata) -> bool {
    if metadata
        .module_path()
        .map(|p| p.contains("naga::front::spv"))
        .unwrap_or(false)
    {
        return false;
    }

    if metadata // skip that one OpenTelemetry subscriber error
        .file()
        .map(|p| p.ends_with("tarpc/src/client.rs"))
        .unwrap_or(false)
        && metadata.line().map(|n| n == 128).unwrap_or(false)
    {
        return false;
    }

    // levels are orderd ERROR < WARN < INFO < DEBUG < TRACE
    if metadata.level() < &tracing::Level::INFO {
        return true; // return true to report this span
    }

    let is_ignored = |m: &str| {
        m.contains("tarp")
            || m.contains("wgpu")
            || m.contains("naga")
            || m.contains("gfx_backend")
            || m.contains("winit::platform_impl")
    };
    match metadata.module_path() {
        Some(module) if is_ignored(module) => false,
        _ => true,
    }
}

use tracing_appender::non_blocking::WorkerGuard;
/// keeps track of internal state needed for logging to work
#[must_use = "must be kept till the end of the progam for logging to work"]
pub struct Logging {
    #[allow(dead_code)]
    file_guard: WorkerGuard,
}

use tracing_appender::{non_blocking, rolling};
pub use tracing_subscriber::filter::LevelFilter as LogLevel;
use tracing_subscriber::{filter, fmt, prelude::*};
pub fn setup_tracing(log_dir: &Path, log_name: &str, level: LogLevel) -> Logging {
    std::fs::create_dir_all("log_dir").unwrap();
    let file_appender = rolling::daily(log_dir, log_name);

    let (non_blocking, file_guard) = non_blocking(file_appender);
    let file = fmt::layer().pretty().with_writer(non_blocking);
    let stdout = fmt::layer().pretty();

    let filter_modules = filter::filter_fn(filter);
    tracing_subscriber::registry()
        .with(stdout)
        .with(file)
        .with(level)
        .with(filter_modules)
        .try_init()
        .unwrap();

    Logging {
        file_guard,
    }
}

pub fn setup_test_tracing() {
    let fmt = tracing_subscriber::fmt::layer().pretty().with_test_writer();

    let filter_modules = filter::filter_fn(filter);
    let _ignore_err = tracing_subscriber::registry()
        .with(fmt)
        .with(filter::LevelFilter::INFO)
        .with(filter_modules)
        .try_init();
}
