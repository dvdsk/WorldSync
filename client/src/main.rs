use client::gui;
use iced::Application;

fn setup_tracing() {
    use tracing_subscriber::{filter, prelude::*};

    let filter_modules = filter::filter_fn(|metadata| {
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
            && metadata.line().map(|n| n == 132).unwrap_or(false)
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
    });
    let fmt = tracing_subscriber::fmt::layer().pretty().with_test_writer();

    let _ignore_err = tracing_subscriber::registry()
        .with(fmt)
        .with(filter::LevelFilter::INFO)
        .with(filter_modules)
        .try_init();
}

#[cfg(not(feature = "deployed"))]
pub fn main() -> iced::Result {
    setup_tracing();
    gui::State::run(iced::Settings::default())
}

#[cfg(feature = "deployed")]
pub fn main() -> iced::Result {
    gui::State::run(iced::Settings::default())
}
