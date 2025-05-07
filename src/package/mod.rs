pub mod build;
pub mod dep;
pub mod generate;
pub mod helpers;
pub mod install;
pub mod message;
pub mod prune;
pub mod remove;
pub mod source;
pub mod vf;
pub mod view;

use std::{
    fmt,
    fs::read_to_string,
};

use anyhow::{
    Context,
    Result,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    error,
    instrument,
    warn,
};
use walkdir::WalkDir;

use crate::{
    package::{
        dep::{
            Dep,
            parse_deps,
        },
        remove::is_hidden,
        source::{
            Source,
            parse_sources,
        },
    },
    sex,
    utils::parse::us_array,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub name:       String,
    pub version:    String,
    pub about:      String,
    pub maintainer: String,
    pub licenses:   Vec<String>,

    pub upstream:      Option<String>,
    pub version_fetch: Option<String>,

    pub tags:         Vec<String>,
    pub sources:      Vec<Source>,
    pub dependencies: Vec<Dep>,
    pub kcfg:         Vec<String>,
}

impl Package {
    /// Creates a new package from its pkg file
    #[instrument(level = "debug")]
    fn new(name: &str) -> Self {
        let out = sex!("/usr/share/to/scripts/gen.sh /var/cache/to/pkgs/{name}/pkg").unwrap();
        let lines = out.lines().map(str::trim).collect::<Vec<_>>();

        let [n, v, a, m, l, u, vf, t, s, d, kcfg] = &lines[..] else {
            panic!("Shouldn't happen lol")
        };

        let u = if u.is_empty() { None } else { Some(u.to_string()) };

        let vf = if vf.is_empty() { None } else { Some(vf.to_string()) };

        // NOTE: Tags are not parsed with a unit-separator IFS. This exception exists because tags
        // should never have spaces and it shortens the pkg syntax.
        let t = t.split_whitespace().map(|s| s.to_string()).collect();
        let l = us_array(l);
        let kcfg = us_array(kcfg);

        Self {
            name: n.to_string(),
            version: v.to_string(),
            about: a.to_string(),
            maintainer: m.to_string(),
            licenses: l,
            upstream: u,
            version_fetch: vf,
            tags: t,
            sources: parse_sources(s),
            dependencies: parse_deps(d),
            kcfg,
        }
    }

    // TODO: Use thiserror
    #[instrument(level = "debug")]
    pub fn from_s_file(name: &str) -> Result<Self> {
        let s_file = format!("/var/cache/to/pkgs/{name}/s");
        let s = read_to_string(&s_file)
            .inspect_err(|e| error!("Failed to read {s_file} for {name}: {e}"))
            .context("Failed to read s_file")?;
        serde_json::de::from_str(&s)
            .inspect_err(|e| error!("Failed to deserialize {name}: {e}"))
            .context("Failed to deserialize package")
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// # Finds the names of all packages in /var/cache/to/pkgs
#[instrument(level = "debug")]
pub fn all_package_names() -> Vec<String> {
    WalkDir::new("/var/cache/to/pkgs")
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
        .map(|de| de.file_name().to_string_lossy().to_string())
        .collect()
}

#[cfg(test)]
mod test {
    use std::{
        path::Path,
        process::{
            ExitCode,
            Termination,
        },
    };

    use super::*;

    // Don't mind the cursed test "skipping" setup
    // Stolen and adapted from https://plume.benboeckel.net/~/JustAnotherBlog/skipping-tests-in-rust

    #[derive(Debug)]
    #[allow(dead_code)] // Used to display skip messages for tests, even though it's not "used"
    struct Skip(&'static str);

    impl Termination for Skip {
        fn report(self) -> ExitCode { 77.into() }
    }

    macro_rules! skip {
        ($reason:expr) => {
            return Err(Skip($reason))
        };
    }

    #[test]
    fn test_all_package_names_depth() -> Result<(), Skip> {
        let pkgdir = Path::new("/var/cache/to/pkgs");

        if !pkgdir.exists() {
            skip!("Missing package directory")
        }

        if !pkgdir.join("efibootmgr").exists() {
            skip!("Missing package 'efibootmgr'")
        }

        assert!(all_package_names().contains(&"efibootmgr".to_owned()));
        Ok(())
    }
}
