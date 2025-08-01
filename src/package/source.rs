// package/source.rs

use std::{
    fmt,
    io::{
        self,
        ErrorKind,
    },
    path::PathBuf,
};

use fshelpers::{mkdir, mkdir_p};
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
use tracing::{
    debug,
    error,
    info,
    instrument,
};

use super::{
    FormError,
    Package,
};
use crate::{
    exec,
    utils::{
        file::is_download,
        parse::us_array,
    },
};

pub fn parse_sources(raw: &str) -> Vec<Source> {
    us_array(raw)
        .iter()
        .map(|s| Source::from_string(s))
        .collect()
}

#[derive(Deserialize, Serialize, Debug, Clone)]
/// # Source struct
///
/// dest is just a filename, not a path
///
/// The raw source string can be explicit or implicit with kind and dest.
/// url -> dest is explicit, otherwise dest is anything after the final /
/// kind is guessed if not specified at the start with any of the following:
/// d, -> download
/// g, -> git
/// p, -> pkg
///
/// A raw source string might look something like the following:
/// - Download (guess dest): "https://link.to/archive.zip"
/// - Download (explicit dest): "https://link.to/archive.tar.xz -> source.txz"
/// - Download (explicit dest, explicit kind): "d,https://link.to/archive.tar.xz -> source.txz"
///
/// - Git (guess kind): "https://github.com/toxikuu/to.git"
/// - Git (explicit kind): "g,https://github.com/toxikuu/to.git -> to"
/// - Git (guess dest): "https://github.com/toxikuu/to.git"
/// - Git (explicit dest): "https://github.com/toxikuu/to.git -> to"
///
/// - Pkg (guess dest): "linux" # to reuse the linux kernel sources
/// - Pkg (explicit dest): "linux -> kernel-src"
#[derive(PartialEq, Eq, Hash)]
pub struct Source {
    pub kind: SourceKind,
    pub url:  String, // dl from pardl (ex: https://link.com/tarball.tar.gz -> tb.tar.gz)
    pub dest: String,
}

impl Source {
    #[instrument(level = "debug")]
    fn from_string(str: &str) -> Self {
        if let Some((kind, dl)) = str.split_once(',') {
            // explicit
            let kind = match kind {
                | "d" => SourceKind::Download,
                | "g" => SourceKind::Git,
                | "p" => SourceKind::Pkg,
                | _ => panic!("Unknown source kind '{kind}'"),
            };

            if matches!(kind, SourceKind::Pkg) {
                return Self {
                    kind,
                    url: dl.to_string(),
                    dest: dl.to_string(),
                }
            }

            if let Some((url, dest)) = dl.split_once(" -> ") {
                Self {
                    kind,
                    url: url.to_string(),
                    dest: dest.to_string(),
                }
            } else {
                let (_, dest) = dl.rsplit_once('/').expect("Invalid url");
                let dest = if matches!(kind, SourceKind::Git) {
                    dest.trim_end_matches(".git")
                } else {
                    dest
                };

                Self {
                    kind,
                    url: dl.to_string(),
                    dest: dest.to_string(),
                }
            }
        } else {
            // guess
            let kind = if is_download(str) {
                SourceKind::Download
            } else if !str.contains("://") {
                SourceKind::Pkg
            } else {
                SourceKind::Git
            };

            let dl = str;

            if let Some((url, dest)) = dl.split_once(" -> ") {
                Self {
                    kind,
                    url: url.to_string(),
                    dest: dest.to_string(),
                }
            } else {
                let (_, dest) = dl.rsplit_once('/').expect("Invalid url");
                let dest = if matches!(kind, SourceKind::Git) {
                    dest.trim_end_matches(".git")
                } else {
                    dest
                };

                Self {
                    kind,
                    url: dl.to_string(),
                    dest: dest.to_string(),
                }
            }
        }
    }

    pub fn path(&self, package: &Package) -> PathBuf {
        PathBuf::from(format!(
            "/var/cache/to/sources/{}/{}",
            package.name, self.dest
        ))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Hash, Eq)]
pub enum SourceKind {
    Download,
    Git,
    Pkg, // sources from another package
}

#[derive(Error, Debug)]
pub enum SourceError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Form error: {0}")]
    FormError(#[from] FormError),
}

impl Package {
    // NOTE: This will not be oxidized as so much of this already relies on bash that I'd rather
    // just continue relying on bash than write hundreds of lines of rust to do the same shit
    // worse.
    //
    /// # Fetches all the sources for a package
    /// Accounts for a specific source already existing
    pub fn fetch_sources(&self) -> Result<(), SourceError> {
        info!("Fetching sources for {self}");
        mkdir_p(self.sourcedir())?;
        let pkgfile = self.pkgfile();
        for source in &self.sources {
            let url = &source.url;
            let path = source.path(self);
            debug!("Fetching source: {source:#?}");
            // TODO: Add support for SourceKind::Custom
            match source.kind {
                | SourceKind::Git => {
                    if path.exists() {
                        // NOTE: This is probably overkill and redundant, but I'm not a git wizard.
                        exec!(
                            // TODO: Try dropping the `git fetch --depth=1 origin` part
                            r#"  cd '{s}' && tource '{p}' && git fetch --depth=1 origin "${{tag:-{v}}}" && gco "${{tag:-{v}}}"  "#,
                            s = path.display(),
                            p = pkgfile.display(),
                            v = self.version.version,
                        )?;
                    } else {
                        exec!(
                            r#"  tource '{p}' && git clone --depth=1 --recursive '{url}' '{s}' && cd '{s}' && gco "${{tag:-{v}}}"  "#,
                            p = pkgfile.display(),
                            s = path.display(),
                            v = self.version.version,
                        )?;
                    }
                },

                // A package source has all its sources copied from the original package to a
                // subdirectory sharing its name in the current package's source directory
                | SourceKind::Pkg => {
                    let name = url;
                    let package = Package::from_s_file(name)?;
                    package.fetch_sources().inspect_err(|e| {
                        error!("Failed to fetch sources for source package '{name}': {e}");
                    })?;

                    // Copy only the latest sources for the source package
                    for source in &package.sources {
                        let origin = source.path(&package);
                        let dest = path.join(
                            origin
                                .file_name()
                                .ok_or(io::Error::from(ErrorKind::InvalidFilename))?,
                        );

                        // If the destination doesn't exist, or its modtime is older than the
                        // origin, copy
                        if !dest.exists()
                            || origin.metadata()?.modified()? > dest.metadata()?.modified()?
                        {
                            mkdir(&path)?;
                            exec!("cp -af --no-preserve=xattr '{}' '{}'", origin.display(), dest.display())?;
                        }
                    }
                },

                | _ => {
                    if !path.exists() {
                        exec!(
                            "curl -fSL -# -C - --retry 3 -o '{s}'.part '{url}' && mv -vf '{s}'.part '{s}'",
                            s = path.display(),
                        )?;
                    }
                },
            }
        }

        Ok(())
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.url) }
}
