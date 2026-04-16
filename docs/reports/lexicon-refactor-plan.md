# Lexicon / Dictionary Refactor Plan

> **Type**: Plan
> **Created**: 2026-04-17
> **Branch**: `refactor-core-engine`
> **Scope**: `ios/Sources/TaigiKeyboard/Lexicon/` (25 files, ~3.6k LoC) + `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/` (18 files, ~5.2k LoC)
> **Goal**: Restructure for clarity, single-responsibility, and cross-platform alignment **without breaking runtime behavior**
> **Method**: Review-first (Codex + simplify) → fix all findings (incl. low priority) → implement → verify → commit
> **Constraint**: Per `feedback_review_before_impl` — every finding fixed before task is considered complete; no deferral

---

## 0. Executive Summary

**Verdict**: The Lexicon / dictionary modules are functionally correct and conservatively engineered, but suffer from **scope creep in 5 god-files**, **inconsistent package organization across platforms**, **duplicated tone / SQL / bitmask logic**, and **thin test coverage on persistence + orchestration layers**.

| Concern | iOS | Android |
|---|---|---|
| Total LoC | ~3.6k | ~5.2k |
| Files | 25 | 18 |
| Subfolders | 5 (Database/Models/Services/Trie/Utils) | 0 (flat) |
| Files >500 LoC | 3 | 5 |
| Untested critical files | ~10 | ~9 |
| Singleton coupling | Heavy | Heavy |

**Top three refactor levers** (cross-platform):
1. Split the four 500+ LoC services (`NextWordService`, `CustomDictionaryService` / `Repository`, `LexiconService`, Android `TPSConverter`)
2. Reorganize Android into subpackages mirroring iOS structure
3. Extract shared concerns (scoring, tone-stripping, SQL helpers, bitmask filter) into focused utilities

**Hard constraint**: 6 files are on the runtime hot path AND lack unit tests. Any change to them requires either (a) adding tests first, or (b) behaviour-equivalence verification by hand. See §6.

> **Plan revision history**
> - 2026-04-17 v0.1 — initial deep-read pass.
> - 2026-04-17 v0.2 — incorporated independent Codex review (32 findings); §0.1 below.

### 0.1 Critical risks added by Codex review

These are blockers that must be resolved within Stage 0, before any other work:

- **Asset freshness is gated on `BuildConfig.VERSION_CODE` only** (Android `LexiconService.copyAssetsIfNeeded` and `TrieService.getTriePath`). Any change to binary-format / bitmask / trie key prefixes during this refactor that does NOT bump `versionCode` will leave users running on stale assets. **Fix prerequisite**: add an `asset_build_ts` (or content-hash) check; trigger re-copy when the bundled asset's `build_ts` differs from the on-disk `dictionary_app_version.txt`.
- **`NextWordService` has an undocumented init-order dependency on `LexiconService`**: `NextWordService` reads `association.bin` from `filesDir`, but `copyAssetsIfNeeded()` lives inside `LexiconService.init()`. If `NextWordService.init()` runs first (or in parallel without sequencing), its reader is null. **Fix prerequisite**: make asset-copy ownership explicit (a single `AssetBootstrap` step that both services depend on) before any `NextWordService` split.
- **Build pipeline (`dictionary/build/`) was incorrectly listed as out-of-scope.** It is the source-of-truth for every cross-platform invariant in §4 of `binary-format.md`. **Fix prerequisite**: add a Stage 0 audit pass over the 11 build steps to confirm they match the documented formats.
- **Existing engine docs are out of date**: `nextword.md` and `trie.md` documented the wrong schema / wrong POJ→TL location; both have been corrected on this branch (2026-04-17). Treat docs as suspect until each one has been audited against current code.

---

## 1. Inventory

### 1.1 iOS Lexicon (`ios/Sources/TaigiKeyboard/Lexicon/`)

```
Database/
  SQLiteConnectionManager.swift       262
  AssociationBinaryReader.swift       211
  UserFrequencyRepository.swift       503  ⚠
  DictionaryBinaryReader.swift        188
  CustomDictionaryRepository.swift    546  ⚠
  DictionaryRepository.swift          221
Utils/
  TextProcessor.swift                 231  ← misnamed (CandidateProcessor)
  ResourceBundleResolver.swift         37
Models/
  LexiconConstants.swift               23
  TaigiWord.swift                      16
  EnabledDictionaries.swift            70
  DictionaryError.swift                28
  DictionarySearchResult.swift        116  ← model + URL/encoding logic
  InputType.swift                       6
  CustomDictionaryEntry.swift          24
Trie/
  InputNormalizer.swift                94
  RomanizationConverter.swift          17  ← thin facade, candidate for removal
  marisa_bridge.cpp / .h          176/63
  TrieService.swift                   159
Services/
  CustomDictionaryService.swift       256
  BackupService.swift                 174
  UserFrequencyService.swift           86
  NextWordService.swift               684  ⚠⚠
  LexiconService.swift                201
```

### 1.2 Android dictionary (`android/.../ime/dictionary/`)

```
(flat — no subpackages)
NextWordService.kt              824  ⚠⚠
TPSConverter.kt                 662  ⚠
LexiconService.kt               587  ⚠
CustomDictionaryService.kt      568  ⚠
ToneConverterModels.kt          481  ⚠ (lookup-table heavy)
TaigiPhonetics.kt               448
AssociationBinaryReader.kt      251
SuggestionCaseTransformer.kt    243
DictionaryBinaryReader.kt       216
TrieService.kt                  186
InputNormalizer.kt              162
BackupService.kt                142
DictionarySearchResult.kt       111
DictionaryModels.kt             104
ToneConverter.kt                103
ToneUtilities.kt                 95
EnabledDictionaries.kt           72
ToneRestoration.kt               44
```

> **Verification note for line refs in this document**: numbers below were extracted from a deep-read pass and should be re-verified at edit time (files may shift). Treat all `file:line` references as anchors to start reading from, not as authoritative.

---

## 2. iOS Findings (against the 8 criteria)

### 2.1 Best practices (Swift / KeyboardKit / SQLite / C++ bridge)
- **MED** `SQLiteConnectionManager.swift` — `@unchecked Sendable` + `NSLock` + `DispatchQueue` + `await MainActor.run` mix is fragile. Replace with `actor` model on refactor.
- **MED** `TrieService.swift` — double-check locking via `DispatchQueue.sync` is correct but verbose; `currentHandle` repeatedly hits the queue.
- **MED** `marisa_bridge.cpp` — `try { ... } catch (...)` swallows errors and returns `0` / `nullptr`; loses error info crossing the FFI boundary. Use sentinel codes.
- **LOW** `RomanizationConverter.swift` — pure facade over `TaigiPhonetics`; consider deletion.
- **LOW** `DictionaryBinaryReader.swift` — magic-byte literal `[0x54, 0x4B, 0x44, 0x42]` would read clearer as `"TKDB"`.

### 2.2 Redundancy
- **HIGH** `CustomDictionaryRepository.swift` — toned vs toneless search SQL templates duplicated; differ only in column name.
- **HIGH** `NextWordService.swift` constants — scoring weights / decay half-life duplicated in Android `NextWordService.kt`. Should be a documented cross-platform constant set.
- **HIGH** SQLite bind-text boilerplate (`sqlite3_bind_text(stmt, n, x, -1, SQLiteConnectionManager.sqliteTransient)`) repeats 40+ times across 3 files.
- **HIGH** Bitmask filter logic appears in `DictionaryBinaryReader.passesFilter()` and `AssociationBinaryReader.passesFilter()` with subtle variant/khiin differences; must drift-track manually.
- **MED** `DateFormatter` ("yyyy-MM-dd HH:mm:ss" UTC) defined separately in `UserFrequencyRepository` and `NextWordService`.
- **MED** CSV parsing in `CustomDictionaryService.parseCSV` / `parseCSVLine` is generic utility code mis-housed in a service.

### 2.3 Clean code (SRP)
- **HIGH** `NextWordService.swift` (684 LoC) — binary reader wrap + SQLite schema + migrations + scoring + pruning + import in one type. Split: `AssociationDictPredictor`, `UserAssociationStore`, `PredictionScorer`, `AssociationPruner`.
- **HIGH** `CustomDictionaryRepository.swift` (546) — CRUD + schema + capacity + batch import. Split out `CustomDictionarySchema` and `CustomDictionaryBatchImporter`.
- **HIGH** `UserFrequencyRepository.swift` (503) — frequency tracking + pruning + backup export + metadata; decay logic split between this file and `TextProcessor.calculateScore`.
- **HIGH** `LexiconService.search()` orchestrates 5 subsystems (custom search, system query, TPS expansion, capitalization, dedup, sort) inline.
- **HIGH** `CustomDictionaryService.swift` mixes CSV + file I/O + service CRUD + tone-stripping helpers (`generateNotone/Abbrev/RomanNum`).
- **MED** `DictionarySearchResult.swift` — model carrying URL builders + tone-digit conversion.
- **MED** `TextProcessor.swift` — file name says "TextProcessor", contents are `CandidateProcessor` (scoring + dedup + classification). Rename **and** split.

### 2.4 Over-engineering
- **MED** `TrieService` multi-instance support — only `.shared` is used.
- **MED** `SQLiteConnectionManager` flag/mode parameters — all callers pass identical values; sync vs async API duplicated.
- **MED** `EnabledDictionaries.allEnabled` — hardcoded conjunction of 10 booleans; brittle when a dictionary is added.
- **LOW** `NextWordService.Prediction.score: Double` — Int would suffice for ordinal sort.

### 2.5 Regression risk
- **HIGH** `SQLiteConnectionManager.initialize()` — concurrent calls may double-assign `initializationTask`; cross-thread mutation of `isInitialized` via `await MainActor.run`.
- **HIGH** `NextWordService` — `_recordCounter` / `_isUserTablesCreated` accessed both inside and outside `stateLock` inconsistently.
- **HIGH** `CustomDictionaryRepository.isTablesCreated` — guarded without lock; concurrent ensureInit may double-create tables.
- **HIGH** `UserFrequencyRepository.recordWord()` — `recordCounter += 1` unlocked, called from input thread.
- **HIGH** `DictionaryRepository.query()` — calls `EnabledDictionaries.fromSettings()` per query; settings I/O on the hot path.
- **MED** `UserFrequencyRepository.queryFrequencyDataBatch` — string-built `WHERE word IN (...)`; safe today via guard but fragile.
- **MED** `LexiconService.search()` — `userFrequencyService.isConnected()` vs `ensureInitialized()` ordering is implicit.
- **MED** `NextWordService.calculateDecay` — calls `Date().timeIntervalSince1970` per prediction; should accept `currentTime` parameter (as `TextProcessor.sortByScore` already does).

### 2.6 Readability & maintainability
- **HIGH** File name `TextProcessor.swift` does not match type `CandidateProcessor`.
- **HIGH** `LexiconService.search()` — overlapping local names (`input`, `rawInput`, `customSearchKey`, `searchPrefix`).
- **HIGH** `NextWordService.Constants` enum mixes scoring weights, pruning intervals, decay knobs.
- **MED** Hardcoded SQL column names ("roman", "hanzi", "count") scattered; no `enum Columns`.
- **MED** `DictionaryBinaryReader` offset math uses raw integer literals for record sizes; would benefit from `MemoryLayout` or a documented format struct.

### 2.7 AI-friendliness
- **HIGH** `NextWordService.swift` — 684 LoC mixing 6 concerns; impossible to refactor in isolation.
- **HIGH** `DictionarySearchResult` tone conversion — Unicode logic with cryptic comments ("Tone 1 (open) and 4 (checked) are omitted") needs domain knowledge or stronger doc-comments.
- **HIGH** `LexiconService.search()` — orchestration with no extracted seams; "add a new dictionary" requires reasoning over 5 paths.
- **MED** `marisa_bridge.cpp` — Chinese-only comments at the FFI boundary.
- **MED** `EnabledDictionaries` bit layout — comment "must match dictionary.bin" is an unenforced invariant.

### 2.8 Efficient review scope
- **HIGH** `CustomDictionaryRepository`, `NextWordService`, `TextProcessor` all force >200-line context loads to understand a single public method.

---

## 3. Android Findings (against the 8 criteria)

### 3.1 Best practices
- **GOOD** Sealed `DictionaryError`, coroutine-first I/O, schema versioning with backfill, mmap binary readers, strict UTF-8 decode.
- **MED** Inconsistent null handling — `?: throw` (LexiconService) vs optional chaining elsewhere.
- **MED** `CustomDictionaryService` — `generateNotone/Abbrev/RomanNum` results passed unchecked into SQL `arrayOf(...)`.
- **LOW** `DictionaryError.DatabaseConnectionFailed` drops the underlying exception cause.
- **LOW** `@Volatile` + `Mutex` together is redundant; pick one.

### 3.2 Redundancy
- **HIGH** Tone-mark extraction logic appears in `InputNormalizer`, `DictionarySearchResult`, and `TaigiPhonetics` — three slightly different implementations.
- **HIGH** Bitmask filter logic in `DictionaryBinaryReader.passesFilter` vs `AssociationBinaryReader.passesFilter` (mirror of iOS issue).
- **MED** SQL `INSERT ... ON CONFLICT` patterns duplicated between `NextWordService` and `CustomDictionaryService`.
- **MED** TL↔POJ final substitution table partially duplicated between `TaigiPhonetics` and `TPSConverter`.
- **LOW** `ToneConverterModels` keeps two near-identical maps (POJ vs TL) where one parameterized map would do — but extraction risk is non-trivial; defer.

### 3.3 Clean code (SRP)
- **HIGH** `NextWordService.kt` (824) — same shape as iOS counterpart; same split applies.
- **HIGH** `CustomDictionaryService.kt` (568) — CRUD + CSV + DatabaseHelper inner class (100+ LoC) + migrations.
- **HIGH** `LexiconService.kt` (587) — orchestration + custom merge + dedup + scoring + case transform + asset copy.
- **MED** `TPSConverter.kt` (662) — focused on TPS, but contains palatalization / nasals / boundary detection that could split.
- **MED** `TaigiPhonetics.kt` (448) — parser + assembler + POJ↔TL converter + tone placement; split into `SyllableParser` / `Assembler` / `ToneMarkPlacement`.
- **MED** `SuggestionCaseTransformer.transformText` — implicit dependency on letter counting; make explicit.

### 3.4 Over-engineering
- **MED** `ToneConverterModels` — 480+ LoC of lookup tables; consolidate later (high risk now).
- **LOW** `EnabledDictionaries.allAssociationSourcesEnabled()` — verify usage before deletion.
- **LOW** `TPSConverter.adjustTPSInitialKey()` context-sensitive boundary detection; check whether real keyboard input ever exercises it.
- **LOW** `TaigiPhonetics.placePOJToneMark` / `placeTLToneMark` — long but core; document via reference rather than rewrite.

### 3.5 Regression risk
- **HIGH** `NextWordService` schema migration v3→v4 drops & recreates table; assumes no concurrent writes (undocumented).
- **HIGH** Trie-prefix double-application risk: `InputNormalizer.buildSearchKey()` semantics vs `LexiconService.searchWithTrie()` line ~199 prepending again. Document and pick one place.
- **MED** `CustomDictionaryService` insert path — `notone/abbrev/romanNum` generation can throw; not wrapped.
- **MED** `InputNormalizer` tone-mark loop uses *last* match if multiple marks present — silent fallback for malformed input.
- **MED** Binary readers — magic + version check, no checksum; corruption surfaces as silent decode errors.

### 3.6 Readability & maintainability
- **HIGH** Flat package structure for 18 files / 5.2k LoC — no signal of which files are core vs leaf.
- **MED** Naming: `record()` (DictionaryBinaryReader fetch-by-id) vs `recordAssociation()` (write-log) — same verb, opposite meaning.
- **MED** Field abbreviations `notone` / `romanNum` / `abbrev` opaque to new readers.
- **LOW** `final_` underscore-suffix for Kotlin keyword avoidance is jarring; rename to `ending` / `coda`.
- **LOW** Magic-string trie prefixes (`"tl:"`, `"poj:"`, `"hanzi:"`) used both via `DictionaryConstants.triePrefix` and inline.

### 3.7 AI-friendliness
- **HIGH** `LexiconService.search()` — to reason about it, must read 7 other files.
- **HIGH** Phonetics knowledge spread across `TaigiPhonetics` / `TPSConverter` / `ToneConverterModels`; no single source-of-truth.
- **MED** `TaigiPhonetics.kt` opening comment "Ported from `references/taigi-converter/src/`" — verify whether `taigi-converter` submodule is still canonical (per `kesi deprecated` memory, status uncertain).
- **MED** Mixed Chinese/English comments inside `InputNormalizer.kt`.

### 3.8 Efficient review scope
- **HIGH** Same offenders as iOS — `NextWordService`, `CustomDictionaryService`, `LexiconService` each force loading 500+ LoC to understand any single public method.

---

## 4. Cross-Platform Alignment Audit

### 4.1 Structural divergence

| Concern | iOS | Android | Action |
|---|---|---|---|
| Folder structure | 5 subfolders | flat | Reorganize Android into `core` / `phonetics` / `models` / `persistence` / `storage` (or mirror iOS exactly) |
| Tone stack files | `TaigiPhonetics`, `RomanizationConverter`, `InputNormalizer`, `TextProcessor` (partial) | `TaigiPhonetics`, `ToneConverter`, `ToneRestoration`, `ToneUtilities`, `ToneConverterModels`, `TPSConverter`, `InputNormalizer`, `SuggestionCaseTransformer` | Decide canonical decomposition; align names |
| `CandidateProcessor` (iOS `TextProcessor`) | exists | merged into `LexiconService` | Either extract on Android or fold into iOS — pick one shape |
| `SuggestionCaseTransformer` | logic in `CandidateProcessor` (iOS) | dedicated file | Mirror on iOS |
| `RomanizationConverter` facade | exists | not present | Remove on iOS |
| `LexiconConstants` | dedicated | inside `DictionaryModels.kt` | Mirror on Android |

### 4.2 Behavioural alignment risk
- **Scoring weights / decay constants** (`USER_WEIGHT`, `DICT_WEIGHT`, `LEARNING_BONUS`, decay half-life) are duplicated in both `NextWordService`s. **Any single-side tweak silently diverges the platforms.** Treat as tracked cross-platform constant set; document in `docs/engine/nextword.md`.
- **Bitmask layout** for `EnabledDictionaries` — must match the dictionary build script. Both platforms assume the same layout but neither validates at startup.
- **Trie key format** (`tl:` / `poj:` / `hanzi:` prefixes) — both platforms hand-build keys; one mismatch = empty results.
- **Tone digit conversion** (`DictionarySearchResult` URL building) — implemented separately on both sides; high drift risk.

### 4.3 Cross-platform fixture corpus (Stage 4/5 prerequisite)

Per Codex finding 17/18: the existing `EngineIntegrationTests` (iOS) and `EngineIntegrationTest` (Android) only cover normalization / tone round-trip — neither asserts that `LexiconService.search()`, `NextWordService.predict()`, or backup import/export produce **identical** results across platforms.

Before Stage 4 / Stage 5 (per-platform service splits) can begin, both platforms must share a **fixture corpus** with parity assertions:

| Fixture | Platforms | Assertion |
|---|---|---|
| `search-corpus.json` | iOS + Android | identical TaigiWord lists for ~20 sample inputs across TL/POJ/TPS/Hanji |
| `nextword-corpus.json` | iOS + Android | identical prediction lists for ~10 prev_word seeds |
| `custom-dict-corpus.json` | iOS + Android | identical lookup results after seeded entries |
| `backup-roundtrip.json` | iOS + Android | round-trip equivalence for a fixed export blob |

Without this gate, Stage 4 (iOS split) followed by Stage 5 (Android split) creates a window where the two platforms behave subtly differently and divergence goes undetected.

### 4.3 Naming divergence (sample)
| iOS | Android | Note |
|---|---|---|
| `TextProcessor` (file) / `CandidateProcessor` (type) | folded into `LexiconService` | rename + split |
| `LexiconConstants` | `DictionaryModels.kt` constants | name + location |
| `SQLiteConnectionManager` | (no analogue, each service holds own SQLiteDatabase) | Android lacks central manager; consider adding |
| `RomanizationConverter` | (none) | drop on iOS |
| (none) | `SuggestionCaseTransformer` | add on iOS |
| (none) | `ToneUtilities` | iOS folds case logic into `TextProcessor` |

---

## 5. Test Coverage Baseline

### 5.1 iOS (`ios/TaigiKeyboardTests/`, ~11 files / 307 cases)
- **Covered**: `InputNormalizer`, `TaigiPhonetics`, `ToneConverter`, `ToneRestoration`, `ToneUtilities`, `TPSConverter`, `CustomDictionaryService` (CRUD), `DictionaryBinaryReader` (content-only), `TextProcessor` (partial).
- **Untested (critical)**: `LexiconService`, `TrieService`, `NextWordService`, `UserFrequencyService`, `BackupService`, `CustomDictionaryRepository`, `UserFrequencyRepository`, `SQLiteConnectionManager`, `ResourceBundleResolver`, **all `Models/`**.

### 5.2 Android (`android/app/src/test/.../dictionary/`, ~10 files / 239 cases)
- **Covered**: `InputNormalizer`, `TaigiPhonetics`, `ToneConverter`, `ToneRestoration`, `TPSConverter`, `ToneConverterModels`, `SuggestionCaseTransformer`, `CustomDictionaryService` (CRUD), `DictionaryCoverage` (binary read).
- **Untested (critical)**: `LexiconService`, `TrieService`, `NextWordService`, `BackupService`, `AssociationBinaryReader`, `DictionaryBinaryReader` (logic, not content), `TaigiWord`, `DictionarySearchResult`, `EnabledDictionaries`.

---

## 6. Regression Hot Zones (must touch carefully)

Files that are (a) on the runtime hot path AND (b) untested or thinly tested. **No refactor on these without first writing characterization tests OR doing manual round-trip behaviour verification.**

### iOS
1. `LexiconService.swift` — search entry point, called from `AutocompleteService`
2. `TrieService.swift` — runtime
3. `NextWordService.swift` — runtime, called from `NextWordController`
4. `DictionaryRepository.swift` — partial coverage only
5. `SQLiteConnectionManager.swift` — DB lifecycle, untested
6. `UserFrequencyService.swift` — runtime, untested

### Android
1. `LexiconService.kt` — called from `TaigiAutocompleteService`
2. `TrieService.kt` — runtime
3. `NextWordService.kt` — runtime, called from `NextWordHandler`
4. `DictionaryBinaryReader.kt` — content-only tests; logic untested
5. `BackupService.kt` — data migration risk, untested
6. `AssociationBinaryReader.kt` — runtime, untested

### External consumers to keep stable (API surface to NOT break)
- iOS: `AutocompleteService.swift:57` (LexiconService.shared), `NextWordController.swift`, `DictionarySearchViewModel.swift:16`, `TaigiKeyboardApp.swift:30` (CustomDictionaryService.seed), `CustomDictionaryView.swift`.
- Android: `TaigiAutocompleteService.kt:58` (LexiconService.search), `TaigiKeyboard.kt:29`, `NextWordHandler.kt`, `SmartbarManager.kt:14` (SuggestionCaseTransformer), `DataManagementActivity.kt`, `CustomDictionaryActivity.kt`, `EditEntryDialog.kt`.

---

## 7. Refactor Proposals (prioritized)

Each item is **review-first**: run Codex (`codex:rescue`) + `simplify` skill on the *plan* and *current code*, fix all surfaced findings, then implement, then re-run `simplify`.

### P0 — Foundational (do these before any larger split)

Per Codex findings 7, 15, 16: golden snapshots alone are insufficient. P0-A must cover four behaviour categories.

**P0-A. Characterization tests for hot zones — four categories.**
- **(a) Search/predict golden snapshots** — `LexiconService.search()`, `NextWordService.predict()`, `TrieService.lookup/prefixSearch`, `DictionaryRepository.query`. Same on both platforms.
- **(b) Init idempotence and ordering** — calling `init()` twice / concurrently must not double-create tables, double-copy assets, or race the connection manager.
- **(c) Migration paths** — for each schema version transition (user_association v0→v3→v4, custom_dictionary v1→v5), assert no data loss and correct UNIQUE constraints. iOS `SQLiteConnectionManager.initialize` is also in scope.
- **(d) Clock-injected ranking** — `PredictionScorer` and `CandidateProcessor.calculateScore` currently call `Date()` / `System.currentTimeMillis()` directly. Tests must inject `currentTime` to deterministically pin decay/recency behaviour. (This may require small surface-preserving edits to make `currentTime` an injected parameter; that edit is part of P0.)
- Risk: Medium (some refactor of clock APIs to make code testable). Effort: 6–10h per platform.

**P0-B. Document the bitmask & trie-key contracts.** ✅ partially done (2026-04-17)
- `docs/engine/binary-format.md` written.
- `docs/engine/trie.md` corrected (POJ→TL location).
- `docs/engine/nextword.md` corrected (schema v4).
- Still TODO: add startup assertion (debug only) that round-trips a sentinel key through `TrieService.lookup` + `DictionaryBinaryReader.record`.

**P0-C. Build-pipeline audit.** *(new — Codex finding 6)*
- Walk `dictionary/build/` 01–11 steps; confirm they emit the byte layouts and sort orders documented in `binary-format.md` §1, §2, §3, §4.
- Document the fixture-regen command (Codex finding 24) so phonetics-related stages can re-derive `DictionaryCoverageTest` data.
- Risk: Trivial (read-only audit). Effort: 2–3h.

**P0-D. Asset freshness fix.** *(new — Codex Critical 1)*
- Add `build_ts` (or content-hash) gate in addition to `versionCode` in Android `LexiconService.copyAssetsIfNeeded` and `TrieService.getTriePath`.
- Mirror the gate on iOS bundle resolution if there's an analogous staleness path.
- Risk: Medium (touches asset boot path). Effort: 3–4h. Required before any binary-format change ships.

**P0-E. Asset bootstrap ownership.** *(new — Codex Critical 2)*
- Extract a single `AssetBootstrap` (or equivalent) that owns asset-copy and is the explicit dependency of both `LexiconService` and `NextWordService` (both platforms).
- Eliminates the current "if NextWord initialized first, reader is null" race.
- Risk: Medium-High (rewires init order). Effort: 4–6h per platform. Required before P1-A (NextWord split).

**P0-F. Cross-platform fixture corpus.** *(new — Codex High 17/18)*
- Add `tests/fixtures/cross-platform/` with the four corpora listed in §4.3.
- Both platforms load and assert against the same JSON.
- Required gate for Stages 4 / 5 / 6.

### P1 — High-value structural refactors (per platform, mirrored)

**P1-A. Split `NextWordService` → 3 types.** (iOS 684, Android 824)

*(Revised per Codex Med 20: pruning shares DB/threshold/transaction context with `recordAssociation`, so it folds into `UserAssociationStore` instead of being its own type.)*

- `AssociationDictPredictor` — wraps `AssociationBinaryReader`, returns dict-side predictions.
- `UserAssociationStore` — schema, migrations, query, write, **and pruning** (shares same DB/transaction).
- `PredictionScorer` — `calculateUserScore`, `calculateDecay`; weights and clock as injected config (per P0-A category d).
- Public `NextWordService` becomes a thin coordinator.
- Risk: **High** — needs P0-A, P0-E, P0-F first.

**P1-B. Split `CustomDictionaryService` / `Repository`.**
- iOS: `CustomDictionarySchema`, `CustomDictionaryBatchImporter`, slim `Repository`, slim `Service`.
- Android: pull `DatabaseHelper` inner class out; extract `CsvDictionaryImporter`.
- Risk: Medium.

**P1-C. Split `LexiconService`.**

*(Revised per Codex High 13: Android `LexiconService` also owns asset copy + binary-reader init. Lifecycle/bootstrap must split out FIRST, separate from query orchestration.)*

- **Step 1** (both platforms): extract `LexiconBootstrap` — owns asset copy / binary reader init / dependency wiring. Public surface unchanged.
- **Step 2** (both platforms): extract `SearchOrchestrator.merge(custom, system)`, `TPSExpansion`, `CandidateRanker` (delegating to `CandidateProcessor`).
- Inline TPS / capitalization branches become testable units.
- Risk: Medium-High (orchestration code on hot path).

**P1-D. Rename and split `TextProcessor` (iOS).**
- Rename file → `CandidateProcessor.swift`.
- Split scoring → `ScoringCalculator`, classification → `TextClassifier`, base-stripping → reuse `ToneStripper` (see P2-C).
- Risk: Low-Medium.

### P2 — Shared utilities (eliminate duplication)

**P2-A. Extract bitmask `mask-building` primitives only — NOT a unified filter.**

*(Revised per Codex Med 23: dictionary filter has 3 layers (variant/khiin/source), association filter only has source-OR. They are deliberately different. A unified filter would erase real semantic differences.)*

- Share only: `bitForSource(DictionarySource) -> Int`, `enabledMask(EnabledDictionaries) -> Int`.
- Both `passesFilter` functions stay separate, but call the shared mask builder.
- Risk: Low.

**P2-B. Extract SQL bind / prepared-statement helper (iOS).**
- Reduce 40+ repetitions of `sqlite3_bind_text(stmt, n, x, -1, sqliteTransient)`.
- Risk: Low.

**P2-C. Share LOW-LEVEL diacritic stripping only — do NOT unify tone-mark extraction.**

*(Revised per Codex High 14: `InputNormalizer.normalizeSyllable`, `DictionarySearchResult.toTLDigit`, and `TaigiPhonetics.stripToneMark` look similar but have different semantics — input normalization vs URL-key encoding vs assembler reverse-pass. A unified extractor would conflate them.)*

- Share only: `stripCombiningDiacritics(String) -> String` (Unicode NFD → drop combining marks → NFC). This is platform-Unicode-trivial and identical-purpose across all three call sites.
- Each caller continues to own the higher-level "what to do with the stripped form" logic (assign default tone vs append digit vs build URL).
- Risk: Low.

**P2-D. Centralize date formatter and scoring constants.**
- iOS: move `DateFormatter` to `LexiconConstants`; same for any scoring weights.
- Android: same; document cross-platform invariant.
- Risk: Trivial.

### P3 — Structural alignment

**P3-A. Reorganize Android into idiomatic Kotlin subpackages.**

*(Revised per Codex Med 22 / Low 32: do NOT mirror iOS `Database/Models/Services/Trie/Utils` PascalCase. Existing repo uses lowercase functional packages like `ime/core`, `ime/text/smartbar`. Stay consistent.)*

- Suggested lowercase subpackages: `dictionary.core` (Lexicon, NextWord, Backup), `dictionary.phonetics` (TaigiPhonetics, ToneConverter, ToneRestoration, ToneUtilities, TPSConverter, InputNormalizer, SuggestionCaseTransformer), `dictionary.models`, `dictionary.persistence` (CustomDictionary, repositories), `dictionary.storage` (binary readers, TrieService).
- Pure file moves + import updates. No DI framework or reflection-package-scanning to worry about.
- Risk: Low (compile-time churn only).

**P3-B. Extract iOS `SuggestionCaseTransformer` / add Android `LexiconConstants`.**

*(Promoted per Codex Med 28: iOS case-transform logic is currently mixed into `TextProcessor` / `CandidateProcessor`. Android already has a dedicated `SuggestionCaseTransformer`. Pulling iOS's case logic into a sibling module makes the cross-platform shape match — and is also a prerequisite for the iOS `TextProcessor` rename / split in P1-D.)*

- iOS: extract case transformation from `TextProcessor` into `SuggestionCaseTransformer.swift`.
- Android: extract `LexiconConstants` from `DictionaryModels.kt` for parity.
- Risk: Low.

**P3-C. Drop iOS `RomanizationConverter` facade.**
- Risk: Trivial.

### P4 — Smaller cleanups (must still be done — no deferral)

- iOS `TrieService` — drop unused multi-instance API.
- iOS `SQLiteConnectionManager` — collapse sync vs async API to async-only; drop unused flag parameters. **Defer until after iOS service splits** (Codex High 3): `LexiconService.search()` and `DictionarySearchViewModel` still call sync APIs on the hot path; remove sync callers first.
- iOS `EnabledDictionaries.allEnabled` — derive from a single source list.
- Android `EnabledDictionaries.allAssociationSourcesEnabled()` — verify usage; remove if dead.
- Android `final_` naming — rename to `ending` or `coda`.
- ~~Magic-byte literal in `DictionaryBinaryReader`~~ — **already commented inline** as `// "TKDB"` (Codex Low 31). Skip.
- ~~iOS `RomanizationConverter` facade removal~~ — 2 pass-through methods, stateless (Codex Low 30); inline opportunistically, not a stage slot.
- Translate Chinese-only comments in `marisa_bridge.cpp` and `InputNormalizer.kt` to English (per CLAUDE.md "documentation in English").

---

## 8. Staged Execution Plan

> Staging groups items so that each stage ends with a green build and passing tests. Each stage = one PR off `refactor-core-engine`.

| Stage | Scope | Items | Stop-line |
|---|---|---|---|
| **0** | Baseline | P0-A (4 categories), P0-B, P0-C, P0-D, P0-E, P0-F; create `IMPLEMENTATION_PLAN.md` | All hot-zone characterization tests green (incl. init/migration/concurrency/clock); engine docs accurate; build pipeline audited; asset freshness fixed; AssetBootstrap extracted; cross-platform fixture corpus committed |
| **1** | Symmetry & cleanup | P3-B (incl. iOS case-transform extraction), P4 (most — exclude iOS sync→async collapse) | Inventory aligned; no dead code; no Chinese comments outside data files |
| **2** | Shared utilities (narrow scopes) | P2-A (mask primitives only), P2-B, P2-C (diacritic stripping only), P2-D | Targeted duplication eliminated; tests still green |
| **3** | Android reorg | P3-A (lowercase packages) | Subpackages introduced; everything compiles |
| **4** | iOS service splits | P1-D, P1-B (iOS), P1-C (iOS Step 1 lifecycle then Step 2 query) | Cross-platform fixture corpus passes both platforms before & after |
| **5** | Android service splits | P1-B (Android), P1-C (Android Step 1 lifecycle then Step 2 query) | Cross-platform fixture corpus passes both platforms before & after |
| **5b** | iOS sync→async collapse (deferred from P4) | iOS `SQLiteConnectionManager` async-only, `LexiconService.search` async path | Hot path no longer has sync DB access |
| **6** | NextWord split (highest risk last) | P1-A (3-way split, iOS then Android) | Cross-platform fixture corpus passes both platforms before & after |

**Per-stage workflow** (per `feedback_review_before_impl`):
1. Write stage-specific TODO list inside `IMPLEMENTATION_PLAN.md`.
2. Codex review of *plan + relevant current code*.
3. Run `simplify` skill on relevant files (read-only mode) → list findings.
4. **Fix every finding (incl. low priority).**
5. Implement.
6. Re-run `simplify` to verify.
7. Manual build + test (user runs Xcode + gradle per `feedback_manual_build_test`).
8. Commit per logical scope, concise messages.
9. PR open against `develop` (per `feedback_branching`).

---

## 9. Open Questions for User

Before starting Stage 0:

1. **Order of platforms**: iOS-first all stages, or interleave per-stage iOS+Android? (Plan above interleaves; favours symmetry.)
2. **Folder reorg on Android (P3-A)**: confirmed lowercase functional packages (`dictionary.core / phonetics / models / persistence / storage`) per Codex Med 22 — repo convention is lowercase. ✅ resolved.
3. **`taigi-converter` submodule**: per `project_kesi_deprecated` memory, status is uncertain. Should `TaigiPhonetics` reduction defer to a fixed/migrated submodule, or proceed independently?
4. **Scoring constants**: are `USER_WEIGHT` / `DICT_WEIGHT` / `LEARNING_BONUS` / decay half-life intentionally tunable per-platform, or should they be locked to a shared spec? If locked, where does the spec live?
5. ~~**Test target on iOS**~~: ✅ resolved (Codex Low 29) — iOS uses `PBXFileSystemSynchronizedRootGroup`; new test files are auto-tracked. No manual Xcode action needed.
6. **Behaviour-equivalence bar**: for hot zones without tests (e.g. `BackupService`), the bar is **characterization tests required** per P0-A; backup specifically is now in scope (Codex Med 25). Confirm this raises the Stage 0 effort estimate appropriately.
7. **Build pipeline edits in this branch**: Codex finding 6 says build pipeline (`dictionary/build/`) is part of the source-of-truth surface. Are build-script edits ALLOWED in this branch (e.g. to add `build_ts` consistency checks), or must any pipeline change be a separate branch / approval gate?
8. **Asset bootstrap ownership (P0-E)**: which platform should lead — iOS first (smaller diff) or Android (where the version-code-only gate bug lives today)?

---

## 10. Out of Scope (for this branch)

- Switching SQLite to SQLite.swift / Room — too large; orthogonal.
- Replacing `marisa-trie` with another trie library.
- ~~Re-deriving binary file formats; build-script changes.~~ — **revised per Codex 6/24**: build-pipeline AUDIT (P0-C) and limited consistency edits (e.g. `build_ts` checks) are IN scope when they unblock asset-freshness or fixture-regen. Format changes still out of scope.
- Localization of error messages.
- UI-side changes (`AutocompleteService`, `SmartbarManager` etc.) — only stable API consumers; don't touch unless required to keep them building.

---

## 11. Done Definition

This refactor is complete when:
- Every P0–P4 item is closed (no skipped low-priority items).
- All existing tests still pass on both platforms.
- New characterization tests for hot zones exist and pass — covering all 4 P0-A categories (golden snapshot, init idempotence, migration, clock-injected ranking).
- Cross-platform fixture corpus (§4.3) passes on both platforms with identical output assertions.
- `docs/engine/` reflects new module names and binary-format contracts; build-pipeline doc lists each step's responsibility.
- A final cross-platform diff (iOS Lexicon vs Android dictionary) shows symmetric file inventory and matching public-API names.
- `simplify` skill returns no findings on the touched files.
- Asset freshness (P0-D) verified by toggling a `build_ts` and confirming app picks up new assets without `versionCode` bump.
- Manual smoke test of: TL/POJ/TPS input, custom dictionary CRUD, next-word prediction, backup/restore — passes on both platforms.
