# TaigiKeyboard 台語齒盤

A Taiwanese input method for iOS, Android, macOS, Windows, and Linux.

![iOS 17+](https://img.shields.io/badge/iOS-17%2B-blue)
![Android 9+](https://img.shields.io/badge/Android-9%2B-green)
![macOS 14+](https://img.shields.io/badge/macOS-14%2B-lightgrey)
![Windows 10+](https://img.shields.io/badge/Windows-10%2B-blue)
![Linux Fcitx5 | IBus](https://img.shields.io/badge/Linux-Fcitx5%20%7C%20IBus-orange)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue)](LICENSE)

## Download

| Platform | Where |
| --- | --- |
| iOS / iPadOS | [App Store](https://apps.apple.com/app/id6751871806) |
| Android | [Google Play](https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard) |
| macOS / Windows | [taigikeyboard.tw](https://taigikeyboard.tw) |
| Linux (`.deb`, `.rpm`, Arch; Fcitx5 or IBus) | [GitHub Releases](https://github.com/taigikeyboard/taigikeyboard/releases) |

## Features

- Romanization input in POJ, TL, and TPS
- Hanji and mixed Hanji–Romanization output
- Continuous multi-syllable typing
- Tone marks, tone numbers, and tone variation
- Autocomplete and next-word prediction
- Custom dictionary and frequency learning
- Lossless TL ↔ POJ ↔ TPS conversion

## How it works

One Rust engine (`engine/`) holds phonetics, composing, lexicon, and ranking. Each platform is a thin shell over it:

| Path | Stack |
| --- | --- |
| `ios/` | Swift + KeyboardKit |
| `android/` | Kotlin + Jetpack Compose |
| `macos/` | Swift + InputMethodKit |
| `windows/` | Rust + Text Services Framework |
| `linux/` | Fcitx5 addon (C++) and IBus engine (Rust) |
| `desktop/` | Rust crates shared by Windows and Linux |
| `dictionary/` | Dictionary sources and build pipeline |

## Building

See [`docs/BUILDING.md`](docs/BUILDING.md). Clone with `--recurse-submodules`, then run `make build` once.

## Contributing

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — build, test, and submit changes
- [`docs/README.md`](docs/README.md) — architecture and engine docs
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting and data handling
- [`CHANGELOG.md`](CHANGELOG.md) — release history

## License

Source code: [Apache License 2.0](LICENSE).

**Dictionary data is not Apache-licensed.** Each source keeps its own terms, and one carries a NonCommercial clause, so the compiled dictionary shipped in every app is non-commercial. See [`dictionary/LICENSE`](dictionary/LICENSE) for per-source terms and required attribution, and [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md) for fonts and vendored code. Code signing: [`docs/CODE_SIGNING_POLICY.md`](docs/CODE_SIGNING_POLICY.md).
