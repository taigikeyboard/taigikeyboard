ENGINE := engine
DICT := dictionary

# Ensure rustup-installed cargo is on PATH. `make` runs commands in /bin/sh,
# which does not source the user's interactive shell config. Without this,
# `cargo: command not found` if zsh doesn't `source ~/.cargo/env`.
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test test-crate doc dict dogfood help \
        fmt lint \
        i18n i18n-test \
        macos-engine macos-protos macos-release \
        update-submodules

# Default — regenerate platform proto, full clean, rebuild iOS xcframework
# + Android jniLibs + macOS xcframework. Build ONLY — does NOT run tests
# (use `make test`). The only build entry point: it refreshes EVERY committed
# generated artefact, so no platform can go stale behind an engine change.
# macOS ships no release yet; it is built here anyway to keep that invariant
# (USER 2026-08-15: 「我覺得可以併入到 make build,只是現階段不 release」).
# Requires `brew install protobuf swift-protobuf` for the proto step.
build:
	@echo "==> [1/6] Regenerating platform proto (Swift + Java)"
	bash $(ENGINE)/scripts/gen-platform-protos.sh
	@echo "==> [2/6] Regenerating macOS proto (Swift)"
	bash $(ENGINE)/scripts/gen-macos-protos.sh
	@echo "==> [3/6] Cleaning protos build cache"
	cd $(ENGINE) && cargo clean -p protos
	@echo "==> [4/6] Building iOS xcframework"
	cd $(ENGINE) && bash scripts/build-xcframework.sh
	@echo "==> [5/6] Building Android jniLibs"
	cd $(ENGINE) && bash scripts/build-android-libs.sh
	@echo "==> [6/6] Building macOS xcframework"
	cd $(ENGINE) && bash scripts/build-macos-xcframework.sh
	@echo ""
	@echo "✓ build complete — Xcode: Clean Build Folder ⇧⌘K → Build"

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
# merge + bin + fst + audit + deploy to Android/iOS (build.sh).
dict:
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

# ---------------------------------------------------------------------------
# macOS-only shortcuts (docs/architecture/macos-roadmap.md D1). `make build`
# already runs both; these exist to iterate on macOS without paying the full
# 3-5 min iOS + Android rebuild.
# No `cargo clean -p protos` here: engine/protos/build.rs declares
# `rerun-if-changed=proto`, so a .proto edit already rebuilds protos (verified);
# `build`'s clean is belt-and-braces from D9.4.
# ---------------------------------------------------------------------------

macos-engine:
	bash $(ENGINE)/scripts/build-macos-xcframework.sh

macos-protos:
	bash $(ENGINE)/scripts/gen-macos-protos.sh

# Signed + notarized installer package for web distribution, and the only
# entry point for one — `macos/Makefile` is the dev loop and stops at `bundle`.
# Prerequisites, flags, and the one-time Developer ID setup:
# docs/architecture/macos-release.md.
macos-release:
	bash macos/scripts/release-app.sh $(RELEASE_FLAGS)

# ---------------------------------------------------------------------------
# Formatting & lint — apply across all stacks (`fmt`) or check (`lint`).
# ---------------------------------------------------------------------------
#   Rust    rustfmt (fmt) + clippy -D warnings (lint)
#   Swift   SwiftFormat (Nick Lockwood) — config: .swiftformat. Install: brew install swiftformat
#   Kotlin  Spotless Gradle plugin — wired in android/app/build.gradle (spotlessCheck doubles as ktlint).
# To check Rust formatting without writing: `cd engine && cargo fmt --all -- --check`.

fmt:
	cd $(ENGINE) && cargo fmt --all
	swiftformat ios macos
	cd android && ./gradlew spotlessApply

lint:
	cd $(ENGINE) && cargo clippy --workspace --all-targets --locked -- -D warnings
	cd android && ./gradlew spotlessCheck

# Pull the latest remote-default-branch commit for the taigi-converter submodule
# into the working tree. Submodules always record a pinned SHA, so review + commit
# the gitlink bump afterwards.
update-submodules:
	git submodule update --init --remote --recursive
	@echo ""
	@echo "✓ submodule pulled to latest. Gitlink bump to review + commit:"
	@git submodule status

help:
	@echo "  make build              Full Rust rebuild: proto regen + iOS + Android + macOS (no tests)"
	@echo "  make test               cargo test --workspace (canonical, includes doctests)"
	@echo "  make test-crate         cargo test -p \$$CRATE (touched-target round workflow)"
	@echo "  make doc                Build rustdoc HTML for engine workspace and open in browser"
	@echo "  make dict               Full dictionary regen + deploy to Android/iOS"
	@echo "  make i18n               Regenerate app-UI i18n native resources from i18n/*.json"
	@echo "  make i18n-test          Run the i18n codegen + production-content unit tests"
	@echo "  make dogfood            Print continuous-input dogfood test table (TL/POJ/TPS + 漢字)"
	@echo "  make macos-engine       macOS-only shortcut: rebuild macos/RustEngine xcframework"
	@echo "  make macos-protos       macOS-only shortcut: regenerate macOS .pb.swift"
	@echo "  make macos-release      Signed + notarized .pkg for web distribution (Developer ID)"
	@echo "  make update-submodules  Pull latest for all submodules (review + commit gitlink bumps)"
	@echo ""
	@echo "  make fmt                Apply formatting across Rust + Swift + Kotlin"
	@echo "  make lint               cargo clippy + spotlessCheck (Android Lint disabled)"
