// package/build.rs

use std::{
    collections::HashSet,
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

use fshelpers::{
    mkdir_p,
    mkf_p,
};
use thiserror::Error;
use tracing::{
    debug,
    error,
    info,
    trace,
    warn,
};

use super::{
    Package,
    source::SourceError,
};
use crate::{
    CONFIG,
    exec,
    package::dep::DepKind,
};

// TODO: Make stagefile configurable
const STAGEFILE: &str = "/var/tmp/lfstage/stages/lfstage3@2025-04-24_14-18-39.tar.xz";
const MERGED: &str = "/var/lib/to/chroot/merged";

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Failed to clean overlay")]
    CleanOverlay,

    #[error("Failed to setup overlay")]
    SetupOverlay,

    #[error("Failed to fetch sources")]
    FetchSources(#[from] SourceError),

    #[error("Failed to populate overlay")]
    PopulateOverlay,

    #[error("Failed to execute pre-build hook")]
    PreBuildHook,

    #[error("Failed to build")]
    Build,

    #[error("Failed QA checks")]
    QA,

    #[error("Failed to cache")]
    Cache,

    #[error("Failed to save distfile")]
    SaveDistfile,
}

impl Package {
    pub fn build(&self) -> Result<(), BuildError> {
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

    fn pre_build_hook(&self) -> Result<(), BuildError> {
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
        .map_err(|_| BuildError::PreBuildHook)
    }

    // NOTE: Dependencies should be installed after the chroot is entered
    fn populate_overlay(&self) -> Result<(), BuildError> {
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
        .map_err(|_| BuildError::PopulateOverlay)?;

        if !self.dependencies.is_empty() {
            debug!("Copying dependencies to overlay")
        }

        fn copy_to_chroot(path: PathBuf) -> Result<(), BuildError> {
            let dest = Path::new(MERGED).join(
                path.strip_prefix("/")
                    .map_err(|_| BuildError::PopulateOverlay)?,
            );
            mkf_p(&dest).map_err(|_| BuildError::PopulateOverlay)?;
            copy(&path, dest).map(drop).map_err(|_| {
                error!("Failed to copy {} to chroot", path.display());
                BuildError::PopulateOverlay
            })
        }

        for source in &self.sources {
            trace!("Copying over source {source:?}");
            let source_path = source.path(self);
            let source_dest = Path::new(MERGED).join("S").join(&source.dest);

            if source_path.is_dir() {
                dircpy::copy_dir(&source_path, &source_dest)
                    .map_err(|_| BuildError::PopulateOverlay)?;
            } else {
                copy(&source_path, &source_dest).map_err(|_| BuildError::PopulateOverlay)?;
            }
        }

        // Resolve only needed dependencies
        //
        // The idea is to resolve the shallow build dependencies, so extraneous deep build
        // dependencies aren't present in the chroot.
        //
        // We first find all the shallow build dependencies, then all required deep dependencies.
        // Those are then copied to the build environment. We also use a HashSet for extra
        // deduplication.
        let mut deps = self
            .dependencies
            .iter()
            .filter(|d| d.kind == DepKind::Build)
            .map(|d| d.to_package().expect("Failed to form package for dep"))
            .collect::<HashSet<_>>();

        deps.extend(
            self.resolve_deps()
                .into_iter()
                .filter(|d| d.depkind.expect("Dep should have a kind") == DepKind::Required),
        );

        #[rustfmt::skip]
        // TODO: Copy aliases as well
        for dep in &deps {
            copy_to_chroot(dep.distfile())
                .map_err(|_| BuildError::PopulateOverlay)?; // missing distfile (probably)
            copy_to_chroot(dep.pkgfile())
                .map_err(|_| BuildError::PopulateOverlay)?; // missing pkgfile (probably)
            copy_to_chroot(dep.sfile())
                .map_err(|_| BuildError::PopulateOverlay)?; // missing sfile (probably)

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
            write(deps_file, deps_str).map_err(|_| BuildError::PopulateOverlay)?; // deps file
            debug!("Wrote deps file for {self}");
            trace!("Deps_str: {deps_str}");
        }

        Ok(())
    }

    fn chroot_and_run(&self) -> Result<(), BuildError> {
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
        .map_err(|_| BuildError::Build)
    }

    // TODO: Refactor this to be more structured like the lint system later
    // TODO: Add a QA error subtype
    fn qa(&self) -> Result<(), BuildError> {
        // D empty
        {
            let destdir = Path::new(MERGED).join("D");
            if read_dir(destdir)
                .map_err(|_| BuildError::QA)?
                .last()
                .is_none()
            {
                warn!("QA: $D is empty");
                return Err(BuildError::QA)
            }
        }

        // /usr/local used
        {
            let usrlocal = Path::new(MERGED).join("usr/local");
            if read_dir(usrlocal).is_ok() {
                return Err(BuildError::QA)
            }
        }

        Ok(())
    }

    fn save_distfile(&self) -> Result<(), BuildError> {
        mkdir_p(self.distdir()).map_err(|_| BuildError::SaveDistfile)?;
        exec!(
            "cp -vf '/var/lib/to/chroot/upper/pkg.tar.zst' '{}'",
            self.distfile().display()
        )
        .map_err(|_| BuildError::SaveDistfile)?;

        info!("Saved distfile for {self}");
        Ok(())
    }

    /// # """"Cache"""" reusable stuff
    fn cache_stuff(&self) -> Result<(), BuildError> {
        const LOWER: &str = "/var/lib/to/chroot/lower";
        const UPPER: &str = "/var/lib/to/chroot/upper";

        // Cache make-ca certificates
        if self.dependencies.iter().any(|d| d.name == "make-ca") {
            debug!("Caching make-ca certificates if needed");

            mkdir_p(Path::new(LOWER).join("etc/ssl")).map_err(|_| BuildError::Cache)?;
            mkdir_p(Path::new(LOWER).join("etc/pki")).map_err(|_| BuildError::Cache)?;
            exec!(
                "
                if [ -d {UPPER}/etc/ssl/certs ]; then
                    cp -af {UPPER}/etc/ssl/certs {LOWER}/etc/ssl/
                fi
                "
            )
            .map_err(|_| BuildError::Cache)?;

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
            .map_err(|_| BuildError::Cache)?;
        }

        // Cache rustup toolchains to avoid redownloading them
        if self.dependencies.iter().any(|d| d.name == "rust") {
            debug!("Caching rustup toolchains if needed");

            mkdir_p(Path::new(LOWER).join("opt/rustup/toolchains"))
                .map_err(|_| BuildError::Cache)?;
            mkdir_p(Path::new(LOWER).join("opt/rustup/update-hashes"))
                .map_err(|_| BuildError::Cache)?;

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
            .map_err(|_| BuildError::Cache)?;
        }

        Ok(())
    }
}

fn setup_overlay() -> Result<(), BuildError> {
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
    .map_err(|_| BuildError::SetupOverlay)
}

fn clean_overlay() -> Result<(), BuildError> {
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
    .map_err(|_| BuildError::CleanOverlay)?;

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
