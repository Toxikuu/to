use std::process::exit;

use clap::Args;
use tracing::error;

use super::CommandError;
use crate::{
    config::CONFIG,
    imply_all,
    package::Package,
};

/// View information about a package
#[derive(Args, Debug)]
pub struct Command {
    /// The package(s) to view
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,

    /// Level of detail, from 0 to 4
    #[arg(
        long,
        short = 'l',
        value_name = "LEVEL",
        num_args = 1,
        default_value_t = 0
    )]
    pub detail: u8,

    /// Show dependencies
    #[arg(long, short = 'd')]
    pub dependencies: bool,

    /// Show reverse dependencies
    #[arg(long, short = 'D')]
    pub dependants: bool,

    /// Show deep dependencies
    #[arg(long, short = '!')]
    pub deep: bool,

    /// View the filetree of a package's distfile
    #[arg(long, short = 'T')]
    pub tree: bool,

    /// With `--tree`, the command to use
    #[arg(long)]
    pub tree_command: Option<String>,

    /// Show messages
    #[arg(long, short)]
    pub messages: bool,

    /// Pretty-print the package struct
    #[arg(long, short = 'x')]
    pub debug: bool,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        let pkgs: Vec<Package> = imply_all!(self)
            .iter()
            .map(|p| Package::from_s_file(p))
            .collect::<Result<_, _>>()?;

        let pkgslen = pkgs.len();
        for (i, pkg) in pkgs.iter().enumerate() {
            if self.messages {
                pkg.view_all_messages(pkgslen > 1);
                continue
            }

            if self.tree {
                let tree_command = self.tree_command.as_ref().unwrap_or(&CONFIG.tree_command);
                println!("File tree for {pkg:-}:");
                if let Err(e) = pkg.view_filetree(tree_command) {
                    error!("Failed to view file tree for {pkg:-}: {e}");
                    exit(1);
                };
                continue
            }

            if self.dependants {
                pkg.view_dependants();
                continue
            }

            if self.dependencies {
                if self.deep {
                    pkg.view_deep_dependencies();
                } else {
                    pkg.view_dependencies();
                }
                continue
            }

            if self.dependants {
                if self.deep {
                    todo!("Deep dependants");
                } else {
                    todo!("Dependants");
                }
                #[allow(unreachable_code)] // stop `todo!()`s from complaining
                continue
            }

            if self.debug {
                pkg.debug_view();
                continue
            }

            pkg.view(self.detail);

            // Formatting
            if pkgslen > 1 && i != pkgslen - 1 && self.detail > 0 {
                println!()
            }
        }

        Ok(())
    }
}
