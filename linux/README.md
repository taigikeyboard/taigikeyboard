# Linux input method

TaigiKeyboard for Linux: a Fcitx5 addon (primary) and an IBus engine (second)
over one Rust core, plus a GTK 4 / libadwaita settings window, over the
desktop-shared crates in `../desktop` and the engine in `../engine`. Design record and phase
table: `docs/architecture/linux-roadmap.md`.

## Layout

| Crate | Kind | Role |
|---|---|---|
| `crates/taigi-linux-platform` | lib, pure, host-testable | XDG paths + install prefix, IBus key event → `KeyEventSnapshot`, settings launcher, session locale. |
| `crates/taigi-linux-core` | lib, pure | The framework-independent half: runtime (settings, stores, lexicon, coordinator), the key path over the shared `ComposingManager`, the `Emit` effect list both shells replay, the candidate page model. |
| `crates/taigi-linux-ffi` | staticlib (the one `unsafe` crate) | The C ABI (`include/taigikeyboard.h`) the Fcitx5 addon calls: opaque runtime / engine handles, a key in, a reply of effects out. |
| `fcitx5/` | C++ addon `libtaigikeyboard.so` (CMake) | The Fcitx5 shell: `InputMethodEngineV3` over the C ABI — client preedit, commit, candidate list, status area. Built only on Linux (`make build-fcitx5`) and in CI. |
| `crates/taigikeyboard-ibus` | bin `ibus-engine-taigikeyboard` | The IBus shell: bus discovery, `org.freedesktop.IBus.Factory` + `Engine` objects (zbus), hand-serialised IBus wire types, replaying `taigi-linux-core`. |
| `crates/taigikeyboard-settings` | lib + bin `taigikeyboard-settings` | The settings window (GTK 4 + libadwaita): 一般 / 外觀 / 快速齒 / 詞庫來源 / 自訂詞庫 / 關於 (+ unlisted 辭典搜尋); `tests/panes.rs` mounts the whole window. |
| `data/taigikeyboard.xml.in` | component XML | What ibus-daemon reads to know the engine exists (`make component` renders the prefix). |
| `data/tw.taigikeyboard.Settings.desktop`, `data/icons/` | launcher entry + hicolor icons | The settings window in the app grid; the icons are generated with the other desktops' by `tools/desktop/make-app-icon.swift`. |
| `packaging/control.in` | Debian control | `make deb` packs the install layout with dpkg-deb (`docs/architecture/linux-release.md`). |

Behaviour oracle is the macOS input method (`../macos`); the Rust it runs
on is the Windows port (`../windows`, `../desktop`). Deltas are named in the
roadmap's divergence table.

## Working without a Linux machine

```sh
make check          # from linux/: i18n check + native tests + clippy + cross build + fmt
```

Prerequisites on macOS: `brew install gtk4 libadwaita pkgconf zig cargo-zigbuild`,
`rustup target add x86_64-unknown-linux-gnu` (the toolchain file does it on
first use). `zbus` and gtk4-rs both build natively on the Mac, so every crate
is clippy-checked here; `make check-target` cross-builds the engine binary
for the shipping target through `cargo zigbuild`. These gates prove the code
compiles; they do not prove behaviour. The CI job
(`.github/workflows/linux-build.yml`, nightly and on manual dispatch — not per
pull request) adds a real Ubuntu build, the test run
and an `ibus-daemon` smoke; the dogfood run-book in the roadmap owns the rest.

## System packages

Build dependencies, as the CI jobs in `.github/workflows/linux-build.yml`
install them (plus Rust from rustup and protoc 36.0 — `mise install` at the
repository root):

```sh
# Ubuntu / Debian
sudo apt-get install ibus dbus-daemon xvfb libgtk-4-dev libadwaita-1-dev pkg-config \
  fcitx5 fcitx5-modules-dev extra-cmake-modules cmake ninja-build dpkg-dev desktop-file-utils

# Fedora
sudo dnf install git gcc gcc-c++ make cmake ninja-build extra-cmake-modules fcitx5-devel \
  gtk4-devel libadwaita-devel pkgconf-pkg-config rpm-build unzip findutils

# Arch
sudo pacman -S --needed base-devel git cmake ninja extra-cmake-modules fcitx5 gtk4 \
  libadwaita pkgconf unzip rustup
```

`xvfb`, `dbus-daemon` and `ibus` are only for the smoke tests; `dpkg-dev`,
`rpm-build` are only for packaging.

## On a Linux machine

```sh
make build                       # cargo build --release
sudo make install PREFIX=/usr    # both shells, their registration files, dictionaries, the settings launcher
fcitx5 -r                        # and/or: ibus restart
make deb                         # or: the .deb, from the same install layout (target/taigikeyboard_<version>_amd64.deb)
```

Then add 台語齒盤 (language `nan`) in `fcitx5-configtool` (Fcitx5) or the desktop's input-source settings (IBus).
For a development tree, `TAIGIKEYBOARD_DATA_DIR=<repo root>` points the
engine at the repository's own `dictionaries/` without installing them;
`RUST_LOG=debug` on the engine process logs every key's intent.

User data: `~/.config/taigikeyboard/settings.json`,
`~/.local/share/taigikeyboard/{user_frequency,user_association,custom_dictionary,learned_phrases}.db`.
`make uninstall` leaves both directories alone.
