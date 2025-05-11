// package/lint.rs
//! Code related to linting pkg files
// TODO: Gate as a maintainer feature

//! Supported lints include:
//! - Ensuring default field values are not present
//! - Ensuring

use std::{
    fmt,
    fs::read_to_string,
};

use anyhow::{
    Result,
    bail,
};
use thiserror::Error;

use super::Package;

#[derive(Debug, Error)]
pub enum LintError {
    #[error("Failed to read file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Linted: {0}")]
    Linted(#[from] Lint),
}

#[derive(Debug, Error)]
pub enum Lint {
    DefaultValues,
    DefOpportunity,
    IlOpportunity,
    NoCheckout,
    AliasInDependencies,
}

impl fmt::Display for Lint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            | Lint::DefaultValues => "Default Values",
            | Lint::DefOpportunity => "Def Opportunity",
            | Lint::IlOpportunity => "Il Opportunity",
            | Lint::NoCheckout => "No Checkout",
            | Lint::AliasInDependencies => "Alias in Dependencies",
        };
        write!(f, "{s}")
    }
}

impl Package {
    pub fn lint(&self) -> Result<()> {
        let pkgfile = self.pkgfile();
        let contents = read_to_string(pkgfile)?;

        // TODO: Consider implementing Default (or maybe has_defaults to avoid confusion) for
        // Package instead of linting this way
        let lines = contents.lines();
        let lines_vec = lines.clone().collect::<Vec<_>>();

        if lints::default_values(lines) {
            bail!(Lint::DefaultValues)
        }

        if lints::def_opportunity(&lines_vec) {
            bail!(Lint::DefOpportunity)
        }

        if lints::il_opportunity(&lines_vec) {
            bail!(Lint::IlOpportunity)
        }
        if lints::no_checkout(self, &lines_vec) {
            bail!(Lint::NoCheckout)
        }

        if lints::alias_in_dependencies(self) {
            bail!(Lint::AliasInDependencies)
        }

        Ok(())
    }
}

mod lints {
    use std::str::Lines;

    use crate::package::{
        Package,
        source::SourceKind,
    };

    /// # Checks whether default values have been used
    /// This lint checks key value pairs to see if they match those in the pkg template
    pub fn default_values(lines: Lines<'_>) -> bool {
        let defaults = [
            ("n", "NAME"),
            ("v", "VERSION"),
            ("a", "ABOUT"),
            ("m", "MAINTAINER"),
            ("l", "LICENSE"),
            ("u", "UPSTREAM"),
        ];

        for line in lines {
            if let Some(kv) = line.split_once('=') {
                if defaults.contains(&kv) {
                    return true
                }
            }
        }
        false
    }

    /// # Checks whether def could have been used
    /// This lint checks windows of all lines to see whether they contain a def
    pub fn def_opportunity(lines_vec: &[&str]) -> bool {
        let defs = [
            ["cfg", "mk", "mi"],
            ["cm", "mk", "mi"],
            ["cm", "nj", "ni"],
            ["ms", "nj", "ni"],
        ];

        defs.iter()
            .any(|def| lines_vec.windows(def.len()).any(|w| w == def))
    }

    /// # Checks whether il could have been used
    /// This lint checks whether a line contains 'install' and the licenses path
    pub fn il_opportunity(lines_vec: &[&str]) -> bool {
        lines_vec
            .iter()
            .any(|l| l.contains("install -") && l.contains("/usr/share/licenses"))
    }

    /// # Checks whether git checkout was omitted
    /// This lint only runs if a package has a git source
    pub fn no_checkout(package: &Package, lines_vec: &[&str]) -> bool {
        if package
            .sources
            .iter()
            .any(|s| matches!(s.kind, SourceKind::Git))
        {
            lines_vec
                .iter()
                .all(|l| !l.contains("git checkout ") && !l.contains("gco"))
        } else {
            false
        }
    }

    /// # Checks whether a package has an alias in its dependencies
    /// Checks if the pkdir is a symlink
    pub fn alias_in_dependencies(package: &Package) -> bool {
        package.dependencies.iter().any(|d| {
            d.to_package()
                .expect("Failed to form dependency")
                .pkgdir()
                .is_symlink()
        })
    }
}
