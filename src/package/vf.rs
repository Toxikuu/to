// package/vf.rs
//! Functions for fetching package upstreams

use anyhow::{
    Context,
    Result,
    anyhow,
    bail,
};
use tracing::{
    debug,
    error,
};

use crate::{
    package::Package,
    sex,
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

    pub async fn vf(&self) -> Result<(String, String, String, bool)> {
        let uv = self.version_fetch().await.map_err(|e| {
            error!("Failed to fetch upstream version for {self}: {e}");
            anyhow!("Failed to fetch upstream")
        })?;

        let n = &self.name;
        let v = &self.version;

        match uv {
            | Some(uv) => Ok((n.clone(), v.clone(), uv.clone(), *v == uv)),
            | None => {
                debug!("Upstream version fetching is disabled for {self}. Skipping...");
                bail!("Disabled");
            },
        }
    }
}

/// # Displays the vf info for a single package
pub fn display_vf(n: &str, v: &str, uv: &str, is_current: bool) {
    if is_current {
        println!("\x1b[37;1m[\x1b[32m*\x1b[37m]\x1b[0m \x1b[32m{n:<32}\x1b[0m {v} ~ {uv}");
    } else {
        println!("\x1b[37;1m[\x1b[31m-\x1b[37m]\x1b[0m \x1b[31m{n:<32}\x1b[0m {v} ~ {uv}");
    }
}
