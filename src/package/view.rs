// package/view.rs

use std::{
    io,
    process::exit,
};

use fshelpers::{
    mkdir_p,
    rmdir_r,
};
use tracing::error;

use super::Package;
use crate::{
    exec,
    sex,
};

impl Package {
    /// # Print out information about a package, with varying levels of detail
    ///
    /// **Detail levels:**
    /// - 0 => name, version
    /// - 1 => 0, about
    /// - 2 => 1, tags, licenses
    /// - 3 => 2, dependencies, kcfg
    /// - 4 => 3, upstream, sources, distfile, maintainer
    pub fn view(&self, detail: u8) {
        // TODO: Format this with [*] name@version instead
        println!("{self:+}");

        if detail == 0 {
            return;
        }

        let about = &self.about;
        println!(" \x1b[3m{about}\x1b[0m");

        if detail == 1 {
            return;
        }

        let tags = if self.tags.is_empty() { "No tags" } else { &self.tags.join(", ") };

        let licenses = &self.licenses.join(", ");

        println!("󰓻 \x1b[3m{tags}\x1b[0m");
        println!(" \x1b[3m{licenses}\x1b[0m");

        if detail == 2 {
            return;
        }

        let deps = if self.dependencies.is_empty() {
            "None"
        } else {
            &self
                .dependencies
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n - ")
        };

        let kcfg = if self.kcfg.is_empty() { "None" } else { &self.kcfg.join("\n - ") };

        println!("\n󰪴 \x1b[1mDependencies:\n\x1b[0;3m - {deps}\x1b[0m");
        println!(" \x1b[1mKernel config options:\n\x1b[0;3m - {kcfg}\x1b[0m");

        if detail == 3 {
            return;
        }

        let upstream = &self.upstream.as_deref().unwrap_or("No upstream");
        let sources = &self
            .sources
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let distfile = &self.distfile();
        let pkgfile = &self.pkgfile();

        println!("\n󰘬 \x1b[3m{upstream}\x1b[0m");
        println!(" \x1b[3m{sources}\x1b[0m");
        println!("󰏗 \x1b[3m{}\x1b[0m", distfile.display());
        println!(" \x1b[3m{}\x1b[0m", pkgfile.display());
    }

    pub fn view_dependants(&self) {
        let deps = &self.dependants().unwrap_or_else(|e| {
            error!("Failed to form one or more packages: {e}");
            exit(1);
        });

        if deps.is_empty() {
            println!("Nothing depends on {self}");
            return;
        }

        println!("󰪴 \x1b[1mDependants:\x1b[0m");
        for dep in deps {
            println!("{dep:+}");
        }
    }

    // # View a package's file tree, with a custom tree command
    //
    // This function extracts the package's distfile to `/var/tmp/to/tree`, complaining if the
    // distfile doesn't exist. It then executes a custom tree command.
    //
    // # Arguments
    // * `tree_command`     - The `tree` command to execute on `/var/tmp/to/tree`
    //
    // # Errors
    // - I/O errors
    // - Distfile extraction or tree command failed
    pub fn view_filetree(&self, tree_command: &str) -> io::Result<()> {
        if !self.distfile().exists() {
            error!("No distfile for {self:-} -- can't view filetree");
            exit(1)
        }

        rmdir_r("/var/tmp/to/tree")?;
        mkdir_p("/var/tmp/to/tree")?;
        exec!(
            "
            tar xvf '{distfile}'        \
            -C /var/tmp/to/tree         \
            --keep-directory-symlink    \
            --numeric-owner             \
            --no-overwrite-dir          \
            --exclude=MANIFEST
            ",
            distfile = self.distfile().to_string_lossy()
        )?;

        println!("{}", sex!("cd /var/tmp/to/tree; {tree_command}")?);
        Ok(())
    }

    pub fn view_dependencies(&self) {
        let deps = &self.dependencies;
        if deps.is_empty() {
            println!("No dependencies for {self}");
            return;
        }

        println!("󰪴 \x1b[1mDependencies:\x1b[0m");
        for dep in deps {
            if let Ok(d) = dep.to_package() {
                let kind = dep.kind;
                let dep = format!("{d:+}");
                println!("{dep:<48} ({kind})");
            } else {
                error!("Failed to form package from dependency: {dep}")
            }
        }
    }

    pub fn view_deep_dependencies(&self) {
        let deps = &self.resolve_deps(|_| true);
        if deps.is_empty() {
            println!("No dependencies for {self}");
            return;
        }

        println!("󰪴 \x1b[1mDeep dependencies:\x1b[0m");
        for dep in deps {
            let kind = dep.depkind.unwrap();
            let dep = format!("{dep:+}");
            println!("{dep:<48} ({kind})");
        }
    }

    pub fn debug_view(&self) {
        println!("{self:#?}");

        let deps = &self.resolve_deps(|_| true);
        println!("\nDeep dependencies:");
        for dep in deps {
            println!(" - {dep}");
        }
    }
}
