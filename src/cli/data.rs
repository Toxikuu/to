use std::process::exit;

use clap::Args;

use super::CommandError;
use crate::package::Package;

#[derive(Args, Debug)]
pub struct Command {
    /// Package name, optionally with the version
    #[arg(value_name = "PACKAGE")]
    pub package: String,

    /// Print the upstream of a package
    #[arg(long, short = 'U')]
    pub upstream: bool,

    /// Print the version of a package
    #[arg(long, short = 'V')]
    pub version: bool,

    /// Print the installed version of a package
    #[arg(long, short = 'I')]
    pub installed_version: bool,

    /// Exit 0 if the package is installed
    #[arg(long, short = 'i')]
    pub is_installed: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkg = Package::from_s_file(&self.package)?;

        if self.version {
            println!("{}", pkg.version);
        }

        if self.installed_version {
            println!("{}", pkg.installed_version().unwrap_or_default())
        }

        if self.upstream {
            println!("{}", pkg.upstream.as_deref().unwrap_or_default())
        }

        if self.is_installed {
            if pkg.is_installed() { exit(0) } else { exit(1) }
        }

        Ok(())
    }
}
