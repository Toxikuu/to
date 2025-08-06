use clap::Args;
use color_eyre::{eyre::bail, Result as Eresult};
use crate::utils::health::check_health;

// TODO: Consider supporting per-package health-checks (probably out of scope)

/// Check health
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub async fn run(&self) -> Eresult<()> {
        if check_health() > 0 {
            bail!("Unhealthy")
        }

        Ok(())
    }
}
