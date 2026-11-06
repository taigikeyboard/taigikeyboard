# Dev loop for the web version. `cargo` runs from here so that rustup reads this
# directory's rust-toolchain.toml. Tools: `mise install` at the repo root.
#
# The repo-root `make web-check` forwards here.

export PATH := $(HOME)/.cargo/bin:$(PATH)
SHELL := /bin/bash

TARGET := wasm32-unknown-unknown
PKG := target/pkg
WASM := target/$(TARGET)/release/taigi_web.wasm
SITE := target/site
FONT_SOURCES := ../assets/fonts/font
FONTS := target/fonts
ICONS := ../ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/AppIcon.appiconset

.PHONY: help bindgen-version i18n-check build site serve test smoke e2e lint fmt check clean

help:
	@echo "Web version — dev loop"
	@echo ""
	@echo "  make check   - The gate: native tests + build + Node smoke test + clippy + biome + fmt --check"
	@echo "  make test    - Native tests of the session and the key translation"
	@echo "  make build   - Release wasm, bindings and wasm-opt into $(PKG)/"
	@echo "  make smoke   - Build, then install the real dictionaries and assert top candidates"
	@echo "  make site    - Stage the page, bindings, dictionaries, fonts and manifest into $(SITE)/"
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

serve: site
	node scripts/serve.mjs $(SITE) 8000

test: i18n-check
	cargo test --locked

smoke: build
	node --test 'tests/*.test.mjs'

# Separate from `check`: it needs three browsers downloaded. `install` is a no-op
# for browsers already in place; on CI it also brings their system libraries.
e2e: site
	npm ci
	npx playwright install $(if $(CI),--with-deps) chromium firefox webkit
	npx playwright test

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
