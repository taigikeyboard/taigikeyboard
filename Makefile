ENGINE := engine
DICT := dictionary

# Ensure rustup-installed cargo is on PATH. `make` runs commands in /bin/sh,
# which does not source the user's interactive shell config. Without this,
# `cargo: command not found` if zsh doesn't `source ~/.cargo/env`.
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test doc dict help \
        fmt fmt-check lint \
        fmt-rust fmt-check-rust lint-rust \
        fmt-swift fmt-check-swift \
        fmt-kotlin fmt-check-kotlin lint-kotlin

# Default — regenerate platform proto, full clean, rebuild iOS xcframework
# + Android jniLibs, run tests. The only build entry point.
# Requires `brew install protobuf swift-protobuf` for the proto step.
build:
	@echo "==> [1/5] Regenerating platform proto (Swift + Java)"
	bash $(ENGINE)/scripts/gen-platform-protos.sh
	@echo "==> [2/5] Cleaning protos build cache"
	cd $(ENGINE) && cargo clean -p protos
	@echo "==> [3/5] Building iOS xcframework"
	cd $(ENGINE) && bash scripts/build-xcframework.sh
	@echo "==> [4/5] Building Android jniLibs"
	cd $(ENGINE) && bash scripts/build-android-libs.sh
	@echo "==> [5/5] cargo test --workspace"
	cd $(ENGINE) && cargo test --workspace
	@echo ""
	@echo "✓ build complete — Xcode: Clean Build Folder ⇧⌘K → Build"

test:
	cd $(ENGINE) && cargo test --workspace

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

help:
	@echo "  make build       Full Rust rebuild: proto regen + iOS + Android + tests"
	@echo "  make test        cargo test --workspace"
	@echo "  make doc         Build rustdoc HTML for engine workspace and open in browser"
	@echo "  make dict        Full dictionary regen + deploy to Android/iOS"
	@echo ""
	@echo "  make fmt         Apply formatting across Rust + Swift + Kotlin"
	@echo "  make fmt-check   Verify formatting without writes (CI-style)"
	@echo "  make lint        cargo clippy + spotlessCheck (Android Lint disabled)"
	@echo ""
	@echo "  Per-platform: fmt-rust / fmt-swift / fmt-kotlin"
	@echo "                fmt-check-rust / fmt-check-swift / fmt-check-kotlin"
	@echo "                lint-rust / lint-kotlin"
