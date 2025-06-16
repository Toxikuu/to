// package/vf.rs
//! Functions for fetching package upstreams

use std::{
    fs::{
        read_to_string,
        write,
    },
    io,
    path::PathBuf,
    time::{
        Duration,
        SystemTime,
    },
};

use fshelpers::{
    mkdir_p,
    rmf,
};
use permitit::Permit;
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
use tracing::{
    debug,
    error,
    trace,
    warn,
};

use crate::{
    package::Package,
    sex,
    utils::{
        commit_hash::try_shorten,
        parse::is_commit_sha,
    },
};

#[derive(Debug, Error)]
pub enum VfError {
    #[error("Failed to fetch version")]
    Fetch,

    #[error("Version fetching is disabled for this package")]
    Disabled,

    #[error("Cache error: {0}")]
    Cache(#[from] VfCacheError),
}

#[derive(Debug, Error)]
pub enum VfCacheError {
    #[error("No cache")]
    NoCache,

    #[error("Cache too old")]
    TooOld,

    #[error("Not recaching")]
    NotRecaching,

    #[error("I/O error")]
    Io(#[from] io::Error),
}

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
    pub async fn version_fetch(&self, ignore_cache: bool) -> Result<Option<String>, VfError> {
        let Some(u) = &self.upstream else {
            return Ok(None);
        };

        if ignore_cache {
            debug!("Ignoring vf cache for {self:-}")
        } else {
            // Try to uncache
            // TODO: Kinda cursed, not sure if I wanna keep it this way. Maybe I should only cache the
            // upstream version instead of the whole Vf struct.
            match Vf::uncache(self) {
                | Ok(vf) => {
                    debug!("Vf cache hit for {self:-}");
                    return Ok(Some(vf.uv));
                },
                | Err(e) => {
                    trace!("Cache miss for {self:-}: {e}");
                },
            }
        }

        let vf_cmd = if let Some(vf) = &self.version_fetch {
            if vf == "no" {
                debug!("Version fetch is disabled for {self:-}");
                return Ok(None);
            }
            format!("u={u} {vf} | tail -n1")
        } else if is_commit_sha(&self.version) {
            format!("git ls-remote '{u}' HEAD | grep '\\sHEAD$' | cut -f1")
        } else {
            format!("gr '{u}' | vfs | sort -V | tail -n1")
        };

        let estimate = sex!("{vf_cmd}").map_err(|_| VfError::Fetch)?;

        Ok(Some(
            estimate
                .to_ascii_lowercase()
                .trim_start_matches(&self.name)
                .trim_start_matches('-')
                .trim_start_matches('v')
                .trim()
                .to_string(),
        ))
    }

    pub async fn vf(&self, ignore_cache: bool) -> Result<Vf, VfError> {
        let uv = self.version_fetch(ignore_cache).await.inspect_err(|e| {
            error!("Failed to fetch upstream version for {self:-}: {e}");
        })?;

        let n = &self.name;
        let v = &self.version;

        match uv {
            | Some(uv) => {
                let vf = Vf::new(n, v, &uv);
                if let Err(e) = vf
                    .cache()
                    .permit(|e| matches!(e, VfCacheError::NotRecaching))
                {
                    warn!("Failed to cache vf for {self:-}: {e}")
                };
                Ok(vf)
            },
            | None => {
                debug!("Upstream version fetching is disabled for {self:-}. Skipping...");
                Err(VfError::Disabled)
            },
        }
    }
}

// TODO: Consider tracking whether it has been cached
#[derive(Debug, Serialize, Deserialize)]
pub struct Vf {
    /// Name
    pub n:          String,
    /// Version
    pub v:          String,
    /// Upstream Version
    pub uv:         String,
    pub is_current: bool,
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
        let v = try_shorten(&self.v); // TODO: Document that only the local version is shortened, and why
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
    // PERF: Cache only uv instead of the whole vf struct
    pub fn cache(&self) -> Result<(), VfCacheError> {
        let cache_file = Self::cache_file(&self.n);

        mkdir_p(
            cache_file
                .parent()
                .expect("Cache file should have a parent"),
        )?;

        if cache_file.exists() {
            debug!("Not recaching vf for {}@{}", &self.n, try_shorten(&self.v));
            return Err(VfCacheError::NotRecaching)
        }

        let ser = serde_json::to_string_pretty(&self).unwrap(); // PERF: Reference line 190
        write(cache_file, &ser)?;
        debug!("Cached vf for {}@{}", &self.n, try_shorten(&self.v));

        Ok(())
    }

    pub fn cache_file(n: &str) -> PathBuf { PathBuf::from("/var/cache/to/data").join(n).join("vf") }

    /// # Attempts to uncache a vf
    /// Won't uncache if there is no cache file or the cache file is more than 4 hours old
    // TODO: Consider just taking package_name: &str instead of &Package
    pub fn uncache(package: &Package) -> Result<Vf, VfCacheError> {
        let cache_file = Vf::cache_file(&package.name);

        if !cache_file.exists() {
            return Err(VfCacheError::NoCache)
        }

        let four_hours_ago = SystemTime::now() - Duration::from_hours(4);
        if cache_file.metadata()?.modified()? < four_hours_ago {
            rmf(cache_file)?;
            return Err(VfCacheError::TooOld)
        }

        let contents = read_to_string(cache_file)?;
        let vf = serde_json::from_str(&contents).unwrap(); // PERF: Reference line 190
        Ok(vf)
    }
}
