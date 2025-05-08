// structs/cli.rs
//! Defines clap commands
// TODO: Consider splitting this up into multiple files, or maybe moving function-specific bits to
// their respective files under package/

use anyhow::{
    Context,
    Result,
};
use clap::{
    Args,
    Parser,
};
use futures::future::join_all;
use once_cell::sync::Lazy;
use permitit::Permit;
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    exec,
    exec_interactive,
    package::{
        Package,
        all_package_names,
        vf::{
            display_vf,
        },
    },
    server::{
        self,
        core::ADDR,
    },
};

const SCRIPT_DIR: &str = "/usr/share/to/scripts";

#[derive(Debug, Parser)]
pub struct Command {
    #[command(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Debug, Args)]
pub struct GenerateArgs {
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Arch Linux package, GitHub repo, or standard
    #[arg(value_name = "TEMPLATE", num_args=1..)]
    pub templates: Vec<String>,

    #[arg(long, short)]
    pub finalize_only: bool,

    #[arg(long, short)]
    pub skip_checks: bool,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    /// Just the package name
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    #[arg(long, short)]
    pub skip_checks: bool,
}

#[derive(Debug, Args)]
pub struct BumpArgs {
    /// Name@NewVersion
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// Don't perform any checks before committing
    #[arg(long, short)]
    pub skip_checks: bool,
}

#[derive(Debug, Args)]
pub struct AliasArgs {
    /// Package name, optionally with the version
    #[arg(value_name = "PACKAGE", num_args = 2)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    #[arg(long, short)]
    pub force: bool,

    #[arg(long, short)]
    pub suppress_messages: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    #[arg(long, short)]
    pub force: bool,

    // Force remove critical packages (bad idea)
    #[arg(long = "i-am-really-stupid")]
    pub remove_critical: bool,

    #[arg(long, short)]
    pub suppress_messages: bool,
}

#[derive(Debug, Args)]
pub struct PruneArgs {
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ViewArgs {
    #[arg(value_name = "PACKAGE", num_args=1..)]
    pub packages: Vec<String>,

    /// View messages for a package
    #[arg(long, short)]
    pub messages: bool,

    /// Detail, from 0 to 4
    #[arg(long, short, value_name = "LEVEL", num_args = 1, default_value_t = 0)]
    pub detail: u8,

    /// View all details of a package for debugging
    // TODO: feature(tools)
    #[arg(long, short)]
    pub debug: bool,
}

#[derive(Debug, Args)]
pub struct LintArgs {
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct VfArgs {
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,

    #[arg(long, short)]
    pub outdated_only: bool,
}

#[derive(Debug, Args)]
pub struct ServeArgs {}

#[derive(Debug, Args)]
pub struct PushArgs {
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct PullArgs {
    #[arg(value_name = "PACKAGE", num_args=0..)]
    pub packages: Vec<String>,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    // Server
    /// Run an HTTP server hosting distfiles
    Serve(ServeArgs),

    // Maintainer
    /// Generate a package
    Generate(GenerateArgs),
    /// Add a package
    Add(AddArgs),
    /// Edit a package
    Edit(EditArgs),
    /// Bump a package's version
    Bump(BumpArgs),
    /// Alias a package
    Alias(AliasArgs),
    /// Build a package
    Build(BuildArgs),
    /// Lint a package's pkg file
    /// This should be executed after the package is generated
    Lint(LintArgs),
    /// Fetch a package's upstream version
    Vf(VfArgs),
    /// Push a package's distfile to the server
    Push(PushArgs),
    /// Pull a package's distfile from the server
    Pull(PullArgs),

    // User
    /// Install the latest version of a package
    Install(InstallArgs),
    /// Remove a package
    Remove(RemoveArgs),
    /// Prune stale files for a package
    Prune(PruneArgs),
    /// View a package
    View(ViewArgs),
}

pub static CLI: Lazy<SubCommand> = Lazy::new(SubCommand::parse);

#[derive(Debug)]
pub struct CommandHandler {
    pub cmd: SubCommand,
}

macro_rules! none_to_all {
    ($args:expr) => {
        if $args.packages.is_empty() {
            all_package_names()
        } else {
            $args.packages.to_vec()
        }
    };
}

macro_rules! form_package_or_continue {
    ($arg:expr) => {
        match Package::from_s_file($arg) {
            | Ok(pkg) => pkg,
            | Err(e) => {
                error!("Failed to form {}: {}", $arg, e);
                continue;
            },
        }
    };
}

impl CommandHandler {
    pub fn new(cmd: SubCommand) -> Self { Self { cmd } }

    pub async fn handle(&self) -> Result<()> {
        match &self.cmd {
            // Server
            | SubCommand::Serve(args) => self.handle_serve(args).await,

            // Maintainer
            | SubCommand::Generate(args) => self.handle_generate(args),
            | SubCommand::Add(args) => self.handle_add(args),
            | SubCommand::Alias(args) => self.handle_alias(args),
            | SubCommand::Edit(args) => self.handle_edit(args),
            | SubCommand::Bump(args) => self.handle_bump(args).await,
            | SubCommand::Build(args) => self.handle_build(args),
            | SubCommand::Lint(args) => self.handle_lint(args),
            | SubCommand::Vf(args) => self.handle_vf(args).await,
            | SubCommand::Push(args) => self.handle_push(args).await,
            | SubCommand::Pull(args) => self.handle_pull(args).await,

            // User
            | SubCommand::Install(args) => self.handle_install(args),
            | SubCommand::Remove(args) => self.handle_remove(args),
            | SubCommand::Prune(args) => self.handle_prune(args),
            | SubCommand::View(args) => self.handle_view(args),
        }
    }

    // TODO: Remove this allow once I add arguments for serve
    #[allow(unused_variables)]
    async fn handle_serve(&self, args: &ServeArgs) -> Result<()> { server::core::serve().await }

    async fn handle_push(&self, args: &PushArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);
            let dist = pkg.distfile();
            let distfile = dist.display();
            let filename = dist.file_name().unwrap().display();
            if exec!("curl --data-binary '@{distfile}' '{ADDR}/up/{filename}'").is_err() {
                error!("Failed to push {distfile} for {pkg} with curl")
            }
        }
        Ok(())
    }

    async fn handle_pull(&self, args: &PullArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);
            let dist = pkg.distfile();
            let distfile = dist.display();
            let filename = dist.file_name().unwrap().display();
            // This curl is silent, fails, shows errors, follows redirects, resumes, retries, and writes to a partfile
            // TODO: Rewrite this natively (reference the pardl proof-of-concept)
            if exec!(
                "curl -fsSL -C - --retry 3 -o '{distfile}'.part '{ADDR}/{filename}' && mv -vf '{distfile}'.part '{distfile}'"
            ).is_err() {
                error!("Failed to pull {distfile} for {pkg} with curl")
            }
        }
        Ok(())
    }

    fn handle_generate(&self, args: &GenerateArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let name = pkg_str.split_once('@').map(|(n, _)| n).unwrap_or(pkg_str);
            // TODO: Consider making generate return a result
            Package::generate(name);
        }
        Ok(())
    }

    fn handle_add(&self, args: &AddArgs) -> Result<()> {
        for template in &args.templates {
            if exec_interactive!(
                "FINALIZE_ONLY={} SKIP_CHECKS={} {SCRIPT_DIR}/add-package {template}",
                args.finalize_only,
                args.skip_checks,
            )
            .is_err()
            {
                error!("Failed to add package from {template}");
                continue;
            };

            info!("Added package (from {template})");
        }
        Ok(())
    }

    fn handle_edit(&self, args: &EditArgs) -> Result<()> {
        for pkg_str in &args.packages {
            let name = pkg_str.split_once('@').map(|(n, _)| n).unwrap_or(pkg_str);
            if exec_interactive!(
                "SKIP_CHECKS={} {SCRIPT_DIR}/edit-package {name}",
                args.skip_checks,
            )
            .is_err()
            {
                error!("Failed to edit {pkg_str}");
                continue;
            };
            let pkg = form_package_or_continue!(name);
            info!("Edited {pkg}");
        }
        Ok(())
    }

    async fn handle_bump(&self, args: &BumpArgs) -> Result<()> {
        for pkg_str in &args.packages {
            let (name, curr, new) = if let Some((name, newv)) = pkg_str.split_once('@') {
                let Ok(pkg) = Package::from_s_file(name)
                    .inspect_err(|e| error!("Failed to form {pkg_str}: {e}"))
                else {
                    continue;
                };
                (pkg.name.clone(), pkg.version, newv.to_string())
            } else {
                let pkg = form_package_or_continue!(pkg_str);
                (
                    pkg.name.clone(),
                    pkg.version.clone(),
                    pkg.version_fetch().await?.unwrap_or_default(),
                )
            };

            exec_interactive!(
                "SKIP_CHECKS={} CURR={curr} NEW={new} {SCRIPT_DIR}/bump-package {name}",
                args.skip_checks,
            )?;

            info!("Bumped {name}@{curr} to {new}");
        }
        Ok(())
    }

    fn handle_alias(&self, args: &AliasArgs) -> Result<()> {
        let from = args.packages.first().context("Invalid syntax")?;
        let from = from.split_once('@').map_or(from.as_str(), |(n, _)| n);

        let to = args.packages.last().context("Invalid syntax")?;
        let to = to.split_once('@').map_or(to.as_str(), |(n, _)| n);

        exec_interactive!("{SCRIPT_DIR}/alias-package {from} {to}")?;
        info!("Created alias {to} for {from}");
        Ok(())
    }

    fn handle_build(&self, args: &BuildArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);
            pkg.build()
                .with_context(|| format!("Failed to build {pkg_str}"))?;
        }
        Ok(())
    }

    fn handle_lint(&self, args: &LintArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);
            match pkg.lint() {
                | Ok(_) => info!("Lints passed for {pkg}"),
                | Err(e) => warn!("Lints failed for {pkg}: {e}"),
            }
        }
        Ok(())
    }

    fn handle_install(&self, args: &InstallArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);
            // TODO: Consider making install return install status
            // - Updated
            // - Installed
            // - Reinstalled
            // - Did fuck all because it was already installed and at the latest version
            pkg.install(args.force)
                .permit(|e| e.to_string() == "Already installed")
                .with_context(|| format!("Failed to install {pkg_str}"))?;
        }
        Ok(())
    }

    fn handle_remove(&self, args: &RemoveArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);
            pkg.remove(args.force, args.remove_critical)
                .with_context(|| format!("Failed to remove {pkg_str}"))?;
        }
        Ok(())
    }

    fn handle_prune(&self, args: &PruneArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        for pkg_str in &pkgs {
            let pkg = form_package_or_continue!(pkg_str);

            pkg.prune()
                .with_context(|| format!("Failed to prune {pkg_str}"))?;
        }
        Ok(())
    }

    fn handle_view(&self, args: &ViewArgs) -> Result<()> {
        let pkgs = none_to_all!(args);

        let pkgslen = pkgs.len();
        for p in pkgs.iter().enumerate() {
            let (i, pkg_str) = p;

            let pkg = form_package_or_continue!(pkg_str);

            if args.messages {
                pkg.view_all_messages(pkgslen > 1);
                continue;
            }

            if args.debug {
                pkg.debug_view();
            } else {
                pkg.view(args.detail);
            }

            if pkgslen > 1 && i != pkgslen - 1 && args.detail > 0 {
                println!();
            }
        }

        Ok(())
    }

    async fn handle_vf(&self, args: &VfArgs) -> Result<()> {
        let pkg_strs = none_to_all!(args);

        let mut pkgs = Vec::new();
        for pkg_str in pkg_strs.iter() {
            let pkg = form_package_or_continue!(pkg_str);
            pkgs.push(pkg);
        }

        let tasks = pkgs
            .iter()
            .map(|p| {
                let p_clone = p.clone();
                tokio::spawn(async move { p_clone.vf().await })
            })
            .collect::<Vec<_>>();

        for res in join_all(tasks).await {
            match res {
                | Ok(Ok((n, v, uv, is_current))) => display_vf(&n, &v, &uv, is_current),
                | Ok(_) => {},
                | Err(e) => {
                    error!("Task join error: {e}");
                },
            }
        }

        Ok(())
    }
}
