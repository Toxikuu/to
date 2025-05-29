#![feature(iter_intersperse)]
#![feature(stmt_expr_attributes)]
#![feature(duration_constructors_lite)]
#![feature(path_add_extension)]

use std::{
    io,
    process::exit,
    str::FromStr,
    sync::OnceLock,
};

use clap::Parser;
use config::CONFIG;
use tracing::{
    Level,
    error,
    level_filters::LevelFilter,
    trace,
    warn,
};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling,
};
use tracing_subscriber::{
    EnvFilter,
    fmt::time,
    prelude::*,
};
use utils::file::exists;

mod cli;
mod config;
mod package;
mod server;
mod utils;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

#[tokio::main]
async fn main() {
    init();
    if let Err(e) = cli::Cli::parse().run().await {
        error!("{e}");
        unravel!(e);
        exit(1)
    }
}

// TODO: Move the init stuff elsewhere
fn log() {
    let file_appender = rolling::never("/var/log", "to.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let level = LevelFilter::from_str(&CONFIG.log_level).unwrap_or(LevelFilter::DEBUG);
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .with_env_var("LOG_LEVEL")
        .from_env_lossy()
        // Silence some loud crates
        .add_directive("dircpy=warn".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("hyper_util=warn".parse().unwrap());

    // Trace-level logs are only written to stdout as they take up a lot of space
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_timer(time::uptime())
        .with_writer(file_writer.with_max_level(Level::DEBUG).and(io::stdout))
        .compact()
        .init();

    if LOG_GUARD.set(guard).is_err() {
        error!("The log() function was called more than once.");
        error!("Please report this as a bug.");
    }
}

fn init() {
    log();
    trace!("Initializing...");
    check_health(); // TODO: Run this once per boot
}

// TODO: Make this modular, split into maintainer and user dependencies
fn check_health() {
    trace!("Checking health");
    // Git is also strongly recommended, but is technically unneeded for most functionality.
    let programs = &[
        "zstd", "tar", "bash", "chroot", "env", "grep", "cp", "touch", "tee", "sed", "mkdir",
    ];

    for program in programs {
        if !exists(program) {
            error!("To dependency '{program}' not found");
            warn!("To functionality is likely to be impaired");
        }
    }
}
