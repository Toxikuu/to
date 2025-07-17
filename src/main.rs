#![feature(iter_intersperse)]
#![feature(stmt_expr_attributes)]
#![feature(duration_constructors_lite)]
#![feature(path_add_extension)]

use std::process::exit;

use clap::Parser;
use config::CONFIG;
use tracing::error;

mod cli;
mod config;
mod package;
mod server;
mod utils;

#[tokio::main]
async fn main() {
    utils::log::init();
    if let Err(e) = cli::Cli::parse().run().await {
        error!("{e}");
        unravel!(e);
        exit(1)
    }
}
