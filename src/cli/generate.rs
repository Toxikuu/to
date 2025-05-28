use clap::Args;

use super::CommandError;
use crate::{
    imply_all,
    package::Package,
};

#[derive(Args, Debug)]
pub struct Command {
    /// The packages to generate
    #[arg(value_name = "PACKAGE", num_args = 1..)]
    pub packages: Vec<String>,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs = imply_all!(self);

        for pkg in &pkgs {
            let name = pkg.split_once('@').map(|(n, _)| n).unwrap_or(pkg);
            Package::generate(name)?;
        }

        Ok(())
    }
}
