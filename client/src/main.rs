use iced::Application;
use client::gui;

fn setup_tracing() {
    use tracing_subscriber::{filter, prelude::*};

    let filter_modules = filter::filter_fn(|metadata| {
        if let Some(module) = metadata.module_path() {
            !module.contains("tarp") 
            && !module.contains("wgpu")
            && !module.contains("naga")
            && !module.contains("gfx_backend")
            && !module.contains("winit::platform_impl")
        } else {
            true
        }
    });
    let fmt = tracing_subscriber::fmt::layer()
        .pretty()
        .with_test_writer();

    let _ignore_err = tracing_subscriber::registry()
        .with(fmt)
        .with(filter::LevelFilter::INFO)
        .with(filter_modules)
        .try_init();
}

pub fn main() -> iced::Result {
    setup_tracing();
    gui::State::run(iced::Settings::default())
}

