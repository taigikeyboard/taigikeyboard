ENGINE := engine
DICT := dictionary

# Ensure rustup-installed cargo is on PATH. `make` runs commands in /bin/sh,
# which does not source the user's interactive shell config. Without this,
# `cargo: command not found` if zsh doesn't `source ~/.cargo/env`.
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test doc dict help

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

help:
	@echo "  make build  Full Rust rebuild: proto regen + iOS + Android + tests"
	@echo "  make test   cargo test --workspace"
	@echo "  make doc    Build rustdoc HTML for engine workspace and open in browser"
	@echo "  make dict   Full dictionary regen + deploy to Android/iOS"
