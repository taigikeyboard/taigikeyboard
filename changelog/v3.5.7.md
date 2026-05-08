## v3.5.7

Cumulative release notes since v3.5.0. v3.5.1–v3.5.6 were internal Rust-extraction slices that shipped together with the v3.5.7 polish round, the Android UI Compose migration, and the new build / CI pipeline as a single user-facing release.

### Shared (iOS + Android)

#### New Features

- Two new ButTaiwan font options: **源樣明體** (GenYoMin, serif/ming) and **源樣烏體** (GenYoGothic, sans-serif/gothic). Both licensed under SIL Open Font License 1.1. (v3.5.0)
- End-to-end keystroke trace IDs (debug builds only) — every keystroke gets a trace ID propagated through the input pipeline for diagnostics. (v3.5.7)

#### Bug Fixes

- Restored POJ candidates for words containing `o͘` / `ⁿ` — about 21% of romanization queries had been missing matches. (v3.5.0)
- Next-word candidate now inserts the active romanization (POJ) instead of raw TL when in POJ mode. (v3.5.0)
- Candidates no longer remain on screen after backspacing past the composing buffer. (v3.5.0)
- Emoji key now commits the active candidate atomically before inserting the emoji (parity with every other character key). (v3.5.0)
- Icon rendering fixed on devices configured for non-Latin locales. (v3.5.0)
- Tab3 (Hanji 漢字) character range corrected — input classification slice (#202) fixes which Unicode ranges are treated as Hanji on lookup. (v3.5.7)
- Font display name corrected: **源樣黑體 → 源樣烏體** (matches official ButTaiwan name across font picker, copyright screen, and settings). (v3.5.7)

#### Refactoring — Phase IV — Rust shared core

Six cross-platform algorithm slices migrated from per-platform Swift / Kotlin into a shared Rust workspace under `engine/`, called from both platforms via `RustEngineBridge` over a protobuf envelope. Every slice is behavior-preserving and dogfood-verified.

- **v3.5.1** — Phonetics conversion (TL ↔ POJ ↔ TPS) fully on Rust. New `engine/phonetics/` crate; D9.4 cleanup moved `adjust_nasal_marker_case` + `nfd_preprocess` into Rust.
- **v3.5.2** — Candidate ranking pipeline migrated; phonetics-API derivation split + surface narrowing.
- **v3.5.3** — Engine workspace cleanup: duplicate FFI envelope removed; platform Rust mirrors deleted (path G — JVM-test compat retired).
- **v3.5.4** — Composing buffer engine migrated.
- **v3.5.5** — Next-word prediction engine migrated.
- **v3.5.6** — Lexicon read path migrated (prefix index + bundled binary record reader + bigram association lookup + multi-source search + Hanji-prefix search). New `engine/lexicon/` crate (~615 LOC, 11 modules, 53 new tests).
- **v3.5.7** — Three follow-up slices: case-transform helper, `EnabledDictionaries` bitmask, and lexicon input classification. Dead FFI surfaces dropped (`hasToneMarks`, `tpsToTl`); unused `SYLLABLE_RE`-based convert API removed.

By v3.5.7, all cross-platform algorithms run from the Rust workspace. Platform-side code remains for KeyboardKit / IME glue, settings injection, user-data SQLite (`user_association.db`, `user_frequency.db`, `custom_dictionary.db`), URL builders, and protobuf bridge wrappers.

#### Changes — On-disk dictionary formats

- Prefix index migrated **MARISA trie → fst** (`dictionary.fst`, 10 MB) — wire format `key_bytes (UTF-8) || 0xFF || rowid_le_4`, BurntSushi `fst` crate. (v3.5.6)
- Bundled records archive: **TKDB binary** (`dictionary.bin`, 5.1 MB) — `Header(16B) || Offset table || Records(bitmask u16 || frequency u32 || hanzi_len u8 || tl_len u8 || hanzi || tl)`. Replaces the legacy MARISA + SQLite trie pipeline. (v3.5.6)
- Bundled bigram mmap: `association.bin` (4.1 MB). (v3.5.6)

### iOS

#### Bug Fixes

- `isLkkDictEnabled` reset value corrected to `true` — `resetToDefaults()` was writing `false`, contradicting the documented default (LKK 漢羅文 mixed-script suggestions: on) and the cold-start fallback. (v3.5.7, #226)

#### Refactoring

- 11-phase project restructure into `Engine/` (KeyboardKit-free pure logic) and `KeyboardExtension/` (UIKit / KeyboardKit boundary). (v3.5.0)
- `CompositionRoot` introduced for service-graph DI; KeyboardKit-managed singletons stripped from `Phonetics/`, `Lexicon/`, `NextWord/`. (v3.5.0)
- `ComposingManager` and `NextWordController` split into engine / platform halves. (v3.5.0)
- Lexicon platform read-path source removed after Rust swap: `Database/{AssociationBinaryReader,DictionaryBinaryReader,DictionaryRepository}.swift`, `Trie/{InputNormalizer,TrieService,marisa_bridge.{cpp,h}}`, `InputNormalizerTests.swift` — 768 LOC. (v3.5.6)
- zh-TW human-readable annotations swept across keyboard core (Round A) and app / UI layer (Round B) — 157 files. (v3.5.7, #224 / #225)
- Lexicon `RustEngineBridge` access promotion: 3 helpers `internal` so the new `+Lexicon.swift` extension can share the diagnostics ring buffer + request-id sequence. (v3.5.6)

### Android

#### Bug Fixes

- Restored **armeabi-v7a** ABI to the app build — recovers ~3,806 older 32-bit ARM devices that lost keyboard support after the v3.5.x Rust migration. (v3.5.7)
- ProGuard / R8 keep rules added for Rust JNI + engine protobuf classes — fixes engine-init crash on release builds. (v3.5.7)
- `versionCode` scheme rebased to packed format `3_050_703` to clear Play Store monotonic floor. (v3.5.7)

#### Refactoring — UI architecture (Roadmap Item 4)

Android UI migrated to **Jetpack Compose** in four phases:

- **Phase A** — IME root wrapped in `ComposeView` host shell. (#228)
- **Phase B** — Smartbar candidate strip migrated to Compose. (#229)
- **Phase C** — Popup layer migrated. (#230)
- **Phase D** — Keyboard body migrated; **1 637 LOC** of legacy `View`-based code retired. (#231)

Pre-Compose extraction: immutable `KeyboardLayoutData` + pure `KeyboardLayoutSolver` factored out of the layout engine. (#227)

#### Refactoring — Module structure (Roadmap Item 1)

- Tab[1-4] → semantic naming (`HomeTab`, `SettingsTab`, `DictionaryTab`, `DiagnosticsTab`). (#212)
- `Common/` + `util/` god-bags broken apart by domain. (#213)
- One-off reports date-prefixed; `docs/` root orphans relocated. (#214 / #215)

#### Refactoring — Phase II A1-A10

- Service-graph DI via `CompositionRoot` + `TaigiKeyboardApplication` lifecycle owner; IME composition root no longer leaks across services. (v3.5.0)
- `EngineSettings` interface — engine code no longer reads `SharedPreferences` directly. (v3.5.0)
- `ComposingState` / `NextWordEngine` extracted from `TextInputManager`; `NextWordScorer` / `NextWordPredictor` extracted from `NextWordService`. (v3.5.0)
- Tab3 (Dictionary) and Tab4 (Diagnostic) migrated to dedicated ViewModels. (v3.5.0)
- `DictionaryError` reshaped into a sealed class; `Outcome<T, E>` adopted for repository return types. (v3.5.0)
- zh-TW human-readable annotations on engine bridges (Phase B per-method). (v3.5.7)

### Dictionary

- POJ entries regenerated to fix stale data; fail-fast pipeline guards added so future drift fails the build instead of silently shipping. (v3.5.7, #203)
- Build pipeline streamlined: per-source pipeline (`run.sh`) + aggregate merge / bin / fst / audit / deploy (`build.sh`) consolidated under `make dict`. (v3.5.7, #233)
- Build pipeline SQLite intermediates removed: `dictionary.db`, `trie.db`, `word_association.csv` (177 577 lines). (v3.5.6 part 2, #200)
- Dictionary build now shells exclusively to `engine/build-helpers/fst-builder` Rust binary — no PyPI MARISA / Python-C bindings. (v3.5.6)

### Build / Tooling

- **LICENSE** switched MIT → **CC BY-NC-SA 4.0** (non-commercial, share-alike). (v3.5.7)
- **Root `Makefile`** with `build` / `test` / `doc` / `dict` / `help` targets — single canonical entry point for Rust workspace + iOS xcframework + Android jniLibs + dictionary regeneration. (v3.5.7)
- **GitHub Actions CI** (`f017d353`, 2026-05-09): PR-time Rust + Android build, proto-drift check, markdown lint, actionlint — all on Linux runners. (v3.5.7)
- **Dependabot** configured for Cargo + GitHub Actions + Gradle minor / patch group. (v3.5.7)
- Markdown lint config (`.markdownlint.json`); MD031 / 032 / 034 / 037 / 040 relaxed to accept existing `rules/` style. (v3.5.7)
- iOS Swift formatter config (`.swiftformat`) committed. (v3.5.7)
- Android lint job tuned to fit a 4-minute budget: `lint { ignoreTestSources = true }` in `app/build.gradle.kts`, `lintDebug` only on CI. (v3.5.7)
- Root `.gradle-home/` gitignored; gradle wrapper jar committed so `./gradlew` runs in CI. (v3.5.7)
- Root `README.md` added; PR template + security workflow added. (v3.5.7)
- Dependency bumps via Dependabot:
  - `prost 0.12.6 → 0.14.3`, `prost-build 0.12.6 → 0.14.3` (engine)
  - `thiserror 1.0.69 → 2.0.18` (engine)
  - `dorny/paths-filter 3 → 4`, `actions/upload-artifact 4 → 7`, `actions/setup-java 4 → 5`
  - Gradle minor-and-patch group across 3 deps

### Documentation

- zh-TW human-readable annotations across all engine crates and platform engine bridges. (v3.5.7)
- Roadmap items documented: Item 1 (Android module refactor), Items 2-3 (librime spelling-algebra + continuous input — research), Item 4 (Android UI Compose migration plan). (v3.5.7)
- Migration inventory CSV (`docs/engine/migration-inventory.csv`) — canonical 13-column, 5-state status table for the Rust migration. (v3.5.6 era)
- Phase IV-B audit + plan: `docs/engine/lexicon-slice-{audit,plan}.md`. (v3.5.6)
- Engine binary format spec: `docs/engine/binary-format.md` (TKDB). (v3.5.6)
- Engine proto AS-IMPLEMENTED: `docs/engine/rust-core-proto.md`. (v3.5.6)
- FFI safety + Rust core proto draft: Phase II.5 Round C. (v3.5.1)
- Skills: `release-helper` (renamed from `update-changelog`, auto full-vs-incremental mode), `migration-residue` (audit), `shared-core-confidence` (audit).

### Removed

- iOS Lexicon platform read-path source (`Database/{AssociationBinaryReader,DictionaryBinaryReader,DictionaryRepository}.swift`, `Trie/{InputNormalizer,TrieService,marisa_bridge.{cpp,h}}`, `InputNormalizerTests.swift`) — 768 LOC.
- Android Lexicon platform read-path source + JVM unit tests (path G — delete platform mirrors).
- Dictionary build pipeline SQLite intermediates: `dictionary.db`, `trie.db`, `word_association.csv`.
- Dead FFI surfaces: `hasToneMarks`, `tpsToTl`.
- Unused `SYLLABLE_RE`-based phonetics convert API.
- Stale `GEMINI.md`, `archive/` folder, unused project-local skills, ktlint / swiftformat PostToolUse hooks.

### New Files

- `engine/Cargo.toml` workspace + new crates: `phonetics`, `composing`, `nextword`, `ranking`, `lexicon`, `dispatch`, `protos`, `mmap_host`, `swift-ffi`, `android-jni`, `build-helpers/fst-builder`.
- `engine/protos/proto/{envelope,phonetics,composing,nextword,ranking,lexicon,case_transform,enabled_dictionaries}.proto`.
- `Makefile`, `LICENSE`, `README.md`, `.markdownlint.json`, `.swiftformat`, `.github/{dependabot.yml,pull_request_template.md,workflows/{ci,security}.yml}`.
- `dictionary/output/{dictionary.fst,dictionary.bin,association.bin}` (replaces legacy MARISA / SQLite outputs).
- iOS: `Engine/RustEngine/RustTaigi.xcframework/` (replaces native C++ bridges).
- Android: `engine/{CaseTransformBridge,LexiconBridge,RustEngineBridge}.kt` + jniLibs.
