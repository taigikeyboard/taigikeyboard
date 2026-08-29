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
| `crates/taigi-windows-update` (PR9) | lib | update manifest, download, Authenticode verification. |
| `crates/taigi-windows-tsf` (PR5) | cdylib `TaigiKeyboard.dll` | The COM text service + candidate window. |
| `crates/taigi-windows-settings` (PR7) | bin `TaigiKeyboardSettings.exe` | The settings window. |

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
```

Registration, installer and release flow arrive with PR5 / PR10
(`docs/architecture/windows-release.md`).
