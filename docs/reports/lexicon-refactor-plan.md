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

**Top two refactor levers** (revised v0.3 per Codex audit — cross-platform alignment is no longer a top lever):
1. **Behaviour preservation**: characterization tests on hot zones, asset freshness fix, init-order race fix.
2. **Local clarity**: split the 500+ LoC services into focused responsibilities — but prefer same-file `private` types/functions over new files unless the new home materially reduces review scope.

Cross-platform symmetry, package reorganization, and shared utility extraction are pursued only when they solve a concrete maintenance problem — not as ends in themselves.

**Hard constraint**: 6 files are on the runtime hot path AND lack unit tests. Any change to them requires either (a) adding tests first, or (b) behaviour-equivalence verification by hand. See §6.

> **Plan revision history**
> - 2026-04-17 v0.1 — initial deep-read pass.
> - 2026-04-17 v0.2 — incorporated independent Codex review (32 findings); §0.1 below.
> - 2026-04-17 v0.3 — incorporated Codex over-engineering audit (30 findings); §0.2 below. **Net effect**: ~50% fewer new files, ~50% fewer new abstractions, stage count 7 → 5, "no skipped low-priority" rule retired.

---

## Active Stage Tracker

**Current stage**: Stage 0 (Baseline) — in progress.

| Stage 0 item | Status | Notes |
|---|---|---|
| Plan v0.3 (Codex over-engineering audit applied) | ✅ done | This document |
| Engine doc fixes (`trie.md`, `nextword.md`, `binary-format.md`) | ✅ done | Committed in PR #129 |
| P0-A characterization tests — Android (narrowed scope per v0.3) | ⏳ pending | Hot-zone files only; idempotence only at known concurrency hotspots |
| P0-A characterization tests — iOS | ⏳ pending | Same scope |
| P0-C build-pipeline audit (read-only) | ⏳ pending | Awaits Q7 confirmation |
| P0-D asset freshness fix (`build_ts` gate) | ⏳ pending | Awaits Q7 + Q8 |
| P0-E `LexiconService.ensureAssetsCopied()` static method + explicit call from `NextWordService.init()` | ⏳ pending | Replaces v0.2's `AssetBootstrap` class |
| P0-F cross-platform fixture corpus (search + nextword only) | ⏳ pending | 2 corpora, not 4 |

**Outstanding open questions** (block Stage 0 completion): Q3, Q4, Q7, Q8 — see §9.

When Stage 0 closes: update this table to "✅ Stage 0 complete", change "Current stage" to Stage 1, and start a Stage 1 sub-table.

---

### 0.1 Critical risks added by Codex review

These are blockers that must be resolved within Stage 0, before any other work:

- **Asset freshness is gated on `BuildConfig.VERSION_CODE` only** (Android `LexiconService.copyAssetsIfNeeded` and `TrieService.getTriePath`). Any change to binary-format / bitmask / trie key prefixes during this refactor that does NOT bump `versionCode` will leave users running on stale assets. **Fix prerequisite**: add an `asset_build_ts` (or content-hash) check; trigger re-copy when the bundled asset's `build_ts` differs from the on-disk `dictionary_app_version.txt`.
- **`NextWordService` has an undocumented init-order dependency on `LexiconService`**: `NextWordService` reads `association.bin` from `filesDir`, but `copyAssetsIfNeeded()` lives inside `LexiconService.init()`. If `NextWordService.init()` runs first (or in parallel without sequencing), its reader is null. **Fix prerequisite**: make asset-copy ownership explicit (a single `AssetBootstrap` step that both services depend on) before any `NextWordService` split.
- **Build pipeline (`dictionary/build/`) was incorrectly listed as out-of-scope.** It is the source-of-truth for every cross-platform invariant in §4 of `binary-format.md`. **Fix prerequisite**: add a Stage 0 audit pass over the 11 build steps to confirm they match the documented formats.
- **Existing engine docs are out of date**: `nextword.md` and `trie.md` documented the wrong schema / wrong POJ→TL location; both have been corrected on this branch (2026-04-17). Treat docs as suspect until each one has been audited against current code.

### 0.2 v0.3 calibration shift (Codex over-engineering audit)

A second Codex pass — narrowly targeted at "is this overshooting?" — surfaced 30 findings: 9 DROP, 13 SIMPLIFY, 1 KEEP, 7 already aligned. Headlines:

**Dropped from this branch (cargo-cult / noun inflation / process scaffolding):**
- `AssetBootstrap` and `LexiconBootstrap` as separate classes — replaced by `LexiconService.ensureAssetsCopied()` static method that `NextWordService.init()` calls explicitly. Solves the same race without a new noun.
- `BitmaskFilter` shared primitives — the two `passesFilter` paths are deliberately different; cross-reference comment cheaper than abstraction.
- iOS `SuggestionCaseTransformer` extraction "for parity with Android" — file-shape symmetry is not a maintenance win.
- Android `LexiconConstants` extraction "for parity with iOS" — same critique.
- Android `SQLiteConnectionManager` "for parity with iOS" — speculative; no demonstrated Android problem.
- `IMPLEMENTATION_PLAN.md` as a separate file — duplicates §8 / §9; folded into "Active Stage" pointer at top.
- Startup sentinel assertion (P0-B) — duplicates fixture tests.
- "Symmetric file inventory" as a Done-criterion — symmetry-for-symmetry, not product quality.
- "No skipped low-priority items" rule — actively *anti-prioritization*; retains abstractions that should be cut. Replaced by "low-priority items may defer to follow-up branch with explicit rationale".

**Simplified scopes:**
- `SearchOrchestrator` / `CandidateRanker` / `TPSExpansion` → become `private func` inside `LexiconService` instead of new types.
- iOS `TextProcessor` split → just rename to `CandidateProcessor.swift`; do NOT further split into `ScoringCalculator` + `TextClassifier`.
- `CustomDictionarySchema` / `CustomDictionaryBatchImporter` / `CsvDictionaryImporter` → start as same-file `private` sections; only extract to new files if any one stays >150 LoC after isolation.
- iOS SQL bind helper → file-local extension, not cross-file utility.
- Cross-platform fixture corpus → narrow to **search + nextword** only (not all four corpora upfront).
- Init-idempotence test matrix → only services with known concurrency hotspots (per §2.5 / §3.5), not every service.
- Clock injection → only decay-sensitive paths, not all of `PredictionScorer` + `CandidateProcessor`.
- Per-stage Codex+simplify ritual → once per stage at the boundary, not per sub-item.
- Android subpackage reorg → only move files whose new home materially reduces review scope; defer broad moves until after service splits land.
- Cross-platform alignment audit (§4) demoted from "top-3 lever" to "secondary, opportunistic".

**KEPT (Codex agreed):**
- `ToneStripper.stripCombiningDiacritics()` (P2-C narrowed) — three callers do identical NFD/drop-combining/NFC; copy-paste fragility cost > tiny abstraction cost.
- All P0 baseline items (asset freshness, characterization tests, build-pipeline audit, fixture corpus) — but with narrowed breadth as listed above.
- NextWord 3-way split target — but execute as same-file private types first; only extract to separate files when each section stays substantial after isolation.

**Net effect:**
- Stage count: 7 → 5 (collapse Stage 1 cleanup with Stage 3 reorg into a single "symmetry & cleanup" pass; merge 5b back into Stage 5).
- New files projected: ~25 → ~12.
- New types/abstractions projected: ~30 → ~15.
- Process overhead per stage cut roughly in half.

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

**P0-A. Characterization tests for hot zones — narrowed scopes (v0.3).**
- **(a) Search/predict golden snapshots** — `LexiconService.search()`, `NextWordService.predict()`, `TrieService.lookup/prefixSearch`, `DictionaryRepository.query`. Same on both platforms.
- **(b) Init idempotence — only known-concurrency-hotspot services** (per §2.5 / §3.5): `SQLiteConnectionManager.initialize`, `NextWordService` `_recordCounter` / `_isUserTablesCreated`, `CustomDictionaryRepository.isTablesCreated`. NOT every service blanket-tested.
- **(c) Migration paths** — for each schema version transition (user_association v0→v3→v4, custom_dictionary v1→v5), assert no data loss and correct UNIQUE constraints.
- **(d) Clock-injected ranking — only decay-sensitive paths.** `PredictionScorer.calculateDecay` and the recency-decay branch of `CandidateProcessor.calculateScore`. Other scoring branches (user-freq cap, length, exact match) stay clock-free; do NOT widen the whole API just to pass `currentTime`.
- Risk: Medium (small surface-preserving edits to inject clock at the two decay sites). Effort: 4–6h per platform (down from 6–10h in v0.2).

**P0-B. Document the bitmask & trie-key contracts.** ✅ done (2026-04-17)
- `docs/engine/binary-format.md` written.
- `docs/engine/trie.md` corrected (POJ→TL location).
- `docs/engine/nextword.md` corrected (schema v4).
- ~~Startup sentinel assertion~~ — **dropped in v0.3** (Codex audit: duplicates fixture tests, adds another startup path to maintain).

**P0-C. Build-pipeline audit.** *(new — Codex finding 6)*
- Walk `dictionary/build/` 01–11 steps; confirm they emit the byte layouts and sort orders documented in `binary-format.md` §1, §2, §3, §4.
- Document the fixture-regen command (Codex finding 24) so phonetics-related stages can re-derive `DictionaryCoverageTest` data.
- Risk: Trivial (read-only audit). Effort: 2–3h.

**P0-D. Asset freshness fix.** *(new — Codex Critical 1)*
- Add `build_ts` (or content-hash) gate in addition to `versionCode` in Android `LexiconService.copyAssetsIfNeeded` and `TrieService.getTriePath`.
- Mirror the gate on iOS bundle resolution if there's an analogous staleness path.
- Risk: Medium (touches asset boot path). Effort: 3–4h. Required before any binary-format change ships.

**P0-E. Make asset-copy ownership explicit (no new class).** *(v0.3 simplification)*
- Expose `LexiconService.ensureAssetsCopied(context)` as a `static`/class method.
- `NextWordService.init()` calls it explicitly at the top of its init path.
- No new `AssetBootstrap` / `LexiconBootstrap` types — the race is solved by making the dependency explicit, not by adding a noun.
- Risk: Low (one method extraction + one explicit call). Effort: 1–2h per platform. Required before P1-A.

**P0-F. Cross-platform fixture corpus — narrowed to 2 corpora (v0.3).**
- Add `tests/fixtures/cross-platform/`:
  - `search-corpus.json` — ~15 inputs across TL/POJ/TPS/Hanji.
  - `nextword-corpus.json` — ~10 prev_word seeds.
- ~~`custom-dict-corpus.json`~~ and ~~`backup-roundtrip.json`~~ deferred — neither is on the runtime hot path; add later only if Stages 4–5 surface drift in those areas.
- Both platforms load and assert against the same JSON.
- Required gate for Stages 4 / 5.

### P1 — High-value structural refactors (per platform, mirrored)

**P1-A. Split `NextWordService` — same-file types first, separate files only when justified (v0.3).** (iOS 684, Android 824)

*(Revised per Codex audit item 11: avoid turning `NextWordService` into a thin shell. Start by isolating responsibilities as `private` types/sections within the same file, then only extract to separate files when a section remains substantial after isolation.)*

Step 1 (in-file isolation):
- `private struct/class AssociationDictPredictor` — wraps `AssociationBinaryReader`, returns dict-side predictions.
- `private struct/class UserAssociationStore` — schema, migrations, query, write, pruning (shares same DB/transaction; pruning folded in per Codex Med 20).
- `private enum/class PredictionScorer` — `calculateUserScore`, `calculateDecay`; clock injected only at the decay site.
- Public `NextWordService` orchestrates; remains the only file initially.

Step 2 (only if needed): extract any private type to its own file when it stays >200 LoC after isolation. Goal is fewer god-files; **not** maximizing file count.

- Risk: High — needs P0-A, P0-E, P0-F first.

**P1-B. Slim `CustomDictionaryService` / `Repository` — same-file isolation first (v0.3).**
- iOS: isolate schema methods as `private` section in `CustomDictionaryRepository.swift`; isolate batch-import logic as a `private` section. Only extract to `CustomDictionarySchema.swift` / `CustomDictionaryBatchImporter.swift` if either stays >150 LoC after isolation.
- Android: same approach — keep `DatabaseHelper` inside the file but as a `private inner class`; isolate CSV parsing as `private` section. Only extract to separate files if the size justifies.
- Risk: Medium.

**P1-C. Slim `LexiconService` — private functions, no new types (v0.3).**

*(Revised per Codex audit items 4, 7, 9, 10: do NOT extract `LexiconBootstrap` / `SearchOrchestrator` / `CandidateRanker` / `TPSExpansion` as new classes/types. Each becomes a `private func` inside `LexiconService` with the same single-responsibility benefit and zero new files.)*

- Lifecycle: `private func ensureAssetsCopied()` (already factored out per P0-E), `private func ensureReadersLoaded()`, `private func ensureUserConnections()`.
- Search orchestration: `private func mergeCustomAndSystem(...)`, `private func expandTPS(...)`, `private func rankCandidates(...)` (delegates to `CandidateProcessor`).
- Inline TPS / capitalization branches become callable units (testable when `LexiconService` exposes a focused public API).
- Risk: Medium (orchestration code on hot path; private-method extraction is lower risk than new-type extraction).

**P1-D. Rename `TextProcessor` → `CandidateProcessor` (iOS) — no further split (v0.3).**

*(Revised per Codex audit item 14: the rename captures most of the value at 231 LoC; further splitting into `ScoringCalculator` + `TextClassifier` is premature SRP purity for a file this size.)*

- Rename file → `CandidateProcessor.swift`. Type was already `CandidateProcessor` — only the file name was misaligned.
- Reuse `ToneStripper.stripCombiningDiacritics` for the `romanToBase` / `inputToBase` Unicode work (see P2-C).
- Risk: Low (pure rename + Unicode helper swap).

### P2 — Shared utilities (eliminate duplication)

**P2-A. ~~Extract bitmask primitives~~ — DROPPED in v0.3.**

*(Codex audit item 2: extracting only `bitForSource` / `enabledMask` for two ~10-line callers is still abstraction tax. Leave both `passesFilter` functions duplicated, add cross-reference comment in each header.)*

Action: add `// MUST stay in sync with AssociationBinaryReader.passesFilter` to `DictionaryBinaryReader.passesFilter` (and vice versa) on both platforms. No code extraction.

**P2-B. SQL bind helper as file-local extension (iOS) — narrowed (v0.3).**

*(Codex audit item 15: a cross-file SQLite bind utility is too much abstraction. Prefer a file-local helper or extension placed near the repositories that need it.)*

- Add `private extension OpaquePointer { func bindText(_ index: Int32, _ value: String) }` (or similar) inside each repository file that needs it.
- No shared cross-file `SQLitePreparer` class.
- Risk: Low.

**P2-C. Share LOW-LEVEL diacritic stripping only — do NOT unify tone-mark extraction.**

*(Revised per Codex High 14: `InputNormalizer.normalizeSyllable`, `DictionarySearchResult.toTLDigit`, and `TaigiPhonetics.stripToneMark` look similar but have different semantics — input normalization vs URL-key encoding vs assembler reverse-pass. A unified extractor would conflate them.)*

- Share only: `stripCombiningDiacritics(String) -> String` (Unicode NFD → drop combining marks → NFC). This is platform-Unicode-trivial and identical-purpose across all three call sites.
- Each caller continues to own the higher-level "what to do with the stripped form" logic (assign default tone vs append digit vs build URL).
- Risk: Low.

**P2-D. Date formatter + scoring constants — minimal centralization (v0.3).**

*(Codex audit item 16: avoid documentation process around a handful of numbers. Constants stay close to the scorer; one cross-reference comment is enough where drift risk is real.)*

- iOS: move duplicated `DateFormatter` to `LexiconConstants` (one constant, two callers).
- Scoring weights stay where they're used (inside `PredictionScorer` / `CandidateProcessor`). Add one comment block at each platform's scorer that lists the values and links to the other platform's file. No central spec doc.
- Risk: Trivial.

### P3 — Structural alignment (heavily reduced in v0.3)

**P3-A. Android subpackage reorg — selective, AFTER service splits land (v0.3).**

*(Revised per Codex audit items 18, 30: broad package churn in a legacy refactor adds rename noise + merge pain without reducing risk. Defer until after Stage 4/5 service splits are in.)*

- Move only files where the new home **materially reduces review scope** for a subsequent change.
- Suggested initial moves: phonetics group only (`TaigiPhonetics`, `ToneConverter`, `ToneRestoration`, `ToneUtilities`, `TPSConverter`, `InputNormalizer`) → `dictionary.phonetics`. Everything else stays put.
- No five-subpackage rewrite up-front.
- Risk: Low (compile-time churn only).

**P3-B. ~~iOS `SuggestionCaseTransformer` / Android `LexiconConstants`~~ — DROPPED in v0.3.**

*(Codex audit items 5, 17: both items justified primarily on cross-platform parity, which is cargo-cult absent a concrete maintenance problem. iOS case logic stays inside `CandidateProcessor`; Android constants stay inside `DictionaryModels.kt`.)*

**P3-C. Drop iOS `RomanizationConverter` facade.**
- Risk: Trivial.

### P4 — Smaller cleanups (priority-aware in v0.3 — low items may defer)

*(Revised per Codex audit item 26: the v0.2 "no skipped low-priority items" rule is anti-prioritization and tends to retain abstractions that should be cut. v0.3 rule: low-priority items may be deferred to a follow-up branch with explicit rationale logged in the commit.)*

- iOS `TrieService` — drop unused multi-instance API.
- iOS `SQLiteConnectionManager` — collapse sync vs async API to async-only; drop unused flag parameters. **Deferred until after iOS service splits** (Codex v0.2 High 3): `LexiconService.search()` and `DictionarySearchViewModel` still call sync APIs on the hot path; remove sync callers first.
- iOS `EnabledDictionaries.allEnabled` — derive from a single source list.
- Android `EnabledDictionaries.allAssociationSourcesEnabled()` — verify usage; remove if dead.
- Android `final_` naming — rename to `ending` or `coda`.
- ~~Magic-byte literal in `DictionaryBinaryReader`~~ — already commented inline; skip (Codex v0.2 Low 31).
- ~~iOS `RomanizationConverter` facade removal~~ — inline opportunistically, not a stage slot (Codex v0.2 Low 30).
- Translate Chinese-only comments in `marisa_bridge.cpp` and `InputNormalizer.kt` to English (per CLAUDE.md "documentation in English").

---

## 8. Staged Execution Plan

> Staging groups items so that each stage ends with a green build and passing tests. Each stage = one PR off `refactor-core-engine`.

**v0.3 staging — collapsed from 7 stages to 5.** Active stage tracker lives at the top of this document (no separate `IMPLEMENTATION_PLAN.md`).

| Stage | Scope | Items | Stop-line |
|---|---|---|---|
| **0** | Baseline | P0-A (narrowed: 2 services for idempotence, 2 sites for clock injection), P0-B (✅ done), P0-C, P0-D, P0-E (static method, not new class), P0-F (2 corpora) | Hot-zone characterization tests green; engine docs accurate; build pipeline audited; asset freshness fixed; explicit `ensureAssetsCopied()` call from `NextWordService.init()`; search + nextword fixture corpus committed |
| **1** | Cleanup + targeted shared utility | P2-A (drop — comment only), P2-B (file-local), P2-C (`ToneStripper.stripCombiningDiacritics` only), P2-D (DateFormatter + scoring cross-ref comments), P4 (priority-aware; low items may defer) | Targeted duplication eliminated; dead code removed; no Chinese comments outside data files |
| **2** | iOS service work | P1-D (rename only), P1-B iOS (same-file isolation first), P1-C iOS (private functions, no new types) | iOS public surface unchanged; cross-platform fixture corpus passes before & after |
| **3** | Android service work + selective reorg | P1-B Android (same-file), P1-C Android (private functions), P3-A (only phonetics group moves to `dictionary.phonetics`) | Android public surface unchanged; cross-platform fixture corpus passes |
| **4** | NextWord split (highest risk last) | P1-A (in-file isolation Step 1; file extraction Step 2 only if any private type stays >200 LoC) | Both platforms ship; cross-platform fixture corpus passes; iOS sync→async collapse done as part of NextWord touch on iOS side |

**Per-stage workflow** (v0.3 — lighter than v0.2 per Codex audit item 23):

1. Write stage-specific TODO list as a section in this plan (no separate file).
2. **Once per stage** (at the start): Codex review of relevant current code; `simplify` scan of files in scope.
3. Triage findings: HIGH+ → fix in this stage; LOW → defer with rationale comment.
4. Implement in commits sized for review (NOT one commit per micro-item — Codex audit item 24).
5. Manual build + test at the stage boundary (user runs Xcode + gradle per `feedback_manual_build_test`).
6. PR open against `main` (per `feedback_branching` — direct-to-main flow).

`feedback_review_before_impl` still applies; v0.3 changes the *cadence* (per stage, not per item), not the principle.

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

This refactor is complete when (v0.3):
- Every HIGH and MED priority item across P0–P4 is closed; LOW items may be deferred with a rationale logged.
- All existing tests still pass on both platforms.
- Hot-zone characterization tests (golden snapshot + idempotence at known concurrency hotspots + migration + clock-injected decay) exist and pass.
- Cross-platform fixture corpus (search + nextword) passes on both platforms with identical output assertions.
- `docs/engine/` reflects current binary-format contracts; build-pipeline section lists each step (already done 2026-04-17).
- ~~Final cross-platform diff shows symmetric file inventory~~ — **dropped**: symmetry-for-symmetry is not a product-quality metric (Codex audit item 25). Use cross-platform fixture parity as the alignment gate instead.
- `simplify` skill returns no HIGH/MED findings on the touched files. (Low-priority findings allowed if logged.)
- Asset freshness (P0-D) verified by toggling a `build_ts` and confirming the app picks up new assets without a `versionCode` bump.
- Manual smoke test of TL/POJ/TPS input, custom dictionary CRUD, next-word prediction, backup/restore — passes on both platforms.
