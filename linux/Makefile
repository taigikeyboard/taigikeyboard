# Dev loop for the Linux input method. Two audiences:
#
#   * on the macOS host: `make check` is the whole gate — native tests +
#     clippy for the workspace (zbus and gtk4-rs both build on macOS; GTK and
#     libadwaita from Homebrew), a REAL cross build of the engine for the
#     shipping target through `cargo zigbuild` (zig ships the C toolchain
#     the bundled SQLite needs), `cargo fmt --check` and the i18n check
#     (docs/architecture/linux-roadmap.md L12). Compilation proof only,
#     never behaviour — the dogfood run-book owns that;
#   * on a Linux machine: `make build` produces the release binaries,
#     `make install` puts them where ibus-daemon finds them — see README.md.
#
# The repo-root `make linux-check` forwards here.

# The root Makefile's PATH, for a `make` run from this directory.
export PATH := $(shell sh ../tools/mise-path-prefix.sh)$(HOME)/.cargo/bin:$(PATH)
# bash, not sh: the `deb` recipe uses `set -o pipefail` (dash has none).
SHELL := /bin/bash

LINUX_TARGET := x86_64-unknown-linux-gnu
# `cargo zigbuild` cross-links the engine from macOS; on a Linux host plain
# cargo does the same thing natively. PATH is restated because make 3.81 (macOS)
# does not pass its exports to $(shell).
CROSS_CARGO := $(shell PATH="$(PATH)"; command -v cargo-zigbuild >/dev/null 2>&1 && echo "cargo zigbuild" || echo "cargo build")
# Where `make install` puts things; a package build passes DESTDIR.
PREFIX ?= /usr
DESTDIR ?=
# Each distribution places three things its own way: the IBus engine (where
# its own ibus-chewing / ibus-anthy packages put theirs), the bundled
# typefaces and the licence texts (its packaging guideline) (docs/architecture/linux-release.md § Paths per distribution).
# `rpm` / `arch` pass their LAYOUT; the default is the FHS / Debian one, which
# the `.deb` and a source install share.
LAYOUT ?= debian
# Whether to install the bundled typefaces. 0 skips them, their OFL text and
# the `copyright` header's sentence about them — for a downstream package
# that leaves fonts to the system; deb / rpm / arch always carry them.
INSTALL_FONTS ?= 1
# The header's `@FONTS@` lines are that sentence: kept unprefixed, or dropped.
ifeq ($(INSTALL_FONTS),1)
  FONT_LICENSE := ../assets/fonts/OFL-1.1.txt
  FONT_HEADER_SED := s/^@FONTS@//
else ifeq ($(INSTALL_FONTS),0)
  FONT_LICENSE :=
  FONT_HEADER_SED := /^@FONTS@/d
else
  $(error INSTALL_FONTS must be 0 or 1, not '$(INSTALL_FONTS)')
endif
# The notices every layout carries: attribution, the third-party inventory,
# the typefaces' OFL and the dictionary's per-source terms, in the order the
# Debian `copyright` header names them. Debian folds them into its one
# `copyright` file; Fedora also ships the Apache text, which Arch (its
# `licenses` package) and Debian (common-licenses) already hold.
LICENSE_TEXTS := ../NOTICE ../THIRD_PARTY_LICENSES.md $(FONT_LICENSE) ../dictionary/LICENSE
ifeq ($(LAYOUT),debian)
  LIBEXECDIR := $(PREFIX)/libexec
  FONTDIR_TTF := $(PREFIX)/share/fonts/truetype/taigikeyboard
  FONTDIR_OTF := $(PREFIX)/share/fonts/opentype/taigikeyboard
  LICENSEDIR := $(PREFIX)/share/doc/taigikeyboard
else ifeq ($(LAYOUT),fedora)
  LIBEXECDIR := $(PREFIX)/libexec
  FONTDIR_TTF := $(PREFIX)/share/fonts/taigikeyboard
  FONTDIR_OTF := $(FONTDIR_TTF)
  LICENSEDIR := $(PREFIX)/share/licenses/taigikeyboard
  LICENSE_TEXTS += ../LICENSE
else ifeq ($(LAYOUT),arch)
  LIBEXECDIR := $(PREFIX)/lib/ibus
  FONTDIR_TTF := $(PREFIX)/share/fonts/taigikeyboard
  FONTDIR_OTF := $(FONTDIR_TTF)
  LICENSEDIR := $(PREFIX)/share/licenses/taigikeyboard
else
  $(error LAYOUT must be debian, fedora or arch, not '$(LAYOUT)')
endif
VERSION := $(shell grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
# E2E=1 builds the test-mode binaries: engine + platform trace on
# (docs/architecture/e2e-trace-schema.md). They are never packaged — deb /
# rpm / arch refuse E2E=1, and a plain `build` proves the trace absent.
# `make install E2E=1` stays allowed: the e2e driver installs the test-mode
# build into its own throwaway container.
E2E ?=
E2E_TRACE_GUARD := ../engine/scripts/lib/e2e-trace-guard.sh
# INSTALL_FONTS=0 is refused too: the package metadata (control.in, the spec's
# %files, PKGBUILD's license=) names the typefaces.
define refuse_unpackageable
	@if [ "$(E2E)" = 1 ]; then echo "error: E2E=1 binaries are never packaged" >&2; exit 1; fi
	@if [ "$(INSTALL_FONTS)" = 0 ]; then echo "error: deb / rpm / arch always carry the bundled typefaces; INSTALL_FONTS=0 is for make install" >&2; exit 1; fi
endef

.PHONY: help i18n-check test lint check-target check check-cpp fmt build build-fcitx5 install install-common install-ibus install-fcitx5 uninstall component deb rpm arch run-engine run-settings clean

help:
	@echo "Linux input method — dev loop"
	@echo ""
	@echo "  make check           - The gate: i18n check + native tests + clippy + cross build + fmt --check"
	@echo "  make test            - Native tests for the workspace"
	@echo "  make lint            - clippy -D warnings, all targets, native"
	@echo "  make check-target    - Build the engine for $(LINUX_TARGET) (cargo zigbuild on macOS)"
	@echo "  make check-cpp       - Syntax-check the Fcitx5 addon against ../references/fcitx5 headers (macOS host; skipped when the clone is absent)"
	@echo "  make i18n-check      - Fail if the generated i18n Rust is stale"
	@echo "  make fmt             - cargo fmt"
	@echo "  make build           - (Linux) cargo build --release (IBus engine + the FFI static library); E2E=1 = test-mode trace build"
	@echo "  make build-fcitx5    - (Linux) the Fcitx5 addon: cmake over the FFI static library (needs fcitx5-modules-dev, extra-cmake-modules)"
	@echo "  make install         - (Linux) install both shells + dictionaries into PREFIX=$(PREFIX) (DESTDIR honoured); INSTALL_FONTS=0 = skip fonts and their license files"
	@echo "  make install-fcitx5  - (Linux) like install but just for the fcitx5 frontend"
	@echo "  make install-ibus    - (Linux) like install but just for the ibus frontend"
	@echo "  make install-common  - (Linux) like install but just for files common to both frontends; INSTALL_FONTS=0 = skip fonts and their license files"
	@echo "  make uninstall       - (Linux) remove what install put there; user data stays"
	@echo "  make deb             - (Linux) build target/taigikeyboard_<version>_amd64.deb from the install layout (dpkg-deb)"
	@echo "  make rpm             - (Fedora) build target/taigikeyboard-<version>-1.x86_64.rpm from the install layout (rpmbuild)"
	@echo "  make arch            - (Arch, non-root) build target/taigikeyboard-<version>-1-x86_64.pkg.tar.zst from the install layout (makepkg)"
	@echo "  make run-engine      - (Linux) run the IBus engine from this tree against a running ibus-daemon (RUST_LOG=debug, repo dictionaries)"
	@echo "  make run-settings    - Open the settings window from this tree (macOS host too: GTK + libadwaita from Homebrew); PANE=about picks a pane"

# The generated strings live in the desktop-shared crate; a stale copy would
# compile strings that no longer match their source. Same gate the macOS,
# Windows and desktop Makefiles run.
i18n-check:
	python3 ../tools/i18n/check.py

test: i18n-check
	cargo test --workspace

lint:
	cargo clippy --workspace --all-targets -- -D warnings

# The IBus engine and the FFI static library for the shipping target. A real
# build, not `cargo check`: the bundled SQLite's build script compiles C for
# the target, which needs a C toolchain for it — zig's, on macOS.
check-target:
	$(CROSS_CARGO) -p taigikeyboard-ibus -p taigi-linux-ffi --target $(LINUX_TARGET)

check: test lint check-target check-cpp
	cargo fmt -- --check

# The addon's C++ cannot link on the Mac, but it can be parsed: clang in
# -fsyntax-only mode over the fcitx5 headers of the version Ubuntu 24.04
# ships (`git clone --depth 1 --branch 5.1.7 https://github.com/fcitx/fcitx5
# ../references/fcitx5`; gitignored). The three `*_export.h` CMake would
# generate are stubbed. Catches what CI catches five minutes later —
# a typo, a macro absent from 5.1.7 — never a link or runtime fault.
FCITX5_HEADERS := ../references/fcitx5/src/lib
FCITX5_STUBS := target/fcitx5-stubs
check-cpp:
	@if [ ! -d "$(FCITX5_HEADERS)" ]; then echo "check-cpp: no ../references/fcitx5 clone, skipped"; exit 0; fi; \
	mkdir -p $(FCITX5_STUBS); \
	for m in FCITXCORE FCITXUTILS FCITXCONFIG; do \
	  f=$(FCITX5_STUBS)/$$(echo $$m | tr A-Z a-z)_export.h; \
	  printf '#pragma once\n#define %s_EXPORT\n#define %s_DEPRECATED\n#define %s_DEPRECATED_EXPORT\n#define %s_NO_EXPORT\n' $$m $$m $$m $$m > $$f; \
	done; \
	clang++ -std=c++17 -fsyntax-only -Wall -Wextra -DFCITX_GETTEXT_DOMAIN='"fcitx5-taigikeyboard"' -DTAIGIKEYBOARD_VERSION='"0"' \
	  -I $(FCITX5_HEADERS) -I $(FCITX5_STUBS) -I crates/taigi-linux-ffi/include fcitx5/src/engine.cpp && echo "check-cpp: ok"

# Plain `cargo fmt`, never `--all`: `--all` also formats path dependencies,
# i.e. the engine and desktop crates.
fmt:
	cargo fmt

build:
ifeq ($(E2E),1)
	cargo build --release --features taigi-linux-ffi/e2e-trace --features taigikeyboard-ibus/e2e-trace
	source $(E2E_TRACE_GUARD) && e2e_trace_assert_present target/release/libtaigi_linux_ffi.a target/release/ibus-engine-taigikeyboard
else
	cargo build --release
	source $(E2E_TRACE_GUARD) && e2e_trace_assert_absent target/release/libtaigi_linux_ffi.a target/release/ibus-engine-taigikeyboard
endif

# The Fcitx5 addon (linux/fcitx5/): CMake against Fcitx5Core, over the static
# library `build` produced. PREFIX must match the binaries' TAIGIKEYBOARD_PREFIX.
FCITX5_BUILD_DIR := target/fcitx5-build
build-fcitx5: build
	cmake -S fcitx5 -B $(FCITX5_BUILD_DIR) -DCMAKE_BUILD_TYPE=Release \
		-DCMAKE_INSTALL_PREFIX=$(PREFIX) \
		-DTAIGI_FFI_LIB=$(CURDIR)/target/release/libtaigi_linux_ffi.a
	cmake --build $(FCITX5_BUILD_DIR) --parallel

# The component XML with this PREFIX rendered in (`<exec>` / `<setup>`).
component: data/taigikeyboard.xml.in
	sed -e 's#@PREFIX@#$(PREFIX)#g' -e 's#@LIBEXECDIR@#$(LIBEXECDIR)#g' -e 's#@VERSION@#$(VERSION)#g' $< > target/taigikeyboard.xml

# Install everything. The layout is that which the engine resolves at runtime
# (`taigi-linux-platform::paths::InstallLayout`); the PREFIX must match the one
# the binaries were built with (`TAIGIKEYBOARD_PREFIX`, default /usr).
# Targets are split to allow for split packages to be more easily made, but the
# packaging from this file still packs everything in one binary package.
install: install-common install-ibus install-fcitx5
	@echo "installed under $(DESTDIR)$(PREFIX); restart fcitx5 (fcitx5 -r) and/or ibus (ibus restart)"

# Things shared by both frontends: settings, dictionaries, icons, fonts, licences.
install-common: build
	# Settings app:
	install -Dm755 target/release/taigikeyboard-settings $(DESTDIR)$(PREFIX)/bin/taigikeyboard-settings
	install -Dm644 data/tw.taigikeyboard.Settings.desktop $(DESTDIR)$(PREFIX)/share/applications/tw.taigikeyboard.Settings.desktop
	# Dictionaries data:
	install -d $(DESTDIR)$(PREFIX)/share/taigikeyboard/dictionaries
	install -m644 ../assets/dictionaries/dictionary.fst ../assets/dictionaries/dictionary.bin ../assets/dictionaries/association.bin ../assets/dictionaries/syllables.fst $(DESTDIR)$(PREFIX)/share/taigikeyboard/dictionaries/
	# Icons:
	for size in 16 22 24 32 48 64 128 256; do \
	  install -Dm644 data/icons/hicolor/$${size}x$${size}/apps/taigikeyboard.png $(DESTDIR)$(PREFIX)/share/icons/hicolor/$${size}x$${size}/apps/taigikeyboard.png || exit 1; \
	done
	# The tray / panel mode indicator, one icon per state
	# (`ModeIndicator::icon_name`, drawn by tools/desktop/make-app-icon.swift).
	for icon in data/icons/hicolor/*/apps/taigikeyboard-*.png; do \
	  install -Dm644 "$$icon" "$(DESTDIR)$(PREFIX)/share/$${icon#data/}" || exit 1; \
	done
ifeq ($(INSTALL_FONTS),1)
	# The bundled typefaces (the ones macOS and Windows ship), as fontconfig
	# fallbacks for the Han characters the desktop's fonts lack (roadmap L4);
	# users pick them in Fcitx5 / IBus (L7).
	install -Dm644 -t $(DESTDIR)$(FONTDIR_TTF) ../assets/fonts/font/*.ttf
	install -Dm644 -t $(DESTDIR)$(FONTDIR_OTF) ../assets/fonts/font/*.otf
	# A package gets its cache rebuilt by fontconfig's dpkg trigger; a direct
	# install into the live system rebuilds it here. Never for a staging root.
	$(if $(DESTDIR),,-fc-cache $(PREFIX)/share/fonts)
endif
	# License:
ifeq ($(LAYOUT),debian)
	# Debian Policy §12.5: one `copyright` file, every non-common licence
	# verbatim; Apache-2.0 is named in the header (common-licenses).
	install -d $(DESTDIR)$(LICENSEDIR)
	{ sed -e '$(FONT_HEADER_SED)' packaging/copyright-header; for f in $(LICENSE_TEXTS); do \
	    name="$${f#../}"; name="$${name#assets/}"; \
	    printf '\n%s\n%s\n\n' '========================================================================' "$$name"; cat "$$f"; \
	  done; } > $(DESTDIR)$(LICENSEDIR)/copyright
else
	# One file per text, named after its repo path minus `assets/`
	# (assets/fonts/OFL-1.1.txt → fonts-OFL-1.1.txt), so the two `LICENSE`s
	# stay apart.
	for f in $(LICENSE_TEXTS); do \
	  name="$${f#../}"; name="$${name#assets/}"; install -Dm644 "$$f" "$(DESTDIR)$(LICENSEDIR)/$${name//\//-}" || exit 1; \
	done
endif

install-ibus: build component
	install -Dm755 target/release/ibus-engine-taigikeyboard $(DESTDIR)$(LIBEXECDIR)/ibus-engine-taigikeyboard
	install -Dm644 target/taigikeyboard.xml $(DESTDIR)$(PREFIX)/share/ibus/component/taigikeyboard.xml

# The addon's .so + its two .conf files, through CMake's own install rules.
install-fcitx5: build-fcitx5
	DESTDIR=$(DESTDIR) cmake --install $(FCITX5_BUILD_DIR)

uninstall:
	rm -f $(DESTDIR)$(LIBEXECDIR)/ibus-engine-taigikeyboard
	rm -f $(DESTDIR)$(PREFIX)/bin/taigikeyboard-settings
	rm -f $(DESTDIR)$(PREFIX)/lib/fcitx5/libtaigikeyboard.so $(DESTDIR)$(PREFIX)/lib/*/fcitx5/libtaigikeyboard.so
	rm -f $(DESTDIR)$(PREFIX)/share/fcitx5/inputmethod/taigikeyboard.conf $(DESTDIR)$(PREFIX)/share/fcitx5/addon/taigikeyboard.conf
	rm -f $(DESTDIR)$(PREFIX)/share/ibus/component/taigikeyboard.xml
	rm -rf $(DESTDIR)$(PREFIX)/share/taigikeyboard
	rm -f $(DESTDIR)$(PREFIX)/share/applications/tw.taigikeyboard.Settings.desktop
	rm -f $(DESTDIR)$(PREFIX)/share/icons/hicolor/*/apps/taigikeyboard.png $(DESTDIR)$(PREFIX)/share/icons/hicolor/*/apps/taigikeyboard-*.png
	rm -rf $(DESTDIR)$(FONTDIR_TTF) $(DESTDIR)$(FONTDIR_OTF)
	rm -rf $(DESTDIR)$(LICENSEDIR)
	$(if $(DESTDIR),,-fc-cache $(PREFIX)/share/fonts)
	@echo "removed; ~/.config/taigikeyboard, ~/.local/share/taigikeyboard and ~/.local/share/fonts/taigikeyboard are kept"

# The Debian package (roadmap L11): `make install` into a staging root — the
# ONE install layout, so the package and a source install cannot drift —
# plus the control file, packed by dpkg-deb. No cargo-deb: it would carry a
# second copy of the asset list. Only ever built with PREFIX=/usr.
#
# `Depends` is what the binaries actually link, versioned, from
# dpkg-shlibdeps over the three ELF files (dpkg-dev) — an unversioned list
# would let an older distribution install a package whose GLib / libadwaita
# symbols it lacks — plus the framework alternative the control template
# names (`fcitx5 | ibus`).
DEB_ROOT := target/deb/root
DEB_FILE := target/taigikeyboard_$(VERSION)_amd64.deb
deb:
	$(refuse_unpackageable)
	rm -rf target/deb
	$(MAKE) install PREFIX=/usr LAYOUT=debian DESTDIR=$(CURDIR)/$(DEB_ROOT)
	install -d $(DEB_ROOT)/DEBIAN target/deb/debian
	# dpkg-shlibdeps wants a SOURCE-style debian/control beside what it scans
	# (a Source stanza, then the package); the shipped control is the binary
	# one, so a minimal source control lives in packaging/.
	cp packaging/shlibdeps-control target/deb/debian/control
	set -o pipefail; cd target/deb && dpkg-shlibdeps -O \
	  root/usr/bin/taigikeyboard-settings root/usr/libexec/ibus-engine-taigikeyboard root/usr/lib/*/fcitx5/libtaigikeyboard.so \
	  | sed -e 's/^shlibs:Depends=//' > shlibs.txt
	test -s target/deb/shlibs.txt
	sed -e 's#@VERSION@#$(VERSION)#g' -e "s#@SHLIBS@#$$(cat target/deb/shlibs.txt)#" packaging/control.in > $(DEB_ROOT)/DEBIAN/control
	dpkg-deb --build --root-owner-group $(DEB_ROOT) $(DEB_FILE)
	dpkg-deb --info $(DEB_FILE)
	@echo "built $(DEB_FILE)"

# Stage the install, in distribution layout $(4), under $(1)/root and render
# template $(2) into $(3) with the version and that root — shared by `rpm`
# and `arch`.
define stage_and_render
	rm -rf $(1)
	$(MAKE) install PREFIX=/usr LAYOUT=$(4) DESTDIR=$(CURDIR)/$(1)/root
	sed -e 's#@VERSION@#$(VERSION)#g' -e 's#@STAGEROOT@#$(CURDIR)/$(1)/root#g' $(2) > $(3)
endef

# The Fedora package: the same staged install, packed by rpmbuild from
# packaging/taigikeyboard.spec.in (rpm's ELF scan writes the library Requires,
# as dpkg-shlibdeps does for the .deb). Run on Fedora (the CI container).
RPM_DIR := target/rpm
RPM_FILE := target/taigikeyboard-$(VERSION)-1.x86_64.rpm
rpm:
	$(refuse_unpackageable)
	$(call stage_and_render,$(RPM_DIR),packaging/taigikeyboard.spec.in,$(RPM_DIR)/taigikeyboard.spec,fedora)
	rpmbuild -bb --define "_topdir $(CURDIR)/$(RPM_DIR)" $(RPM_DIR)/taigikeyboard.spec
	cp $(RPM_DIR)/RPMS/x86_64/taigikeyboard-$(VERSION)-1.x86_64.rpm $(RPM_FILE)
	rpm -qip $(RPM_FILE)
	@echo "built $(RPM_FILE)"

# The Arch package: the same staged install, packed by makepkg from
# packaging/PKGBUILD.in. Run on Arch as a non-root user (makepkg refuses root).
ARCH_DIR := target/arch
ARCH_FILE := target/taigikeyboard-$(VERSION)-1-x86_64.pkg.tar.zst
arch:
	$(refuse_unpackageable)
	$(call stage_and_render,$(ARCH_DIR),packaging/PKGBUILD.in,$(ARCH_DIR)/PKGBUILD,arch)
	cd $(ARCH_DIR) && PKGDEST=$(CURDIR)/target PKGEXT=.pkg.tar.zst makepkg --force --nodeps
	test -f $(ARCH_FILE)
	pacman -Qip $(ARCH_FILE)
	@echo "built $(ARCH_FILE)"

# The dev loop on a Linux box with ibus-daemon up: the engine from this tree,
# reading the repo's dictionaries, logging every key. `ibus engine
# taigikeyboard` from another terminal selects it (the component XML must
# already be installed once so the daemon knows the name; it exits 1 because
# the engine's layout is `default`, which setxkbmap is never given — `ibus
# engine` with no argument confirms the switch).
run-engine:
	TAIGIKEYBOARD_DATA_DIR=$(CURDIR)/../assets RUST_LOG=debug cargo run -p taigikeyboard-ibus -- --ibus

# The settings window from this tree — the visual smoke the Windows window
# never had on the host (roadmap L3). Writes the real ~/.config/taigikeyboard/settings.json.
PANE ?=
run-settings:
	RUST_LOG=debug cargo run -p taigikeyboard-settings -- $(if $(PANE),--pane $(PANE),)

clean:
	cargo clean
