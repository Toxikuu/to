pub mod actions;
pub mod build;
pub mod dep;
pub mod generate;
pub mod helpers;
pub mod install;
pub mod lint;
pub mod message;
pub mod prune;
pub mod pull;
pub mod remove;
pub mod source;
pub mod vf;
pub mod view;

use std::{
    fmt,
    fs::read_to_string,
};

use dep::DepKind;
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
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
    utils::{
        commit_hash,
        parse::us_array,
    },
};

#[derive(Error, Debug)]
pub enum FormError {
    #[error("Failed to read sfile")]
    Io(#[from] std::io::Error),

    #[error("Failed to deserialize package")]
    Deserialization(#[from] serde_json::Error),
}

/// # The package struct for `to`.
///
/// This struct contains metadata about the package obtained from its s file. It also has numerous
/// methods.
///
/// # Notes
/// This struct has several different `Display` formatting options for use in different contexts:
/// * `{self}`          - name@version
/// * `{self:-}`        - name@version, where the version if truncated if it's a commit hash
/// * `{self:+}`        - name@version, with (full) colors based on install status, where the
///   version is truncated if it's a commit hash
///
/// # Terms
/// * s file            - A file containing the package's serialized metadata.
/// * pkg file          - A bash script defining the package, its metadata, and its build instructions.
/// * dist file         - A zstd-compressed tarball containing distribution-ready files for a package.
/// * dl                - A download-ish URI. This can be the URL for a git repo, a download link
///   for a file, or another package. Download links may also contain a destination specified by
///   'link -> destination'.
///
/// # Fields
/// * `name`            - The package's name.
/// * `version`         - The package's version. Can be semver, datever, or a commit hash. Special
///   versions like 9999 are not currently supported.
/// * `about`           - A brief description of the package.
/// * `maintainer`      - The maintainer of the pkg file, and usually the person who builds the
///   dist file.
/// * `licenses`        - Zero or more licenses under which the package is licensed. Some projects
///   have no license -- iana-etc being a notable example. This should be addressed when displaying
///   licenses.
///
/// * `upstream`        - The package's upstream url, if any.
/// * `version_fetch`   - The command necessary to fetch the package's latest version. This usually
///   references the upstream.
///
/// * `tags`            - Zero or more categorizations/keywords for the package. These are
///   currently not standardized, though they may be eventually.
/// * `sources`         - Zero or more dls, or another package. If the dl is not prefixed by a
///   character and a comma, explicitly indicating a source kind, the source kind is guessed.
/// * `dependencies`    - Zero or more dependencies. These may be build-only, runtime-only, or
///   always required.
/// * `kcfg`            - Zero or more kernel config options required for the correct functioning
///   of a package. These are formatted as `option = y/m` or `option_suboption = n`. In other words,
///   the `CONFIG_` prefix may be elided, and the yes-module-no tristate can be expressed by the first
///   character of those states, delimited by a '/'. For instance, `y/m` means yes or module.
#[derive(Deserialize, Serialize, Debug, Clone, Eq, Hash, PartialEq)]
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

    #[serde(skip)]
    pub depkind: Option<DepKind>,
}

impl Package {
    /// Creates a new package from its pkg file
    #[instrument(level = "debug")]
    fn new(name: &str) -> Self {
        let out = sex!("/usr/share/to/scripts/gen.sh /var/db/to/pkgs/{name}/pkg").unwrap();
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
            depkind: None,
        }
    }

    pub fn is_dependency(&self) -> bool { self.depkind.is_some() }

    // TODO: Use thiserror
    #[instrument(level = "debug")]
    pub fn from_s_file(name: &str) -> Result<Self, FormError> {
        let s_file = format!("/var/db/to/pkgs/{name}/s");
        let s = read_to_string(&s_file).inspect_err(|e| error!("Failed to read {s_file}: {e}"))?;
        serde_json::from_str(&s).map_err(|e| {
            error!("Failed to deserialize {s_file}: {e}");
            FormError::Deserialization(e)
        })
    }
}

/// See the documentation for `Package`
impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.sign_minus() {
            write!(
                f,
                "{}@{}",
                self.name,
                commit_hash::try_shorten(&self.version)
            )
        } else if f.sign_plus() {
            if !self.is_installed() {
                write!(f, "  \x1b[30;1m{}@{}\x1b[0m", self.name, self.version)
            } else if self.is_current() {
                write!(f, "  \x1b[32;1m{}@{}\x1b[0m", self.name, self.version)
            } else {
                // WARN: This branch ({package:+} formatting) is very subject to change
                write!(
                    f,
                    "  \x1b[31;1m{}@{iv} -> {}\x1b[0m",
                    self.name,
                    self.version,
                    iv = self
                        .installed_version()
                        .expect("Package is installed but iv not found")
                )
            }
        } else {
            write!(f, "{}@{}", self.name, self.version)
        }
    }
}

/// # Finds the names of all packages in /var/db/to/pkgs
#[instrument(level = "debug")]
pub fn all_package_names() -> Vec<String> {
    WalkDir::new("/var/db/to/pkgs")
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
        let pkgdir = Path::new("/var/db/to/pkgs");

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
