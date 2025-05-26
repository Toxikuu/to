// package/build.rs

use std::{
    fs::{
        copy,
        read_dir,
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
    bail,
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
    package::dep::DepKind,
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
        self.pre_build_hook()?;
        self.chroot_and_run()?;
        self.qa()?;
        self.cache_stuff()?;
        self.save_distfile()?;

        Ok(())
    }

    fn pre_build_hook(&self) -> Result<()> {
        debug!("Checking for pre-build steps for {self}...");
        let pkgfile = &self.pkgfile();

        exec!(
            "
            source {pkgfile}
            if is_function p; then
                echo 'Executing pre-build steps for {self}'
                p
            fi
            ",
            pkgfile = pkgfile.display(),
        )
        .with_context(|| format!("Failed to execute pre-build steps for {self}"))
    }

    // NOTE: Dependencies should be installed after the chroot is entered
    fn populate_overlay(&self) -> Result<()> {
        let name = &self.name;
        info!("Populating overlay for {name}");
        // TODO: Consider dropping `/etc/to/exclude` support
        // - Not sure if I wanna do this because I already wrote and used `il()` :shrug:
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
                        "Failed to (recursively) copy {source:?} from {source_path:?} to {source_dest:?} for {self:-}"
                    )
                })?;
            } else {
                copy(&source_path, &source_dest).with_context(|| {
                    format!(
                        "Failed to copy {source:?} from {source_path:?} to {source_dest:?} for {self:-}"
                    )
                })?;
            }
        }

        // Resolve all non-runtime dependencies to be copied to the build environment
        let deps = self
            .resolve_deps()
            .into_iter()
            .filter(|d| d.depkind.expect("Dep should have a kind") != DepKind::Runtime)
            .collect::<Vec<_>>();

        #[rustfmt::skip]
        // TODO: Copy aliases as well
        for dep in &deps {
            copy_to_chroot(dep.distfile())
                .with_context(|| format!("Missing distfile for {dep:-}"))?;
            copy_to_chroot(dep.pkgfile())
                .with_context(|| format!("Missing pkgfile for {dep:-}"))?;
            copy_to_chroot(dep.sfile())
                .with_context(|| format!("Missing sfile for {dep:-}"))?;

            trace!("Copied over dependency {dep:-}")
        }

        // Write chroot/deps
        let deps_str = deps
            .iter()
            .map(|p| p.name.clone())
            .intersperse(" ".to_string())
            .collect::<String>();
        let deps_str = deps_str.trim();

        if deps_str.is_empty() {
            debug!("Not writing deps file since {self:-} has no dependencies");
        } else {
            let deps_file = format!("{MERGED}/deps");
            write(deps_file, deps_str).context("Failed to write deps file")?;
            debug!("Wrote deps file for {self}");
            trace!("Deps_str: {deps_str}");
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

    // TODO: Refactor this to be more structured like the lint system later
    // TODO: Add a QA error type
    fn qa(&self) -> Result<()> {
        // D empty
        {
            let destdir = Path::new(MERGED).join("D");
            if read_dir(destdir)?.last().is_none() {
                bail!("D is empty")
            }
        }

        // /usr/local used
        {
            let usrlocal = Path::new(MERGED).join("usr/local");
            if read_dir(usrlocal).is_ok() {
                bail!("usrlocal used")
            }
        }

        Ok(())
    }

    fn save_distfile(&self) -> Result<()> {
        mkdir_p(self.distdir())?;
        exec!(
            "cp -vf '/var/lib/to/chroot/upper/pkg.tar.zst' '{}'",
            self.distfile().display()
        )?;
        info!("Saved distfile for {self}");
        Ok(())
    }

    /// # """"Cache"""" reusable stuff
    fn cache_stuff(&self) -> Result<()> {
        const LOWER: &str = "/var/lib/to/chroot/lower";
        const UPPER: &str = "/var/lib/to/chroot/upper";

        // Cache make-ca certificates
        if self.dependencies.iter().any(|d| d.name == "make-ca") {
            debug!("Caching make-ca certificates if needed");

            mkdir_p(Path::new(LOWER).join("etc/ssl"))?;
            mkdir_p(Path::new(LOWER).join("etc/pki"))?;
            exec!(
                "
                if [ -d {UPPER}/etc/ssl/certs ]; then
                    cp -af {UPPER}/etc/ssl/certs {LOWER}/etc/ssl/
                fi
                "
            )
            .context("Failed to cache make-ca certificates (ssl)")?;

            exec!(
                "
                if [ -d {UPPER}/etc/pki/anchors ]; then
                    rm -rf {LOWER}/etc/pki/anchors/*
                    cp -af {UPPER}/etc/pki/anchors/* {LOWER}/etc/pki/anchors
                fi

                if [ -d {UPPER}/etc/pki/tls ]; then
                    rm -rf {LOWER}/etc/pki/tls/*
                    cp -af {UPPER}/etc/pki/tls/* {LOWER}/etc/pki/tls
                fi
                "
            )
            .context("Failed to cache make-ca certificates (pki)")?;
        }

        // Cache rustup toolchains to avoid redownloading them
        if self.dependencies.iter().any(|d| d.name == "rust") {
            debug!("Caching rustup toolchains if needed");

            mkdir_p(Path::new(LOWER).join("opt/rustup/toolchains"))?;
            mkdir_p(Path::new(LOWER).join("opt/rustup/update-hashes"))?;
            // TODO: Copying bs may happen here where nightly gets copied to nightly/nightly.
            // Testing needed. Probably use rsync if that happens.
            exec!(
                "
                if [ -d {UPPER}/opt/rustup/toolchains ]; then
                    cp -af {UPPER}/opt/rustup/toolchains/* {LOWER}/opt/rustup/toolchains/
                fi

                if [ -d {UPPER}/opt/rustup/update-hashes ]; then
                    cp -af {UPPER}/opt/rustup/update-hashes/* {LOWER}/opt/rustup/update-hashes/
                fi
                "
            )
            .context("Failed to cache rustup toolchains")?
        }

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

#[cfg(test)]
mod test {
    use crate::package::{
        Package,
        dep::DepKind,
    };

    #[test]
    fn depression() {
        // The xorg-server package is chosen since it has all sorts of dependencies
        let deps = Package::from_s_file("xorg-server").unwrap().resolve_deps();

        // Ensure dependency information is maintained
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Required));
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Runtime));
        assert!(deps.iter().any(|d| d.depkind.unwrap() == DepKind::Build));

        // Ensure all dependency packages have `depkind` and confirm `is_dependency()` works
        assert!(deps.iter().all(|d| d.is_dependency()));

        // Print out all dependency information
        eprintln!("{:#?}", &deps);
    }
}
