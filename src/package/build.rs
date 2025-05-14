// package/build.rs

use std::{
    fs::{
        copy,
        write,
    },
    path::{
        Path,
        PathBuf,
    },
};

use anyhow::{
    Context,
    Result,
};
use fshelpers::{
    mkdir_p,
    mkf_p,
};
use tracing::{
    debug,
    info,
    trace,
};

use super::Package;
use crate::{
    CONFIG,
    exec,
};

// TODO: Make stagefile configurable
const STAGEFILE: &str = "/var/tmp/lfstage/stages/lfstage3@2025-04-24_14-18-39.tar.xz";
const MERGED: &str = "/var/lib/to/chroot/merged";

impl Package {
    pub fn build(&self) -> Result<()> {
        clean_overlay()?;
        setup_overlay()?;
        self.fetch_sources()?;
        self.populate_overlay()?;
        self.chroot_and_run()?;
        self.save_distfile()?;

        Ok(())
    }

    // NOTE: Dependencies should be installed after the chroot is entered
    fn populate_overlay(&self) -> Result<()> {
        let name = &self.name;
        info!("Populating overlay for {name}");
        // TODO: Copy files from /usr/share/to/copied to $CHROOT/
        // Use sane defaults in copied/
        exec!(
            r#"
            cd {MERGED}
            mkdir -pv B D S etc/to

            cp -vf {}                               pkg     # copy pkg file
            cp -vf /usr/share/to/scripts/runner.sh  runner  # copy runner

            if [ -d {}/A ]; then cp -af {}/A A; fi

            cp -vf /etc/resolv.conf                 etc/resolf.conf
            if [ -f /etc/to/config.toml ]; then cp -vf /etc/to/config.toml etc/to/config.toml; fi
            echo 'usr/share/doc'                >   etc/to/exclude
            echo 'usr/share/licenses'           >>  etc/to/exclude
        "#,
            self.pkgfile().display(),
            self.pkgdir().display(),
            self.pkgdir().display(),
        )
        .context("Failed to populate overlay")?;

        if !self.dependencies.is_empty() {
            debug!("Copying dependencies to overlay")
        }

        fn copy_to_chroot(path: PathBuf) -> Result<()> {
            let dest = Path::new(MERGED).join(path.strip_prefix("/")?);
            mkf_p(&dest)?;
            copy(&path, dest)
                .map(drop)
                .with_context(|| format!("Failed to copy {} to chroot", path.display()))
        }

        for source in &self.sources {
            trace!("Copying over source {source:?}");
            let source_path = source.path(self);
            let source_dest = Path::new(MERGED).join("S").join(&source.dest);

            if source_path.is_dir() {
                dircpy::copy_dir(&source_path, &source_dest).with_context(|| {
                    format!(
                        "Failed to (recursively) copy {source:?} from {source_path:?} to {source_dest:?} for {self}"
                    )
                })?;
            } else {
                copy(&source_path, &source_dest).with_context(|| {
                    format!(
                        "Failed to copy {source:?} from {source_path:?} to {source_dest:?} for {self}"
                    )
                })?;
            }
        }

        #[rustfmt::skip]
        for dep in self.resolve_deps() {
            copy_to_chroot(dep.distfile())
                .with_context(|| format!("Missing distfile for {dep:?}"))?;
            copy_to_chroot(dep.pkgfile())
                .with_context(|| format!("Missing pkgfile for {dep:?}"))?;
            copy_to_chroot(dep.sfile())
                .with_context(|| format!("Missing sfile for {dep:?}"))?;

            trace!("Copied over dependency {dep:?}")
        }

        // Write chroot/deps
        let deps = self
            .resolve_deps()
            .iter()
            .map(|p| p.name.clone())
            .intersperse(" ".to_string())
            .collect::<String>();
        let deps = deps.trim();

        if deps.is_empty() {
            debug!("Not writing deps file since {self} has no dependencies");
        } else {
            let deps_file = format!("{MERGED}/deps");
            write(deps_file, deps).context("Failed to write deps file")?;
            debug!("Wrote deps file for {self}");
            trace!("Deps: {deps}");
        }
        Ok(())
    }

    fn chroot_and_run(&self) -> Result<()> {
        let strip = CONFIG.strip;
        let tests = CONFIG.tests;
        let cflags = &CONFIG.cflags;
        let jobs = &CONFIG.jobs;

        info!("Entering chroot for {self}");
        exec!(
            r#"
        chroot {MERGED} \
            /usr/bin/env -i             \
                TO_CFLAGS="{cflags}"    \
                TO_JOBS={jobs}          \
                TO_STRIP={strip}        \
                TO_TEST={tests}         \
            /runner
        "#
        )
        .context("Build failure in chroot")?;

        Ok(())
    }

    fn save_distfile(&self) -> Result<()> {
        let dist = self.distfile();
        mkdir_p(self.distdir())?;
        exec!("cp -vf '/var/lib/to/chroot/upper/pkg.tar.zst' {dist:?}")?;
        info!("Saved distfile for {self}");
        Ok(())
    }
}

fn setup_overlay() -> Result<()> {
    exec!(
        r#"
        cd        /var/lib/to/chroot

        # extract the stage3 if it's absent
        if [ ! -d lower/usr ]; then 
            tar xpf {STAGEFILE} -C lower
        fi

        mount -vt overlay overlay -o lowerdir=lower,upperdir=upper,workdir=work merged
        mount -v --bind /dev merged/dev
        mount -vt devpts devpts -o gid=5,mode=0620 merged/dev/pts
        mount -vt proc proc merged/proc
        mount -vt sysfs sysfs merged/sys
        mount -vt tmpfs tmpfs merged/run
        "#
    )
    .context("Failed to set up overlay")
}

fn clean_overlay() -> Result<()> {
    exec!(
        r#"
        mkdir -pv /var/lib/to/chroot
        cd        /var/lib/to/chroot

        mkdir -pv lower merged upper work

        if mountpoint -q merged; then
            umount -lR merged
        fi

        rm -rf upper/* work/*
        "#
    )
    .context("Failed to clean overlay")?;

    Ok(())
}
