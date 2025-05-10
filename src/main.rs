#![feature(iter_intersperse)]
#![feature(stmt_expr_attributes)]

use std::{
    io,
    str::FromStr,
    sync::OnceLock,
};

use anyhow::{
    Context,
    Result,
};
use clap::Parser;
use structs::{
    cli::{
        Command,
        CommandHandler,
    },
    config::CONFIG,
};
use tracing::{
    Level,
    error,
    level_filters::LevelFilter,
    trace,
};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling,
};
use tracing_subscriber::{
    EnvFilter,
    filter::Directive,
    fmt::time,
    prelude::*,
};

mod package;
mod server;
mod structs;
mod utils;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    log();
    trace!("Initialized logging");

    let cmd = Command::parse();
    trace!("Parsed command: {cmd:#?}");

    CommandHandler::new(cmd.cmd)
        .handle()
        .await
        .context("Command failed")?;

    Ok(())
}

fn log() {
    let file_appender = rolling::never("/var/log", "to.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let level = LevelFilter::from_str(&CONFIG.log_level).unwrap_or(LevelFilter::DEBUG);
    let mut filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .with_env_var("LOG_LEVEL")
        .from_env_lossy();

    // Silence verbose debug logs for dircpy
    filter = filter.add_directive(Directive::from_str("dircpy=warn").unwrap());

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
        error!("[UNREACHABLE] log() was called more than once");
    }
}
