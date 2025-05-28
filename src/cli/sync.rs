use std::process::exit;

use clap::Args;
use tracing::error;

use super::CommandError;
use crate::{
    config::CONFIG,
    exec,
    utils::file::exists,
};

#[derive(Args, Debug)]
pub struct Command {
    /// The branch to sync
    pub branch: Option<String>,
}

impl Command {
    pub async fn run(&self) -> Result<(), CommandError> {
        if !exists("git") {
            return Err(CommandError::MissingDependency("git".to_string()))
        }

        if exec!(
            r#"
        cd /var/db/to/pkgs

        if ! [ -d .git ]; then
            git clone --depth=1 --no-single-branch {repo} .
        fi

        git fetch origin {branch}
        git checkout "{branch}" || git checkout -b "{branch}" "origin/{branch}"
        git merge --ff-only origin/{branch}
            "#,
            branch = self
                .branch
                .as_deref()
                .unwrap_or(&CONFIG.package_repo_branch),
            repo = &CONFIG.package_repo,
        )
        .is_err()
        {
            error!("Failed to sync local package database");
            exit(1)
        }

        Ok(())
    }
}
