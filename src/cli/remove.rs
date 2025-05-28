use clap::Args;
use tracing::error;

use super::CommandError;
use crate::package::Package;

#[derive(Args, Debug)]
pub struct Command {
    /// The package(s) to remove
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// Whether to forcibly remove the package
    #[arg(long, short)]
    pub force: bool,

    /// Whether to remove critical packages (BAD IDEA)
    #[arg(long = "im-really-fucking-stupid")]
    pub remove_critical: bool,

    /// Whether to suppress messages
    #[arg(long, short)]
    pub suppress_messages: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = self
            .packages
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        for pkg in &pkgs {
            pkg.remove(self.force, self.remove_critical, self.suppress_messages)
                .inspect_err(|e| error!("Failed to remove {pkg:-}: {e}"))?;
        }

        Ok(())
    }
}
