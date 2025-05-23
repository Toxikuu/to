// package/vf.rs
//! Functions for fetching package upstreams

use std::{
    fs::{
        read_to_string,
        write,
    },
    path::PathBuf,
    time::{
        Duration,
        SystemTime,
    },
};

use anyhow::{
    Context,
    Result,
    anyhow,
    bail,
};
use fshelpers::{
    mkdir_p,
    rmf,
};
use serde::{
    Deserialize,
    Serialize,
};
use tracing::{
    debug,
    error,
    trace,
};

use crate::{
    package::Package,
    sex,
    structs::cli::{
        CLI,
        SubCommand,
    },
    utils::parse::is_commit_sha,
};

impl Package {
    /// # Fetches the upstream version of a package
    /// *vf is short for version fetch*
    ///
    /// The command used for this fetch is defined in the vf field in a pkg file. If no command is
    /// provided, a sane default is used. However, since many packages don't use a sane tagging
    /// scheme, or haven't historically, a vf must often be manually specified.
    ///
    /// Matches don't have to be exact -- just good enough. Leading `$n-`'s are stripped, as are
    /// leading `v`'s. Matches are then trimmed. Only the last line of a match is taken.
    ///
    /// Version fetching may be disabled by specifying `no` as the value for vf.
    ///
    /// A version is usually a tag, though it can also be a commit sha. Default behavior differs
    /// for both. If the version is a commit sha, generally no vf is needed since the latest commit
    /// is pretty predictable with a simple `git ls-remote $u HEAD`.
    ///
    /// The gr and vfs bash functions are tentatively defined in `envs/base.env`.
    pub async fn version_fetch(&self) -> Result<Option<String>> {
        let Some(u) = &self.upstream else {
            return Ok(None);
        };

        if let SubCommand::Vf(args) = &*CLI {
            if args.ignore_cache {
                debug!("Ignoring vf cache for {self}");
            } else {
                // Try to uncache
                // TODO: Kinda cursed, not sure if I wanna keep it this way. Maybe I should only cache the
                // upstream version instead of the whole Vf struct.
                match Vf::uncache(self) {
                    | Ok(vf) => {
                        debug!("Vf cache hit for {self}");
                        return Ok(Some(vf.uv));
                    },
                    | Err(e) => {
                        trace!("Cache miss for {self}: {e}");
                    },
                }
            }
        }

        let vf_cmd = if let Some(vf) = &self.version_fetch {
            if vf == "no" {
                debug!("Version fetch is disabled for {self}");
                return Ok(None);
            }
            format!("u={u} {vf} | tail -n1")
        } else if is_commit_sha(&self.version) {
            format!("git ls-remote '{u}' HEAD | grep '\\sHEAD$' | cut -f1")
        } else {
            format!("gr '{u}' | vfs | sort -V | tail -n1")
        };

        let estimate =
            sex!("{vf_cmd}").with_context(|| format!("Failed to fetch version for {self}"));

        Ok(Some(
            estimate?
                .to_ascii_lowercase()
                .trim_start_matches(&self.name)
                .trim_start_matches('-')
                .trim_start_matches('v')
                .trim()
                .to_string(),
        ))
    }

    pub async fn vf(&self) -> Result<Vf> {
        let uv = self.version_fetch().await.map_err(|e| {
            error!("Failed to fetch upstream version for {self}: {e}");
            anyhow!("Failed to fetch upstream")
        })?;

        let n = &self.name;
        let v = &self.version;

        match uv {
            | Some(uv) => Ok(Vf::new(n, v, &uv)),
            | None => {
                debug!("Upstream version fetching is disabled for {self}. Skipping...");
                bail!("Disabled");
            },
        }
    }
}

// TODO: Consider tracking whether it has been cached
#[derive(Debug, Serialize, Deserialize)]
pub struct Vf {
    /// Name
    n:          String,
    /// Version
    v:          String,
    /// Upstream Version
    uv:         String,
    is_current: bool,
}

impl Vf {
    fn new(n: &str, v: &str, uv: &str) -> Self {
        Self {
            n:          n.to_string(),
            v:          v.to_string(),
            uv:         uv.to_string(),
            is_current: v == uv,
        }
    }

    /// # Displays the vf for a single package
    pub fn display(&self) {
        let n = &self.n;
        let v = &self.v;
        let uv = &self.uv;

        if self.is_current {
            println!("\x1b[37;1m[\x1b[32m*\x1b[37m]\x1b[0m \x1b[32m{n:<32}\x1b[0m {v} ~ {uv}");
        } else {
            println!("\x1b[37;1m[\x1b[31m-\x1b[37m]\x1b[0m \x1b[31m{n:<32}\x1b[0m {v} ~ {uv}");
        }
    }

    /// # Caches a Vf
    /// The Vf gets serialized to json and written to /var/cache/to/data/$n/vf
    /// A Vf is not recached if the cache file exists, since it is assumed that the Vf must have
    /// been uncached if the cache file exists.
    // TODO: Consider only caching uv instead of the whole vf struct
    pub fn cache(&self) -> Result<()> {
        let cache_file = Self::cache_file(&self.n);

        mkdir_p(
            cache_file
                .parent()
                .expect("Cache file should have a parent"),
        )?;

        if cache_file.exists() {
            debug!("Not recaching vf for {}@{}", &self.n, &self.v);
            bail!("Not recaching")
        }

        let ser = serde_json::to_string_pretty(&self)?;
        write(cache_file, &ser).context("Failed to write cache")?;
        debug!("Cached vf for {}@{}", &self.n, &self.v);

        Ok(())
    }

    pub fn cache_file(n: &str) -> PathBuf { PathBuf::from("/var/cache/to/data").join(n).join("vf") }

    /// # Attempts to uncache a vf
    /// Won't uncache if there is no cache file or the cache file is more than 4 hours old
    // TODO: Consider just taking package_name: &str instead of &Package
    pub fn uncache(package: &Package) -> Result<Vf> {
        let cache_file = Vf::cache_file(&package.name);

        if !cache_file.exists() {
            bail!("No cache")
        }

        let four_hours_ago = SystemTime::now() - Duration::from_hours(4);
        if cache_file.metadata()?.modified()? < four_hours_ago {
            rmf(cache_file)?;
            bail!("Cache too old")
        }

        let contents = read_to_string(cache_file)?;
        let vf = serde_json::from_str(&contents)?;
        Ok(vf)
    }
}
