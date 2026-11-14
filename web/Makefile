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
PKG_JS := target/pkg-js
WASM := target/$(TARGET)/release/taigi_web.wasm
SITE := target/site
CHROMEOS := target/chromeos
EXTENSION := target/extension
FONT_SOURCES := ../assets/fonts/font
FONTS := target/fonts
# Lexical in one module (vendor/lexical.js), its licence beside it.
VENDOR := target/vendor
ICONS := ../ios/Sources/TaigiKeyboard/App/Assets/Assets.xcassets/AppIcon.appiconset

.PHONY: help bindgen-version i18n-check build site site-stage chromeos extension extension-stage extension-fixtures extension-e2e serve test smoke e2e check-live typecheck lint fmt check clean

help:
	@echo "Web version — dev loop"
	@echo ""
	@echo "  make check   - The gate: native tests + build + Node smoke test + clippy + typecheck + biome + fmt --check"
	@echo "  make test    - Native tests of the session and the key translation"
	@echo "  make build   - Release wasm, bindings and wasm-opt into $(PKG)/"
	@echo "  make smoke   - Build, then install the real dictionaries and assert top candidates"
	@echo "  make site    - Stage the page, bindings, dictionaries, fonts and manifest into $(SITE)/"
	@echo "  make chromeos - Stage the ChromeOS input method into $(CHROMEOS)/ and zip it"
	@echo "  make extension - Stage the browser extension into $(EXTENSION)/ and zip it"
	@echo "  make extension-e2e - Load the staged extension in Chromium and type through it"
	@echo "  make check-live - Hash the live files against their manifest"
	@echo "  make typecheck - TypeScript checkJs (--noEmit) over all handwritten JS"
	@echo "  make lint    - clippy -D warnings, typecheck, biome, fmt --check"
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

# cargo decides whether the wasm is stale; the bindings and wasm-opt (~13 s)
# rerun only when cargo rewrote it or the CLI pins moved, so `check`, `site`
# and `chromeos` in one session pay for them once. A sub-make, because make
# reads $(WASM)'s mtime before this recipe's cargo runs.
build: bindgen-version
	cargo build --release --locked --target $(TARGET) -p taigi-web
	@$(MAKE) --no-print-directory target/.pkg-stamp

target/.pkg-stamp: $(WASM) ../mise.toml
	wasm-bindgen --target web --out-dir $(PKG) $(WASM)
	wasm-opt -O3 --strip-debug --strip-producers $(PKG)/taigi_web_bg.wasm -o $(PKG)/taigi_web_bg.wasm
	touch $@

# The browser extension's pure-JS fallback (~18 s), for sites whose CSP omits
# 'wasm-unsafe-eval'. Cached on the same inputs as target/.pkg-stamp.
$(PKG_JS)/.stamp: $(WASM) scripts/build-wasm2js.mjs ../mise.toml node_modules/.package-lock.json
	node scripts/build-wasm2js.mjs $(WASM) $(PKG_JS)
	touch $@

# The pinned packages, once per lock change: the bundle and Playwright share them.
node_modules/.package-lock.json: package-lock.json
	npm ci --no-audit --no-fund

# The editor's Lexical, bundled because the page has no bundler and an import map
# would be an inline script.
$(VENDOR)/lexical.js: vendor/lexical.js node_modules/.package-lock.json
	mkdir -p $(VENDOR)
	npx esbuild vendor/lexical.js --bundle --format=esm --minify --legal-comments=none \
		--define:process.env.NODE_ENV='"production"' --outfile=$@
	cp node_modules/lexical/LICENSE $(VENDOR)/LICENSE-lexical.txt

# Slow, so only when a font or the range in fonts.css changes.
$(FONTS)/.stamp: scripts/subset-fonts.sh src/fonts.css $(wildcard $(FONT_SOURCES)/*)
	scripts/subset-fonts.sh $(FONT_SOURCES) $(FONTS)
	touch $@

site: build site-stage
site-stage: $(FONTS)/.stamp $(VENDOR)/lexical.js
	rm -rf $(SITE)
	mkdir -p $(SITE)
	cp -R src/. $(SITE)/
	cp -R $(PKG) $(SITE)/pkg
	rm $(SITE)/pkg/*.d.ts
	mkdir -p $(SITE)/fonts $(SITE)/icons
	cp $(FONTS)/*.woff2 ../assets/fonts/OFL-1.1.txt $(SITE)/fonts/
	cp $(ICONS)/180.png $(ICONS)/196.png $(ICONS)/512.png $(SITE)/icons/
	cp -R $(VENDOR) $(SITE)/vendor
	node scripts/stage-assets.mjs $(SITE) ../assets/dictionaries

# The ChromeOS input method (docs/architecture/chromeos-roadmap.md). The
# dictionaries go in as they are: the package is local, so no hash and no gzip.
chromeos: build
	rm -rf $(CHROMEOS) target/taigi-keyboard-chromeos.zip
	mkdir -p $(CHROMEOS)/pkg $(CHROMEOS)/dictionaries $(CHROMEOS)/icons $(CHROMEOS)/generated
	cp chromeos/manifest.json chromeos/*.js chromeos/offscreen.html $(CHROMEOS)/
	cp -R chromeos/_locales $(CHROMEOS)/
	cp src/settings.js src/i18n.js src/learning.js src/style.css src/chevron.svg $(CHROMEOS)/
	cp src/generated/strings.js $(CHROMEOS)/generated/
	node scripts/stage-chromeos.mjs src/index.html chromeos/options.html $(CHROMEOS)/options.html
	cp $(PKG)/taigi_web.js $(PKG)/taigi_web_bg.wasm $(CHROMEOS)/pkg/
	cp ../assets/dictionaries/* $(CHROMEOS)/dictionaries/
	cp $(ICONS)/16.png $(ICONS)/48.png $(ICONS)/128.png $(CHROMEOS)/icons/
	cd $(CHROMEOS) && zip -qrX ../taigi-keyboard-chromeos.zip .

# The browser extension (docs/architecture/browser-extension.md): the engine
# and the dictionaries in the package, as ChromeOS has them, and the page's
# panel, stylesheet and settings form cut by stage-extension.mjs.
EXTENSION_PAGE_MODULES := panel.js surface.js i18n.js settings.js
extension: build extension-stage
extension-stage: node_modules/.package-lock.json
	@$(MAKE) --no-print-directory $(PKG_JS)/.stamp
	rm -rf $(EXTENSION) target/taigi-keyboard-extension.zip
	mkdir -p $(EXTENSION)/pkg $(EXTENSION)/dictionaries $(EXTENSION)/icons $(EXTENSION)/generated
	cp extension/manifest.json extension/*.js extension/popup.html extension/popup.css $(EXTENSION)/
	cp -R extension/_locales $(EXTENSION)/
	cp $(addprefix src/,$(EXTENSION_PAGE_MODULES)) src/style.css src/chevron.svg chromeos/options.js $(EXTENSION)/
	cp src/generated/strings.js $(EXTENSION)/generated/
	node scripts/stage-extension.mjs src extension $(EXTENSION)
	cp $(PKG)/taigi_web.js $(PKG)/taigi_web_bg.wasm $(PKG_JS)/taigi_web_asm.js $(PKG_JS)/taigi_web_js.js $(EXTENSION)/pkg/
	cp ../assets/dictionaries/* $(EXTENSION)/dictionaries/
	cp $(ICONS)/16.png $(ICONS)/32.png $(ICONS)/48.png $(ICONS)/128.png $(EXTENSION)/icons/
	cd $(EXTENSION) && zip -qrX ../taigi-keyboard-extension.zip .

# The extension's browser gate: Playwright loads the staged extension into its
# own Chromium (EXTENSION_BROWSER names another Chromium's executable).
# The fixtures are served from one directory with the pinned Quill, Lexical and CodeMirror 5 / 6 beside them.
EXTENSION_FIXTURES := target/extension-fixtures
extension-fixtures: $(VENDOR)/lexical.js node_modules/.package-lock.json
	rm -rf $(EXTENSION_FIXTURES)
	mkdir -p $(EXTENSION_FIXTURES)
	cp extension/fixtures/* $(VENDOR)/lexical.js \
		node_modules/quill/dist/quill.js node_modules/quill/dist/quill.snow.css \
		node_modules/codemirror/lib/codemirror.js node_modules/codemirror/lib/codemirror.css \
		node_modules/codemirror/mode/javascript/javascript.js \
		$(EXTENSION_FIXTURES)/
	npx esbuild extension/fixtures/codemirror6-setup.js --bundle --format=esm --minify \
		--legal-comments=none --outfile=$(EXTENSION_FIXTURES)/codemirror6-setup.js

extension-e2e: extension extension-fixtures
	npx playwright install $(if $(CI),--with-deps) chromium
	npx playwright test --config extension/playwright.config.mjs

serve: site
	node scripts/serve.mjs $(SITE) 8000

test: i18n-check
	cargo test --locked

smoke: build node_modules/.package-lock.json
	node --test 'tests/*.test.mjs'

# Separate from `check`: it needs browsers downloaded. `install` is a no-op for
# browsers already in place; on CI it also brings their system libraries, and
# stays on Chromium, because Firefox's and WebKit's are what apt spends its time on.
E2E_BROWSERS := $(if $(CI),chromium,chromium firefox webkit)
e2e: site node_modules/.package-lock.json
	npx playwright install $(if $(CI),--with-deps) $(E2E_BROWSERS)
	npx playwright test $(addprefix --project=,$(E2E_BROWSERS))

# The deploy runs the same script with its build (docs/architecture/web-release.md).
# By hand, from Taiwan, this times the download from there.
LIVE_URL := https://taigikeyboard.tw/web/
check-live:
	node scripts/check-live.mjs $(LIVE_URL)

# Every JS runtime environment (`tsconfig.json` project references) is checked
# without emitting files (`checkJs` + JSDoc).
TSCONFIGS := tsconfig.page.json tsconfig.sw.json tsconfig.chromeos-worker.json tsconfig.chromeos.json tsconfig.extension-worker.json tsconfig.extension.json tsconfig.fixtures.json tsconfig.node.json
typecheck: node_modules/.package-lock.json
	for cfg in $(TSCONFIGS); do ./node_modules/.bin/tsc --noEmit -p $$cfg || exit 1; done

# `mmap-host` by name: its wasm backend compiles only for this target, so the
# engine workspace's clippy never reads it.
lint: typecheck
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
