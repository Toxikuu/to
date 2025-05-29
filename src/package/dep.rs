// package/dep.rs

use std::{
    collections::HashSet,
    fmt,
};

use permitit::Permit;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    debug,
    instrument,
    trace,
};

use super::{
    FormError,
    Package,
    install::InstallError,
};
use crate::utils::parse::us_array;

pub fn parse_deps(raw: &str) -> Vec<Dep> {
    us_array(raw).iter().map(|s| Dep::from_string(s)).collect()
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Hash)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy, Hash)]
// NOTE: Doc dependency support has been dropped as I'd rather just include them as make
// dependencies for the packages for which I want documentation.
pub enum DepKind {
    Required,
    Runtime,
    Build,
}

impl fmt::Display for DepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            | Self::Required => write!(f, "Required"),
            | Self::Runtime => write!(f, "Runtime"),
            | Self::Build => write!(f, "Build"),
        }
    }
}

impl Dep {
    pub fn from_string(str: &str) -> Self {
        if let Some((kind, str)) = str.split_once(',') {
            let kind = match kind {
                | "b" => DepKind::Build,
                | "r" => DepKind::Runtime,
                | _ => panic!("Unknown dep kind: {kind}"),
            };

            Self { name: str.to_string(), kind }
        } else {
            Self {
                name: str.to_string(),
                kind: DepKind::Required,
            }
        }
    }

    /// # Convert a dependency to a package
    ///
    /// This function does not sacrifice dependency data. `DepKind` is added as a field to
    /// `Package`.
    pub fn to_package(&self) -> Result<Package, FormError> {
        let mut pkg = Package::from_s_file(&self.name)?;
        pkg.depkind = Some(self.kind);
        Ok(pkg)
    }
}

impl Package {
    #[instrument(skip(self, resolved, seen, order), level = "trace")]
    fn deep_deps(
        &self,
        resolved: &mut HashSet<Dep>,
        seen: &mut HashSet<Package>,
        order: &mut Vec<Package>,
    ) {
        for dep in &self.dependencies {
            if resolved.insert(dep.clone()) {
                dep.to_package()
                    .expect("Failed to form dependency package")
                    .deep_deps(resolved, seen, order);
            }
        }

        // Avoid resolving dependencies for packages we've already resolved
        if seen.insert(self.clone()) {
            order.push(self.clone());
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub fn resolve_deps(&self) -> Vec<Package> {
        debug!("Resolving dependencies for {self:-}");
        let mut resolved = HashSet::new();
        let mut seen = HashSet::new();
        let mut order = Vec::new();
        self.deep_deps(&mut resolved, &mut seen, &mut order);

        order.retain(|d| d.name != self.name);

        trace!("Resolved dependencies for {self:-}:");
        for dep in &order {
            trace!(" - {dep} ({})", dep.depkind.unwrap());
        }

        order
    }

    /// # Installs dependencies for a package
    ///
    /// This function does not install build dependencies. It optionally installs runtime
    /// dependencies.
    ///
    /// # Arguments
    /// * `force`           - Whether to forcibly install all dependencies
    /// * `install_runtime` - Whether to install runtime dependencies (they are unwanted and
    ///   problematic in the build chroot)
    /// * `visited`         - A hashset of already visited dependencies to avoid infinite recursion
    ///
    /// # Errors
    /// - Will fail if a dependency could not be converted to a package (which shouldn't happen)
    /// - Will fail if a dependency could not be installed
    #[instrument(skip(self, install_runtime, visited, force))]
    pub fn install_deps(
        &self,
        force: bool,
        install_runtime: bool,
        visited: &mut HashSet<String>,
        suppress: bool,
    ) -> Result<(), InstallError> {
        for dep in &self.dependencies {
            if dep.kind == DepKind::Build {
                // trace!("Not installing build dependency '{dep}'");
                continue;
            }

            if dep.kind == DepKind::Runtime && !install_runtime {
                // trace!("Not installing runtime dependency '{dep}'");
                continue;
            }

            trace!("Installing dependency '{dep}' for '{self:-}'");

            // Install all required dependencies
            dep.to_package()?
                .install_inner(force, force, visited, suppress)
                .permit(|e| matches!(e, InstallError::AlreadyInstalled))?;
        }

        Ok(())
    }
}

impl fmt::Display for Dep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name) }
}

#[cfg(test)]
mod test {
    use crate::package::{
        Package,
        dep::DepKind,
    };

    #[test]
    /// Confirm make-ca isn't a dependency of itself, as it sometimes wants to be.
    fn make_ca_runtime_of_self() {
        let pkg = Package::from_s_file("make-ca").unwrap();

        let all_deps = pkg.resolve_deps();
        dbg!(&all_deps);

        assert!(all_deps.iter().all(|d| d.name != "make-ca"))
    }

    #[test]
    /// Since make-ca is both a runtime and a required dependency, some weird shit used to happen.
    /// Test to ensure that weird shit doesn't happen.
    ///
    /// By weird shit, I mean the dependency resolver used to only hash dependencies by name, not
    /// also by kind. This should be fixed now.
    fn rust_make_ca_dep() {
        let pkg = Package::from_s_file("rust").unwrap();

        let all_deps = pkg.resolve_deps();
        eprintln!("Deps:");
        for dep in &all_deps {
            eprintln!("{:>16} ({})", dep.name, dep.depkind.unwrap())
        }

        assert!(
            all_deps
                .iter()
                .filter(|d| d.name == "make-ca")
                .collect::<Vec<_>>()
                .len()
                > 1
        );

        assert!(
            all_deps
                .iter()
                .any(|d| d.name == "make-ca" && d.depkind.unwrap() == DepKind::Required)
        );

        assert!(
            all_deps
                .iter()
                .any(|d| d.name == "make-ca" && d.depkind.unwrap() == DepKind::Runtime)
        );

        let deps = all_deps
            .into_iter()
            .filter(|d| d.depkind.expect("Dep should have a kind") != DepKind::Runtime)
            .collect::<Vec<_>>();

        assert!(deps.iter().any(|d| d.name == "make-ca"));
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Required));
    }
}
