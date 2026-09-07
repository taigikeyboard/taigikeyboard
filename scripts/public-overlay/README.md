# Taigi Keyboard — Windows

A Taiwanese (Tâi-gí) input method for Windows, built on the Text Services
Framework. It accepts POJ, Tâi-lô and TPS romanization and produces Hanji
(漢字), Hanlo or romanized output, with tone variation, autocomplete and
continuous input.

Homepage and downloads: <https://taigikeyboard.tw/>

## Layout

| Path | Contents |
| --- | --- |
| `windows/` | The TSF text service, the WinUI 3 settings application, and the packaging scripts |
| `engine/` | The shared Rust engine — segmentation, ranking, romanization, dictionary lookup |
| `ios/Resources/` | The compiled dictionary and the fonts the installer stages into the product |

The engine is shared with this project's iOS, Android and macOS applications,
which are developed separately.

## Building

The text service links against `msctf` and builds only on Windows with the MSVC
toolchain. `.github/workflows/windows-build.yml` builds the installer end to end
and is the authoritative recipe; it pins the `protoc` and Inno Setup versions.

```
bash windows/scripts/release-app.sh --skip-sign
```

The engine's own tests run anywhere:

```
cargo test --manifest-path engine/Cargo.toml
```

## Licence

Apache License, Version 2.0 — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
Components under other terms are inventoried in
[`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).
