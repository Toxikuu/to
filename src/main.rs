#![feature(iter_intersperse)]
#![feature(stmt_expr_attributes)]
#![feature(duration_constructors_lite)]
#![feature(path_add_extension)]

use clap::Parser;
use config::CONFIG;

use color_eyre::{eyre::Context, Result};

mod cli;
mod config;
mod package;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    utils::log::init();
    color_eyre::install().wrap_err("Failed to initialize error reporter")?;
    cli::Cli::parse().run().await
}
