# Dev loop for the web version. `cargo` runs from here so that rustup reads this
# directory's rust-toolchain.toml. Tools: `mise install` at the repo root.
#
# The repo-root `make web-check` forwards here.

export PATH := $(HOME)/.cargo/bin:$(PATH)
SHELL := /bin/bash

TARGET := wasm32-unknown-unknown
# SQLite's C (`sqlite-wasm-rs`) is compiled and archived for this target with
# zig (mise.toml). Apple's clang has no WebAssembly backend; Apple's `ar` cannot
# index wasm objects ("not a mach-o file") and GNU `ar` does not know them, so
# the link would miss every `sqlite3_*`.
export CC_wasm32_unknown_unknown := $(CURDIR)/scripts/zig-cc-wasm32.sh
export AR_wasm32_unknown_unknown := zig ar
PKG := target/pkg
WASM := target/$(TARGET)/release/taigi_web.wasm
SITE := target/site
CHROMEOS := target/chromeos
FONT_SOURCES := ../assets/fonts/font
FONTS := target/fonts
ICONS := ../ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/AppIcon.appiconset

.PHONY: help bindgen-version i18n-check build site chromeos serve test smoke e2e check-live lint fmt check clean

help:
	@echo "Web version — dev loop"
	@echo ""
	@echo "  make check   - The gate: native tests + build + Node smoke test + clippy + biome + fmt --check"
	@echo "  make test    - Native tests of the session and the key translation"
	@echo "  make build   - Release wasm, bindings and wasm-opt into $(PKG)/"
	@echo "  make smoke   - Build, then install the real dictionaries and assert top candidates"
	@echo "  make site    - Stage the page, bindings, dictionaries, fonts and manifest into $(SITE)/"
	@echo "  make chromeos - Stage the ChromeOS input method into $(CHROMEOS)/ and zip it"
	@echo "  make check-live - Hash the live files against their manifest"
	@echo "  make lint    - clippy -D warnings, biome, fmt --check"
	@echo "  make fmt     - cargo fmt, biome format"

# The CLI refuses a module built against another crate version, with an error
# that names neither pin.
bindgen-version:
	@crate=$$(grep -A1 '^name = "wasm-bindgen"$$' Cargo.lock | sed -n 's/^version = "\(.*\)"$$/\1/p'); \
	cli=$$(wasm-bindgen --version | cut -d' ' -f2); \
	if [ "$$crate" != "$$cli" ]; then \
		echo "wasm-bindgen crate $$crate (web/Cargo.toml) != CLI $$cli (mise.toml)" >&2; \
		exit 1; \
	fi

# `src/generated/strings.js` is written by the repo-root `make i18n`.
i18n-check:
	python3 ../tools/i18n/check.py

build: bindgen-version
	cargo build --release --locked --target $(TARGET) -p taigi-web
	wasm-bindgen --target web --out-dir $(PKG) $(WASM)
	wasm-opt -O3 --strip-debug --strip-producers $(PKG)/taigi_web_bg.wasm -o $(PKG)/taigi_web_bg.wasm

# Slow, so only when a font or the range in fonts.css changes.
$(FONTS)/.stamp: scripts/subset-fonts.sh src/fonts.css $(wildcard $(FONT_SOURCES)/*)
	scripts/subset-fonts.sh $(FONT_SOURCES) $(FONTS)
	touch $@

site: build $(FONTS)/.stamp
	rm -rf $(SITE)
	mkdir -p $(SITE)
	cp -R src/. $(SITE)/
	cp -R $(PKG) $(SITE)/pkg
	rm $(SITE)/pkg/*.d.ts
	mkdir -p $(SITE)/fonts $(SITE)/icons
	cp $(FONTS)/*.woff2 ../assets/fonts/OFL-1.1.txt $(SITE)/fonts/
	cp $(ICONS)/180.png $(ICONS)/196.png $(ICONS)/512.png $(SITE)/icons/
	node scripts/stage-assets.mjs $(SITE) ../assets/dictionaries

# The ChromeOS input method (docs/architecture/chromeos-roadmap.md). The
# dictionaries go in as they are: the package is local, so no hash and no gzip.
chromeos: build
	rm -rf $(CHROMEOS) target/taigi-keyboard-chromeos.zip
	mkdir -p $(CHROMEOS)/pkg $(CHROMEOS)/dictionaries $(CHROMEOS)/icons $(CHROMEOS)/generated
	cp chromeos/manifest.json chromeos/*.js chromeos/offscreen.html $(CHROMEOS)/
	cp src/settings.js src/i18n.js src/learning.js src/style.css src/chevron.svg $(CHROMEOS)/
	cp src/generated/strings.js $(CHROMEOS)/generated/
	node scripts/stage-chromeos.mjs src/index.html chromeos/options.html $(CHROMEOS)/options.html
	cp $(PKG)/taigi_web.js $(PKG)/taigi_web_bg.wasm $(CHROMEOS)/pkg/
	cp ../assets/dictionaries/* $(CHROMEOS)/dictionaries/
	cp $(ICONS)/16.png $(ICONS)/48.png $(ICONS)/128.png $(CHROMEOS)/icons/
	cd $(CHROMEOS) && zip -qrX ../taigi-keyboard-chromeos.zip .

serve: site
	node scripts/serve.mjs $(SITE) 8000

test: i18n-check
	cargo test --locked

smoke: build
	node --test 'tests/*.test.mjs'

# Separate from `check`: it needs browsers downloaded. `install` is a no-op for
# browsers already in place; on CI it also brings their system libraries, and
# stays on Chromium, because Firefox's and WebKit's are what apt spends its time on.
E2E_BROWSERS := $(if $(CI),chromium,chromium firefox webkit)
e2e: site
	npm ci
	npx playwright install $(if $(CI),--with-deps) $(E2E_BROWSERS)
	npx playwright test $(addprefix --project=,$(E2E_BROWSERS))

# The deploy runs the same script with its build (docs/architecture/web-release.md).
# By hand, from Taiwan, this times the download from there.
LIVE_URL := https://taigikeyboard.tw/web/
check-live:
	node scripts/check-live.mjs $(LIVE_URL)

# `mmap-host` by name: its wasm backend compiles only for this target, so the
# engine workspace's clippy never reads it.
lint:
	cargo clippy --workspace --target $(TARGET) --locked -- -D warnings
	cargo clippy -p mmap-host --target $(TARGET) --locked -- -D warnings
	biome check .
	cargo fmt -- --check

# Plain `cargo fmt`, never `--all`: `--all` also formats path dependencies, i.e.
# the engine crates.
fmt:
	cargo fmt
	biome format --write .

check: test smoke lint

clean:
	cargo clean
