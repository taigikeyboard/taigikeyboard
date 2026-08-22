# Dev loop for the macOS input method. Build → bundle → install into
# ~/Library/Input Methods (no sudo). Engine artefacts come from the repo-root
# `make build`; this Makefile never rebuilds Rust.
#
# Anything that needs bundle identity or the install protocol lives in
# scripts/ — a Makefile recipe is a poor place for multi-step shell, and
# scripts/lib/bundle-identity.sh is the one reader of App/Info.plist.

.PHONY: help i18n-check build test bundle install uninstall reload log clean

CONFIGURATION ?= debug

# Only what the recipes below print or filter on; the scripts read the rest.
BUNDLE_ID := $(shell /usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" App/Info.plist)

help:
	@echo "$(BUNDLE_ID) — macOS input method"
	@echo ""
	@echo "  make build      - Build the SwiftPM executable"
	@echo "  make test       - Run swift test"
	@echo "  make bundle     - Assemble and validate the .app"
	@echo "  make i18n-check - Fail if the generated i18n Swift is stale"
	@echo "  make install    - Bundle, then deploy to ~/Library/Input Methods"
	@echo "  make uninstall  - Remove the installed input method"
	@echo "  make reload     - Restart the running input method process"
	@echo "  make log        - Stream OSLog output"
	@echo "  make clean      - Remove build artefacts"
	@echo ""
	@echo "  CONFIGURATION=release make install   (default: $(CONFIGURATION))"
	@echo ""
	@echo "  Releases are cut from the repository root: make macos-release"

# The Swift under Strings/Generated is generated from i18n/*.json by the repo-root `make i18n`, so a
# build can otherwise link strings that no longer match their source. This is the macOS counterpart of
# the Gradle `checkI18nGenerated` task Android runs on preBuild.
#
# `build` and `test` are the only entry points to their own `swift build` / `swift test`, so the
# prerequisite covers them. `bundle` needs nothing here: scripts/bundle-app.sh runs the same check
# itself, right where it runs its own `swift build`, so calling that script directly is gated too.
i18n-check:
	python3 ../tools/i18n/check.py

build: i18n-check
	swift build --configuration $(CONFIGURATION)

test: i18n-check
	swift test

bundle:
	bash scripts/bundle-app.sh $(CONFIGURATION)

install: bundle
	bash scripts/install-app.sh install

uninstall:
	bash scripts/install-app.sh uninstall

reload:
	bash scripts/install-app.sh reload

log:
	log stream --predicate 'subsystem == "$(BUNDLE_ID)"' --level debug --style compact

clean:
	rm -rf .build
