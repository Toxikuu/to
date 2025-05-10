// package/source.rs

use std::{
    fmt,
    path::{
        Path,
        PathBuf,
    },
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
    debug,
    info,
    instrument,
};

use super::Package;
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
/// z, -> zip
/// t, -> tar
/// g, -> git
/// p, -> pkg
///
/// A raw source string might look something like the following:
/// - Git (guess kind): "https://github.com/toxikuu/to.git"
/// - Git (explicit kind): "g,https://github.com/toxikuu/to.git -> to"
/// - Git (guess dest): "https://github.com/toxikuu/to.git"
/// - Git (explicit dest): "https://github.com/toxikuu/to.git -> to"
///
/// - Zip (guess dest): "https://link.to/archive.zip"
/// - Zip (explicit dest): "https://link.to/archive.zip -> source.zip"
/// - Zip (guess kind, explicit dest): "https://link.to/archive.zip -> source.zip"
/// - Zip (explicit kind, explicit dest): "z,https://link.to/archive.zip -> source.zip"
///
/// - Tar (guess dest): "https://link.to/archive.tar.xz"
/// - Tar (explicit dest): "https://link.to/archive.tar.xz -> source.txz"
///
/// - Pkg (guess dest): "linux" # to reuse the linux kernel sources
/// - Pkg (explicit dest): "linux -> kernel-src"
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SourceKind {
    Download,
    Git,
    Pkg, // sources from another package
}

impl Package {
    // NOTE: This will not be oxidized as so much of this already relies on bash that I'd rather
    // just continue relying on bash than write hundreds of lines of rust to do the same shit.
    //
    /// # Fetches all the sources for a package
    /// Accounts for a specific source already existing
    pub fn fetch_sources(&self) -> Result<()> {
        let name = &self.name;
        info!("Fetching sources for {self}");
        exec!("mkdir -pv /var/cache/to/sources/{name}")?;
        for source in &self.sources {
            let url = &source.url;
            let path = source.path(self);
            let path_str = path.display();
            debug!("Fetching source: {source:#?}");
            match source.kind {
                | SourceKind::Git => {
                    if path.exists() {
                        exec!("cd '{path_str}' && git pull")?;
                    } else {
                        exec!("git clone --depth=256 '{url}' '{path_str}'")?;
                    }
                },

                | SourceKind::Pkg => {
                    let name = url;
                    let package = Package::from_s_file(name)?;
                    package.fetch_sources().with_context(|| {
                        format!("Failed to fetch sources for source package '{name}'")
                    })?;

                    // Copy only the latest sources for the source package
                    for source in &package.sources {
                        let origin = source.path(&package);
                        let dest =
                            path.join(origin.file_name().context("Source has no file name")?);

                        // If the destination doesn't exist, or its modtime is older than the
                        // origin, copy
                        if !dest.exists()
                            || origin.metadata()?.modified()? > dest.metadata()?.modified()?
                        {
                            exec!("cp -af '{}' '{}'", origin.display(), dest.display())?;
                        }
                    }
                },

                | _ => {
                    if !path.exists() {
                        exec!(
                            "curl -fSL -# -C - --retry 3 -o '{path_str}'.part '{url}' && mv -vf '{path_str}'.part '{path_str}'"
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
