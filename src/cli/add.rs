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
    /// The template to use when adding the package
    ///
    /// Supported templates include GitHub repositories, Arch Linux packages,
    ///
    /// The fallback template is `name@version`
    #[arg(value_name = "TEMPLATE", num_args=1..)]
    pub templates: Vec<String>,

    /// Skip templating
    #[arg(long, short)]
    pub finalize_only: bool,

    /// Skip building
    #[arg(long, short)]
    pub skip_build: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        for template in &self.templates {
            if exec_interactive!(
                "FINALIZE_ONLY={} SKIP_BUILD={} {}/add-package {template}",
                &self.finalize_only,
                &self.skip_build,
                super::SCRIPT_DIR,
            )
            .is_err()
            {
                error!("Failed to add package from {template}");
                exit(1)
            };

            info!("Added package from {template}");
        }
        Ok(())
    }
}
