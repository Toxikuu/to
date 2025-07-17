// utils/health.rs

use tracing::{error, warn, info, debug};
use crate::utils::file::exists;

#[derive(Copy, Clone)]
enum ToDepKind {
    Required,
    Recommended,
    Optional,
}

/// Dependency for to, used for checking health
struct ToDep {
    kind: ToDepKind,
    name: &'static str,
    /// The message to display if the dependency is missing
    message: &'static str,
}

impl ToDep {
    fn new(kind: ToDepKind, name: &'static str, message: &'static str) -> Self {
        Self { kind, name, message }
    }

    /// Checks whether a dependency exists, logging if it does not
    fn check(&self) -> bool {
        debug!("Checking the presence of to dependency '{}'", self.name);
        let exists = exists(self.name);
        if !exists {
            match self.kind {
                ToDepKind::Required => {
                    error!("Required to dependency '{}' not found", self.name);
                    error!("{}", self.message);
                },
                ToDepKind::Recommended => {
                    warn!("Recommended to dependency '{}' not found", self.name);
                    warn!("{}", self.message);
                },
                ToDepKind::Optional => {
                    info!("Optional to dependency '{}' not found", self.name);
                    info!("{}", self.message);
                }
            }
        }
        exists
    }
}

const TODEPS: &[(ToDepKind, &str, &str)] = &[
    (ToDepKind::Required,    "zstd",   "Cannot build or install packages"),
    (ToDepKind::Required,    "tar",    "Cannot build or install packages"),
    (ToDepKind::Required,    "bash",   "All package-related functions unavailable"),
    (ToDepKind::Required,    "grep",   "Cannot perform many actions"),
    (ToDepKind::Required,    "cp",     "Cannot perform many actions"),
    (ToDepKind::Required,    "touch",  "Cannot perform many actions"),
    (ToDepKind::Required,    "tee",    "Cannot perform many actions"),
    (ToDepKind::Required,    "sed",    "Cannot perform many actions"),
    (ToDepKind::Required,    "mkdir",  "Cannot perform many actions"),
    (ToDepKind::Recommended, "chroot", "Cannot build packages"),
    (ToDepKind::Recommended, "env",    "Cannot build packages"),
];

pub fn check_health() -> u8 {
    info!("Checking health");
    let mut n = 0;

    for dep in TODEPS.iter().map(|v| ToDep::new(v.0, v.1, v.2)) {
        if matches!(dep.kind, ToDepKind::Recommended | ToDepKind::Required) && !dep.check() {
            n += 1
        }
    }

    if n == 0 {
        info!("All dependencies present")
    } else {
        warn!("Missing some dependencies")
    }

    n
}
