use clap::Args;
use tracing::{
    error,
    info,
};

use color_eyre::eyre::{Result as Eresult, bail};
use crate::exec_interactive;

/// Add a package to the package repository
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
    pub async fn run(&self) -> Eresult<()> {
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
                bail!("Failed to add package from {template}");
            };

            info!("Added package from {template}");
        }
        Ok(())
    }
}
