# Installing TaigiKeyboard

Desktop installers and Linux packages are on the [Releases](https://github.com/taigikeyboard/taigikeyboard/releases) page. Each file has a matching `.sha256`.

## iOS / iPadOS

1. Install from the [App Store](https://apps.apple.com/app/id6751871806). Requires iOS 17 or later.
2. Settings → General → Keyboard → Keyboards → Add New Keyboard → TaigiKeyboard.
3. Tap TaigiKeyboard and turn on Allow Full Access.
4. In any text field, press and hold the globe key to switch to it.

## Android

1. Install from [Google Play](https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard). Requires Android 9 or later.
2. Open the app and follow its setup guide to enable the keyboard. Menu names vary by phone brand.

## macOS

Requires macOS 14 or later.

```sh
brew install --cask taigikeyboard/tap/taigikeyboard
```

Or download `TaigiKeyboard-<version>.pkg` from [taigikeyboard.tw](https://taigikeyboard.tw) and open it.

Then add it: System Settings → Keyboard → Text Input → Input Sources → Edit → +. Search for `taigi`, choose Multiple languages → TaigiKeyboard, then Add. The input-source list is cached per login session, so if TaigiKeyboard is not listed after a first install, log out and back in. Select TaigiKeyboard from the menu bar's input menu.

See the [macOS installation screenshots](../media/screenshots/install/README.md#macos) for the installer and input-source setup in English, Traditional Chinese and Japanese.

## Windows

Requires Windows 10 or later, x64.

1. Download `TaigiKeyboard-<version>.exe` from [taigikeyboard.tw](https://taigikeyboard.tw) and run it. The installer is not code-signed, so SmartScreen may warn: More info → Run anyway.
2. Open Settings → Time & language → Language & region. Under Preferred languages, open the three-dot menu beside Chinese (Traditional, Taiwan), then choose Language options. Add the language first if it is not listed.
3. Scroll to Keyboards → Installed keyboards and check for TaigiKeyboard. If it is missing, click Add a keyboard and select TaigiKeyboard. If it still does not appear, sign out and sign in again.
4. Press Windows + Space to switch to TaigiKeyboard.

See the [Windows setup screenshots](../media/screenshots/install/README.md#windows) for SmartScreen, installer, Settings pages, and the keyboard menu (Windows 11 in English, Traditional Chinese, and Japanese). The UAC secure-desktop prompt is described but has no screenshot.

## Linux

x86_64 and arm64 (Arch Linux: x86_64 only). Works with Fcitx5 or IBus. Check your architecture with `uname -m`: `x86_64`, or `aarch64` for arm64.

| Distribution | Architecture | Package | Install |
| --- | --- | --- | --- |
| Ubuntu 24.04+, Debian 13+, Mint 22+, Pop!_OS 24.04+ | x86_64 | `taigikeyboard_<version>_amd64.deb` | `sudo apt install ./taigikeyboard_<version>_amd64.deb` |
| Ubuntu 24.04+, Debian 13+ | arm64 | `taigikeyboard_<version>_arm64.deb` | `sudo apt install ./taigikeyboard_<version>_arm64.deb` |
| Fedora 44+ | x86_64 | `taigikeyboard-<version>-1.x86_64.rpm` | `sudo dnf install ./taigikeyboard-<version>-1.x86_64.rpm` |
| Fedora 44+ | arm64 | `taigikeyboard-<version>-1.aarch64.rpm` | `sudo dnf install ./taigikeyboard-<version>-1.aarch64.rpm` |
| Arch Linux | x86_64 | `taigikeyboard-<version>-1-x86_64.pkg.tar.zst` | `sudo pacman -U taigikeyboard-<version>-1-x86_64.pkg.tar.zst` |

Ubuntu 22.04 and Debian 12 are too old (the settings window needs GTK 4.12 and libadwaita 1.5).

After installing, restart the input method framework:

```sh
fcitx5 -r        # Fcitx5
ibus restart     # IBus
```

Then add 台語齒盤 in `fcitx5-configtool` (Fcitx5), or in your desktop's input source settings (IBus).

These packages add no repository, so they do not update themselves. Install the newer package the same way to upgrade. Settings and learned words in `~/.config/taigikeyboard/` and `~/.local/share/taigikeyboard/` are kept across upgrades and removal.

Arch users can also install the community-maintained AUR package `taigikeyboard`.

To build from source, see [BUILDING.md](BUILDING.md).
