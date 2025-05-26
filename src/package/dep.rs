// package/dep.rs

use std::{
    collections::HashSet,
    fmt,
};

use anyhow::Result;
use permitit::Permit;
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    instrument,
    trace,
};

use super::Package;
use crate::utils::parse::us_array;

pub fn parse_deps(raw: &str) -> Vec<Dep> {
    us_array(raw).iter().map(|s| Dep::from_string(s)).collect()
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy)]
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
    #[instrument(level = "debug")]
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
    pub fn to_package(&self) -> Result<Package> {
        let mut pkg = Package::from_s_file(&self.name)?;
        pkg.depkind = Some(self.kind);
        Ok(pkg)
    }
}

impl Package {
    #[instrument(level = "trace")]
    fn deep_deps(
        &self,
        resolved: &mut HashSet<String>,
        seen: &mut HashSet<String>,
        order: &mut Vec<Package>,
    ) {
        for dep in &self.dependencies {
            // PERF: I'm like 99% sure !resolved.contains is redundant
            if !resolved.contains(&dep.name) {
                resolved.insert(dep.name.clone());

                dep.to_package()
                    .expect("Failed to form dependency package")
                    .deep_deps(resolved, seen, order);
            }
        }

        if seen.insert(self.to_string().clone()) {
            order.push(self.clone());
        }
    }

    #[instrument(level = "debug")]
    pub fn resolve_deps(&self) -> Vec<Package> {
        let mut resolved = HashSet::new();
        let mut seen = HashSet::new();
        let mut order = Vec::new();
        self.deep_deps(&mut resolved, &mut seen, &mut order);

        let deps = order
            .iter()
            .filter(|d| d.name != self.name)
            .cloned()
            .collect::<Vec<_>>();

        trace!("Resolved dependencies for {self:-}:");
        for dep in &deps {
            trace!(" - {dep} ({})", dep.depkind.unwrap());
        }

        deps
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
    ) -> Result<()> {
        for dep in &self.dependencies {
            if dep.kind == DepKind::Build {
                trace!("Not installing build dependency '{dep}'");
                continue;
            }

            if dep.kind == DepKind::Runtime && !install_runtime {
                trace!("Not installing runtime dependency '{dep}'");
                continue;
            }

            dep.to_package()?
                .install_inner(force, visited)
                .permit(|e| e.to_string() == "Already installed")?;
        }

        Ok(())
    }
}

impl fmt::Display for Dep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name) }
}
