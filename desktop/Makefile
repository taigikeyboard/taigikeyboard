# Dev loop for the desktop-shared crates (`taigi-desktop-core`,
# `taigi-desktop-storage`, `taigi-desktop-update`): the pure Rust the Windows and Linux input methods
# link (the update check: Windows only). Everything here runs natively on any host — no cross target, no
# OS handle. The shells' own gates (`make windows-check`, `make linux-check`)
# compile these crates for their targets through the graph.
#
# The repo-root `make desktop-check` forwards here.

export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: help i18n-check test lint fmt check clean

help:
	@echo "Desktop-shared crates — dev loop"
	@echo ""
	@echo "  make check       - The gate: i18n check + native tests + clippy + fmt --check"
	@echo "  make test        - Native tests for every crate"
	@echo "  make lint        - clippy -D warnings, all targets"
	@echo "  make i18n-check  - Fail if the generated i18n Rust is stale"
	@echo "  make fmt         - cargo fmt"

# `crates/taigi-desktop-core/src/strings/generated.rs` is written by the
# repo-root `make i18n`; a stale copy would compile strings that no longer
# match their source. Same gate the macOS and Windows Makefiles run.
i18n-check:
	python3 ../tools/i18n/check.py

test: i18n-check
	cargo test --workspace

lint:
	cargo clippy --workspace --all-targets -- -D warnings

# Plain `cargo fmt`, never `--all`: `--all` also formats path dependencies, i.e.
# the engine crates.
fmt:
	cargo fmt

check: test lint
	cargo fmt -- --check

clean:
	cargo clean
