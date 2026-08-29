# Windows input method

Taigi Keyboard for Windows: a Text Services Framework (TSF) text service written
in Rust over the shared engine in `../engine`, with UX parity to the macOS
input method. Design record and phase table:
`docs/architecture/windows-roadmap.md`.

## Layout

| Crate | Kind | Role |
|---|---|---|
| `crates/taigi-windows-core` | lib, `unsafe_code = forbid`, no C deps | Everything that does not need a Win32 handle: settings model, engine bridge, composing orchestration, key classifier, candidate geometry, shortcuts, strings. Tested natively on any host. |
| `crates/taigi-windows-storage` (PR4) | lib | rusqlite user stores, settings file, CSV. |
| `crates/taigi-windows-tsf` (PR5a) | cdylib `TaigiKeyboard.dll`, the ONE crate with `unsafe` | DLL exports, registration, the `TextService` COM object (sinks, tray button + menu, settings reload, context identity), per-process runtime. Not host-testable: `make check-dll` links it under mingw and checks the export table. |
| `crates/taigi-windows-platform` (PR7) | lib, the few Win32 calls both binaries need | locale, `ShellExecuteW`, beep, single-instance mutex, local date, debug logger; host stubs so the exe tests natively. |
| `crates/taigi-windows-update` (PR9) | lib | update manifest, checker, download (`ureq` over schannel), Authenticode + VERSIONINFO verification, toast. |
| `crates/taigi-windows-settings` (PR7-9) | bin `TaigiKeyboardSettings.exe` (eframe/egui `=0.31.1`) | The settings window: 一般 / 外觀 / 快捷鍵 / 自訂詞庫 / 詞庫來源 (+ unlisted 辭典搜尋), the update flow, `--check-updates` headless. |
| `build-support/resource.rs` | build-script include | The icon + VERSIONINFO both binaries embed (`rc.exe` / `llvm-rc` / `windres`). |
| `installer/` + `scripts/` (PR10) | Inno Setup script, scheduled-task definition, release + publish scripts | `docs/architecture/windows-release.md`. |

## Working without a Windows machine

```sh
make check          # from windows/: i18n check + native tests + gnu clippy + msvc check
```

Prerequisites on macOS: `rustup target add` the four targets in
`rust-toolchain.toml` (rustup does it on first use) and `brew install mingw-w64`
for the `x86_64-pc-windows-gnu` cross toolchain.

These gates prove the code compiles for Windows; they do not prove behaviour.
The dogfood run-book in the roadmap owns that.

## On a Windows machine

```sh
make build          # cargo build --release --target x86_64-pc-windows-msvc
make release        # build, sign, package (Inno Setup), publish — see below
```

A development install is `regsvr32 target\x86_64-pc-windows-msvc\release\TaigiKeyboard.dll`
from an elevated prompt (`/u` to unregister) with `Dictionaries\` and `Fonts\`
copied beside the DLL (`scripts/release-app.sh` stages exactly that layout).
Releases: `docs/architecture/windows-release.md` (`make windows-release` at the
repository root); the update manifest contract: `updates/README.md`.
