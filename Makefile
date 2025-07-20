-include config.mk
.PHONY: all check test clean build install uninstall

# TODO: Make the configure flags actually do stuff

GIT_BRANCH ?= master
GIT_REPO   ?= https://github.com/Toxikuu/to-pkgs.git
VERSION    ?= $(shell ./version.sh)

all: build

check:
	checkmake Makefile
	VERSION=TEST cargo nextest run

test: check

build:
ifeq ($(ENABLE_TOOLS),1)
	@echo "Building to with tools..."
	VERSION=$(VERSION) cargo +nightly build --release
else
	@echo "Building to..."
	VERSION=$(VERSION) cargo +nightly build --release --no-default-features
endif

# TODO: Clean docs as well once I write some
clean:
	rm -f config.mk
	cargo clean

install:
	install -Dm755 target/release/to           -t $(DESTDIR)/usr/bin/
	install -Dm644 $(wildcard envs/*.env)      -t $(DESTDIR)/usr/share/to/envs/

ifeq ($(ENABLE_TOOLS),1)
	install -Dm644 $(wildcard git-templates/*) -t $(DESTDIR)/usr/share/to/git-templates/
	install -Dm644 template                    -t $(DESTDIR)/usr/share/to/
	cp -avf scripts                               $(DESTDIR)/usr/share/to/
endif

	install -dm755                                $(DESTDIR)/etc/to
ifeq ($(ENABLE_CONF),1)
	install -Dm644 $(wildcard etc/*)           -t $(DESTDIR)/etc/to/
endif

# TODO: Add completions
ifeq ($(ENABLE_COMP),1)
	install -Dm644 completions/bash               $(DESTDIR)/usr/share/bash-completion/completions/to
	install -Dm644 completions/zsh                $(DESTDIR)/usr/share/zsh/site-functions/_to
	install -Dm644 completions/fish               $(DESTDIR)/usr/share/fish/vendor_completions.d/to.fish
endif

# TODO: Install docs once I write some
ifeq ($(ENABLE_DOCS),1)
	@echo "SOON"
endif

ifeq ($(ENABLE_TOOLS),1)
	install -dm755                                $(DESTDIR)/var/lib/to/chroot
	install -dm755                                $(DESTDIR)/var/cache/to/sources
endif

	install -dm755                                $(DESTDIR)/var/db/to/data
	install -dm755                                $(DESTDIR)/var/db/to/pkgs
	install -dm755                                $(DESTDIR)/var/cache/to/data
	install -dm755                                $(DESTDIR)/var/cache/to/dist

# TODO: Drop support for this and just use `to sync`, or use variables
ifeq ($(ENABLE_GIT),1)
	$(info GIT_REPO = $(GIT_REPO))
	$(info GIT_BRANCH = $(GIT_BRANCH))
	@if [ -d "$(DESTDIR)/var/db/to/pkgs/.git" ]; then \
		echo "Package repo exists, skipping clone."; \
	else \
		echo "Cloning package repo..."; \
		git clone --depth=1 --no-single-branch --branch $(GIT_BRANCH) $(GIT_REPO) $(DESTDIR)/var/db/to/pkgs; \
	fi
endif

uninstall:
	rm -f $(DESTDIR)/usr/bin/to
	rm -rf $(DESTDIR)/usr/share/to
	@echo "You may also want to remove $(DESTDIR)/etc/to, $(DESTDIR)/var/cache/to, and $(DESTDIR)/var/db/to"

dev:
	VERSION=$(VERSION) cargo +nightly build --release

	sudo install -dm755                                /var/lib/to/chroot
	sudo install -dm755                                /var/cache/to/sources

	sudo install -dm755                                /var/db/to/data
	sudo install -dm755                                /var/db/to/pkgs
	sudo install -dm755                                /var/cache/to/data
	sudo install -dm755                                /var/cache/to/dist

	sudo install -Dm755 target/release/to           -t /usr/bin/
	sudo install -Dm644 $(wildcard envs/*.env)      -t /usr/share/to/envs/

	sudo install -Dm644 $(wildcard git-templates/*) -t /usr/share/to/git-templates/
	sudo install -Dm644 template                    -t /usr/share/to/
	sudo cp -avf scripts                               /usr/share/to/

	sudo install -dm755                                /etc/to
	sudo install -Dm644 $(wildcard etc/*)           -t /etc/to/



	sudo install -dm755                                /var/lib/to/chroot/lower/var/lib/to/chroot
	sudo install -dm755                                /var/lib/to/chroot/lower/var/cache/to/sources

	sudo install -dm755                                /var/lib/to/chroot/lower/var/db/to/data
	sudo install -dm755                                /var/lib/to/chroot/lower/var/db/to/pkgs
	sudo install -dm755                                /var/lib/to/chroot/lower/var/cache/to/data
	sudo install -dm755                                /var/lib/to/chroot/lower/var/cache/to/dist

	sudo install -Dm755 target/release/to           -t /var/lib/to/chroot/lower/usr/bin/
	sudo install -Dm644 $(wildcard envs/*.env)      -t /var/lib/to/chroot/lower/usr/share/to/envs/

	sudo install -Dm644 $(wildcard git-templates/*) -t /var/lib/to/chroot/lower/usr/share/to/git-templates/
	sudo install -Dm644 template                    -t /var/lib/to/chroot/lower/usr/share/to/
	sudo cp -avf scripts                               /var/lib/to/chroot/lower/usr/share/to/

	sudo install -dm755                                /var/lib/to/chroot/lower/etc/to
	sudo install -Dm644 $(wildcard etc/*)           -t /var/lib/to/chroot/lower/etc/to/
