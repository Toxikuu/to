use std::process::exit;
use clap::Args;
use super::CommandError;
use crate::utils::health::check_health;

// TODO: Consider supporting per-package health-checks (probably out of scope)

/// Check health
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        if check_health() > 0 {
            exit(1)
        }

        Ok(())
    }
}
