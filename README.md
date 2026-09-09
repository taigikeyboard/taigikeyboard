# TaigiKeyboard 台語齒盤

A Taiwanese input method for iOS, Android, macOS, and Windows. Romanization input in POJ, TL and TPS, Hanji, tone marks, autocomplete, and cross-system Romanization conversion.

[![Windows build](https://github.com/taigikeyboard/taigikeyboard/actions/workflows/windows-build.yml/badge.svg?branch=main)](https://github.com/taigikeyboard/taigikeyboard/actions/workflows/windows-build.yml)
![iOS 17+](https://img.shields.io/badge/iOS-17%2B-blue)
![Android 11+](https://img.shields.io/badge/Android-11%2B-green)
![macOS 14+](https://img.shields.io/badge/macOS-14%2B-lightgrey)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue)](LICENSE)

## Why

Taiwanese is spoken by millions, and no operating system vendor ships a keyboard
for it — Apple, Google and Microsoft all include Mandarin input methods and none
include Taiwanese. A language that cannot be typed cannot be written online,
cannot accumulate text corpora, and cannot feed the tooling every other language
takes for granted. An input method is the foundational layer of digital language
infrastructure, and for Taiwanese that layer has been missing.

Existing alternatives each cover one slice: one romanization system, or one
platform, or romanization without Hanji. This project aims at full coverage —
all three romanization systems in use today, Hanji and 漢羅 mixed script,
continuous multi-syllable input, and the same behavior on all four platforms.

## Download

| Platform | Where |
| --- | --- |
| iOS / iPadOS | [App Store](https://apps.apple.com/app/id6751871806) |
| Android | [Google Play](https://play.google.com/store/apps/details?id=com.siansiansu.taigikeyboard) |
| macOS / Windows | [taigikeyboard.tw](https://taigikeyboard.tw) |

## Features

- Romanization input: POJ, TL, TPS
- Hanji input via romanization
- Tone marks, tone numbers, tone variation
- Autocomplete and next-word prediction
- Custom dictionary and user frequency learning
- Lossless TL ↔ POJ ↔ TPS conversion

## Architecture

All four platforms share a Rust core. Algorithms for phonetics, composing, lexicon, ranking, next-word and dispatch live in `engine/`. Platform code is thin glue: Swift on iOS and macOS via swift-bridge, Kotlin on Android via JNI, Rust all the way down on Windows. Protobuf carries payloads across the FFI boundary.

| Path | Stack |
| --- | --- |
| `engine/` | Rust workspace; xcframework for iOS, JNI `.so` for Android on arm64-v8a and armeabi-v7a |
| `ios/` | Swift + KeyboardKit |
| `android/` | Kotlin + Jetpack Compose UI; FlorisBoard-derived view hierarchy |
| `macos/` | Swift + InputMethodKit; SwiftPM |
| `windows/` | Rust + Text Services Framework; Inno Setup installer |
| `dictionary/` | Source data + FST/mmap build pipeline |
| `taigi-converter/` | Canonical TL/POJ/TPS converter, a git submodule |

## Documentation

- `docs/README.md`: engine, UI, architecture index
- `knowledge/taigi-phonetics-reference.md`: TL/POJ/TPS cross-reference
- `.claude/rules/`: per-platform style guides, security rules, AI workflow
- `CHANGELOG.md`: release history

## License

Source code is released under the [Apache License, Version 2.0](LICENSE).

**The dictionary data is not.** Each of the fourteen sources keeps its own
terms — CC0, CC BY, CC BY-SA, CC BY-ND, CC BY-NC-SA, 開放政府資料授權條款, and
a few still unverified. Two carry NonCommercial terms, and because the compiled
dictionary shipped inside every application package merges all sources into one
inseparable index, **that compiled dictionary must be treated as
non-commercial**. See [`dictionary/LICENSE`](dictionary/LICENSE) for the
per-source table and the attribution every build owes.

Fonts, vendored code, and bundled third-party data are inventoried in
[`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to build, test, and submit changes
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting and what this software does with user data
- [`docs/CODE_SIGNING_POLICY.md`](docs/CODE_SIGNING_POLICY.md) — who may release a signed binary, and how to verify one
