# Dev loop for the macOS input method. Build → bundle → install into
# ~/Library/Input Methods (no sudo). Engine artefacts come from the repo-root
# `make build`; this Makefile never rebuilds Rust.
#
# Anything that needs bundle identity or the install protocol lives in
# scripts/ — a Makefile recipe is a poor place for multi-step shell, and
# scripts/lib/bundle-identity.sh is the one reader of App/Info.plist.

.PHONY: help build test bundle install uninstall reload log clean

CONFIGURATION ?= debug

# Only what the recipes below print or filter on; the scripts read the rest.
BUNDLE_ID := $(shell /usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" App/Info.plist)

help:
	@echo "$(BUNDLE_ID) — macOS input method"
	@echo ""
	@echo "  make build      - Build the SwiftPM executable"
	@echo "  make test       - Run swift test"
	@echo "  make bundle     - Assemble and validate the .app"
	@echo "  make install    - Bundle, then deploy to ~/Library/Input Methods"
	@echo "  make uninstall  - Remove the installed input method"
	@echo "  make reload     - Restart the running input method process"
	@echo "  make log        - Stream OSLog output"
	@echo "  make clean      - Remove build artefacts"
	@echo ""
	@echo "  CONFIGURATION=release make install   (default: $(CONFIGURATION))"

build:
	swift build --configuration $(CONFIGURATION)

test:
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
