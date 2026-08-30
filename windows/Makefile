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

.PHONY: help i18n-check test check-gnu check-msvc check-dll check-exe check fmt lint build install uninstall reload release clean

# Crates with no C dependency: these are the ones the MSVC target can be
# checked for from macOS. A crate that pulls in C (the storage crate's bundled
# rusqlite, and so the settings exe over it) is gnu-only here — see
# rust-toolchain.toml. The TSF crate links the
# `windows` bindings, which do not build for a macOS host at all: its unit
# tests are type-checked by `check-gnu` (`--all-targets`) and RUN only on a
# Windows machine (`cargo test -p taigi-windows-tsf`). Both rosters grow with
# the workspace; they name what exists today.
MSVC_CHECKABLE := -p taigi-windows-core -p taigi-windows-platform -p taigi-windows-update
HOST_TESTABLE  := -p taigi-windows-core -p taigi-windows-storage -p taigi-windows-platform -p taigi-windows-update -p taigi-windows-settings

help:
	@echo "Windows input method — dev loop"
	@echo ""
	@echo "  make check       - Host-side gate: i18n check + native tests + gnu clippy + msvc check"
	@echo "  make test        - Native tests for the host-testable crates"
	@echo "  make check-gnu   - clippy -D warnings, full graph, target x86_64-pc-windows-gnu"
	@echo "  make check-msvc  - cargo check for the C-free crates, target x86_64-pc-windows-msvc"
	@echo "  make check-dll   - GNU release link of TaigiKeyboard.dll + export-table check (W13)"
	@echo "  make check-exe   - GNU release link of TaigiKeyboardSettings.exe (W13)"
	@echo "  make i18n-check  - Fail if the generated i18n Rust is stale"
	@echo "  make fmt         - cargo fmt"
	@echo "  make build       - (Windows) cargo build --release for the shipping target"
	@echo "  make install     - (Windows, elevated) build and register the text service from this checkout"
	@echo "  make uninstall   - (Windows, elevated) unregister it"
	@echo "  make reload      - (Windows, elevated) re-register without rebuilding"
	@echo "  make release     - (Windows) build, sign, package with Inno Setup and publish (scripts/release-app.sh)"

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

# The DLL really links (lto, panic=unwind) and exports the four COM entry
# points unmangled — what regsvr32 will look for. mingw's objdump reads the
# PE export table; the names must appear exactly, so a renamed or `#[no_mangle]`
# -less export fails here rather than on the user's machine.
DLL_GNU := target/x86_64-pc-windows-gnu/release/TaigiKeyboard.dll
check-dll:
	cargo build -p taigi-windows-tsf --release --target x86_64-pc-windows-gnu
	@for symbol in DllGetClassObject DllCanUnloadNow DllRegisterServer DllUnregisterServer; do \
		x86_64-w64-mingw32-objdump -p $(DLL_GNU) | grep -qw "$$symbol" || { echo "missing export: $$symbol"; exit 1; }; \
	done
	@echo "exports OK: $(DLL_GNU)"

# The settings window really links as a Windows GUI executable (eframe +
# glow + winit under mingw), the second artifact the installer ships.
EXE_GNU := target/x86_64-pc-windows-gnu/release/TaigiKeyboardSettings.exe
check-exe:
	cargo build -p taigi-windows-settings --release --target x86_64-pc-windows-gnu
	@test -f $(EXE_GNU) || { echo "missing: $(EXE_GNU)"; exit 1; }
	@echo "exe OK: $(EXE_GNU)"

check: test check-gnu check-msvc check-dll check-exe
	cargo fmt -- --check

# Plain `cargo fmt`, never `--all`: `--all` also formats path dependencies, i.e.
# the engine crates, which carry pre-existing rustfmt drift that is not ours.
fmt:
	cargo fmt

lint:
	cargo clippy --workspace --all-targets -- -D warnings

build:
	cargo build --release --target x86_64-pc-windows-msvc

# The development install, counterpart of the macOS Makefile's install/
# uninstall/reload: registers the build tree in place rather than installing
# into Program Files. Elevated, because regsvr32 writes HKLM. `install` builds
# for itself instead of depending on `build`: it has to unregister and set the
# loaded dll aside FIRST, or the linker cannot replace it.
DEV_INSTALL := powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-dev.ps1

install:
	$(DEV_INSTALL) install

uninstall:
	$(DEV_INSTALL) uninstall

reload:
	$(DEV_INSTALL) reload

# The whole release on a Windows machine (Git Bash): docs/architecture/windows-release.md.
release:
	bash scripts/release-app.sh --force --publish $(RELEASE_FLAGS)

clean:
	cargo clean
