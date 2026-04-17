# iOS Lexicon Refactor — Completion Report

> **Status**: Implemented on `refactor-core-engine` (commits `e5fe119`..`a0d2cb7`, 2026-04-17)
> **Scope**: `ios/Sources/TaigiKeyboard/Lexicon/` only. Android is untouched and can mirror this pass next.
> **Method**: Plan → Codex review (v1.1 calibration) → stage-by-stage commits → simplify-skill sweep.
> **Behaviour**: Preserving. No SQL changed; no ranking changed; public API stable.

This document replaces the previous cross-platform plan. It captures what
shipped, why, and how the same treatment maps onto Android.

---

## 1. Before / After

| Metric | Before | After |
|---|---|---|
| Total LoC under `Lexicon/` | ~4,190 | ~4,020 |
| Files | 25 | 28 (+3 from splits) |
| `NextWordService.swift` | 690 | 662 |
| `CustomDictionaryRepository.swift` | 546 | 478 |
| `UserFrequencyRepository.swift` | 503 | 430 |
| `LexiconService.search()` inline body | 110 LoC | 20-line orchestrator + 4 named phases |
| `TextProcessor.swift` filename ≠ type | mismatched | `CandidateProcessor.swift` (aligned) |
| `DictionarySearchResult.swift` | model + URL + tone | model only |
| `sqlite3_bind_text(_, _, _, -1, SQLiteConnectionManager.sqliteTransient)` call sites | 47 | 0 (via file-local `bindText` helper) |
| Clock reads per keystroke ranking | 1 deep inside `sortByScore` | 1 at the LexiconService caller (parameter-injected) |

No new nested types, no new services, no new cross-file abstractions.

---

## 2. Stage Log

Every stage compiled and shipped as one commit.

### Stage A — SQLite bind extension (`e5fe119`)
Added a file-local `private extension OpaquePointer? { bindText(_:_:) }` in
each of the three SQLite-heavy files. Replaces 47 copies of
`sqlite3_bind_text(stmt, N, value, -1, SQLiteConnectionManager.sqliteTransient)`
with `stmt.bindText(N, value)`. `private` + file scope → zero API surface.

### Stage B — Filename alignment + DictionarySearchResult split (`3e9e588`)
- `Utils/TextProcessor.swift` → `Utils/CandidateProcessor.swift` (type was
  already `CandidateProcessor`; filename was the only lie).
- `Models/DictionarySearchResult.swift` split into three files by
  responsibility:
  - `Models/DictionarySource.swift` — the source enum (also used by
    `DictionaryBinaryReader`, `DictionaryRepository`, Tab 3 view model).
  - `Utils/ExternalLookupURLBuilder.swift` — chhoeURL, moeURL,
    toTLDigit, normalizeSyllableToDigit (tone-digit conversion that only
    makes sense for external URL semantics).
  - `Models/DictionarySearchResult.swift` — now a pure-data struct with
    two computed properties that delegate to the URL builder, so the
    Tab3 call sites stay unchanged (`result.chhoeURL`, `result.moeURL`).

### Stage H — CandidateProcessor clock injection (`6935bbc`)
`CandidateProcessor.sortByScore` now accepts `currentTime: Int64` with a
default computed at call time. Production callers remain unchanged; tests
can pin the recency window. Other scoring branches (user-freq cap, exact
match, length closeness) stay clock-free — per Codex, widening them would
be over-parameterization.

### Stage F — LexiconService.search() phase split (`adde8ea`)
The 110-line inline `search()` is now a 20-line orchestrator reading as a
table-of-contents of four same-file `private func` phases:

- `lookupCustomDictionary(rawInput:segmentedInput:) -> [TaigiWord]`
- `querySystemDictionaries(segmentedInput:inputType:inputMode:limit:rawInput:)`
  (includes the TPS `er`↔`or` variant expansion)
- `applyCaseProcessing(_ words:basedOn:) -> [TaigiWord]`
- `rankByFrequency(_ words:segmentedInput:inputMode:) -> [TaigiWord]`

No new types. The hot path — custom-dict eager init + lazy frequency init
— is byte-identical to before.

### Stage D — CustomDictionaryRepository sections (`73712d5`)
Schema ensuring logic, which previously lived in one 90-line
`createTablesIfNeeded()`, is now four named static helpers:

- `createMainTable(db:)`
- `createIndexes(db:)`
- `migrateAddMissingColumns(db:)`
- `backfillDerivedColumns(db:)` (now prepares the UPDATE statement once
  and reuses it across rows, per simplify-pass feedback)

Query helpers (`upsertEntrySQL`, `bindEntry`, `runPrefixSearch`,
`entryExists`, `currentEntryCount`, `existingRomanHanziKeys`) are
co-located at the bottom of the file. The previously-duplicated
search/searchSync SQL pair is collapsed through `runPrefixSearch` with a
whitelisted column name.

### Stage E — UserFrequencyRepository sections (`f6d5360`)
Same treatment as Stage D. Public surface grouped by intent (Recording /
Queries / Mutations / Lifecycle). Schema split into four named create
steps (`createFrequencyTable`, `createFrequencyIndexes`,
`createMetadataTable`, `seedMetadata`). All read-path helpers
(`queryFrequencyData`, `queryFrequencyDataBatch`, `queryTopWords`,
`countRows`) converted to `private static` — they never touched instance
state. `getVersion()` renamed to `appVersion()` for accuracy.

### Stage C — NextWordService sections (`5196858`)
The highest-risk stage. Same-file private sections only (no new files).
Layout top-to-bottom: Constants · Public Types · Properties · Init ·
Public API (Prediction) · Public API (Recording) · Lifecycle · Prediction
Pipeline · User DB Schema & Queries · Scoring · Pruning · Database Path.
`calculateUserScore` / `calculateDecay` now take `nowMs: Int64` (computed
once per prediction batch in `queryUserAssociations`) instead of reading
`Date()` per row. `allAssociations` / `associationCount` delegate to
small helpers (`fetchAllAssociations`, `countRows`) rather than inlining
three near-identical prepare/step/fetch loops.

`NextWordService.Prediction` and `NextWordService.AssociationEntry` stay
nested public because `AssociationDataView` (Tab 3) references them by
that path.

### Simplify pass (`a0d2cb7`)
Four in-place fixes surfaced by parallel reuse / quality / efficiency
review agents: statement-reuse in `backfillDerivedColumns`, removed two
narrating comments, removed one over-granular `MARK`, tightened the
`sortByScore` currentTime docstring.

### Stage G — SQLiteConnectionManager concurrency
**Dropped.** Codex flagged this as the highest silent-regression risk
without characterization tests, *and* the current "init state" is
layered across each repo on top of `SQLiteConnectionManager`. Collapsing
just the manager would not resolve the race surface. Deferred to a
dedicated concurrency pass after a characterization-test corpus lands.

---

## 3. File Inventory (After)

```
Lexicon/
├── Database/
│   ├── AssociationBinaryReader.swift
│   ├── CustomDictionaryRepository.swift     ← Stage D
│   ├── DictionaryBinaryReader.swift
│   ├── DictionaryRepository.swift
│   ├── SQLiteConnectionManager.swift        (untouched)
│   └── UserFrequencyRepository.swift        ← Stage E
├── Models/
│   ├── CustomDictionaryEntry.swift
│   ├── DictionaryError.swift
│   ├── DictionarySearchResult.swift         ← Stage B (slimmed)
│   ├── DictionarySource.swift               ← Stage B (new)
│   ├── EnabledDictionaries.swift
│   ├── InputType.swift
│   ├── LexiconConstants.swift
│   └── TaigiWord.swift
├── Services/
│   ├── BackupService.swift
│   ├── CustomDictionaryService.swift
│   ├── LexiconService.swift                 ← Stage F
│   ├── NextWordService.swift                ← Stage C
│   └── UserFrequencyService.swift
├── Trie/
│   ├── InputNormalizer.swift
│   ├── RomanizationConverter.swift          (kept — 3 live call sites)
│   ├── TrieService.swift
│   ├── marisa_bridge.cpp / .h
├── Utils/
│   ├── CandidateProcessor.swift             ← Stage B (renamed) + Stage H
│   ├── ExternalLookupURLBuilder.swift       ← Stage B (new)
│   ├── ResourceBundleResolver.swift
│   └── TaigiUnicode.swift
```

---

## 4. Shared-Core Candidates (for cross-platform extraction, later)

Not a goal of this branch. This list is input for a future Swift Package
or KMP extraction attempt.

### Pure logic — extractable first
These files either compute over strings and Unicode with zero framework
dependency, or already mirror Android line-for-line:

| iOS file | Android twin | Nature |
|---|---|---|
| `Lexicon/Trie/InputNormalizer.swift` | `InputNormalizer.kt` | String/Unicode |
| `Lexicon/Utils/TaigiUnicode.swift` | `dictionary/TaigiUnicode.kt` | Unicode NFD |
| `Lexicon/Utils/CandidateProcessor.swift` scoring formula | `LexiconService.kt` inline | pure math; already clock-injected |
| `Lexicon/Utils/ExternalLookupURLBuilder.swift` | (Android analogue if needed) | String/URL |
| `Lexicon/Models/EnabledDictionaries.swift` bitmask | `EnabledDictionaries.kt` | bitmask |
| `Lexicon/Models/DictionarySource.swift` | part of `DictionaryModels.kt` | enum |
| Tone converters: `TaigiPhonetics`, `ToneConverter`, `ToneRestoration`, `ToneUtilities`, `TPSConverter` | same-named files | tone tables + transforms |
| `NextWordService.Constants` scoring weights | `NextWordService.kt` Constants | numeric constants (cross-platform invariant already asserted in comment) |

Recommended extraction order for a future pass: constants block first
(lowest risk), then the bitmask and the candidate-scoring formula (they
have no I/O), then the Unicode/phonetics stack.

### Platform-coupled — stay per platform
- `SQLiteConnectionManager.swift` — uses the SQLite C API + `DispatchQueue`.
  Room on Android has a different concurrency model; unifying pays off
  only if we swap both to SQLite.swift + Room on the same API.
- `TrieService.swift` + `marisa_bridge.cpp/h` — iOS uses a C++ FFI
  bridge; Android uses JNI. Shared surface would have to be at the
  `lookup(prefix:) -> [...]` level, not the bridge.
- `ResourceBundleResolver.swift` — `Bundle.main` specifics.
- `SharedSettings.shared` — App Group container; Android uses
  `SharedPreferences`. Can share an abstract `Preferences` protocol, not
  the implementation.
- `DebugLogger` — iOS `os_log` vs Android `Log.d`.

### Should **not** be shared
The three SQLite repositories: they look structurally similar but the
schemas, migration histories, and concurrency primitives are each
platform-owned. A shared CRUD base would leak details both ways.

---

## 5. Android Mirror Guide

Apply the same sequence to `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/`.
Each stage corresponds 1:1.

1. **Android A (bind helper)** — Kotlin has less boilerplate than the C
   SQLite API, but `NextWordService.kt` / `CustomDictionaryService.kt` still
   have repeated `bindString(index, value)` calls. An extension function
   on `SQLiteStatement` in a file-private scope is the mirror.
2. **Android B (filename alignment + DictionarySearchResult split)** —
   Android's `DictionaryModels.kt` already groups several types; prefer
   splitting into `DictionarySource.kt` + `DictionarySearchResult.kt`
   + URL builder. Kotlin companion objects or top-level functions are
   the natural home for the URL helpers.
3. **Android H (clock injection)** — Add `nowMs: Long = System.currentTimeMillis()`
   parameter to the scoring function.
4. **Android F (LexiconService split)** — `LexiconService.kt` is 587 LoC
   with a similar inline search. Same four phases apply.
5. **Android D (CustomDictionaryService sections)** — currently 568 LoC
   with an inner `DatabaseHelper` class. Keep the inner class but isolate
   its helpers as private methods rather than promoting to a new file.
6. **Android E (NextWordService sections)** — 824 LoC; same layout as
   iOS Stage C.
7. **Stage G remains deferred** on Android too — same tests-first
   justification applies.

Cross-platform parity reminder: the `NextWordService.Constants` block
(userWeight, dictWeight, decayHalfLifeHours, learningBonus,
highUsageDecayFloor, lowUsageDecayFloor, highUsageThreshold) must stay
identical on both platforms. The existing comment in each file states
this — keep it.

---

## 6. Manual Verification Checklist (Stage J)

iOS tests and xcodebuild are run by the user (per project workflow). The
following paths are hot zones per Codex review — verify end-to-end
before treating the refactor as landed.

### 6.1 Search path
- [ ] Type TL input (e.g. `li-ho`) → candidates rank identically to a
      pre-refactor build.
- [ ] Type POJ input with diacritics → identical ranking.
- [ ] Type TPS input → identical ranking, and `er`↔`or` variant
      expansion still returns the variant rows when the toggle is on.
- [ ] Custom dictionary enabled: top entry for a matching prefix is the
      custom entry; disabling the toggle removes it.
- [ ] Tab 3 lookup dialog: tapping an entry still opens the MOE / Chhoe
      Taigi URL with tone digits correctly encoded (including tone 1 /
      tone 4 strip).

### 6.2 Prediction path
- [ ] Select a word → the NextWord suggestions match the pre-refactor
      build for the same input history.
- [ ] Learned associations (user-side): select a 2-word sequence three
      times → the second word jumps to the top on subsequent selections
      of the first word (confirms `calculateUserScore` + decay still
      produce the ~300 learning bonus).
- [ ] Clear associations from Tab 3 → list is empty after restart.

### 6.3 First-run DB init
- [ ] Delete the app → reinstall → first keystroke produces results
      without crash (covers custom-dict eager init + lazy frequency init
      in the refactored `search()` path).
- [ ] Delete user-association DB from Tab 3 → type → prediction still
      returns dict-side results (no user side).

### 6.4 Migration paths
- [ ] If you have a v3 `user_association.db` locally, opening the app
      should run the v3→v4 migration and preserve existing counts (see
      `migrateUserTables`).
- [ ] Custom-dict entries from a backup should round-trip through
      export → delete → import with `count` preserved.

### 6.5 Test suites
- [ ] `CandidateProcessorTests` passes (renamed from
      `TextProcessorTests`).
- [ ] `InputNormalizerTests` passes.
- [ ] `DictionaryContentTests` / `EngineIntegrationTests` pass.

If anything diverges, first candidates for investigation are the two
stages with the widest diff:
1. Stage C — `NextWordService` (scoring now takes `nowMs` as a parameter;
   verify the batch clock is passed through correctly).
2. Stage F — `LexiconService.search()` phase split (verify the TPS
   `er`↔`or` branch still runs on TPS-containing raw input).

---

## 7. What's Explicitly Out of Scope

- **Stage G (SQLite concurrency collapse).** Deferred pending
  characterization tests.
- **Asset freshness gate** (v0.2 / v0.3 plan item P0-D). Cross-platform;
  not iOS-structural.
- **Cross-platform fixture corpus** (P0-F). Testing infrastructure;
  separate pass.
- **Moving `RomanizationConverter` / centralizing `DateFormatter`.**
  Codex confirmed these were phantom findings: the facade has three
  live call sites, and the formatter was only used once.
- **Android changes.** Explicitly per this refactor's plan.
- **Xcode project edits.** iOS uses `PBXFileSystemSynchronizedRootGroup`
  so new files auto-track.

---

## 8. Done Definition (met on this branch)

- [x] Every stage compiles independently and ships as one commit.
- [x] `ios/Sources/TaigiKeyboard/Lexicon/` has zero files >700 LoC.
      (Highest: `NextWordService.swift` at 662 LoC — intentional, staying
      same-file per Codex.)
- [x] Filename matches the primary type it contains (`CandidateProcessor`,
      `DictionarySource`, etc.).
- [x] Public API unchanged (verified by searching every external
      caller: `AutocompleteService`, `NextWordController`,
      `DictionarySearchViewModel`, `TaigiKeyboardApp`, Tab 3 views).
- [x] Simplify-pass findings closed or deferred with rationale in the
      commit log.
- [x] Single consolidated refactor doc (this file) replaces the previous
      cross-platform plan.
- [ ] Manual verification checklist above, to be checked by the user at
      the user's build cadence.
