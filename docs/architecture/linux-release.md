# Linux release — `.deb` / `.rpm` / Arch packages on the desktop draft, distro-managed after that

> **Type**: Reference (living)
> **Keywords**: `linux`, `release`, `deb`, `dpkg-deb`, `Fcitx5`, `IBus`, `desktop train`
> **Related**: desktop-release.md (the shared flow), linux-roadmap.md (L10 / L11), windows-release.md (the sibling half)

How the Linux half of a desktop version ships. The shared flow — one draft
release `desktop-<version>` for all three desktops, tested and published by a
person — is `desktop-release.md`; this file is only what the Linux artifact
is and how it is built.

## The artifacts

Three x86_64 packages, each with its `.sha256`, attached to the
`desktop-<version>` draft beside the macOS `.pkg` and the Windows `.exe`:

| Asset | For | Built on |
|---|---|---|
| `taigikeyboard_<version>_amd64.deb` | Ubuntu 24.04+ / Debian 13+ / Mint 22+ / Pop!_OS 24.04+ | `ubuntu-24.04` runner |
| `taigikeyboard-<version>-1.x86_64.rpm` | Fedora 44+ | `fedora:44` container |
| `taigikeyboard-<version>-1-x86_64.pkg.tar.zst` | Arch Linux and its rolling derivatives (`pacman -U`) | `archlinux:latest` container |

The floor is the settings window's GTK 4.12 / libadwaita 1.5: Ubuntu 22.04 and
Debian 12 ship older ones and are not targeted. Flatpak / Snap / AppImage are
not offered — an IME's addon and engine have to be registered with the host's
Fcitx5 / IBus, which a sandboxed or self-contained bundle cannot do. No AUR
entry: that needs a maintainer account and is a separate decision.

All three are packed from the SAME staged `make install PREFIX=/usr` root —
no second build inside rpmbuild or makepkg. The Fcitx5 addon lands in each
distribution's own library dir through CMake's `GNUInstallDirs`
(`lib/x86_64-linux-gnu`, `lib64`, `lib`); every other path is identical.
`/usr/libexec` on Arch departs from its packaging guideline (`/usr/lib`),
accepted to keep one layout; the component XML names the absolute path.
One package holds both shells, the way `fcitx5-chewing` and
`ibus-chewing` come from one source:

| Path | What |
|---|---|
| `/usr/lib/<multiarch>/fcitx5/libtaigikeyboard.so` + `/usr/share/fcitx5/{addon,inputmethod}/taigikeyboard.conf` | The Fcitx5 addon (primary) and its registration (`Library=export:libtaigikeyboard` → that file) |
| `/usr/libexec/ibus-engine-taigikeyboard` + `/usr/share/ibus/component/taigikeyboard.xml` | The IBus engine (second) and its component registration |
| `/usr/bin/taigikeyboard-settings` + `/usr/share/applications/tw.taigikeyboard.Settings.desktop` + `/usr/share/icons/hicolor/*/apps/taigikeyboard.png` | The GTK 4 / libadwaita settings window, its launcher entry and icon |
| `/usr/share/taigikeyboard/dictionaries/*` | The dictionary artifacts the engine reads at first key |
| `/usr/share/fonts/{truetype,opentype}/taigikeyboard/*` + `/usr/share/doc/taigikeyboard/fonts-OFL-1.1.txt` | The four bundled typefaces (the macOS / Windows set) as fontconfig fallbacks; fontconfig's dpkg trigger rebuilds the cache (roadmap L4) |

`Depends: fcitx5 | ibus, fontconfig` (its dpkg trigger caches the bundled typefaces) plus what the three binaries link, versioned, from
`dpkg-shlibdeps` at pack time (`linux/packaging/control.in`, `@SHLIBS@`) — GTK
4.14, libadwaita 1.5, GLib 2.80 on Ubuntu 24.04, the runner that builds it.
Built and checked in CI on Ubuntu 24.04 (compile, `ibus-daemon` smoke, the
package's contents and the addon file name); installing and typing with it on
a real desktop is S74, not yet done. A `.deb` installed by hand adds no apt
source: a newer version is another download.

User data is never in the package: `~/.config/taigikeyboard/settings.json` and
`~/.local/share/taigikeyboard/*.db` survive `apt remove`.

## How it is built

`make -C linux deb` on a Linux machine (or the CI runner):

1. `make install PREFIX=/usr DESTDIR=target/deb/root` — the ONE install
   layout, so the package and a source install cannot drift (both shells,
   the registration files, the dictionaries, the desktop entry, the icons).
2. `packaging/control.in` rendered with the version from `linux/Cargo.toml`
   (moved by `make version-desktop x.y.z` with the other two desktops, and
   held to their number by `release_notes.py check-versions --train desktop`) and
   the `Depends` `dpkg-shlibdeps` computes over the settings window, the IBus
   engine and the Fcitx5 addon.
3. `dpkg-deb --build --root-owner-group`. No `cargo-deb`: it would carry a
   second copy of the asset list. No maintainer scripts: the hicolor icon
   cache is refreshed by the theme package's own dpkg trigger, and Fcitx5 /
   IBus are restarted in the user's session (`fcitx5 -r`, `ibus restart`),
   never from root.

The Fcitx5 addon is C++ over the Rust C ABI and compiles only on Linux
(`linux-roadmap.md` L1); the Mac only syntax-checks it (`make -C linux check-cpp`).
That is why the package is built on the GitHub-hosted runner, never on the
maintainer's Mac.

### The Fedora and Arch packages

- `make -C linux rpm` (on Fedora): `packaging/taigikeyboard.spec.in` copies the
  staged root into the buildroot; rpm's ELF scan writes the library
  `Requires` (the dpkg-shlibdeps of this format) beside `(fcitx5 or ibus)` and
  `fontconfig`. No `%{?dist}` in `Release`, so the asset name is fixed.
- `make -C linux arch` (on Arch, non-root — makepkg refuses root):
  `packaging/PKGBUILD.in` copies the staged root into `$pkgdir`. pacman has no
  alternative dependencies, so `fcitx5` is required (the addon links
  `libFcitx5Core`) and `ibus` is optional. Arch is rolling: a package built
  against one Fcitx5 may need a rebuild after its ABI moves.

CI installs each of the three into a FRESH container of its distribution
(`install-check`, one matrix, the same checks for every format — no build
dependency around to hide a missing runtime one): files in place, every
linked library resolved, `ibus-daemon` spawning the engine from the installed
component, Fcitx5 listing the input method and loading the addon (it is
`OnDemand`; the check asks for it over D-Bus `GetConfig`), fontconfig listing
the bundled typefaces and an existing hicolor icon cache naming the icon.
That is packaging proof only; typing on a real desktop is S74.

## Staging and publishing

`.github/workflows/linux-build.yml` is the Linux half of a desktop release,
the way `windows-build.yml` is the Windows half:

- On every pull request touching `linux/**` / `desktop/**` / `engine/**` it
  builds the package and keeps it as a workflow artifact (`linux-deb`), checks
  the expected paths are inside it, that the addon file is the one the
  `.conf` names, and the desktop entry (`desktop-file-validate`). Nothing
  reaches a draft; the build job can only read.
- `scripts/stage-desktop.sh` (`make desktop-release`) dispatches it on `main`
  beside the Windows run, with the staged commit as `source_sha`, and waits;
  the `attach` job (the only one that can write) runs only with a
  `source_sha`, refuses any commit but that one, refuses a published release
  or a draft targeting another commit, and uploads the `.deb` and its
  `.sha256` to the `desktop-<version>` draft the macOS half created — never
  over an asset already there. A dispatch from any other ref, or without
  `source_sha`, only builds. The build job also checks the package's control
  `Version` / `Architecture` and that the build rewrote nothing tracked
  (`Cargo.lock` included — `make version-desktop` refreshes the members'
  versions in it).
- A `desktop-<version>` **publish** rebuilds for provenance and keeps the
  artifact; the published assets stay what the maintainer tested.

Publishing stays a person's (`desktop-release.md`). No signing: a `.deb`
downloaded from the project page is verified by its `.sha256`, as the unsigned
Windows channel is; an apt repository with its own key is outside this slice.

## No in-app update

Linux packages are updated by the package manager (`linux-roadmap.md` L10):
an input method is a system package, and one that checks for its own updates
works against the distribution that packages it. The input method checks for
nothing, manually or automatically; the 一般 pane shows the running version
and a 去下載 link to taigikeyboard.tw, the panel menu has no 檢查更新 row, the
`update*` settings keys stay unwritten and `taigi-desktop-update` is not linked
(no TLS stack in the package). The announcement (`scripts/announce-release.sh`)
still writes `_data/linux_release.json`, `_data/linux_rpm_release.json` and
`_data/linux_arch_release.json` (version, download URL, `sha256`, release page)
in the same website commit as the other two — the landing page's Linux
buttons read them — but there is no appcast and nothing to poll. A release
missing one format leaves that file on its previous version, so the site
splits the Linux button into three only when all three name the same tag. The
website shows that button only while its `enable_linux_download` is `true`;
that switch hides the entry point, not the asset, which is public the moment
the release is published.

## Installing by hand

```sh
sudo apt install ./taigikeyboard_<version>_amd64.deb             # Ubuntu / Debian
sudo dnf install ./taigikeyboard-<version>-1.x86_64.rpm          # Fedora
sudo pacman -U ./taigikeyboard-<version>-1-x86_64.pkg.tar.zst    # Arch
fcitx5 -r            # or: ibus restart
```

Then add 台語齒盤 in `fcitx5-configtool` (Fcitx5) or the desktop's input-source
settings (IBus). First-machine acceptance is the dogfood run-book in
`linux-roadmap.md` and `dogfood-checklist.md` S74. Uninstall: `sudo apt remove taigikeyboard`
(`sudo dnf remove taigikeyboard`, `sudo pacman -R taigikeyboard`).
