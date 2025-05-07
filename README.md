# To

<!-- TODO: update the below line when I'm comfortable with the state of matter of
this project -->
**Stupid-simple, rock-solid (EVENTUALLY CURRENTLY WIP AND NOT SOLID AT ALL DO
NOT USE HERE BE DRAGONS) package manager for LFS**

## Features
- Containerized builds via OverlayFS and stage files
- Surprisingly half-decent dependency resolution
- Distribution tarballs (hereafter referred to as distfiles) (featuring zstd!)!!
- Utilities to hold your hand while you maintain packages (aka mostly foolproof
bash scripts, emphasis on the mostly)
- Manifest system (meaning stray files get deleted and shit)

## Non-Features
So remember how I said `to` is stupid simple? I explicitly omit basic features,
such as:
<!-- TODO: update the below line when `to` is not in its current state -->
- Repositories (there is only one. you must maintain your own, keeping with the
spirit of LFS and all. you can fork
<!-- TODO: write documentation on repo maintenance and update the below line -->
[mine](https://github.com/Toxikuu/to-pkgs.git) as reference. but the
documentation won't come until much later)
- Backups (you're responsible for making your own. please make your own backups
if you're stupid or brave enough to use this, especially in its current state)
- Split packages (that shit looks gross in a build file and it's complex)

## Dependencies
### Build
- Rust
### Runtime
- OverlayFS support in your kernel (CONFIG_OVERLAY_FS=*)
- Curl (used to download shit)
- Git (used to git shit)
- LFS[^2]
- LFStage (currently required, but eventually optional -- it builds the chroot
stage files)

<!-- TODO: Verify whether LFS is required cus lowkey idt it is -->
[^2]: If you wanna try it somewhere else have fun, but this expects LFS.

## Installation
Please don't. I made this for myself and myself alone. This will hopefully be
stable and documented enough for other people to use eventually, but currently
this is just for me. You can use it if you want, but good luck figuring out what
the fuck all my one character variables mean.

If you hate yourself and disregard warnings of dragons:
```bash
./configure --prefix=/usr # You are running LFS after all
make
make install
```

## Pro-Tips
WORKING ON IT. SOON I PROMISE.

## Documentation
NOT WORKING ON IT YET. PROBABLY NOT SOON. NO PROMISES.

## TODO
- [ ] Write an explanation of how `to` works
    - [ ] Also write documentation (eventually)
- [ ] Fucking finish this readme
    - [ ] Write an information section talking about how `to` has optional
    features for maintainers, servers, and end users (so you can have a single
    machine that builds packages and runs a server and just download and install
    all the packages to your other machines)
    - [ ] Also maybe format things properly and make this look like it wasnt
    written in 5 minutes while high off sleep deprivation
- [ ] Write `to lint`
- [ ] Add QA checks that are done post build
- [*] Add `--debug` for `to view`, and change the default behavior to give
      something human readable.
- [*] Add `to alias`
    - [ ] Consider supporting aliases in package definition dependencies
        - [ ] imo probably don't; just lint for their use in `to lint`
    - [*] Drop support for implicit package generation
- [*] Add message support
