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

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
// NOTE: Doc dependency support has been dropped as I'd rather just include them as make
// dependencies for the packages for which I want documentation.
pub enum DepKind {
    Required,
    Runtime,
    Build,
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

    pub fn to_package(&self) -> Result<Package> { Package::from_s_file(&self.name) }
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

        trace!("Resolved dependencies: {order:#?}");
        order
            .iter()
            .filter(|d| d.name != self.name)
            .cloned()
            .collect::<Vec<_>>()
    }

    /// Installs dependencies for a package, excepting build dependencies
    #[instrument]
    pub fn install_deps(&self, force: bool, visited: &mut HashSet<String>) -> Result<()> {
        for dep in &self.dependencies {
            if dep.kind == DepKind::Build {
                continue;
            }

            let dp = dep.to_package()?;
            dp.install_inner(force, visited)
                .permit(|e| e.to_string() == "Already installed")?;
        }

        Ok(())
    }
}

impl fmt::Display for Dep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.name) }
}
