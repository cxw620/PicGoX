//! PicGo-like image uploader written in Rust, GUI powered by iced.

// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "alloc-mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod args;
mod gui;
mod task;

use std::time::Duration;

use anyhow::Result;

fn main() -> Result<()> {
    init_tracing();

    tracing::info!("Starting PicGoX...");

    dbg!("dadada");

    // Tokio runtime
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    // Background task handler
    task::TaskHandler::init().spawn(&runtime);

    // Initialize tray item
    gui::tray::Tray::init()?;

    // Initialize GUI, will block the main thread.
    gui::Handler::init(&runtime).run()?;

    // GUI quit, stop the tokio runtime.
    runtime.shutdown_timeout(Duration::from_secs(5));

    tracing::info!("Exited normally. Goodbye!");

    Ok(())
}

#[allow(unused_imports)]
fn init_tracing() {
    use tracing_appender::rolling;
    use tracing_subscriber::{
        filter::LevelFilter,
        fmt::{time::ChronoLocal, writer::MakeWriterExt},
        layer::SubscriberExt,
        util::SubscriberInitExt,
        EnvFilter, Layer,
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(ChronoLocal::new("[%F %X %Z]".to_string()))
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        );

    // TODO: Config controlling log level, log file name and directory.
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(ChronoLocal::new("[%F %X %Z]".to_string()))
        .with_writer(rolling::never("./", "run.log"))
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        );

    tracing_subscriber::registry().with(fmt_layer).with(file_layer).init();
}
