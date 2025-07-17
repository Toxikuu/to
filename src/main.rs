#![feature(iter_intersperse)]
#![feature(stmt_expr_attributes)]
#![feature(duration_constructors_lite)]
#![feature(path_add_extension)]

use std::process::exit;

use clap::Parser;
use config::CONFIG;
use tracing::{
    error,
    trace,
    warn,
};
use utils::file::exists;

mod cli;
mod config;
mod package;
mod server;
mod utils;

#[tokio::main]
async fn main() {
    init();
    if let Err(e) = cli::Cli::parse().run().await {
        error!("{e}");
        unravel!(e);
        exit(1)
    }
}

fn init() {
    utils::log::init();
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
