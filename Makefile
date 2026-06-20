ENGINE := engine
DICT := dictionary

# Ensure rustup-installed cargo is on PATH. `make` runs commands in /bin/sh,
# which does not source the user's interactive shell config. Without this,
# `cargo: command not found` if zsh doesn't `source ~/.cargo/env`.
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test test-crate doc dict dogfood help \
        fmt fmt-check lint \
        fmt-rust fmt-check-rust lint-rust \
        fmt-swift fmt-check-swift \
        fmt-kotlin fmt-check-kotlin lint-kotlin \
        update-submodules

# Default — regenerate platform proto, full clean, rebuild iOS xcframework
# + Android jniLibs. Build ONLY — does NOT run tests (use `make test`).
# The only build entry point.
# Requires `brew install protobuf swift-protobuf` for the proto step.
build:
	@echo "==> [1/4] Regenerating platform proto (Swift + Java)"
	bash $(ENGINE)/scripts/gen-platform-protos.sh
	@echo "==> [2/4] Cleaning protos build cache"
	cd $(ENGINE) && cargo clean -p protos
	@echo "==> [3/4] Building iOS xcframework"
	cd $(ENGINE) && bash scripts/build-xcframework.sh
	@echo "==> [4/4] Building Android jniLibs"
	cd $(ENGINE) && bash scripts/build-android-libs.sh
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

# Generate continuous-input dogfood test table (TL/POJ/TPS + 漢字) from the
# built dictionary.csv. Random each run; prints to stdout for manual on-device
# testing. Requires `make dict` to have produced dictionary/output/dictionary.csv.
dogfood:
	python3 $(DICT)/tools/gen_dogfood.py

# ---------------------------------------------------------------------------
# Formatting & lint
# ---------------------------------------------------------------------------
# Umbrella targets fan out to per-platform recipes. Per-platform recipes can
# also be invoked directly (e.g. `make fmt-rust`) when iterating on one stack.
#
#   Rust    rustfmt + clippy (canonical local pre-commit gate)
#   Swift   SwiftFormat (Nick Lockwood) — config: .swiftformat
#           Install:  brew install swiftformat
#   Kotlin  Spotless Gradle plugin — wired in android/app/build.gradle
#           No extra install; uses the project's Gradle wrapper.

fmt: fmt-rust fmt-swift fmt-kotlin

fmt-check: fmt-check-rust fmt-check-swift fmt-check-kotlin

lint: lint-rust lint-kotlin

# --- Rust ---
fmt-rust:
	cd $(ENGINE) && cargo fmt --all

fmt-check-rust:
	cd $(ENGINE) && cargo fmt --all -- --check

lint-rust:
	cd $(ENGINE) && cargo clippy --workspace --all-targets --locked -- -D warnings

# --- Swift ---
fmt-swift:
	swiftformat ios

fmt-check-swift:
	swiftformat --lint ios

# --- Kotlin ---
# Requires Spotless plugin in android/app/build.gradle (see CLAUDE-managed
# Makefile docs). `spotlessCheck` doubles as ktlint lint.
fmt-kotlin:
	cd android && ./gradlew spotlessApply

fmt-check-kotlin:
	cd android && ./gradlew spotlessCheck

lint-kotlin: fmt-check-kotlin

# Pull the latest tracked-branch commit for every submodule (taigi-emojis -> main,
# taigi-converter -> its remote default branch) into the working tree. Submodules
# always record a pinned SHA, so review + commit the gitlink bumps afterwards.
update-submodules:
	git submodule update --init --remote --recursive
	@echo ""
	@echo "✓ submodules pulled to latest. Gitlink bumps to review + commit:"
	@git submodule status

help:
	@echo "  make build              Full Rust rebuild: proto regen + iOS + Android (no tests)"
	@echo "  make test               cargo test --workspace (canonical, includes doctests)"
	@echo "  make test-crate         cargo test -p \$$CRATE (touched-target round workflow)"
	@echo "  make doc                Build rustdoc HTML for engine workspace and open in browser"
	@echo "  make dict               Full dictionary regen + deploy to Android/iOS"
	@echo "  make dogfood            Print continuous-input dogfood test table (TL/POJ/TPS + 漢字)"
	@echo "  make update-submodules  Pull latest for all submodules (review + commit gitlink bumps)"
	@echo ""
	@echo "  make fmt                Apply formatting across Rust + Swift + Kotlin"
	@echo "  make fmt-check          Verify formatting without writes (CI-style)"
	@echo "  make lint               cargo clippy + spotlessCheck (Android Lint disabled)"
	@echo ""
	@echo "  Per-platform: fmt-rust / fmt-swift / fmt-kotlin"
	@echo "                fmt-check-rust / fmt-check-swift / fmt-check-kotlin"
	@echo "                lint-rust / lint-kotlin"
