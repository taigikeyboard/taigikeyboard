# Taigi Keyboard

A Taiwanese (Tâi-gí / 台語) input method for iOS and Android. Romanization input (POJ, TL, TPS), Hanji (漢字), tone marks, autocomplete, and cross-system Romanization conversion.

[![CI](https://github.com/taigikeyboard/taigikeyboard/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/taigikeyboard/taigikeyboard/actions/workflows/ci.yml)
[![Security](https://github.com/taigikeyboard/taigikeyboard/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/taigikeyboard/taigikeyboard/actions/workflows/security.yml)
![iOS 17+](https://img.shields.io/badge/iOS-17%2B-blue)
![Android 9+](https://img.shields.io/badge/Android-9%2B-green)
[![License: CC BY-NC-SA 4.0](https://img.shields.io/badge/License-CC_BY--NC--SA_4.0-lightgrey)](LICENSE)

## Features

- Romanization input: POJ (Pe̍h-ōe-jī), TL (台羅), TPS (台語注音)
- Hanji (漢字) input via romanization
- Tone marks, tone numbers, tone variation
- Autocomplete and next-word prediction
- Custom dictionary and user frequency learning
- Lossless TL ↔ POJ ↔ TPS conversion

## Architecture

Both platforms share a Rust core. Algorithms (phonetics, composing, lexicon, ranking, next-word, dispatch) live in `engine/`. Platform code is thin glue: Swift on iOS via swift-bridge, Kotlin on Android via JNI. Protobuf carries payloads across the FFI boundary.

| Path | Stack |
| --- | --- |
| `engine/` | Rust workspace; xcframework for iOS, JNI `.so` for Android (arm64-v8a + armeabi-v7a) |
| `ios/` | Swift + KeyboardKit |
| `android/` | Kotlin + Jetpack Compose UI; FlorisBoard-derived view hierarchy |
| `dictionary/` | Source data + FST/mmap build pipeline |
| `taigi-converter/` | Canonical TL/POJ/TPS converter (git submodule) |

## Documentation

- `docs/README.md`: engine, UI, architecture index
- `knowledge/taigi-phonetics-reference.md`: TL/POJ/TPS cross-reference
- `rules/`: per-platform style guides, security rules, AI workflow
- `CHANGELOG.md`: release history (latest: v3.5.6)

## License

Released under [CC BY-NC-SA 4.0](LICENSE) — non-commercial use only; derivative works must be shared under the same license.
