// package/view.rs

use tracing::error;

use super::Package;

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

    pub fn view_dependencies(&self) {
        let deps = &self.dependencies;
        if deps.is_empty() {
            println!("No dependencies for {self}");
            return;
        }

        println!("󰪴 \x1b[1mDependencies:\x1b[0m");
        for dep in deps {
            if let Ok(d) = dep.to_package() {
                d.view(0);
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

        // TODO: Use Dep From Package to display depkind too
        println!("󰪴 \x1b[1mDeep dependencies:\x1b[0m");
        for dep in deps {
            dep.view(0)
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
