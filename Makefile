ENGINE := engine
DICT := dictionary

# Ensure rustup-installed cargo is on PATH. `make` runs commands in /bin/sh,
# which does not source the user's interactive shell config. Without this,
# `cargo: command not found` if zsh doesn't `source ~/.cargo/env`.
export PATH := $(HOME)/.cargo/bin:$(PATH)

# A bare `make` prints the target list; it used to run `build`, which needs macOS
# with every toolchain installed (docs/BUILDING.md).
.DEFAULT_GOAL := help

.PHONY: build protos ios-libs android-libs macos-libs test test-crate doc dict dogfood e2e help \
        fmt lint hooks scan-secrets scan-secrets-full scan-private \
        i18n i18n-test \
        macos-release desktop-release desktop-patch desktop-announce version-mobile version-desktop \
        windows-check windows-release desktop-check linux-check \
        update-submodules

# Every generated artefact at once — platform bindings, then the iOS xcframework,
# the Android jniLibs and the macOS xcframework. Build ONLY — does NOT run tests
# (use `make test`). Refreshing all of them together is what keeps no platform
# stale behind an engine change; macOS is built here although it has its own
# release flow (USER 2026-08-15: "I think it can be merged into make build, just not released at this stage").
# macOS only, with every toolchain in docs/BUILDING.md. The steps run in this
# order, one after another (sub-makes, so `-j` cannot reorder them): the bindings
# must be regenerated before any native build compiles the protos crate.
build:
	$(MAKE) --no-print-directory protos
	$(MAKE) --no-print-directory ios-libs
	$(MAKE) --no-print-directory android-libs
	$(MAKE) --no-print-directory macos-libs
	@echo ""
	@echo "✓ build complete — Xcode: Clean Build Folder ⇧⌘K → Build"

# Regenerate the committed Swift (iOS + macOS) and Java protobuf bindings from
# engine/protos/proto/, then drop the protos crate's build cache so the next
# native build recompiles it. Needs protoc 36.2 and protoc-gen-swift; with any
# other protoc the binding step is skipped with a warning (gen-platform-protos.sh).
protos:
	@echo "==> Regenerating platform proto (Swift + Java)"
	bash $(ENGINE)/scripts/gen-platform-protos.sh
	@echo "==> Regenerating macOS proto (Swift)"
	bash $(ENGINE)/scripts/gen-macos-protos.sh
	@echo "==> Cleaning protos build cache"
	cd $(ENGINE) && cargo clean -p protos

# One platform's engine binary — what that platform links. Each needs only its
# own toolchain (docs/BUILDING.md § 2), so a single-platform contributor runs one.
ios-libs:
	@echo "==> Building iOS xcframework"
	cd $(ENGINE) && bash scripts/build-xcframework.sh

android-libs:
	@echo "==> Building Android jniLibs"
	cd $(ENGINE) && bash scripts/build-android-libs.sh

macos-libs:
	@echo "==> Building macOS xcframework"
	cd $(ENGINE) && bash scripts/build-macos-xcframework.sh

test:
	cd $(ENGINE) && cargo test --workspace

# Per-crate scoped test for touched-target round workflow
# (~/.claude/rules/round-workflow.md § Pre-commit quality gates).
# Usage: make test-crate CRATE=phonetics
test-crate:
	@if [ -z "$(CRATE)" ]; then echo "Usage: make test-crate CRATE=<name>"; exit 2; fi
	cd $(ENGINE) && cargo test -p "$(CRATE)"

# Generate rustdoc HTML for the workspace and open in browser. Excludes
# android-jni because it shares `[lib] name = "rust_taigi"` with swift-ffi
# and rustdoc cannot emit two crates to the same target/doc/<name>/ path.
# To inspect android-jni instead, run:
#   cd engine && cargo doc --no-deps -p android-jni --document-private-items --open
doc:
	cd $(ENGINE) && cargo doc --no-deps --workspace --document-private-items --exclude android-jni --open

# Full dictionary regeneration: per-source pipeline (run.sh) then aggregate
# merge + bin + fst + audit + deploy to dictionaries/, which every platform
# packages (build.sh).
# The submodule check is here as well as in dictionary/common/taigi_bridge.py
# because they answer different questions. This one fails in milliseconds before
# a ~10-minute run starts, for the one entry point people actually type; the
# Python one is the correctness boundary that also covers the tests and direct
# `python3 -m pipeline.run` invocations.
dict:
	@test -f taigi-converter/src/converter.js || { \
	  echo "make dict: taigi-converter submodule is not checked out."; \
	  echo "  git submodule update --init --recursive   (or: make update-submodules)"; \
	  exit 1; }
	bash $(DICT)/run.sh
	bash $(DICT)/build.sh

# Generate i18n native resources + Kotlin accessors from i18n/*.json (mirror of `make dict`:
# committed output, not a per-compile step). Re-run after editing any i18n/*.json source. (in-app
# i18n/content/*.json is a separate nested schema the platforms decode directly — NOT emitted here;
# the glob is non-recursive, so the content/ subfolder is skipped.)
# POJ is hand-authored alongside tailo (the `poj` value in each key); no derive step.
i18n:
	python3 tools/i18n/generate.py

# Unit tests for the i18n codegen core (validation, escaping, scope filter, GeneratedMap, format/plural).
# Pure-Python, no Android/iOS toolchain needed — runs the same logic the platform builds compile against.
i18n-test:
	python3 tools/i18n/test_i18n.py

# Generate continuous-input dogfood test table (TL/POJ/TPS + 漢字) from the
# built dictionary.csv. Random each run; prints to stdout for manual on-device
# testing. Requires `make dict` to have produced dictionary/output/dictionary.csv.
dogfood:
	python3 $(DICT)/tools/gen_dogfood.py

# End-to-end run (docs/architecture/e2e-testing-roadmap.md): drive PLATFORM's
# test-mode build through every e2e/scenarios/*.json, then analyze. Report:
# $(E2E_RUN)/report.md; exit 1 on a failed scenario or a bug / perf finding.
# E2E_ONLY=<scenario-id> runs one scenario.
# `:=` inside ifndef: the timestamp is taken once, so the driver and the
# analyzer see the same directory.
ifndef E2E_RUN
E2E_RUN := $(CURDIR)/e2e/runs/$(shell date +%Y%m%d-%H%M%S)
endif
e2e:
	@test -n "$(PLATFORM)" && test -x tools/e2e/$(PLATFORM)/run.sh \
	  || { echo "usage: make e2e PLATFORM=<platform with tools/e2e/<platform>/run.sh>" >&2; exit 1; }
	tools/e2e/$(PLATFORM)/run.sh "$(E2E_RUN)" $(E2E_ONLY)
	python3 tools/e2e/analyze.py --run "$(E2E_RUN)"

# Cut a macOS release: build, sign, notarize, and stage the package on this
# version's DRAFT desktop release. The only entry point for one — `macos/Makefile`
# is the dev loop and stops at `bundle`. Nothing here reaches a user: the draft
# has no tag and no public download, and `make desktop-announce` is what
# announces the release a person publishes after testing it.
#
# `--publish` is baked in because staging IS the point of this target, and
# `--force` because re-cutting the same version is the normal case: a release is
# tested by running this flow, and the local package from the previous attempt
# must not be what stops the next one.
#
# The two throwaway builds contradict publishing and so are refused here by
# design; run the script directly for those. Prerequisites and the one-time
# Developer ID setup: docs/architecture/macos-release.md.
macos-release:
	bash macos/scripts/release-app.sh --force --publish $(RELEASE_FLAGS)

# Host-side gate for the Windows input method (no Windows machine needed):
# i18n check + native tests for the pure crates + clippy against the
# x86_64-pc-windows-gnu target (full crate graph via mingw-w64) + cargo check
# against x86_64-pc-windows-msvc for the C-free crates. Compilation proof only;
# behaviour is the dogfood run-book's (docs/architecture/windows-roadmap.md W13).
windows-check:
	$(MAKE) -C windows check

# The desktop-shared crates (`desktop/`: the pure Rust Windows and Linux both
# link): native tests + clippy + fmt + i18n check, any host.
desktop-check:
	$(MAKE) -C desktop check

# The Linux input method's host gate (docs/architecture/linux-roadmap.md L12):
# native tests + clippy for `linux/` (zbus + GTK build on macOS), a cross
# build of the engine for x86_64-unknown-linux-gnu via cargo-zigbuild, fmt,
# i18n check. Compilation proof only; behaviour is the dogfood run-book's.
linux-check:
	$(MAKE) -C linux check

# Stage the THREE desktop installers on this version's draft release: the
# package here, the Windows installer and the Linux .deb on GitHub-hosted
# runners (scripts/stage-desktop.sh). The builds cannot share a machine, so
# this drives the others rather than pretending they are one build. All or
# none: a draft holding installers from two commits is not something a tag
# can describe (a patch is a smaller release, not a half: desktop-patch).
# Nothing it does reaches a user — publishing the draft stays a person's, and
# that publish announces the release itself.
desktop-release:
	bash scripts/stage-desktop.sh

# A patch release of ONE platform (PLATFORM=macos|windows|linux): its own
# version, holding only that platform's installers; announcing it leaves the
# other platforms on the version they have (docs/architecture/desktop-release.md).
desktop-patch:
	@test -n "$(PLATFORM)" || { echo "usage: make desktop-patch PLATFORM=macos|windows|linux" >&2; exit 2; }
	bash scripts/stage-desktop.sh $(PLATFORM)

# Announce a desktop release a person has already published: prove both
# installers download anonymously, point the website at them, wait for the live
# appcasts. Publishing the release runs this automatically
# (`.github/workflows/announce-release.yml`); this target is the same script by
# hand, for a re-run after a failed job or an expired token.
desktop-announce:
	bash scripts/announce-release.sh $(RELEASE_FLAGS)

# Cut a Windows release — on a Windows machine, from Git Bash: release
# builds, signing, the Inno Setup installer, and staging it on the same draft
# desktop release the Mac's package goes on (windows/scripts/release-app.sh;
# procedure and one-time setup in docs/architecture/windows-release.md). Like
# the macOS target it announces nothing — `make desktop-announce` does, after a
# person has tested and published. There is no certificate, so today the
# operator types `make windows-release RELEASE_FLAGS=--skip-sign` — unsigned is
# the Windows release channel (that doc's § Signing status), and the flag stays
# explicit rather than defaulted so nothing publishes unsigned by accident.
# Unlike macos-release, --publish therefore does NOT refuse the skip flag.
windows-release:
	bash windows/scripts/release-app.sh --force --publish $(RELEASE_FLAGS)

# Set a release train's marketing version. Two trains, two numbers: mobile
# (iOS + Android share one) and desktop (macOS + Windows + Linux share one),
# moving independently:
#
#   make version-mobile 3.6.7
#   make version-desktop 3.7.0
#
# Policy and semantics: docs/architecture/manual-release-notes.md § Set the version.
# The iOS `.pbxproj` is user-owned (Claude may not edit it); this target is the
# maintainer running that edit.
#
# The version is a bare word, which make reads as a second goal — so it gets a
# do-nothing rule, declared only while one version target is among the goals. A
# mistyped target on any other command line still fails the way it should. One
# train per invocation: both targets on one line would read the same bare word.
VERSION_TARGETS := version-mobile version-desktop
VERSION_ARGS := $(filter-out $(VERSION_TARGETS),$(MAKECMDGOALS))
ifneq ($(filter $(VERSION_TARGETS),$(MAKECMDGOALS)),)
ifneq ($(words $(filter $(VERSION_TARGETS),$(MAKECMDGOALS))),1)
$(error one release train per invocation: make version-mobile X.Y.Z  or  make version-desktop X.Y.Z)
endif
ifneq ($(VERSION_ARGS),)
$(eval $(VERSION_ARGS):;@:)
endif
endif

version-mobile version-desktop:
	@version="$(firstword $(VERSION_ARGS) $(VERSION))"; \
	if [ -z "$$version" ]; then echo "Usage: make $@ <MAJOR.MINOR.PATCH>"; exit 2; fi; \
	python3 tools/release_notes.py set-versions --train "$(patsubst version-%,%,$@)" --version "$$version"

# ---------------------------------------------------------------------------
# Formatting & lint — apply across all stacks (`fmt`) or check (`lint`).
# ---------------------------------------------------------------------------
#   Rust    rustfmt over all four workspaces (fmt); clippy -D warnings over the
#           engine and desktop-shared crates (lint). The Windows and Linux
#           workspaces are linted by `make windows-check` / `make linux-check`,
#           which need their cross targets.
#   Swift   SwiftFormat (Nick Lockwood) — config: .swiftformat (version: mise.toml)
#   Kotlin  Spotless Gradle plugin — wired in android/app/build.gradle.kts
#           (spotlessCheck doubles as ktlint).
# CI runs the same checks: engine.yml, checks.yml, android.yml on pull requests;
# linux-build.yml nightly.
RUST_WORKSPACES := engine desktop windows linux

fmt:
	for ws in $(RUST_WORKSPACES); do cargo fmt --all --manifest-path $$ws/Cargo.toml || exit 1; done
	swiftformat ios macos
	cd android && ./gradlew spotlessApply

lint:
	for ws in $(RUST_WORKSPACES); do cargo fmt --all --check --manifest-path $$ws/Cargo.toml || exit 1; done
	cargo clippy --manifest-path $(ENGINE)/Cargo.toml --workspace --all-targets --locked -- -D warnings
	cargo clippy --manifest-path desktop/Cargo.toml --workspace --all-targets --locked -- -D warnings
	swiftformat --lint ios   # one directory per call: `--lint ios macos` reads macos as --lint's value
	swiftformat --lint macos
	cd android && ./gradlew spotlessCheck

# Point git at the repo's tracked hooks. Per clone, not per checkout — core.hooksPath
# lives in .git/config, so a fresh clone has no gate until this runs. That is why the
# same scan also runs in CI, which no clone can skip.
hooks:
	git config core.hooksPath .githooks
	@echo "✓ core.hooksPath = .githooks — staged changes are now scanned before every commit"
	@command -v gitleaks >/dev/null 2>&1 || echo "⚠ gitleaks not installed; the hook will pass through until you run: brew install gitleaks"

# Scan for credentials. Both targets are scripts/gitleaks-scan.sh, which is also
# what CI runs — the coverage rules, the baseline checks and the exit-code
# handling live there so the local gate and the CI gate cannot drift apart.
#
# scan-secrets covers everything .gitleaks-scanned's watermark does not already
# vouch for; on an 845 MB history that is ~2 s against ~72 s, and rescanning
# immutable commits under the same rules can only find what the recorded run
# already did. scan-secrets-full rescans everything and prints the new baseline
# to write into .gitleaks-scanned — needed after a gitleaks upgrade, a
# .gitleaks.toml or .gitleaksignore change, or a history rewrite.
scan-secrets:
	./scripts/gitleaks-scan.sh

scan-secrets-full:
	./scripts/gitleaks-scan.sh --full

# Scan every tracked text file for the personal identifiers in the maintainer's
# private denylist (see scripts/private-denylist-scan.sh; the pre-commit hook runs
# the same list over staged additions). Binaries and untracked files are not read.
# Without the list it checks nothing and passes.
scan-private:
	./scripts/private-denylist-scan.sh --tree

# Pull the latest remote-default-branch commit for the taigi-converter submodule
# into the working tree. Submodules always record a pinned SHA, so review + commit
# the gitlink bump afterwards.
update-submodules:
	git submodule update --init --remote --recursive
	@echo ""
	@echo "✓ submodule pulled to latest. Gitlink bump to review + commit:"
	@git submodule status

help:
	@echo "Build and test (commands per platform: docs/BUILDING.md)"
	@echo "  make test               cargo test --workspace (engine, includes doctests)"
	@echo "  make test-crate CRATE=<name>  cargo test for one engine crate"
	@echo "  make doc                Build rustdoc HTML for the engine workspace and open it"
	@echo "  make protos             Regenerate the committed Swift + Java protobuf bindings (protoc 36.2)"
	@echo "  make ios-libs           Engine xcframework for iOS (macOS host)"
	@echo "  make android-libs       Engine jniLibs for Android (cargo-ndk + NDK)"
	@echo "  make macos-libs         Engine xcframework for the macOS input method (macOS host)"
	@echo "  make build              All four above, in order (macOS host, every toolchain; no tests)"
	@echo "  make desktop-check      Native gate for the desktop-shared crates (desktop/)"
	@echo "  make linux-check        Host-side compile + test gate for the Linux input method"
	@echo "  make windows-check      Host-side compile + test gate for the Windows input method"
	@echo ""
	@echo "Data and content"
	@echo "  make dict               Full dictionary regeneration into dictionaries/ (needs taigi-converter)"
	@echo "  make i18n               Regenerate app-UI i18n native resources from i18n/*.json"
	@echo "  make i18n-test          Run the i18n codegen + production-content unit tests"
	@echo "  make dogfood            Print continuous-input dogfood test table (TL/POJ/TPS + 漢字)"
	@echo "  make e2e PLATFORM=linux End-to-end run on the Linux VM (test-mode build) + analyzer report"
	@echo "  make e2e PLATFORM=linux-desktop  Same, inside both Linux VMs' real desktop sessions (GNOME+IBus, KDE+Fcitx5)"
	@echo "  make update-submodules  Pull latest for all submodules (review + commit gitlink bumps)"
	@echo ""
	@echo "Quality and hooks"
	@echo "  make fmt                Apply formatting: rustfmt (4 workspaces) + SwiftFormat + Spotless"
	@echo "  make lint               Check it: rustfmt + clippy (engine, desktop) + SwiftFormat + Spotless"
	@echo "  make hooks              Activate the repo's git hooks in this clone (secret scan on commit)"
	@echo "  make scan-secrets       Scan for credentials since the last clean full scan"
	@echo "  make scan-secrets-full  Rescan the whole history and re-baseline .gitleaks-scanned"
	@echo "  make scan-private       Scan tracked files against the maintainer's private denylist"
	@echo ""
	@echo "Maintainer releases"
	@echo "  make version-mobile 3.6.7   Set the mobile train's version (iOS + Android)"
	@echo "  make version-desktop 3.7.0  Set the desktop train's version (macOS + Windows + Linux)"
	@echo "  make macos-release      Sign + notarize + stage the package on the draft release"
	@echo "  make windows-release    Build + package + stage the installer on the draft (on Windows)"
	@echo "                          — add RELEASE_FLAGS=--skip-sign until a certificate exists"
	@echo "  make desktop-release    Stage all three desktop installers on the draft (Mac + hosted runners)"
	@echo "  make desktop-patch PLATFORM=linux  Stage ONE platform's patch release on its own draft"
	@echo "  make desktop-announce   Announce a published desktop release (website + appcasts)"
