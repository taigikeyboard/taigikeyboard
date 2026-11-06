# Third-party licences

TaigiKeyboard's own source code is licensed under the Apache License,
Version 2.0 (`LICENSE`). This file inventories everything in this repository —
and everything shipped inside a released application package — that is **not**
covered by that licence.

Dictionary data is inventoried separately, in `dictionary/LICENSE` and
`dictionary/docs/SOURCES.md`, because its terms differ per source.

Not open source, in the shipped packages: that dictionary data, the Windows App
Runtime in the Windows installer (§ Windows runtime), and KeyboardKit and
LicenseKit on iOS (§ Apple platform dependencies).

## Fonts (shipped inside every application package)

All four are licensed under the **SIL Open Font License, Version 1.1**
(OSI-approved). The licence text is in `assets/fonts/OFL-1.1.txt`, beside the files it covers; the per-font
copyright lines below were read out of each font's own `name` table.

| Font | File | Copyright / upstream |
| --- | --- | --- |
| 芫荽 Iansui | `iansui_regular.ttf` | Copyright 2025 The Iansui Project Authors (<https://github.com/ButTaiwan/iansui>). Designed by But Ko / Fontworks Inc.; derived from Klee One. |
| jf open 粉圓 2.1 | `jf_openhuninn_2_1.ttf` | justfont (<https://justfont.com/huninn>). Latin/kana components: Kosugi Maru © 2010 MOTOYA CO.,LTD.; Varela Round © 2011-2016 The Varela Round Project Authors, with Reserved Font Names "Varela" and "Varela Round". |
| 源樣烏體 GenYoGothic2 TW | `genyogothic2tw_r.otf` | ButTaiwan (<https://github.com/ButTaiwan/genyo-font>); derived from Adobe Source Han Sans. |
| 源樣明體 GenYoMin2 TW | `genyomin2tw_r.otf` | ButTaiwan (<https://github.com/ButTaiwan/genyo-font>); derived from Adobe Source Han Serif. |

The four files live once, at `assets/fonts/font/`, and all five platforms package that
one copy. The web page serves CJK-only subsets cut from them at build time, with the licence text
beside them. The file names follow Android's resource-naming rules so its `res/`
merge can read the shared directory unchanged; iOS and macOS ask for the PostScript names inside
the files (`Iansui-Regular`, `jf-openhuninn-2.1`, `GenYoGothic2TW-R`,
`GenYoMin2TW-R`); Windows asks DirectWrite for their family names (`Iansui`,
`jf-openhuninn-2.1`, `GenYoGothic2 TW`, `GenYoMin2 TW`).

## Vendored source

| Component | Path | Licence | Copyright |
| --- | --- | --- | --- |
| ISEmojiView | `ios/Vendor/ISEmojiView/` | MIT | © 2015 isaced |
| androidx material icon path data | `android/app/src/main/java/com/siansiansu/taigikeyboard/ui/components/TaigiIcons.kt` | Apache-2.0 | © 2025 The Android Open Source Project |
| Font Awesome Free 7.3.1 icon path data (`copy`, `trash-can`, regular) | `web/src/index.html` | CC BY 4.0 | © 2026 Fonticons, Inc. (<https://fontawesome.com/license/free>) |
| Font Awesome Free 6.7.2 icon path data (`heart`, `globe`, `github`, `discord`, `envelope`) | `macos/Sources/TaigiInputMethodCore/Settings/Components/FontAwesomeGlyph.swift`, `windows/crates/taigi-windows-settings/src/winui/font_awesome.rs` | CC BY 4.0 | © 2024 Fonticons, Inc. (<https://fontawesome.com/license/free>) |
| Inno Setup Traditional Chinese messages | `windows/installer/Languages/ChineseTraditional.isl` | Inno Setup licence | Translation by Enfeng Tsao, based on Samuel Lee's 5.5.3+ work; from `jrsoftware/issrc` at tag `is-6_7_3`, where it is an unofficial translation under `Files/Languages/Unofficial/`. Compile-time input to the installer's message table only — the file itself is not shipped. |

`TaigiIcons.kt` carries its own Apache-2.0 header; the path data was extracted
verbatim from `androidx.compose.material:material-icons-extended` to drop the
transitive dependency.

The Font Awesome path data draws the About page's sponsor, website, GitHub, Discord
and email links on macOS and Windows, and Copy and Clear on the web page. All carry the attribution comment
Font Awesome ships in its SVGs, which is the credit CC BY 4.0 asks for; the desktop
paths were rewritten as absolute commands and scaled to a 16-unit height, and
no Font Awesome font file is bundled.

## Bundled data (other than the Taiwanese dictionary)

| Component | Path | Licence |
| --- | --- | --- |
| SymSpell `frequency_dictionary_en_82_765.txt`, filtered | `android/app/src/main/assets/english_freq.txt` | MIT © Wolf Garbe (<https://github.com/wolfgarbe/SymSpell>) |

Drives English-mode autocomplete on Android. Frequencies derive from Google
Books Ngram + SCOWL. Regenerate with `android/tools/build_english_freq.py`.

## Submodules and sibling repositories

| Component | Licence |
| --- | --- |
| `taigi-converter/` (git submodule) | MIT © 2026 TaigiKeyboard |
| `emoji/` | MIT © 2026 TaigiKeyboard |

`emoji/` embeds pinned snapshots of Unicode `emoji-test.txt` and CLDR
annotation XML under the **Unicode License** (OSI-approved as `Unicode-3.0`).
Provenance is recorded in `emoji/data/SOURCES.md`.

`taigi-converter` ships `jf-openhuninn-2.1.ttf` for its web demo; same OFL-1.1
terms as above.

## Rust dependencies

The engine, macOS and Windows workspaces admit only OSI-approved licences,
enforced in CI (`.github/workflows/security.yml`) by `cargo deny` against the
allowlist in `engine/deny.toml` and `windows/deny.toml`; the desktop and Linux
workspaces have no `deny.toml` and get `cargo audit` (advisories) only. The one
exception is `webpki-root-certs`, the Mozilla root certificates the updater's
HTTPS client trusts, which is data under CDLA-Permissive-2.0. The allowlist:

    MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause,
    BSD-3-Clause, ISC, Unicode-DFS-2016, Unicode-3.0, Zlib

Unknown registries and unknown git sources are denied. Reproduce the full
transitive inventory with:

```sh
cargo deny --manifest-path engine/Cargo.toml   list
cargo deny --manifest-path windows/Cargo.toml  list
```

Direct dependencies of note (engine and Windows workspaces; the Linux
GTK / libadwaita / zbus stack and other `desktop/` / `linux/` crate dependencies
are not yet inventoried here):

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

## Windows runtime (shipped in the Windows installer)

The settings window is WinUI 3, deployed self-contained: these files are installed
beside `TaigiKeyboardSettings.exe`, and listed one by one in
`windows/build-support/windows-app-runtime-files.txt`. The text service loads none
of them.

| Package | Version | Licence |
| --- | --- | --- |
| **Windows App Runtime** (`Microsoft.WindowsAppSDK.Runtime`) | 2.4.0 | **Closed source**, Microsoft Software License Terms, Windows App SDK |
| WebView2 projection (`Microsoft.Web.WebView2`, `Microsoft.Web.WebView2.Core.dll`) | 1.0.4078.44 | BSD-3-Clause © Microsoft Corporation |

Both are fetched from NuGet at build time, by version and digest
(`.github/workflows/windows-build.yml`), and are not vendored here.

## Apple platform dependencies (iOS and macOS)

| Package | Version | Licence |
| --- | --- | --- |
| **KeyboardKit** (iOS only) | 11.1.4 | **Closed source**, © 2016-2026 Kankoda Sweden AB — see below |
| swift-protobuf | 1.38.1 (iOS), 1.39.0 (macOS) | Apache-2.0 |
| KeyboardShortcuts (macOS only) | 3.1.0 | MIT © Sindre Sorhus |
| **LicenseKit** | 2.4.1 | **Closed source, commercial** — see below |

> **KeyboardKit is not open source.** Since 10.x it ships as a binary target
> under its own closed-source licence, free to use without its commercial Pro
> features. This project sets no Pro licence key. SwiftPM downloads the binary
> for **iOS** builds; it is not vendored here.
>
> **LicenseKit is not open source.** It reaches this project only as a
> transitive dependency of KeyboardKit on **iOS**, resolved by SwiftPM.
> It is not vendored here, is not used by any first-party code, and is absent
> from the macOS, Android, Windows and Linux builds — in particular it is absent
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
