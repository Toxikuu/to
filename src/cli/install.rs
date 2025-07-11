use clap::Args;
use tracing::error;

use super::CommandError;
use crate::{
    imply_all,
    package::Package,
};

/// Install a package from its distfile
#[derive(Args, Debug)]
pub struct Command {
    /// The package to install
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// Whether to forcibly install the package
    #[arg(long, short)]
    pub force: bool,

    /// Whether to forcibly install all dependencies
    #[arg(long, short = 'F')]
    pub full_force: bool,

    /// Whether to suppress messages
    #[arg(long, short)]
    pub suppress_messages: bool,

    /// The root directory for package installation
    #[arg(long, short)]
    pub root: Option<String>,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        for pkg in &pkgs {
            pkg.install(self.force, self.full_force, self.suppress_messages, self.root.as_deref())
                .inspect_err(|e| error!("Failed to install {pkg}: {e}"))?;
        }

        Ok(())
    }
}
