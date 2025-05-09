-include config.mk

all: build

build:
ifeq ($(ENABLE_TOOLS),1)
	@echo "Building to with tools..."
	cargo +nightly build --release
else
	@echo "Building to..."
	cargo +nightly build --release --no-default-features
endif

clean:
	# TODO: Clean docs as well once I write some
	rm -f config.mk
	cargo clean

install:
	install -Dm755 target/release/to           -t $(DESTDIR)/usr/bin/
	install -Dm644 $(wildcard envs/*.env)      -t $(DESTDIR)/usr/share/to/envs/

ifeq ($(ENABLE_TOOLS),1)
	install -Dm644 $(wildcard git-templates/*) -t $(DESTDIR)/usr/share/to/git-templates/
	install -Dm755 $(wildcard scripts/*)       -t $(DESTDIR)/usr/share/to/scripts/
	install -Dm644 template                    -t $(DESTDIR)/usr/share/to/
endif

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

ifeq ($(ENABLE_TOOLS),1)
	install -dm755                                $(DESTDIR)/var/cache/to/chroot
	install -dm755                                $(DESTDIR)/var/cache/to/sources
endif

	install -dm755                                $(DESTDIR)/var/cache/to/data
	install -dm755                                $(DESTDIR)/var/cache/to/dist
	install -dm755                                $(DESTDIR)/var/cache/to/pkgs

ifeq ($(ENABLE_GIT),1)
	@if [ -d "$(DESTDIR)/var/cache/to/pkgs/.git" ]; then \
		echo "Package repo exists, skipping clone."; \
	else \
		echo "Cloning package repo..."; \
		git clone --depth=1 --single-branch --branch master https://github.com/Toxikuu/to-pkgs.git $(DESTDIR)/var/cache/to/pkgs; \
	fi
endif

uninstall:
	rm -f $(DESTDIR)/usr/bin/to
	rm -rf $(DESTDIR)/usr/share/to
	@echo "You may also want to remove $(DESTDIR)/etc/to and $(DESTDIR)/var/cache/to"
