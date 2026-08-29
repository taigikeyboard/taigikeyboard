# Dev loop for the Windows input method. Two audiences:
#
#   * on the macOS host (no Windows machine): `make check` is the whole gate —
#     native tests for the host-testable crates + cross-target checks for
#     every crate the workspace currently holds (docs/architecture/
#     windows-roadmap.md W13). Compilation proof only, never behaviour;
#   * on a Windows machine: `make build` produces the release DLL + exe;
#     `make install` registers the DLL (elevated prompt) — see README.md.
#
# The repo-root `make windows-check` / `make windows-release` forward here.

export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: help i18n-check test check-gnu check-msvc check fmt lint build clean

# Crates with no C dependency: these are the ones the MSVC target can be
# checked for from macOS. A crate that pulls in C (the storage crate's
# rusqlite, when it joins) is gnu-only here — see rust-toolchain.toml. Both
# rosters grow with the workspace; they name what exists today.
MSVC_CHECKABLE := -p taigi-windows-core
HOST_TESTABLE  := -p taigi-windows-core

help:
	@echo "Windows input method — dev loop"
	@echo ""
	@echo "  make check       - Host-side gate: i18n check + native tests + gnu clippy + msvc check"
	@echo "  make test        - Native tests for the host-testable crates"
	@echo "  make check-gnu   - clippy -D warnings, full graph, target x86_64-pc-windows-gnu"
	@echo "  make check-msvc  - cargo check for the C-free crates, target x86_64-pc-windows-msvc"
	@echo "  make i18n-check  - Fail if the generated i18n Rust is stale"
	@echo "  make fmt         - cargo fmt"
	@echo "  make build       - (Windows) cargo build --release for the shipping target"

# The Rust under crates/taigi-windows-core/src/strings/generated.rs is written
# by the repo-root `make i18n`; a stale copy would compile strings that no
# longer match their source. Same gate the macOS Makefile runs.
i18n-check:
	python3 ../tools/i18n/check.py

test: i18n-check
	cargo test $(HOST_TESTABLE)

check-gnu:
	cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings

check-msvc:
	cargo check $(MSVC_CHECKABLE) --all-targets --target x86_64-pc-windows-msvc

check: test check-gnu check-msvc
	cargo fmt -- --check

# Plain `cargo fmt`, never `--all`: `--all` also formats path dependencies, i.e.
# the engine crates, which carry pre-existing rustfmt drift that is not ours.
fmt:
	cargo fmt

lint:
	cargo clippy --workspace --all-targets -- -D warnings

build:
	cargo build --release --target x86_64-pc-windows-msvc

clean:
	cargo clean
