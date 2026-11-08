# Windows input method

TaigiKeyboard for Windows: a Text Services Framework (TSF) text service written
in Rust over the shared engine in `../engine`, with UX parity to the macOS
input method. Design record and phase table:
`docs/architecture/windows-roadmap.md`.

## Layout

The shared desktop crates this workspace links (`taigi-desktop-core`, `-storage`, `-update`) are described in [`../desktop/README.md`](../desktop/README.md). The workspace sets `unsafe_code = "forbid"`; `taigi-windows-tsf`, `taigi-windows-platform` and `taigi-windows-update` override it in their own manifests for Win32 / COM calls.

| Crate | Kind | Role |
|---|---|---|
| `crates/taigi-windows-tsf` | cdylib `TaigiKeyboard.dll` | DLL exports, registration, the `TextService` COM object (sinks, tray button + menu, settings reload, context identity), per-process runtime. Not host-testable: `make check-dll` links it under mingw and checks the export table. |
| `crates/taigi-windows-platform` | lib, the few Win32 calls both binaries need | locale, open URL, dialog owner, beep, single-instance claim, library preload, debug logger, local date / UTC offset, system appearance (dark, accent, high contrast); modules `font_file` (DirectWrite font-file facts), `key_translation` + `keyboard_hook` (key-down → `KeyEventSnapshot`, the shortcut recorder's hook), `layout_remap` (Dvorak / Colemak tables), `os_out_buffer`, `tps_keyboard_click`; host stubs so the exe tests natively. |
| `crates/taigi-windows-update` | lib | update download (`ureq` over schannel), Authenticode + VERSIONINFO verification, toast; the manifest + checker are `../desktop/crates/taigi-desktop-update`. |
| `crates/taigi-windows-settings` | bin `TaigiKeyboardSettings.exe` (WinUI 3 via `windows-reactor`, self-contained Windows App Runtime beside it) | The settings window: General / Appearance / Shortcuts / Dictionary Sources (with its pinned search) / Custom Dictionary / Learning Records / Font Management (+ unlisted About), the update flow, `--check-updates` headless. |
| `build-support/resource.rs` | build-script include | The icon + VERSIONINFO both binaries embed (`rc.exe` / `llvm-rc` / `windres`). |
| `installer/` + `scripts/` | Inno Setup script, scheduled-task definition, release + publish scripts | `docs/architecture/windows-release.md`. |

## Working without a Windows machine

```sh
make check          # from windows/: i18n check + native tests + gnu clippy + msvc check
                    #   + check-dll + check-box + cargo fmt --check
```

`check-box` runs clippy and tests for the settings and platform crates, the
settings exe build and i686 + Arm64 checks of the DLL on the maintainer's Windows box
over ssh (`scripts/check-box.sh`), against the pushed `HEAD`; an unreachable box
is a failure, never a skip, so `make check` fails for anyone without that box.
Run the steps that need none:

```sh
make test check-gnu check-msvc check-dll && cargo fmt -- --check
```

Prerequisites on macOS: `rustup target add` the four targets in
`rust-toolchain.toml` (rustup does it on first use) and `brew install mingw-w64`
for the `x86_64-pc-windows-gnu` cross toolchain.

These gates prove the code compiles for Windows; they do not prove behaviour.
The dogfood run-book in the roadmap owns that.

## On a Windows machine

```sh
make build          # cargo build --release --target x86_64-pc-windows-msvc
make install        # build + register this checkout (ELEVATED shell)
make uninstall      # unregister; %APPDATA% user data stays
make reload         # re-register what is already built, no rebuild
make release        # build, sign, package (Inno Setup), publish — see below
```

`make install` is the development install, the counterpart of the macOS
Makefile's: it registers the build tree in place instead of installing into
Program Files, and it does the whole dance — unregister, set the loaded DLL
aside so the linker can replace it, build, stage `Dictionaries\` and `Fonts\`
beside the DLL (the layout `scripts/release-app.sh` also stages, because the
DLL resolves them from its own directory), register, refresh the input
indicator. Needs an elevated shell: `regsvr32` writes HKLM. Underneath it is
`scripts/install-dev.ps1`, which can be run directly with the same three
arguments.

On an Arm64 machine `make install` builds the Arm64 and the x64 text service
and the Arm64 settings exe, links the Arm64X forwarder
(`scripts/build-forwarder.ps1`, needs the Visual Studio "MSVC ARM64/ARM64EC
build tools" component) and registers a staged copy under `target\arm64x-dev`
— the layout `docs/architecture/windows-arm64-roadmap.md` § Install layout
describes.

Releases: `docs/architecture/windows-release.md` (`make windows-release` at the
repository root); the update manifest contract: `updates/README.md`.
