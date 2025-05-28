use std::process::exit;

use clap::Args;
use tracing::{
    error,
    info,
};

use super::CommandError;
use crate::exec_interactive;

#[derive(Args, Debug)]
pub struct Command {
    /// The packages to edit
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// Skip building
    #[arg(long, short)]
    pub skip_build: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        for pkg in &self.packages {
            let name = pkg.split_once('@').map(|(n, _)| n).unwrap_or(pkg);

            if exec_interactive!(
                "SKIP_BUILD={} {}/edit-package {name}",
                self.skip_build,
                super::SCRIPT_DIR,
            )
            .is_err()
            {
                error!("Failed to edit {name}");
                exit(1)
            }

            info!("Edited {name}");
        }
        Ok(())
    }
}
