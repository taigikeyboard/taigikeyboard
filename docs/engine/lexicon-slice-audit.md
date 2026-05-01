# Lexicon Read-Path Slice — Audit (v3.5.6 / OPT-A)

**Status**: pre-impl audit, authored 2026-05-01 on `main` (HEAD `6ca312f`) before opening branch `phase4b/v3.5.6-lexicon-readpath`. Companion to a not-yet-written `lexicon-slice-plan.md`.

**Goal**: catalogue Lexicon **read-path** code on both platforms, lock OPT-A scope (read half only; SQLite write paths stay platform), measure fst vs marisa-rs format-gate, draft the `LexiconPaths` install-time injection contract, and enumerate residual risks before plan finalization.

**Cadence ref**: `project_rust_migration_cadence.md` release map — v3.5.6 = Lexicon (DB + Trie behind FFI), pulling forward only the read half per `project_v3_5_6_lexicon_scope.md` OPT-A scope (Codex-co-decided 2026-05-01, 88% confidence).

**Architecture target**: Rust crate `engine/lexicon/` owns prefix-index + bundled-binary mmap readers + search orchestration; platform retains SQLite write paths (UserFrequency, CustomDictionary, NextWord `user_association.db`) until a future write-path slice with its own version number (per `feedback_no_future_planning.md`).

---

## 1. Lexicon read-path domain definition

The lexicon read path is the chain that turns a romanized or hanji input into a ranked list of dictionary candidates, sourced from **read-only bundled assets** (`dictionary.trie` + `dictionary.bin` + `association.bin`):

1. **Input normalization** — `InputNormalizer` produces a numeric-tone TL-form key for trie lookup. Already routed through `RustEngineBridge.normalizeInput` (Rust `phonetics::api::normalize_input`) post-D9.4 (PR #186/#187). Bridge wrappers stay; the platform-side `InputNormalizer` shim becomes the slice deletion target.
2. **Trie key construction + lookup** — `TrieService` runs `prefix_search` + exact `lookup` over `dictionary.trie` (MARISA RecordTrie behind C bridge / JNI). Key-prefix construction (`tl:` / `poj:` / `hanzi:`) lives in callers today (`DictionaryRepository` iOS, `LexiconService` Android).
3. **Binary record lookup** — `DictionaryBinaryReader` mmaps `dictionary.bin` (TKDB, ~4.4 MB) and decodes records by 1-based rowid.
4. **Bundled bigram lookup** — `AssociationBinaryReader` mmaps `association.bin` (TKWA, ~3.1 MB) and binary-searches prev_word → entries. **READ-ONLY bundled half only**; the mutable per-user `user_association.db` is OUT OF SCOPE.
5. **Bitmask filtering** — 3-layer filter in `DictionaryBinaryReader.passesFilter` (variant exclusion → khiin exclusion → source-OR-with-dev) and 1-layer in `AssociationBinaryReader.passesFilter` (per `binary-format.md` §4).
6. **TPS er↔or expansion** — when input contains TPS and the `isTpsOrMappedToER` toggle is on, the search runs a second trie query against the `or` variant.
7. **Search orchestration** — `LexiconService.search` orchestrates custom-dict lookup (platform-stays SQLite) + system-dict query (steps 1–6) + ranking via `RustEngineBridge.processCandidates` (already Rust, since v3.5.2 / engine/ranking).

**NOT lexicon read-path (out of slice)**:

- SQLite write paths: `UserFrequencyRepository`, `CustomDictionaryRepository`, NextWord `user_association.db` (the SQLite half of NextWord persistence).
- `CustomDictionaryService`, `BackupService`, `CustomDictionaryMigrator`, `CustomDictionarySchema`, `CustomDictionaryCapacityPolicy`, `UserFrequencyPruner`, `UserFrequencySchema`, `SQLiteConnectionManager`, `SQLiteBindingHelpers`.
- `ResourceBundleResolver` (iOS bundle plumbing), `ExternalLookupURLBuilder`, `SharedDatabasePath`, `CandidateProcessor` (auto-cap UI helper) — adapter / URL-construction / display-layer concerns, stay platform.
- `EnabledDictionaries`, `DictionarySource`, `TaigiWord`, `DictionarySearchResult`, `LexiconError` / `DictionaryError` model types — these become proto-shaped value types (proto definitions); the platform-side enum/struct mirror is deleted in this slice but the models themselves live on as proto types.

---

## 2. Inventory — iOS

Path root: `ios/Sources/TaigiKeyboard/Lexicon/` unless noted. LOC counts re-measured 2026-05-01.

### 2.1 In scope (rewrite into Rust + delete platform)

| File | LOC | Role | v3.5.6 disposition |
|---|---:|---|---|
| `Trie/InputNormalizer.swift` | 25 | Wrapper over `RustEngineBridge.normalizeInput` (already Rust); only the call-site survives | **Delete** — callers go directly to `RustEngineBridge.lexicon*` which calls `phonetics::api::normalize_input` internally |
| `Trie/TrieService.swift` | 153 | Handle-based wrapper over C bridge `marisa_bridge.cpp` (`trie_handle_t`); supports up to 8 simultaneous tries | **Delete** — Rust crate owns the prefix index (fst, see §4) |
| `Database/AssociationBinaryReader.swift` | 217 | mmap reader for `association.bin`; binary search by raw UTF-8 byte; `passesFilter` static | **Delete** — Rust `engine/lexicon/src/association_reader.rs` ports byte-for-byte (read-only bundled half) |
| `Database/DictionaryBinaryReader.swift` | 197 | mmap reader for `dictionary.bin`; rowid offset table; `passesFilter` 3-layer; `bitToSource` 12-bit map | **Delete** — Rust `engine/lexicon/src/dictionary_reader.rs` |
| `Database/DictionaryRepository.swift` | 223 | Trie + binary reader orchestrator: `query`, `searchWithSources`, `searchByHanzi`, shared `lookupRowIds` | **Delete** — Rust `engine/lexicon/src/search.rs` orchestrates; bridge wrapper takes its place |
| `Services/LexiconService.swift` | 265 | Top facade: 4-phase pipeline (custom-dict → system-dict → case processing → ranking). Read paths route to repository above; SQLite paths (custom-dict + user-freq) stay | **Reduce** — keep custom-dict + cold-start + auto-cap branches platform-side; system-dict + TPS er↔or branches move to Rust via bridge |

**Estimated iOS deletion** (in-scope above): ~1080 LOC across 6 Swift files.

**Companion native-bridge deletions** (same PR, per `feedback_path_g_delete_mirrors.md`):

| File | Role | Disposition |
|---|---|---|
| `ios/Sources/TaigiKeyboard/Lexicon/Trie/marisa_bridge.cpp` | C++ wrapper around MARISA RecordTrie + handle-table for `trie_handle_t` | **Delete** — Rust `engine/lexicon::prefix_index` replaces; no C++ MARISA dep remains on iOS |
| `ios/Sources/TaigiKeyboard/Lexicon/Trie/marisa_bridge.h` | Public header for the C bridge | **Delete** |
| `ios/Resources/Dictionaries/dictionary.trie` | Bundled MARISA asset | **Delete** (replaced by `dictionary.fst` per §5) |

The Xcode project references for `marisa_bridge.cpp`/`.h` and the bundled `.trie` asset must be removed manually by the user (`feedback_xcode_manual.md`); audit notes the Xcode-side deletion as a manual step in the impl-round commit slicing.

### 2.2 Out of scope (platform-stays)

These files coexist with the slice but are **not** touched in v3.5.6:

| Group | Files | Reason |
|---|---|---|
| SQLite write | `UserFrequencyRepository.swift`, `UserFrequencyService.swift`, `UserFrequencyPruner.swift`, `UserFrequencySchema.swift`, `CustomDictionaryRepository.swift`, `CustomDictionaryService.swift`, `CustomDictionarySchema.swift`, `CustomDictionaryMigrator.swift`, `CustomDictionaryCapacityPolicy.swift`, `CustomDictionaryDerivation.swift`, `BackupService.swift`, `SQLiteConnectionManager.swift`, `SQLiteBindingHelpers.swift`, `SharedDatabasePath.swift` | Future write-path slice (D5/D6/D7 deferred per scope memo) |
| Adapters / URL | `ResourceBundleResolver.swift`, `ExternalLookupURLBuilder.swift` | Bundle path / URL construction — platform forever per `rules/cross-platform-alignment.md` §5.1 |
| Display layer | `Utils/CandidateProcessor.swift` | Auto-cap is UI-side; engine returns neutral candidates |
| Models | `Models/{TaigiWord, DictionarySearchResult, DictionarySource, EnabledDictionaries, FrequencyData, InputType, LexiconConstants, LexiconError, CustomDictionaryEntry}.swift` | Stay as platform-side proto-shaped value types (read from proto into Swift via bridge synth types) |
| Search service | `Services/DictionarySearchService.swift` | Tab3 dictionary explorer — wraps `searchWithSources` / `searchByHanzi`, simple bridge swap, no engine logic |

### 2.3 Tests (`ios/TaigiKeyboardTests/`)

| File | Disposition |
|---|---|
| `LexiconReadPathTests.swift` (if present — to be added) | New `INVARIANT_LEX_*` parity tests live here, asserting `RustEngineBridge.lexicon*` matches pre-swap iOS output for every fixture |
| `RustEngineBridgeTests.swift` (existing) | Add lexicon entries |
| `DictionaryContentTests.swift` (existing) | **Untouched** — counts records via Bundle; not engine-state-dependent |

### 2.4 Call sites (consumers)

Public surface that must be rewired in this PR:

- `Autocomplete/Services/AutocompleteService.swift` — calls `LexiconService.search(...)` for candidate generation
- `Autocomplete/Services/AutocompleteProviders.swift` — same
- `App/Tabs/.../DictionarySearchService.swift` — calls `LexiconService.searchWithSources(...)` / `searchByHanzi(...)` for Tab3
- `App/CompositionRoot.swift` — owns `lexiconService` singleton; replace ctor wiring with bridge handle
- `KeyboardExtension/KeyboardViewController+Setup.swift` — early init for trie + custom dict (preserve)

---

## 3. Inventory — Android

Path root: `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/`. LOC counts re-measured 2026-05-01.

### 3.1 In scope

| File | LOC | Role | v3.5.6 disposition |
|---|---:|---|---|
| `InputNormalizer.kt` | 40 | Wrapper over `RustEngineBridge.normalizeInput` + `tpsToTl` for pre-normalize TPS conversion | **Delete** — bridge wrappers replace; `buildSearchKey` divergence resolved (see §4 D-1) |
| `TrieService.kt` | 169 | JNI bindings to `taigi_trie` (`trie_jni.cpp`) singleton `g_trie`; copy-from-assets via `getTriePath` + `trie_app_version.txt` stamp | **Delete** — Rust crate owns prefix index; Android trie file path resolution moves to platform-side `LexiconPaths` snapshot (see §5) |
| `AssociationBinaryReader.kt` | 259 | mmap reader (`MappedByteBuffer`); binary search by raw UTF-8; strict UTF-8 decode; `passesFilter` static | **Delete** — same Rust port as iOS |
| `DictionaryBinaryReader.kt` | 230 | mmap reader; rowid offset table; strict UTF-8 decode; 3-layer filter; `BIT_TO_SOURCE` 12-bit map | **Delete** — same Rust port as iOS |
| `LexiconService.kt` | 540 | All-in-one facade: ensures init (trie + bin + asset copy), `search`, `searchWithSources`, `searchByHanzi`, `lookupRowIds`, `searchWithTrie`, custom-dict lookup, TPS er↔or expansion | **Reduce** — system-dict + TPS er↔or branches move to Rust via bridge; init code (asset-copy + version stamp) stays platform; custom-dict + ranking-call wiring stays platform. **Must split**: 540 LOC > 500 LOC hard cap per `feedback_rust_extraction_goals.md` goal 4 — splitting also resolves the goal-4 violation flagged in scope memo |

**Estimated Android deletion** (in-scope above): ~1200 LOC across 5 Kotlin files.

**Companion native-bridge + asset deletions** (same PR):

| File | Role | Disposition |
|---|---|---|
| `android/app/src/main/cpp/trie_jni.cpp` | JNI bridge to C++ MARISA singleton `g_trie` | **Delete** — Rust `engine/lexicon::prefix_index` replaces; C++ MARISA dep retired |
| `android/app/src/main/cpp/CMakeLists.txt` (entry for `taigi_trie`) | Build script entry that produces `libtaigi_trie.so` | **Modify** — remove the `taigi_trie` library target; the user-edited `build.gradle` keeps the existing `librust_engine.so` JNI target only |
| `android/app/src/main/assets/dictionary.trie` | Bundled MARISA asset | **Delete** (replaced by `dictionary.fst`) |

CMakeLists.txt and `build.gradle` edits are user-only territory per `feedback_xcode_manual.md` (the Android equivalent — the audit notes them as manual steps).

### 3.2 Out of scope (platform-stays)

| Group | Files | Reason |
|---|---|---|
| Constants / models | `DictionaryConstants.kt`, `EnabledDictionaries.kt`, `DictionarySource.kt`, `InputType.kt`, `TaigiWord.kt`, `DictionarySearchResult.kt`, `DictionaryError.kt`, `CustomDictionaryEntry.kt` | Proto-shaped value types — deletion happens in proto schema work, not file-level |
| SQLite write | `CustomDictionaryService.kt`, `CustomDictionarySchema.kt`, `CustomDictionaryMigrator.kt`, `CustomDictionaryDerivation.kt`, `CustomDictionaryCapacityPolicy.kt`, `BackupService.kt`, `UserFrequencyService.kt`, `UserFrequencySchema.kt`, `UserFrequencyPruner.kt`, related repository classes | Future write-path slice |
| Bundled-but-not-this-slice readers | `NextWordService.kt` (uses `AssociationBinaryReader` for the bundled half via `DictionaryConstants.ASSOC_BIN_NAME`) | The bundled `.bin` reader moves to Rust in this slice; `NextWordService` calls into the new Rust reader through the bridge but the SQLite + scoring callers (`predict()` via `engine/nextword`) are already wired post-v3.5.5 |

### 3.3 Tests (`app/src/test/.../ime/dictionary/`)

| File | Disposition |
|---|---|
| `DictionaryCoverageTest.kt` | **Untouched** — counts records via assets; not engine-state-dependent |
| Per-class JVM unit tests for the 5 in-scope files | **Delete** per `feedback_path_g_delete_mirrors.md` (JVM cannot load `.so`; coverage replaced by Rust workspace tests + `INVARIANT_LEX_*` instrumentation parity on a real device) |

### 3.4 Call sites (consumers)

- `ime/text/composing/TaigiAutocompleteService.kt` — calls `LexiconService.search(...)`
- `ime/text/composing/EnglishAutocompleteService.kt` — does NOT call LexiconService (English path); skip
- `ime/text/CandidateUpdateCoordinator.kt` — receives `Outcome<List<TaigiWord>, …>` from autocomplete service, no direct `LexiconService` import to rewire
- Tab3 dictionary explorer — calls `searchWithSources` / `searchByHanzi`
- `engine/RustEngineBridge.kt` — gains a `LexiconBridge` companion (see §6)
- `ime/CompositionRoot.kt` — owns `lexiconService` singleton; ctor wiring replaced

---

## 4. Cross-platform divergences (with source commits)

Identified by side-by-side read of the 11 in-scope files (iOS 6 + Android 5). Each row is either ported as-is, resolved during the Rust port, or categorized as a parity-correction packaged with this slice. **Tier classification under `rules/cross-platform-alignment.md`**: D-1, D-2, D-3, D-4, D-5, D-6, D-7, D-9, D-10, D-11 are intentional divergences or structural differences resolved by the Rust port without changing observable behavior. **D-8 and D-12 are parity-corrections toward Android per §1b** — both ship in this slice with their own `INVARIANT_LEX_*` tests and (for D-8) explicit user sign-off before implementation. None of the rows are emergency-tier (§1a).

| # | Concern | iOS | Android | Source commit / Resolution |
|---|---|---|---|---|
| D-1 | **`buildSearchKey` pre-normalize TPS step** | None — iOS calls `RustEngineBridge.normalizeInput(input)` directly; Rust normalize handles TPS preprocessing internally. iOS does TPS er↔or expansion **after** normalize, on the original `rawInput` + `segmentedInput`. | `InputNormalizer.buildSearchKey(input, mode)` runs `RustEngineBridge.tpsToTl(input)` BEFORE normalize when input contains TPS; called from `LexiconService.searchWithTrie` (line 235). Result: Android double-applies TPS preprocessing (once in `buildSearchKey`, once in Rust `normalize_input`). | iOS path is canonical. Rust normalize already handles TPS — Android's `buildSearchKey` is redundant. **Resolution**: drop `buildSearchKey` in this slice; both platforms call the Rust lexicon search method which handles input as iOS does today. Origin: `b8c1ccf` (Android binary mmap conversion) brought a separate per-platform pipeline; D9.4-Phonetics (`bfec83c`) made iOS converge to Rust normalize but did not touch Android `buildSearchKey`. |
| D-2 | **TPS er↔or expansion guard** | `containsTPS(rawInput) && isTpsOrMappedToER && segmentedInput.contains("er") → search "or" variant of segmentedInput` | `containsTps(input) && isTpsOrMappedToER → tlInput = tpsToTl(input); if tlInput.contains("er") → search "or" variant of tlInput` | Same intent, different flow. iOS uses the `segmentedInput` directly because it already passed through Rust normalize. Android uses `tpsToTl(input)` because it needs the TL form. **Resolution**: Rust `engine/lexicon` runs the `er↔or` variant search internally given a single `input` + `tps_to_er_enabled` settings flag; both platforms call the same method. The "search the variant only when input contains TPS" gate is an engine-side decision (engine reads its own normalize output). |
| D-3 | **Auto-cap layer location (NOT a feature gap)** | `LexiconService.applyCaseProcessing` runs `CandidateProcessor.capitalize` on both custom-dict and system words **inside the lexicon pipeline**, before ranking. iOS auto-caps at the engine-adjacent layer. | Android **does** auto-cap, but at the **display layer** — `SmartbarManager.updateCandidates` calls `SuggestionCaseTransformer.transform(...)` with the active caps + caps-lock state from `TextInputManager.getCapsState()`. The `LexiconService` in Android intentionally returns un-capitalized rows because the smartbar owns the caps-state read. | Layer divergence, not feature gap. Both platforms apply auto-cap; they do it at different layers. Origin: iOS `adde8ea` (engine-adjacent) vs Android pre-existing smartbar-layer transform. **Resolution**: irrelevant under the Rust port — `engine/lexicon::search` returns **neutral, un-capitalized rows** on both platforms (matches `rules/cross-platform-alignment.md` §5.1 "candidate UI navigation, layout semantics, or styling" stays platform). iOS keeps `applyCaseProcessing` as the post-bridge engine-adjacent step; Android keeps `SuggestionCaseTransformer.transform` as the post-bridge display-layer step. No behavior change on either platform. **No follow-up parity work needed for this divergence**; the previously-considered `LX-FU-1` is withdrawn. |
| D-4 | **Cold-start branch (`mergeOrderOnly`)** | `LexiconService.processCandidates` checks `userFrequencyService.isConnected()`; when false, calls `RustEngineBridge.processCandidates(..., mergeOrderOnly: true)` to skip score-sort but still run engine dedup. Preserves "merged-order on cold-start" — custom-dict ahead of system. | None — Android `UserFrequencyService.frequencyDataBatch` is always callable (per `b71bb8f` Phase II A1 service-graph DI); no cold-start branch needed. | Intentional. iOS has a separate freq-DB connection lifecycle (App Group SQLite); Android's freq layer ensures init is synchronous-callable. **Resolution**: keep iOS-side branch; engine doesn't need to know. The `processCandidates` bridge call already supports `mergeOrderOnly` since v3.5.2 (`c0e51ee`). |
| D-5 | **TPS dedup gate source** | `tpsDedupEnabled = (inputMode == .tps)` — typed enum comparison, never null | `tpsDedupEnabled = (settings?.inputMode == "tps")` — string comparison via nullable settings; null = "skip dedup" | Bridge call surface accepts a `bool`; the gate **decision** is platform-side. **Resolution**: Both platforms construct the bool the same way — derive from current input mode at call time. The string vs enum comparison is internal platform plumbing, no engine impact. |
| D-6 | **DictionaryRepository class on iOS only** | iOS factors `DictionaryRepository` (223 LOC) out of `LexiconService` (`0fcaef5 refactor(iOS): Extract shared lookupRowIds()`) — clean separation between trie+binary orchestration and search facade. | Android inlines everything in `LexiconService.kt` — `searchWithTrie`, `lookupRowIds`, `buildSearchResults` are private members (540 LOC). | Structural difference, not behavioral. **Resolution**: irrelevant under the Rust port — `engine/lexicon/src/search.rs` becomes the single orchestrator on both platforms; the iOS Repository class and the Android inlined methods both delete. |
| D-7 | **Custom-dict prefix derivation** | `CustomDictionaryDerivation.searchPrefix(for:)` returns `(prefix, isToneAware)` — centralized helper used by `lookupCustomDictionary` | Inline logic in `LexiconService.lookupCustomDictionary` — `isToneAware = input.any { it.isDigit() }`, then branch on tone-aware vs `CustomDictionaryDerivation.generateNotone(input)` | Same end behavior; different layering. **Resolution**: out of scope (custom-dict is SQLite write path = future slice). Custom-dict lookup stays platform in v3.5.6; the divergence stays as-is. |
| D-8 | **Hanzi-input guard ordering vs custom-dict lookup** | iOS guards `inputType != .hanzi` **inside** `DictionaryRepository.query` (line 52), which runs **after** `LexiconService.lookupCustomDictionary` (line 96). Net behavior: hanzi input still hits the custom-dict SQLite query (which may return entries), but the system-dict path returns empty. | Android guards `inputType is InputType.Hanzi` at the **top** of `LexiconService.search` (line 76), **before** custom-dict lookup. Net behavior: hanzi input returns `[]` immediately — no custom-dict, no system-dict. | **Real behavioral divergence, not just a layer difference.** Origin: layering decisions in independent platform refactors. **Resolution**: lock at audit time — Android's "early-return-empty" is canonical for the Rust port (consistent with iOS-only semantics being a side-effect of layering, not an intentional product decision). The Rust `engine/lexicon::search` rejects hanzi input first (mirrors Android position). iOS platform-side wrapper deletes the now-redundant pre-engine custom-dict call when `inputType == .hanzi` (custom-dict still callable for non-hanzi inputs as today). This **does** change iOS behavior for hanzi input: a hanzi query that previously returned custom-dict entries now returns `[]`. Categorized as a **parity-correction toward Android** under `rules/cross-platform-alignment.md` §1b — packaged with this slice (it's a single-commit shape that ports without dual-source-of-truth). Add an `INVARIANT_LEX_HANZI_GUARD` test pinning the `[]` return on both platforms post-port. **Plan must lock**: confirm with user before implementation that the iOS hanzi-input → custom-dict regression is acceptable. If user objects, the Rust port instead preserves iOS semantics by accepting hanzi input through search (returning `[]` for the system half) and requiring the platform to gate custom-dict lookup independently. |
| D-9 | **mmap reader ownership lifecycle** | `DictionaryBinaryReader` owned by `DictionaryRepository`; opened lazily in init, never explicitly closed. iOS Bundle lifetime > app lifetime, so mmap'd `Data` lives forever. | Owned by `LexiconService` (`@Volatile var binaryReader`); opened in `ensureInitialized`; `close()` resets to null — subsequent searches reopen via `ensureInitialized`. Asset-copy-from-APK runs every app version bump (`copyAssetsIfNeeded`). | Intentional — Android needs the close+reopen seam because asset files live in `filesDir` which can be replaced on app upgrade while the IME is running. **Resolution**: `engine/lexicon` owns the mmap inside `EngineHandle`; Rust exposes `engine_lexicon_reinstall(paths)` which atomically swaps the mmap'd readers. iOS calls reinstall once at app launch and never again; Android calls it after `copyAssetsIfNeeded` succeeds on a version bump. See §5 for the contract. |
| D-10 | **Trie native bridge: handle vs singleton** | `marisa_bridge.cpp` exposes `trie_handle_t trie_create(path)` — handle-based, supports up to 8 simultaneous tries. iOS only ever creates one (the dictionary trie) but the C bridge is generic. | `trie_jni.cpp` exposes a singleton `g_trie` — only one trie can ever be loaded per process. Android pays the simplicity cost. | Both delete in this slice. **Resolution**: Rust `engine/lexicon::EngineHandle` is a per-IME singleton (mirrors composing + nextword pattern, per `engine/composing/src/handle.rs`). One prefix index per engine handle. The handle-vs-singleton distinction goes away. |
| D-11 | **Asset path origin** | iOS reads bundled assets directly from `.appex` Bundle via `ResourceBundleResolver.dictionaryBundle` — no copy step. App Group container is **only** used for SQLite write paths (`SharedDatabasePath`); read-only assets stay in Bundle. | Android copies bundled assets from `assets/` to `filesDir` via `LexiconService.copyAssetsIfNeeded` (gated by `dictionary_app_version.txt` stamp) and `TrieService.getTriePath` (gated by `trie_app_version.txt` stamp). Native code mmaps the copied file in `filesDir`, NOT the APK asset entry directly (Android assets are inside the APK ZIP, not directly mmap-able). | Intentional, structural. **Resolution**: platform retains the asset-copy + version-stamp mechanics; Rust receives stable absolute paths via `LexiconPaths` snapshot at install/init time (§6). The platform decides when paths change (Android: on version bump; iOS: never within app session). |
| D-12 | **Trie rowid dedup ordering** | `lookupRowIds` returns `Array(Set(exactRowIds + prefixRowIds))` — `Set<Int>` is unordered. The downstream `processCandidates` Rust ranking sorts by score, so non-tied scores recover order; **tied scores rely on the iteration order of the underlying Swift `Set<Int>` hash table** (deterministic per session but not stable across runs). | `lookupRowIds` returns `(exactRowIds + prefixRowIds).distinct()` — preserves first-seen insertion order. Tied-score candidates surface in deterministic insertion order. | Real divergence visible only on tied-score ties. Per the v3.5.5 NextWord locked decision "Iteration order: IndexMap insertion order (parity correction toward Android)" (`project_v3_5_5_nextword_progress.md`), Android's insertion-order behavior is canonical. **Resolution**: Rust `engine/lexicon::search::lookup_rowids` uses `IndexSet<u32>` (insertion-order-preserving dedup) — matches Android, parity-corrects iOS. Categorized as a **parity-correction toward Android** under `rules/cross-platform-alignment.md` §1b — packaged with this slice. Subtle UX shift on iOS: tied-score candidates may surface in a different (now stable + deterministic) order. Add an `INVARIANT_LEX_LOOKUP_ROWIDS_ORDER` test pinning the IndexSet semantics post-port. |

**Cross-platform invariant constants** unaffected by this slice (already pinned in code via `// CROSS-PLATFORM INVARIANT — mirrors …` comments per §3a):

- 12 bitmask bit positions (`bitToSource` iOS ↔ `BIT_TO_SOURCE` Android)
- `passesFilter` 3-layer logic (variant exclusion → khiin exclusion → source-OR-with-dev)
- Association `passesFilter` 1-layer logic
- Trie key prefixes `tl:` / `poj:` / `hanzi:`
- Binary file magic bytes (`TKDB`, `TKWA`) and version `1`
- Little-endian byte order
- Strict UTF-8 decoding (reject record on invalid bytes)

These all port to Rust as-is; the inline `// CROSS-PLATFORM INVARIANT` comments come down because the constants exist in only one place (Rust crate) post-port.

---

## 5. fst vs marisa-rs offline spike

Per scope memo D4, the format gate is decided by an offline spike. Conducted 2026-05-01; results below.

### 5.1 Method

1. Loaded current `dictionary/output/dictionary.trie` via Python `marisa_trie` 1.4.1 — extracted **1,062,540 (key, rowid)** entries (451,740 `tl:` + 451,740 `poj:` + 159,060 `hanzi:`; avg key length 10.7 UTF-8 bytes).
2. Encoded each entry as `key_utf8 + 0xFF + rowid_u32_le` (the same wire format MARISA RecordTrie uses internally per `binary-format.md` §3.1) and dumped to a binary buffer.
3. Built an `fst::Set` (crate `fst = 0.4`) over the BTree-sorted, deduped key list and wrote `dictionary.fst` to disk.
4. Reopened via `memmap2::Mmap`, ran prefix lookups via `Set::range().ge(prefix)` streaming until the prefix matched no longer, decoded trailing rowid by splitting on the last `0xFF`.
5. Compared against a Python MARISA baseline running the same prefix queries.

Spike crate: `/tmp/lex-spike/fst-spike` (intentionally outside the workspace; not committed). Reproducible from scratch by re-running the dump script + `cargo build --release` in the spike crate.

### 5.2 Bundle size

| Asset | Size | Delta |
|---|---:|---:|
| MARISA `dictionary.trie` | 5,482,264 bytes (5.23 MB) | (baseline) |
| fst `dictionary.fst` | 9,605,720 bytes (9.16 MB) | **+3.93 MB (+75%)** |

Total bundled-asset size: today ~12 MB (`dictionary.trie` + `dictionary.bin` + `association.bin`); with fst swap, ~16 MB. Well within iOS app + extension size budget; no bundle-size blocker.

### 5.3 Prefix lookup latency (release build, 50-iter average per probe)

| Prefix | Hits | fst (Rust release) |
|---|---:|---:|
| `tl:gua` | 1,216 | 86.7 µs |
| `tl:hoo` | 1,615 | 112.9 µs |
| `poj:goa` | 1,223 | 90.8 µs |
| `hanzi:好` | 386 | 28.9 µs |
| `tl:kau` | 2,013 | 149.9 µs |
| `tl:sing` | 1,352 | 102.3 µs |

The empirical bound that matters: **all probed prefixes complete in low-hundreds-of-microseconds territory on release-built mmap'd fst**, well under the 50ms keystroke roundtrip budget cited in `rules/rust-best-practices.md` §5 ("async runtime: none — sync only … <50ms keystroke roundtrip"). Per-hit cost ≈ 0.07 µs/hit (linear in hits, as expected for streaming). The Python `marisa_trie` baseline numbers (300–400 µs typical) are **not** included as a comparison column — interpreter + FFI overhead make Python-vs-Rust comparisons indicative only. **No claim is made about fst-vs-native-MARISA relative latency** on real devices; the spike's purpose is to confirm fst latency is in-budget, not to argue fst is faster than MARISA.

### 5.4 Prefix semantics parity

✅ **Hit-set parity confirmed for the 6 probed prefixes**. The fst::Set raw-key encoding `key + 0xFF + rowid_u32_le` mirrors the wire format `marisa_trie.RecordTrie` produces (separator + payload), so `Set::range().ge(prefix)` returns the same `(key, rowid)` pair set that `predictive_search(query=prefix)` returns, given the same stop-on-prefix-mismatch behavior the spike implements. `tl:gua` returned 1,216 pairs in both implementations; the same equality held for the other five probes (5.3). The plan-time `INVARIANT_LEX_*` test suite expands this fixture to multiple prefix lengths per `tl:` / `poj:` / `hanzi:` family before commit-merge to lock parity beyond the 6-probe spike.

### 5.5 Rowid payload preservation

✅ **Direct**. Decoding logic is identical: split at the last `0xFF` byte, decode trailing 4 bytes as `u32_le`. Sample dump from the spike (first 20 hits for `tl:gua`):

```
tl:gua2ai3li2            -> rowid=136060
tl:gua2ai3li2            -> rowid=106930   ← duplicate key, different rowid (RecordTrie one-key→many semantics)
tl:gua2ai3tai5gi2        -> rowid=136062
…
tl:gua2bo5tsinn5         -> rowid=106935
tl:gua2bo5tsinn5         -> rowid=106937   ← same
tl:gua2e5                -> rowid=145503
tl:gua2e5                -> rowid=145504
…
```

The `(key, rowid)` composite encoding preserves MARISA's "multiple values per key" semantics because each `(key, rowid)` pair becomes a distinct fst key. Sorted dedup on insertion is automatic via BTreeSet; total count stays 1,062,540 (no actual duplicates after composite encoding — every `(key, rowid)` pair is unique in the source data).

### 5.6 Format gate decision

**fst APPROVED for v3.5.6 lexicon slice.** All four scope-memo gates pass:

- ✅ Lookup latency: tens-to-low-hundreds of µs across all probed prefixes (well under keystroke budget)
- ✅ Bundle size: +3.93 MB (acceptable on a ~12 MB asset baseline; total ~16 MB stays under iOS extension budget)
- ✅ Prefix semantics parity: hit-set parity confirmed for the 6 probed prefixes (`tl:gua`, `tl:hoo`, `poj:goa`, `hanzi:好`, `tl:kau`, `tl:sing`); plan-time `INVARIANT_LEX_*` tests expand the fixture coverage before merge
- ✅ Rowid payload preservation: composite `(key, rowid)` encoding direct

**Operational consequences:**

- Module name **stays `prefix_index.rs`** (per scope D4 weak preference toward fst — fst path uses `prefix_index.rs`; marisa fallback would have used `trie.rs`).
- Crate dep: add `fst = "0.4"` to `engine/lexicon/Cargo.toml` workspace deps. The `memmap2` dep lives in a **separate small helper crate** (see next bullet), not in `engine/lexicon` itself.
- `unsafe_code = "forbid"` policy reconciliation: `rules/rust-best-practices.md` §4 mandates `#![forbid(unsafe_code)]` at the root of `phonetics` and `engine` crates; `memmap2::Mmap::map` requires `unsafe`. The audit defers the precise resolution to plan time but locks **two viable shapes**, both of which keep `engine/lexicon` itself unsafe-free:
  - **Option A (preferred)** — new sibling crate `engine/mmap-host` (or `engine/bundled-asset`): `~30 LOC`, `#![allow(unsafe_code)]` at crate root, single SAFETY-commented `unsafe` block wrapping `Mmap::map`, exposes a safe `Arc<Mmap>` wrapper and a safe `&[u8]` view. `engine/lexicon` depends on it. Pros: keeps the main domain crate unsafe-free; matches the FFI-marshaling carve-out spirit of §4 ("`unsafe` blocks are confined to FFI marshaling"). Cons: adds a tiny crate.
  - **Option B** — push mmap into `swift-ffi` / `android-jni` (which already are `unsafe`-allowed): the FFI crates own the `Arc<Mmap>` and pass an `Arc<[u8]>`-shaped byte slice to `engine/lexicon::install`. Pros: zero new crates. Cons: mmap state crosses the FFI-thinness boundary cited in §1 ("FFI crates are thin — protobuf in / protobuf out").
  - The plan locks one of A or B before ExitPlanMode. Either choice keeps `engine/lexicon` itself `forbid(unsafe_code)`-clean — the rules deviation is contained to a tiny adjacent crate, not punched into the domain crate.
- Build pipeline: `dictionary/build/create_trie.py` must emit `dictionary.fst` instead of (or alongside) `dictionary.trie`. Atomic switch in same PR — keeping both doubles bundle size.

**marisa-rs path retired** (no fallback needed):

- Spike disconfirmed concerns about fst's "format wouldn't match MARISA's record-trie semantics" — the composite encoding is direct.
- Avoids the `cxx::bridge`-in-our-crate-vs-in-dependency lint conflict cited in scope memo D4.
- Avoids carrying a C++ build dependency through `cargo-ndk` + `cargo build --target aarch64-apple-ios` matrices.

---

## 6. `LexiconPaths` install/init injection contract (draft)

Per scope memo D3: "One-time `LexiconPaths` snapshot at engine install/init … paths are stable, lifetime-sensitive. Do **NOT** put them in per-request `AppConfig`."

### 6.1 Value type (Rust-side)

```rust
// engine/lexicon/src/paths.rs (~50 LOC budget)
#[derive(Clone, Debug)]
pub struct LexiconPaths {
    /// Absolute path to the prefix-index file (fst format).
    /// iOS: <bundle>/dictionary.fst (read directly from .appex Bundle).
    /// Android: <filesDir>/dictionary.fst (post-copy from assets).
    pub prefix_index: PathBuf,

    /// Absolute path to the dictionary record file (TKDB binary).
    /// iOS: <bundle>/dictionary.bin.
    /// Android: <filesDir>/dictionary.bin (post-copy).
    pub dictionary_bin: PathBuf,

    /// Absolute path to the bundled bigram file (TKWA binary).
    /// READ-ONLY bundled half. The mutable per-user user_association.db
    /// is NOT in this struct — that path lives in a future write-path
    /// LexiconWritePaths struct.
    pub association_bin: PathBuf,
}
```

### 6.2 FFI surface (proto-shaped — extends existing `lexicon.proto`)

`engine/protos/proto/lexicon.proto` already exists from the v3.5.2 ranking slice and owns `LexiconRequest.method.process_candidates = 10`. The v3.5.6 slice **extends** the existing envelope by adding new method tags; **`process_candidates = 10` and any other already-allocated tags are preserved.**

New methods (tag numbers 11+ — exact allocation locked at plan time):

- **`Install(LexiconPathsProto) → InstallResult`** — engine opens mmaps + builds in-memory readers, stores in `EngineHandle`. Idempotent: subsequent calls atomically swap to new paths (drop old mmaps after new ones load successfully, ordering enforced by Rust struct-field-drop order inside `EngineHandle`). Errors: `PathNotFound`, `InvalidMagicOrVersion`, `Corrupted` (UTF-8 decode of header fails), `IoError(String)`.
- **`Shutdown() → ()`** — engine drops mmaps + readers. Called by platform on app termination (best-effort) or before re-install. `Drop` impl on the handle does the same teardown — idempotent per `rules/rust-best-practices.md` §2 ("explicit `shutdown(handle)` FFI + `Drop` impl with same teardown path"). `Install` calls before shutdown are still legal — engine handles the swap atomically without requiring an explicit shutdown.
- **`Search(SearchRequest) → SearchResult`** — replaces today's iOS `DictionaryRepository.query` + Android `LexiconService.searchWithTrie`.
- **`SearchWithSources(SearchRequest) → SearchWithSourcesResult`** — replaces `DictionaryRepository.searchWithSources` (iOS) + `LexiconService.searchWithSources` (Android).
- **`SearchByHanzi(SearchByHanziRequest) → SearchWithSourcesResult`** — replaces `DictionaryRepository.searchByHanzi` (iOS) + `LexiconService.searchByHanzi` (Android).
- **`AssocLookup(AssocLookupRequest) → AssocLookupResult`** — bundled-half `association.bin` reader; called from platform NextWord caller (replacing platform `AssociationBinaryReader.lookup` on both sides). See §8 + §12 for the cross-crate isolation argument that justifies routing NextWord through this FFI roundtrip rather than a Rust-side direct dep on `engine/lexicon`.

`LexiconPathsProto` proto:

```protobuf
message LexiconPathsProto {
  string prefix_index = 1;
  string dictionary_bin = 2;
  string association_bin = 3;
}
```

### 6.3 Lifecycle

- **iOS**:
  1. `KeyboardViewController.viewDidLoad` (or `CompositionRoot` ctor) computes paths via `ResourceBundleResolver.dictionaryBundle` and `Bundle.url(forResource:withExtension:)`.
  2. Calls `RustEngineBridge.lexiconInstall(LexiconPaths(prefix_index: …, dictionary_bin: …, association_bin: …))` once.
  3. **Never reinstalls within an app session** — Bundle lifetime > app session.
  4. App teardown — no explicit shutdown call needed (process exit drops the handle; `Drop` runs).

- **Android**:
  1. `LexiconService.ensureInitialized` (or `CompositionRoot` boot) calls `copyAssetsIfNeeded` + `TrieService.getTriePath` (or its replacement once `TrieService.kt` is deleted — the asset-copy + version-stamp logic stays platform; only the C++ trie wrapper goes away).
  2. After copy succeeds, computes paths to `filesDir` and calls `RustEngineBridge.lexiconInstall(...)`.
  3. **Reinstalls on app version bump only** — `dictionary_app_version.txt` and `trie_app_version.txt` stamps drive the decision. The `copyAssetsIfNeeded` -> `lexiconInstall` sequence is atomic from the engine's perspective: engine opens the new mmap'd files (now in `filesDir`), then drops old mmaps after new ones load.
  4. IME service teardown — best-effort `lexiconShutdown` call; not strictly required (process exit handles it).

### 6.4 Why install-time, not per-request

- Paths are **stable** within a "lifecycle epoch" (iOS: app session; Android: until next version bump). Re-passing them on every search is pure overhead.
- `AppConfig` (per-request settings snapshot) carries booleans + enums + timestamps — small, immutable values that change frequently. Paths are large strings that change rarely; conflating the two violates the engine's memory budget assumption that `AppConfig` is cheap to clone.
- Mmap'd readers are **lifetime-bound** to the path — moving them per-request would require reconstructing the mmap on every keystroke, which is the exact pathology this contract avoids.

### 6.5 Error recovery

- If `LexiconInstall` fails (path not found, magic mismatch, etc.), engine returns `InstallResult.error_code` + message; platform shows a user-visible "dictionary unavailable" state and retries on next session. No partial-install state — engine atomically commits or rejects.
- If `LexiconSearch` is called before `LexiconInstall` succeeds, engine returns `EngineError::EngineNotInstalled` (encoded into `Response.error_code`) — platform falls back to "dictionary unavailable" UI, identical to the old `repository.binaryReader == nil` branch on iOS / `Outcome.Failure(DictionaryError.DatabaseNotAvailable)` on Android.

### 6.6 Plan-time follow-ups

- Decide whether `LexiconPaths` lives in `engine/lexicon` (current draft) or in `protos` (so other crates can share it). Lean toward `engine/lexicon` since paths are slice-private; future write-path slice gets its own `LexiconWritePaths` (also slice-private).
- Decide on the proto envelope command code. Must be a new `CMD_LEXICON_*` arm.

---

## 7. Risk matrix

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | **mmap lifetime** — Rust `Mmap` dropped while readers still hold byte-slice references into the mapped region → segfault | Low (with discipline) | Crash | `engine/lexicon::EngineHandle` holds `Arc<Mmap>` for each of (`prefix_index`, `dictionary_bin`, `association_bin`). **Each reader struct keeps its own clone of the `Arc<Mmap>` and indexes through it on demand** (`let bytes: &[u8] = &mmap_arc[..]; bytes.get(offset..)`); the borrow of `&[u8]` is bounded by the reader's `&self` lifetime, not by the handle's lifetime. No `'static` lifetime forgery; no self-referential structs. `Shutdown` drops the handle which drops the readers (each releasing its `Arc<Mmap>` clone); the `Mmap` itself is freed once the last `Arc` clone goes out of scope. Reinstall: build new `Arc<Mmap>` first, swap the atomic field on `EngineHandle` (`std::mem::replace` under `Mutex`), drop old `Mmap` after new readers are live. |
| R2 | **iOS App Group / extension memory budget** — keyboard extension memory is capped (~70 MB on older iOS); mmap'd files count toward virtual memory | Low | Crash on memory pressure | `mmap` is page-resident-on-demand. Total mmap'd region = 9.16 (fst) + 4.4 (TKDB) + 3.1 (TKWA) = ~16.7 MB virtual; resident pages much lower (page cache managed by OS). Well under extension budget. Bundled assets are NOT in App Group container — they live in the `.appex` Bundle directly (per `ResourceBundleResolver`); the App Group is reserved for SQLite write paths (`SharedDatabasePath`) which stay platform in this slice. **No App Group entitlement change needed.** |
| R3 | **Android assets vs internal storage** — Android trie/bin files must be copied from APK assets to `filesDir` because APK ZIP entries cannot be mmap'd directly | Medium (already mitigated) | Functional regression on app upgrade | Platform retains `LexiconService.copyAssetsIfNeeded` + `TrieService.getTriePath` (or renamed equivalent post-deletion of `TrieService.kt`); the copy + version-stamp logic is preserved. Engine receives stable `filesDir` paths via `LexiconPaths` after copy succeeds. Hot-rotation: if APK upgrades while IME is running, the old mmap remains valid (file unlinked but inode held); new mmap installs after process restart. **Already battle-tested in the current implementation** — no new risk from the Rust port. |
| R4 | **fst format build pipeline change** — `dictionary/build/create_trie.py` must produce `dictionary.fst` instead of (or alongside) `dictionary.trie`; readers everywhere must switch atomically | Medium | Build break; bundled-asset bundle size doubles if both formats ship | Plan (next round) covers atomic switch in the same PR: build pipeline emits `.fst` only; old `.trie` artifact removed from `dictionary/output/` and from both platform asset directories (`ios/Resources/Dictionaries/`, `android/app/src/main/assets/`); platform readers (Rust) only know `.fst`. Bundle delta is +3.93 MB net (replacing `.trie` not adding to it). Decision tagged as commit slicing item in plan. |
| R5 | **fst ↔ MARISA semantic drift** — fst::Set range API does not exactly match MARISA `predictive_search` for some pathological prefixes | Low (spike-validated) | Missing or duplicate hits | Spike result §5.4 confirmed direct parity for 6 representative prefixes (1,216 / 1,615 / 1,223 / 386 / 2,013 / 1,352 hits). Plan's `INVARIANT_LEX_*` test suite asserts post-port hit count matches pre-port for an expanded fixture set covering each prefix family (`tl:`, `poj:`, `hanzi:`) at multiple lengths. |
| R6 | **Build pipeline emits non-deterministic fst** — fst output bytes depend on insertion order, which depends on Python sort stability for sources with same key | Low | Diff churn in `dictionary.fst` between rebuilds | fst::SetBuilder requires sorted-input; build script sorts deterministically by `(key, rowid)` ascending before insertion. BTreeSet dedup gives stable ordering. Asserted by build-script test (`pytest dictionary/build/test_create_trie.py` if present, or added in plan). |
| R7 | **Cross-process readers** — keyboard extension and main app both mmap the same Bundle file simultaneously | Low | None expected — read-only mmap is safe | mmap MAP_PRIVATE / MAP_SHARED for read-only files is safe on both platforms. Verified by today's behavior (both processes already mmap `dictionary.bin` independently in current code). No change. |
| R8 | **`unsafe_code = "forbid"` violation in mmap** — `memmap2::Mmap::map` requires `unsafe` | Certain | Workspace lint rejects | `engine/lexicon` itself stays `#![forbid(unsafe_code)]` per `rules/rust-best-practices.md` §4. The `unsafe` block lives outside `engine/lexicon` per §5.6 plan-time choice between Option A (new sibling crate `engine/mmap-host`, `~30 LOC`, `#![allow(unsafe_code)]`, single SAFETY-commented `unsafe`) or Option B (push mmap into `swift-ffi` / `android-jni` which are already unsafe-allowed). Every `unsafe` block ≤ 3 lines with `// SAFETY:` comment; each requires Codex pre-impl review per `rules/cross-platform-alignment.md` §1c. |
| R9 | **Per-platform branch growth in Rust** — TPS er↔or expansion + custom-dict orchestration could leak platform-specific logic into `engine/lexicon` | Medium | Slice scope creep | Engine takes `tps_to_er_enabled: bool` settings flag and runs the `er↔or` variant search transparently. Custom-dict lookup STAYS PLATFORM in v3.5.6 (it's SQLite); the platform calls `engine_lexicon_search(input, settings)` and merges the custom-dict result with the engine-returned system result before passing both into `processCandidates`. Engine never sees custom-dict rows in this slice. |
| R10 | **`LexiconService.kt` 540 LOC violates AI-friendly file size cap** | Certain (today) | Tooling friction; Codex review truncation | Slice DELETES the in-scope portion (~half the file). The remaining platform-side code (init, asset-copy, custom-dict orchestration, ranking call) targets <300 LOC. If the post-port file still exceeds 500 LOC, split into `LexiconService.kt` (facade) + `LexiconAssetCopy.kt` (init/version-stamp) + `LexiconBridge.kt` (per scope memo §Bridge layout, already separate top-level object). |
| R11 | **Logging bridge bootstrap order** — engine log calls before platform `log::set_logger` registration go to `/dev/null`; install errors during early boot are silently lost | Medium (boot path) | Debuggability regression on install failures | Platform-side `CompositionRoot` calls `RustEngineBridge.installLogger()` BEFORE the first `RustEngineBridge.lexiconInstall(...)` (or any other engine call). Plan must enumerate the boot order in `engine_init.md` (or equivalent) and pin an `INVARIANT_LOGGER_BEFORE_INSTALL` test that asserts the order via observable logs. Same pattern composing + nextword slices already follow; no new bootstrap shape, just enforce the order on the new install path. |
| R12 | **Malformed / hostile path strings** — platform passes a UTF-8 string that contains interior NUL bytes (NUL U+0000 is valid UTF-8 and valid proto3 `string`, but breaks any C-string conversion), traversal segments (`..`), or non-canonical paths to `LexiconInstall`; engine opens unintended files, panics on `CString::new`, or follows symlinks unintentionally | Low | Could open arbitrary files within app sandbox; degraded crash-mode if the open fails non-gracefully | Engine explicitly validates each path string before use: rejects strings containing `\0` with `EngineError::InstallPathContainsNul`; rejects non-absolute paths with `EngineError::InstallPathRelative`; does NOT canonicalize (avoids `realpath`-induced symlink follow — platform owns path canonicality). `Install` errors return as proto error variants — never panic on a bad path. App sandbox enforces filesystem boundaries below the engine. |
| R13 | **Install/reinstall race with in-flight searches** — concurrent FFI calls: thread A is mid-`Search` while thread B calls `Install` → reader holds an `Arc<Mmap>` clone but the handle's mmap field is being swapped | Low (Mutex serializes) | Worst case = transient stale-result | All install + reinstall + search FFI calls go through the single `Mutex<Engine>` per `rules/rust-best-practices.md` §2 ("Engine state is `Send + !Sync`; the mutex serializes concurrent native calls"). Search holds the mutex for its whole duration; install completes the swap atomically before releasing. No intermediate state where a half-installed handle is observable to a search. The `Arc<Mmap>` lifetime guarantee from R1 means even if a reader clone outlives the swap, the underlying mmap stays alive. **Concrete gate**: `cargo test --workspace` includes a multi-thread regression test that fans out 8 threads alternating Search/Install, asserts no panic + every Search returns a coherent result set. |

**No P0 risks identified.** R3, R4, R8, R9, R10, R11, R12, R13 are mitigated by existing patterns in the codebase or by plan-encoded ordering / serialization; R1, R2, R5–R7 are inherent to mmap-based reading and were already accepted by the v3.4.7 binary-format conversion (`84a7c53` / `b8c1ccf`).

---

## 8. Persistence boundary justification (OPT-A)

Per cadence release map (decided 2026-04-27) and scope memo `project_v3_5_6_lexicon_scope.md` (locked 2026-05-01): v3.5.6 ships **read path only**. SQLite write paths (`UserFrequencyRepository`, `CustomDictionaryRepository`, NextWord `user_association.db`) defer to a future write-path slice without committing a version number (per `feedback_no_future_planning.md`).

**Why OPT-A**:

1. **Cadence alignment** — pulling the write half forward would either collapse two slices into one mega-PR (~30 commits, 3 new Rust deps `marisa/fst` + `memmap2` + `rusqlite`) or duplicate the SQLite work later. Both violate "one slice per release" + blast-radius cadence rule.
2. **Pure-function shape** — read path is `query → rows`. No concurrency drama, no schema migration, no SQLite write ownership in this slice.
3. **Reference parity** — khiin-rs `khiin_db` is read-only, separate from write-side stores. Same boundary.
4. **Establishes the install-time path-snapshot pattern** — `LexiconPaths` snapshot at engine init (mirrors khiin-rs `loadSettings → AppConfig` cache). The future write-path slice layers `LexiconWritePaths` (SQLite paths) on top using the same install-time pattern.
5. **Bridge surface contract decoupling** — read path uses a stateless lookup model (input bytes in → row bytes out); write path needs a different shape (transaction boundaries, schema migration ownership). Mixing the two in one slice forces premature shape decisions.

**Boundary contract for v3.5.6** (DB ownership preserved; bundled-asset reader ownership changes):

- **Engine**: owns `dictionary.fst` (post-format-swap) + `dictionary.bin` mmap + bundled-half `association.bin` mmap. Single owner of all three readers is `engine/lexicon` (no parallel reader in any other crate). Public surface: `LexiconRequest.method.{ Search, SearchWithSources, SearchByHanzi, AssocLookup, Install, Shutdown }` (extends existing `LexiconRequest` envelope which today owns `process_candidates = 10`).
- **Platform**: SQLite write paths unchanged; calls `engine_lexicon_search` + merges with custom-dict rows (still SQLite); calls `processCandidates` for ranking (already Rust since v3.5.2). Platform NextWord caller switches its bundled-association lookup from `AssociationBinaryReader.lookup(prevWord)` (deleted) to `RustEngineBridge.lexiconAssocLookup(prevWord)` — same data, FFI roundtrip across the same dispatch seam. **The `engine/nextword` Rust crate does NOT directly depend on `engine/lexicon`**; cross-crate isolation per `rules/rust-best-practices.md` §1 + `feedback_rust_extraction_goals.md` goal 1 (high cohesion + low coupling) is preserved by routing the call through the platform layer (or, equivalently in eventual write-path slice, through `engine/dispatch` which is allowed to depend on multiple domain crates).

**Note on row DTO**: `engine_lexicon_search` returns proto-shaped rows (`LexiconRow{id, roman, hanzi, length_score, source_bitmask}`). Platform converts to `TaigiWord` (Swift / Kotlin) at the bridge layer, identical to how composing + nextword + ranking already do it.

**v3.5.7+ will subsume**:

- `UserFrequencyRepository` + `UserFrequencyService` → `engine/lexicon-user-freq` Rust crate (or extension of `engine/lexicon`).
- `CustomDictionaryRepository` + `CustomDictionaryService` + migrations → `engine/lexicon-custom-dict` (or extension).
- NextWord `user_association.db` SQLite half → folded into `engine/nextword` (today only the bigram engine logic is in Rust; SQLite reads stay platform). At that point, the cross-crate isolation question (`engine/nextword` accessing the bundled `association.bin`) is revisited: either keep the FFI-roundtrip route (status quo from v3.5.6), or introduce a stateless `engine/bundled-bigrams` shared crate that both `engine/lexicon` and `engine/nextword` depend on. v3.5.6 does not pre-decide.

---

## 9. Slice scope decisions (locked under auto-mode)

| Q | Decision | Rationale |
|---|---|---|
| Q1 — DB boundary | **OPT-A**: bundled-asset readers + search orchestration Rust; SQLite write paths stay platform | Cadence map (v3.5.6 = Lexicon read half) + Codex 88% confidence pre-rounds |
| Q2 — Trie format | **fst** (spike-confirmed; §5.6) | Pure-Rust + `unsafe_code = "forbid"`-clean (with single mmap helper) + bundle delta acceptable |
| Q3 — `AssociationBinaryReader` boundary | **IN — bundled half only** | Wire-format reader; same code already deletes on both platforms post-port. The mutable user_association.db SQLite half stays platform. |
| Q4 — Custom-dict lookup | **OUT** | SQLite write path; future slice. Custom-dict orchestration stays in `LexiconService` platform-side. |
| Q5 — Auto-cap (D-3) | **Stays platform on both sides at different layers; engine returns neutral rows** | UI/display-layer concern per `rules/cross-platform-alignment.md` §5.1; both platforms already auto-cap (iOS engine-adjacent via `applyCaseProcessing`; Android display-layer via `SuggestionCaseTransformer.transform`). Engine returns un-capitalized rows on both platforms; each platform's existing post-bridge step continues to apply caps. **No follow-up parity work needed** — the previously-considered `LX-FU-1` is withdrawn (see D-3). |
| Q6 — TPS er↔or expansion (D-2) | **IN — moves into Rust** | Engine reads `tps_to_er_enabled` settings + decides internally; both platforms call the same `engine_lexicon_search`. Resolves D-1 redundancy. |
| Q7 — Cold-start branch (D-4) | **Stays iOS-platform** | Engine doesn't need to know; `processCandidates(mergeOrderOnly: true)` already exists since v3.5.2. |
| Q8 — Engine handle | **New `engine/lexicon::EngineHandle` singleton** | Mirrors composing + nextword pattern (per `engine/composing/src/handle.rs`); reuse rejected because slice owns its own mmap'd readers + lifetime |
| Q9 — Proto pattern | **`lexicon.proto` with `oneof` methods (`Search` / `SearchWithSources` / `SearchByHanzi` / `Install` / `Shutdown`) + `oneof` result variants** | Same shape as composing.proto + nextword.proto |
| Q10 — Branch / PR | **`phase4b/v3.5.6-lexicon-readpath`, single PR** | Same pattern as v3.5.4 / v3.5.5; commit slicing per plan |
| Q11 — `inputType` guard layer (D-8) | **At engine entry (`search.rs`)** | Single-source guard; platform layering becomes irrelevant |
| Q12 — DictionaryRepository class (D-6) | **Delete on iOS** | Rust `engine/lexicon::search` is the single orchestrator; iOS Repository class disappears with the rest of the in-scope iOS files |

---

## 10. Anti-pattern adjacency check

Per slice scoping rule (`project_rust_migration_cadence.md`): catalogue per-codepoint scans / static engine-derived tables / engine helpers used only by lexicon that must be pulled into this slice.

**Checked locations**: lexicon trie + binary readers + repository + service + custom-dict adjacent (out of slice) + autocomplete callers (out of slice).

| Pattern | Location | Disposition |
|---|---|---|
| Per-codepoint string scan | None inside read path; all scans live in `RustEngineBridge.normalizeInput` (already Rust) | N/A |
| Static engine-derived table on platform | `bitToSource` / `BIT_TO_SOURCE` (12-bit map) | **In slice** — moves to Rust as `engine/lexicon::dictionary_reader::BIT_TO_SOURCE` |
| Helper utility scoped to lexicon | `LexiconConstants.TriePrefix.prefix(for: mode)` (iOS) / `DictionaryConstants.triePrefix(mode)` (Android) | **In slice** — fold into `engine/lexicon::key_normalizer::trie_prefix(mode)` |
| Phonetic conversion calls inside lexicon | `RustEngineBridge.tlToPoj` (POJ display) / `containsTPS` / `tpsToTl` | **Become internal Rust calls** (`phonetics::api::*`) — no FFI roundtrip |
| Static engine-derived table on platform | `LexiconConstants.Search.defaultLimit = 200` (iOS) / `DictionaryConstants.DEFAULT_SEARCH_LIMIT` (Android) | **Stays platform** — defaults are call-site policy, not engine concern; engine takes explicit `limit` per request |
| Helper utility scoped to lexicon | `EnabledDictionaries.fromSettings(...)` / `EnabledDictionaries(from: settings)` | **Stays platform** — wraps platform settings; engine accepts `enabled_mask: u16` directly via proto |

**Conclusion**: no anti-pattern adjacencies leak outside the slice scope. The slice boundary is clean.

---

## 11. Cross-references

- Scope memo: `project_v3_5_6_lexicon_scope.md` — OPT-A scope, crate layout, naming bans, locked decisions D2–D8
- Round prompts: `project_v3_5_6_next_round_prompts.md` — Prompts 1 (this audit) → 2 (plan) → 3 (impl) → 4 (release)
- Cadence map: `project_rust_migration_cadence.md` — release slot v3.5.6 = Lexicon (DB + Trie behind FFI)
- Roadmap: `project_shared_core_roadmap.md` — IV-B order R3 step 7 (Lexicon DB reads = LAST slice; OPT-A pulls forward only the bundled-read half)
- Rust workspace rules: `rules/rust-best-practices.md` — workspace layout (§1), FFI safety (§2), error handling (§3), `unsafe` (§4), crate choices (§5; fst pinned), opaque handle (§10), non-goals (§11)
- Cross-platform alignment: `rules/cross-platform-alignment.md` §1c (shared-core-candidate constraint), §3a (invariant comments), §5.1 (Rust shared-core non-goals — UI / DB asset-copy stay platform)
- Binary format spec: `docs/engine/binary-format.md` — TKDB / TKWA / MARISA RecordTrie layout (still authoritative; §3 will need a section addition for fst format post-impl)
- Trie feature spec: `docs/engine/trie.md` — query flow + key prefixes (still accurate; bridge layer description updates post-impl)
- Composing slice as reference: `docs/engine/composing-slice-{audit,plan}.md`; `engine/composing/src/{lib,handle,dispatch,api,transition,derived}.rs`
- NextWord slice as reference: `docs/engine/nextword-slice-{audit,plan}.md`; `engine/nextword/src/*`
- Codex sandwich: `feedback_codex_review_sandwich.md` — pre + post Codex pre-impl gates required
- Slice toggles: `feedback_no_slice_toggles.md` — direct swap, no fallback toggle
- Path G: `feedback_path_g_delete_mirrors.md` — platform deletions ship in same PR
- AI-friendly: `feedback_rust_extraction_goals.md` — 4 goals applied throughout (cohesion / Rust idioms / SRP naming / file size)
- Phonetic completeness: `feedback_phonetic_completeness.md` — read paths route through `phonetics::api::*` for normalize/tone-mark detection; no dead-code inference

---

## 12. Open questions for plan finalization

Items the plan (Prompt 2) must lock before ExitPlanMode:

1. **Build-pipeline output**: confirm `dictionary/build/create_trie.py` modification scope. Does the existing pipeline have downstream consumers besides the iOS/Android readers? (Probably no — `create_trie.py` is the producer; readers are the only consumers. `dictionary/tools/query_trie.py` is a debug helper; needs a parallel `query_fst.py` or a format-detection wrapper.)
2. **Proto envelope tag allocation**: pick concrete tag numbers ≥ 11 for `Install`, `Shutdown`, `Search`, `SearchWithSources`, `SearchByHanzi`, `AssocLookup`. Existing `process_candidates = 10` MUST be preserved.
3. **Bridge synth types**: should `LexiconRow` / `LexiconAssocEntry` / `LexiconSearchResult` co-locate with `RustEngineBridge+Lexicon.swift` (iOS) / `LexiconBridge.kt` (Android), or live in a shared types file? (Scope memo says co-locate; confirm at plan time.)
4. **mmap unsafe placement**: lock Option A (new `engine/mmap-host` crate) or Option B (`swift-ffi` / `android-jni` own mmap and pass byte slices). Audit recommends A.
5. **fst commit slicing**: spike crate stays out of repo; plan must specify when the build-pipeline change lands relative to the engine code (recommendation: pipeline change in commit 1, engine in commits 2–N, both in same PR).
6. **D-8 hanzi-input semantics user-confirm**: explicit user sign-off that the iOS-side regression (hanzi input no longer hits custom-dict) is acceptable as a parity-correction toward Android. If user objects, fall back to "Rust accepts hanzi, returns `[]` for system half, platform gates custom-dict separately".

(D-3 auto-cap is NO LONGER an open question — both platforms already auto-cap at different layers; the slice keeps that as-is. The earlier `LX-FU-1` placeholder is withdrawn.)

---

**Audit closed at draft. Awaits Codex post-edit review per `feedback_codex_post_edit_review.md`** (this is a docs-only PR; the post-edit gate is the only Codex gate before merging the audit; the pre-impl gate runs against the plan, not the audit).
