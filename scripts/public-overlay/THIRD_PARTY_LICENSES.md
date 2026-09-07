# Third-party licences

Taigi Keyboard's own source code is licensed under the Apache License,
Version 2.0 (`LICENSE`). This file inventories everything in this repository —
and everything shipped inside a released application package — that is **not**
covered by that licence.

## Fonts (shipped inside every application package)

All four are licensed under the **SIL Open Font License, Version 1.1**
(OSI-approved). The licence text is published at <https://openfontlicense.org/>; the per-font
copyright lines below were read out of each font's own `name` table.

| Font | File | Copyright / upstream |
| --- | --- | --- |
| 芫荽 Iansui | `Iansui-Regular.ttf` | Copyright 2025 The Iansui Project Authors (<https://github.com/ButTaiwan/iansui>). Designed by But Ko / Fontworks Inc.; derived from Klee One. |
| jf open 粉圓 2.1 | `jf-openhuninn-2.1.ttf` | justfont (<https://justfont.com/huninn>). Latin/kana components: Kosugi Maru © 2010 MOTOYA CO.,LTD.; Varela Round © 2011-2016 The Varela Round Project Authors, with Reserved Font Names "Varela" and "Varela Round". |
| 源樣黑體 GenYoGothic2 TW | `GenYoGothic2TW-R.otf` | ButTaiwan (<https://github.com/ButTaiwan/genyo-font>); derived from Adobe Source Han Sans. |
| 源樣明體 GenYoMin2 TW | `GenYoMin2TW-R.otf` | ButTaiwan (<https://github.com/ButTaiwan/genyo-font>); derived from Adobe Source Han Serif. |

The Windows installer stages them from `windows/resources/Fonts/`
(`windows/scripts/release-app.sh`).

## Vendored source

| Component | Path | Licence | Copyright |
| --- | --- | --- | --- |
| ISEmojiView | `ios/Vendor/ISEmojiView/` | MIT | © 2015 isaced |
| androidx material icon path data | `android/app/src/main/java/com/siansiansu/taigikeyboard/ui/components/TaigiIcons.kt` | Apache-2.0 | © 2025 The Android Open Source Project |
| Inno Setup Traditional Chinese messages | `windows/installer/Languages/ChineseTraditional.isl` | Inno Setup licence | Translation by Enfeng Tsao, based on Samuel Lee's 5.5.3+ work; from `jrsoftware/issrc` at tag `is-6_7_3`, where it is an unofficial translation under `Files/Languages/Unofficial/`. Compile-time input to the installer's message table only — the file itself is not shipped. |

`TaigiIcons.kt` carries its own Apache-2.0 header; the path data was extracted
verbatim from `androidx.compose.material:material-icons-extended` to drop the
transitive dependency.

## Bundled data

| Component | Path | Licence |
| --- | --- | --- |
| SymSpell `frequency_dictionary_en_82_765.txt`, filtered | `android/app/src/main/assets/english_freq.txt` | MIT © Wolf Garbe (<https://github.com/wolfgarbe/SymSpell>) |

Drives English-mode autocomplete on Android. Frequencies derive from Google
Books Ngram + SCOWL. Regenerate with `android/tools/build_english_freq.py`.

## Submodules and sibling repositories

| Component | Licence |
| --- | --- |
| `taigi-converter/` (git submodule) | MIT © 2026 TaigiKeyboard |
| `taigi-emojis/` | MIT © 2026 TaigiKeyboard |

`taigi-emojis` embeds pinned snapshots of Unicode `emoji-test.txt` and CLDR
annotation XML under the **Unicode License** (OSI-approved as `Unicode-3.0`).
Provenance is recorded in `taigi-emojis/data/SOURCES.md`.

`taigi-converter` ships `jf-openhuninn-2.1.ttf` for its web demo; same OFL-1.1
terms as above.

## Rust dependencies (engine + Windows)

The Rust workspaces admit only OSI-approved licences, enforced in CI by
`cargo deny` against the allowlist in `engine/deny.toml`:

    MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause,
    BSD-3-Clause, ISC, Unicode-DFS-2016, Unicode-3.0, Zlib

Unknown registries and unknown git sources are denied. Reproduce the full
transitive inventory with:

```sh
cargo deny --manifest-path engine/Cargo.toml   list
cargo deny --manifest-path windows/Cargo.toml  list
```

Direct dependencies of note:

| Crate | Licence | Note |
| --- | --- | --- |
| `prost` | Apache-2.0 | Protobuf runtime across the FFI boundary |
| `fst` | MIT / Unlicense | Prefix index |
| `memmap2` | MIT / Apache-2.0 | Dictionary mmap host |
| `rusqlite` (feature `bundled`) | MIT | **Statically links SQLite**, which is public domain |
| `windows`, `windows-core`, `windows-sys`, `windows-numerics` | MIT / Apache-2.0 | Microsoft Windows API bindings |
| `windows-reactor`, `windows-reactor-setup` | MIT / Apache-2.0 | Pinned git revision of `microsoft/windows-rs` |
| `swift-bridge`, `jni` | MIT / Apache-2.0 | Platform FFI glue |
| `serde`, `serde_json`, `thiserror`, `log`, `uuid`, `tempfile`, `sha2`, `regex`, `once_cell`, `indexmap`, `unicode-normalization`, `unicode-properties` | MIT / Apache-2.0 | |
| `rfd` | MIT | Native file dialogs (Windows settings app) |
| `raw-window-handle` | MIT / Apache-2.0 / Zlib | |

## Apple platform dependencies (iOS and macOS)

| Package | Version | Licence |
| --- | --- | --- |
| KeyboardKit | 10.9.0 | MIT © 2016-2025 Daniel Saidi |
| swift-protobuf | 1.38.1 | Apache-2.0 |
| KeyboardShortcuts (macOS only) | 3.0.1 | MIT © Sindre Sorhus |
| **LicenseKit** | 2.1.3 | **Closed source, commercial** — see below |

> **LicenseKit is not open source.** It reaches this project only as a
> transitive dependency of KeyboardKit 10.x on **iOS**, resolved by SwiftPM.
> It is not vendored here, is not used by any first-party code, and is absent
> from the macOS, Android, and Windows builds — in particular it is absent
> from the Windows installer and from every artifact this project submits for
> code signing. It is recorded here rather than omitted, because an accurate
> inventory is worth more than a clean-looking one.

## Android dependencies

Google/JetBrains first-party libraries, all **Apache-2.0**: `androidx.*`
(appcompat, core-ktx, preference-ktx, activity, lifecycle, compose, datastore),
`com.google.android.material:material`,
`org.jetbrains.kotlinx:kotlinx-coroutines-*`, `com.squareup.moshi:moshi-kotlin`.

`com.google.protobuf:protobuf-javalite` is **BSD-3-Clause**.

## Reference repositories

`references/` holds clones of other input methods consulted while designing
this one. It is `.gitignore`d — **no third-party code is tracked in this
repository through it**, and nothing under it is compiled into a release.
`docs/references/mainstream-ime-comparison.md` records what was studied and
what was deliberately not adopted.
