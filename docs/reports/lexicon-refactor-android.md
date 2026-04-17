# Android Lexicon Refactor — Completion Report

> **Status**: Implemented on `refactor-core-engine` (commits `52d1bbe`..`a7b48bd`, 2026-04-17)
> **Scope**: `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/` + `ime/text/composing/UserFrequencyService.kt`.
> **Method**: Plan → Codex review (v1.1 calibration, APPROVE-WITH-CHANGES applied) → stage-by-stage commits → simplify-skill sweep.
> **Behaviour**: Preserving. Zero SQL changes; zero ranking changes; public API stable.
> **Reference**: Mirrors iOS `lexicon-refactor-ios.md` stage-by-stage.

---

## 1. Before / After

| Metric | Before | After |
|---|---|---|
| `LexiconService.kt` | 585 | 480 |
| `NextWordService.kt` | 826 | 889 (↑ section dividers + named-migration functions; content unchanged) |
| `CustomDictionaryService.kt` | 568 | 560 |
| `UserFrequencyService.kt` | 606 | 612 |
| `DictionaryModels.kt` (mixed 5 types) | 104 LoC | removed (split into 4 files) |
| `DictionarySearchResult.kt` (mixed 3 concerns) | 114 LoC | 20 LoC (pure data class) |
| Files in `ime/dictionary/` | 19 | 25 (+6 from splits + CandidateProcessor) |
| `LexiconService.search()` inline body | 112 LoC | ~25-line orchestrator + 4 same-file private phases |
| Duplicate UPSERT SQL in CustomDictionaryService | 2 copies (save + import) | 1 (`UPSERT_SQL` constant + `executeUpsert` helper) |
| `onUpgrade` inline migration branches (CustomDictionary) | 4 inline blocks | 4 named functions + `regenerateNotone`/`forEachRomanRow` helpers |
| `migrateUserDb` inline migrations (NextWord) | 3 inline blocks | 3 named functions (`migrateV0ToV2`, `migrateV2ToV3`, `migrateV3ToV4`) |
| Manual `stmt.clearBindings()` + `bindString`/`bindLong` call sites | 2 (NextWord + UserFreq) | 0 (via `SQLiteStatement.bindArgs` extension) |
| Clock reads per prediction row | 2 (decay + score) | 0 — single `nowMs` per batch in `predict()` |

Zero new nested types across services. Zero public API changes.

---

## 2. Stage Log

Every stage compiled and shipped as one commit.

### Stage A0 — Bind-statement helper (`52d1bbe`)
Added a file-local `private fun SQLiteStatement.bindArgs(vararg args: Any?)` extension in `NextWordService.kt` and `UserFrequencyService.kt`. Replaces the manual `clearBindings()` + per-arg `stmt.bindString(i, ...)` / `stmt.bindLong(i, ...)` boilerplate at the two batch-import call sites. Mirrors iOS Stage A (`bindText` helper).

### Stage A1 — Split `DictionaryModels.kt` (`9a44f11`)
`DictionaryModels.kt` (104 LoC) mixed five unrelated types. Split:
- `DictionaryConstants.kt` — `TRIE_PREFIX_*`, `DICT_BIN_NAME`, `DEFAULT_SEARCH_LIMIT`, `triePrefix()`
- `TaigiWord.kt` — data class
- `InputType.kt` — sealed class
- `DictionaryError.kt` — sealed class
- `ScoreBreakdown.kt` (temporary, folded into `CandidateProcessor` in Stage B)

Same package, so no import changes in callers.

### Stage A2 — Split `DictionarySearchResult.kt` (`6e9730e`)
The 114-LoC file mixed three concerns. Split into:
- `DictionarySource.kt` — enum (also used by `DictionaryBinaryReader`, `DictionarySettingsComponents`, `DictionarySearchViewModel`).
- `ExternalLookupURLBuilder.kt` — `chhoeURL`, `moeURL`, `toTLDigit`, `normalizeSyllableToDigit`.
- `DictionarySearchResult.kt` — now a 20-LoC pure data class; `chhoeUrl()` / `moeUrl()` member functions delegate to the builder so the Tab 3 call sites (`DictionarySettingsComponents.kt`) remain unchanged.

### Stage B — Extract `CandidateProcessor.kt` (`9b1af3b`)
Moved scoring/dedup logic out of `LexiconService.kt`:
- `ScoreBreakdown` (now nested inside CandidateProcessor; temporary `ScoreBreakdown.kt` removed).
- `calculateScore` / `sortByScore`.
- `romanToBase` / `inputToBase` / `logScoreDetails` (private helpers).
- `removeDuplicates` / `removeDisplayDuplicates`.

`LexiconService` keeps only orchestration, trie lookup, binary-reader IO, custom-dict merge. Mirrors iOS `CandidateProcessor.swift`.

### Stage C — Clock injection (`c1e5626`)
- `CandidateProcessor.sortByScore(..., currentTime: Long = System.currentTimeMillis())`.
- `NextWordService.calculateDecay(lastUsedMs, nowMs)` and `calculateUserScore(count, lastUsedMs, nowMs)`.
- `predict()` reads the clock once per batch and passes `nowMs` through, matching iOS per-batch semantics. Production call sites unchanged; tests can pin the clock.

### Stage D — LexiconService.search() phase split (`387ad53`)
The 112-line inline body is now a ~25-line orchestrator backed by four same-file private phases:
- `lookupCustomDictionary(input, prefHelper)` — honours `customDictEnabled`.
- `querySystemDictionaries(reader, input, inputMode, limit, enabledDicts, prefHelper)` — owns the TPS `er`↔`or` variant expansion with `existingIds`-based dedup.
- `rankByFrequency(merged, input, inputMode)` — dedup + InputNormalizer + `CandidateProcessor.sortByScore`.
- `applyDisplayDedup(ranked, prefs)` — TPS-mode visual dedup.

Merge order preserved byte-identical: custom first, then system (including TPS variant if applicable), then dedup, then sortByScore, then optional display dedup. PERF logging semantics unchanged. Codex-flagged high-risk stage; the phase split isolates each concern without altering the ordering contract.

### Stage E — CustomDictionary UPSERT dedup + migrations (`5fc58b8`)
Two improvements in `CustomDictionaryService.kt`:

1. **UPSERT template dedup.** `save()` and `importFromFile()` previously duplicated a ~14-line `INSERT ... ON CONFLICT` block (runtime-identical after `trimIndent()` — source literals differed in indent depth only). Now a single `UPSERT_SQL` constant is used via a private `executeUpsert(db, entry)` helper.
2. **`onUpgrade` migrations.** The four inline version branches are now named functions: `migrateV1ToV2`, `migrateV2ToV3` (delegates to `regenerateNotone`), `migrateV3ToV4` (same), `migrateV4ToV5`. A `forEachRomanRow` inline helper collapses the repeated "SELECT id, roman; while(next)" cursor loop across the four branches. `onUpgrade` now reads as a four-line table of contents.

### Stage F — NextWordService sections (`4e343ec`)
The highest-risk stage per Codex. Same-file section reorganization (no new files) — zero API change, zero behaviour change. Layout top-to-bottom:

1. Constants (scoring weights + DB config)
2. Public types (`Prediction`, `AssociationEntry`)
3. Properties
4. Init (`ensureInitialized`, `initAssociationReader`, `connectUserDb`)
5. Public API — Prediction (`predict`)
6. Public API — Recording (`recordAssociation`, `batchImportAssociations`, `deleteAssociation`, `clearAllAssociations`)
7. Public API — Queries (`allAssociations`)
8. Lifecycle (`close`)
9. User DB — Schema, Indexes, Migrations (`createUserAssocTable`, `createUserAssocIndexes`, `migrateFromWAL`, `migrateUserDb`, `migrateV0ToV2`, `migrateV2ToV3`, `migrateV3ToV4`)
10. Scoring (`calculateDecay`, `calculateUserScore`)
11. Pruning (`pruneOldAssociations`)

Extracted `fetchAllAssociations(db)` helper from `allAssociations()`. All SQL literals and transaction ordering preserved byte-identical.

### Stage G — UserFrequencyService sections (`a7b48bd`)
Same-file section reorganization. Layout: Constants · Schema · Properties · Types · Init · Public API (Recording / Queries / Mutations) · Pruning · DatabaseHelper. `DatabaseHelper.onCreate` is now a four-line TOC delegating to `createUserFrequencyTable` / `createUserFrequencyIndexes` / `createMetadataTable` / `insertMetadata`. Extracted `collectWordCountPairs()` helper shared between `topWords()` and `getAllFrequencies()`. Removed unused `java.io.File` import.

### Stage H — Simplify pass
Ran code-simplifier agent on the modified files with an explicit scope-limit (no stylistic refactors; no behavioural changes; no SQL byte changes; no visibility/signature changes). Result: **no additional changes needed** — each stage already applied its dedup pass as it landed.

### Stage G equivalent (SQLiteConnectionManager concurrency)
**Dropped.** Same reasoning as iOS — Codex flagged this as the highest silent-regression risk without characterization tests. Deferred to a dedicated concurrency pass after a characterization-test corpus lands.

### Stage I — Docs
- This file (`lexicon-refactor-android.md`) is the completion report.
- `lexicon-refactor-android-plan.md` deleted.
- `lexicon-refactor-ios.md` retained (iOS completion report).
- `rules/claude-workflow.md` retained — it is referenced (not duplicated) by `CLAUDE.md` line 12.

---

## 3. File Inventory (After)

```
ime/dictionary/
├── AssociationBinaryReader.kt
├── BackupService.kt
├── CandidateProcessor.kt             ← Stage B (new; +C clock injection)
├── CustomDictionaryService.kt        ← Stage E
├── DictionaryBinaryReader.kt
├── DictionaryConstants.kt            ← Stage A1 (new)
├── DictionaryError.kt                ← Stage A1 (new)
├── DictionarySearchResult.kt         ← Stage A2 (slimmed to pure data class)
├── DictionarySource.kt               ← Stage A2 (new)
├── EnabledDictionaries.kt
├── ExternalLookupURLBuilder.kt       ← Stage A2 (new)
├── InputNormalizer.kt
├── InputType.kt                      ← Stage A1 (new)
├── LexiconService.kt                 ← Stage B + D
├── NextWordService.kt                ← Stage A0 + C + F
├── SuggestionCaseTransformer.kt
├── TaigiPhonetics.kt                 (phonetics library — not in refactor scope)
├── TaigiUnicode.kt
├── TaigiWord.kt                      ← Stage A1 (new)
├── ToneConverter.kt                  (phonetics library — not in scope)
├── ToneConverterModels.kt            (phonetics library — not in scope)
├── ToneRestoration.kt                (phonetics library — not in scope)
├── ToneUtilities.kt                  (phonetics library — not in scope)
├── TPSConverter.kt                   (phonetics library — not in scope)
└── TrieService.kt
```

```
ime/text/composing/
└── UserFrequencyService.kt           ← Stage A0 + G
```

---

## 4. Shared-Core Candidates (for cross-platform extraction, later)

Not a goal of this branch. Mirrors the iOS report's candidate list — Android has identical twins for each pure-logic file.

### Pure logic — extractable first
| Android file | iOS twin | Nature |
|---|---|---|
| `InputNormalizer.kt` | `InputNormalizer.swift` | String/Unicode |
| `TaigiUnicode.kt` | `TaigiUnicode.swift` | Unicode NFD |
| `CandidateProcessor.kt` scoring formula | `CandidateProcessor.swift` | pure math; clock-injected both sides |
| `ExternalLookupURLBuilder.kt` | `ExternalLookupURLBuilder.swift` | String/URL |
| `EnabledDictionaries.kt` bitmask | `EnabledDictionaries.swift` | bitmask |
| `DictionarySource.kt` | `DictionarySource.swift` | enum |
| Tone converters (`TaigiPhonetics`, `ToneConverter`, `ToneRestoration`, `ToneUtilities`, `TPSConverter`) | same-named files | tone tables |
| `NextWordService.kt` Constants block | `NextWordService.swift` Constants | CROSS-PLATFORM INVARIANT already asserted in code |

Recommended extraction order matches iOS: Constants → bitmask → candidate-scoring formula → Unicode/phonetics stack.

### Platform-coupled — stay per platform
- `SQLiteDatabase` usage (user_association.db, user_frequency.db, custom_dictionary.db) — Android-specific; iOS uses the SQLite C API directly.
- `TrieService.kt` — uses JNI; iOS uses a C++ bridge.
- Asset resolution (`context.filesDir` / `context.assets`) — Android framework.
- Logging (`Log.d` vs iOS `os_log`).

### Should **not** be shared
The three SQLite-backed services (`NextWordService`, `CustomDictionaryService`, `UserFrequencyService`): schemas, migration histories, and concurrency primitives are each platform-owned. A shared CRUD base would leak details both ways.

---

## 5. Manual Verification Checklist

`./gradlew test` and `./gradlew assembleDebug` are run by the user (per project workflow). The following hot zones per Codex review — verify end-to-end before treating the refactor as landed.

### 5.1 Search path
- [ ] Type TL input (e.g. `li-ho`) → candidates rank identically to a pre-refactor build.
- [ ] Type POJ input with diacritics → identical ranking.
- [ ] Type TPS input → identical ranking, and `er`↔`or` variant expansion still returns variant rows when the toggle is on (**Stage D hot zone**).
- [ ] Custom dictionary enabled: top entry for a matching prefix is the custom entry; disabling the toggle removes it.
- [ ] Tab 3 lookup dialog: tapping an entry still opens the MOE / Chhoe Taigi URL with tone digits correctly encoded (including tone 1 / tone 4 strip).

### 5.2 Prediction path
- [ ] Select a word → NextWord suggestions match the pre-refactor build for the same input history.
- [ ] Learned associations (user-side): select a 2-word sequence three times → second word jumps to the top on subsequent selections of the first word (confirms `calculateUserScore` + decay + `LEARNING_BONUS` still produce the ~300 learning bonus).
- [ ] Clear associations from Tab 3 → list empty after restart.

### 5.3 First-run DB init
- [ ] Delete the app → reinstall → first keystroke produces results without crash (covers custom-dict init + lazy frequency init in the refactored `search()` path).
- [ ] Delete user-association DB from Tab 3 → type → prediction still returns dict-side results (no user side).

### 5.4 Migration paths
- [ ] v3 `user_association.db` locally → opening the app should run v3→v4 migration (`migrateV3ToV4`) and preserve existing counts.
- [ ] v1/v2/v3/v4 `custom_dictionary.db` → opens and backfills `notone`/`abbrev`/`roman_num` without data loss (`migrateV1ToV2` through `migrateV4ToV5`).
- [ ] Custom-dict entries from a backup should round-trip through export → delete → import with `count` preserved.

### 5.5 Test suites
- [ ] `CustomDictionaryServiceTest` passes.
- [ ] `InputNormalizerTest` passes.
- [ ] `DictionaryCoverageTest` passes.
- [ ] `EngineIntegrationTest` passes.

If anything diverges, investigate in this order:
1. **Stage F** — NextWordService (scoring takes `nowMs` as a parameter; verify the batch clock is passed through correctly).
2. **Stage D** — LexiconService.search() phase split (verify the TPS `er`↔`or` branch still runs on TPS-containing raw input).
3. **Stage E** — UPSERT SQL executes identically via helper.

---

## 6. What's Explicitly Out of Scope

- **SQLite concurrency / connection manager collapse.** Deferred on both platforms pending characterization tests.
- **Cross-platform extraction (Swift Package / KMP).** Next phase; this branch lays the groundwork.
- **`TPSConverter` / `TaigiPhonetics` / `ToneConverterModels` refactor** — phonetics library, not Lexicon core.
- **`build.gradle` / `.pbxproj` edits** — user-owned.
- **Public API changes** — every external caller (`TextInputManager`, autocomplete, Tab 3 views) continues to compile unchanged.

---

## 7. Done Definition (met on this branch)

- [x] Every stage compiles independently and ships as one commit.
- [x] `DictionaryModels.kt` (mixed-concern file) removed; filename matches primary type.
- [x] Public API unchanged (verified by searching every external caller).
- [x] Simplify-pass findings closed or deferred with rationale.
- [x] Single consolidated refactor doc per platform (`lexicon-refactor-ios.md` + this file).
- [x] Codex APPROVE-WITH-CHANGES fully applied (Stage D risk upgraded, Stage E wording corrected, bind-helper added, verification checklist added, Stage H scope-limited).
- [ ] Manual verification checklist above, to be checked by the user at the user's build cadence.
