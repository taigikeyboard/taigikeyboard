# Android State Audit — Phase II Pre-Phase

**Status**: authored 2026-04-19 as the Phase II kickoff deliverable, immediately after iOS Phase I G0–G10 closed (PR #141). Mirrors `ios-exemplar-plan.md` in purpose — a map between current Android code and the architectural target at `ios-exemplar.md` — but not in shape. Android diverges from iOS in singleton pattern, settings storage (DataStore vs SharedSettings), and module layout; those divergences are real and are flagged here rather than papered over.

**Purpose**: enumerate the concrete deltas between current Android code and the iOS exemplar so Phase II can be scoped as a set of mergeable task groups (A0–A10) rather than a monolithic "make Android match iOS" rewrite.

**Scope boundary**: audit-only. No code changes land with this doc. Task-group scopes below are planning inputs for Phase II rounds, not a commitment to any specific implementation order. Refactor-phase behavior freeze from `rules/cross-platform-alignment.md` applies — every Phase II round preserves observable behavior.

**Mirrors vs one-to-one**: A0–A10 mirror the *intent and gates* of iOS G0–G10. Individual scopes differ where Android architecture differs (larger A1 — whole stateful-`object` surface, not just BackupService; explicit A7 IME composition-root work; DataStore-specific A2 semantics).

**Snapshot commit**: current `main` after PR #141 merged 2026-04-19.

---

## 1. Layer map — Android current vs iOS target

iOS target (`ios-exemplar.md` §1):

```
View → ViewModel → Platform engine executor → Pure Engine
                                                  ↓
                                               Service → Repository → SQLite / binary
```

Android actual, from highest to lowest layer:

| Layer | iOS target | Android reality | Delta |
|---|---|---|---|
| View | SwiftUI View, no service/repo reads | Compose `@Composable` screen — 4 of 9 tab screens call services directly (19 call sites in tab3, 3 in tab4 — all in `InputSettingsScreen`; see §7 A3) | ⚠ Partial — tab3 data-management screens bypass VM layer |
| ViewModel | One VM per feature, `@Published` state, owns async | 1 VM exists (`DictionarySearchViewModel`); others missing | ✗ Gap — 4 VMs to add (A3) |
| Platform engine executor | Per-type (`ComposingManager`, `NextWordController`) — thin wrapper around pure state + effects | `ComposingManager` (class, pure Kotlin — but no `Effect` enum, calls `InputConnection` inline); `NextWordHandler` (class, constructor-injected, reads `System.currentTimeMillis()` directly) | ⚠ Partial — executors exist as classes, but the pure-engine split has not been drawn |
| Pure Engine | `ComposingState` / `NextWordEngine` (Foundation-only) | No extracted pure types; state + I/O + clock reads intermixed in executor | ✗ Gap — A4/A5 extract target |
| Service | Coordinates side effects, constructor DI with `= .shared` default | Kotlin `object` singletons (no constructor, global access). `LexiconService`, `TrieService`, `NextWordService`, `CustomDictionaryService`, `UserFrequencyService`, `BackupService` | ✗ Structural divergence — A1 + A7 |
| Repository | Separate layer on iOS | Not separately named on Android — DB access lives inside the `object` services (`connectUserDb`, `dbHelper`, `AssociationBinaryReader`, `DictionaryBinaryReader`) | ⚠ Shape divergence — no `Repository` suffix; the same work is done inline in services |
| Binary / SQLite | `dictionary.bin`, `association.bin`, user-frequency SQLite | Same file formats (G10 portability audit applies on both platforms) | ✓ Aligned — see `data-artifacts-portability.md` |

**iOS-to-Android type shape table** (from `ios-exemplar.md` §1) status:

- SwiftUI `ObservableObject` VM → `androidx.lifecycle.ViewModel`: partially present (1 of 5+ needed)
- `@Published` → `MutableStateFlow` + `asStateFlow()`: only `DictionarySearchViewModel` uses the idiom
- `EngineSettingsProvider.current` (live) → Kotlin `get()` property: **no Android analog exists yet**; closest is `PrefHelper` with a cached Preferences bag
- `ComposingDelegate`-via-`UITextDocumentProxy` → `InputConnection` wrapper with neutral `Effect` enum: **no Effect enum exists**; ComposingManager calls `ic.setComposingText(...)` / `ic.finishComposingText()` inline
- `OSSignposter` → `Trace.beginSection` / `endSection`: not used (performance baselining deferred, matches iOS G0 decision)
- `LoggerBackend` protocol + `NullLoggerBackend` default: **no Android analog**; `android.util.Log` called directly in `CandidateProcessor`, `InputNormalizer`, `NextWordService`, and elsewhere

---

## 2. DI + composition state

### 2.1 Current composition root

`TaigiKeyboard.onCreate()` (`ime/core/TaigiKeyboard.kt:103`) is the de-facto composition root:

```kotlin
override fun onCreate() {
    // ...
    prefs = PrefHelper(this)          // constructor DI (good)
    prefs.warmUp()
    serviceScope.launch { prefs.migrateFromSharedPreferences() }
    subtypeManager = SubtypeManager(this, prefs)
    activeSubtype = subtypeManager.getActiveSubtype() ?: Subtype.DEFAULT
    // Flow observers (partial EngineSettingsProvider analog)
    serviceScope.launch { prefs.observeInputMode().collect { onInputModeChanged(it) } }
    serviceScope.launch { prefs.observeKeyboardLayoutType().collect { onKeyboardLayoutTypeChanged(it) } }
    // Stateful singletons activated by side-effect .init(context) calls
    UserFrequencyService.init(this)
    CustomDictionaryService.init(this)
    super.onCreate()
    textInputManager.onCreate()   // TextInputManager is a companion-object singleton
    mediaInputManager.onCreate()
}
```

There is **no `Application` subclass**. All IME state bootstraps inside `InputMethodService.onCreate`. The settings `Activity` classes (under `settings/`) have no graph of their own — they read `PrefHelper(context)` per-Activity.

### 2.2 The three-scope problem

iOS has two composition roots (host-app + keyboard extension). Android reality has three lifecycle scopes that must be modeled explicitly:

| Scope | Owner | Typical state |
|---|---|---|
| App-tab graph | Settings `Activity` + Compose UI | `PrefHelper`, ViewModels, dictionary/frequency lookups |
| IME service graph | `TaigiKeyboard : LifecycleInputMethodService` | Subtype, layout manager, engine services, smartbar |
| Per-input-session state | `onStartInput` / `onFinishInput` | Composing state, selection context, next-word context |

The IME composition root problem is **not** the same as service DI. Even if every `object` were replaced with a constructor-injected class, the `TaigiKeyboard ↔ TextInputManager ↔ SmartbarManager ↔ MediaInputManager` `getInstance()` cycle would remain. A7 tracks this separately from A1.

### 2.3 Stateful object singletons (candidates for A1)

Kotlin `object` declarations that hold mutable state, context references, DB handles, or read the clock:

| File | Mutable state | DB / context | Notes |
|---|---|---|---|
| `ime/dictionary/LexiconService.kt:27` | `@Volatile binaryReader`, `isInitialized`, `initMutex` | Yes — copies `dictionary.bin`, opens reader | Core lookup singleton |
| `ime/dictionary/TrieService.kt:19` | `@Volatile isInitialized`, `initMutex` | Yes — loads native `.so`, owns JNI handle | Requires `init(context)` |
| `ime/dictionary/NextWordService.kt:47` | `@Volatile associationReader`, `@Volatile userDatabase`, `@Volatile isInitialized`, `initMutex`, `recordCounter`, decay constants | Yes — SQLite `user_association.db`, reads clock via `System.currentTimeMillis` | Scoring + persistence in one object |
| `ime/dictionary/CustomDictionaryService.kt:27` | `appContext`, `dbHelper`, `initMutex`, `isInitialized` | Yes — SQLite | |
| `ime/text/composing/UserFrequencyService.kt:40` | `appContext`, `dbHelper`, `initMutex`, `isInitialized`, `recordCounter` | Yes — SQLite | |
| `ime/dictionary/BackupService.kt:16` | None directly (delegates) | Indirect via the three services above | iOS G1 analog outlier, but inside a larger surface on Android |

Pure-utility `object`s (stateless, no DI migration needed — A1 explicitly excludes these):

`ToneConverter`, `ToneConverterModels`, `TaigiPhonetics`, `TaigiUnicode`, `SuggestionCaseTransformer`, `TPSConverter`, `ToneUtilities`, `ToneRestoration`, `DictionaryConstants`, `ExternalLookupURLBuilder`.

**Stateless but not shared-core-clean** (A1 adjacent — needs logging/DB-reach cleanup, not DI migration):

- `CandidateProcessor.kt` — imports `android.util.Log` (line 3) and reaches `UserFrequencyService.frequencyDataBatch(...)` directly (line ~112). Scoring math is stateless but not Foundation-only. Cleanup path: route logging through `LoggerBackend` (A1 precondition) and replace direct `UserFrequencyService` reach with a caller-passed `FrequencyData` batch (mirrors iOS `CandidateProcessor.calculateScore` taking `frequencyData` as a parameter). See §8 decision #7.
- `InputNormalizer.kt` — also imports `android.util.Log`. Logging-only cleanup via `LoggerBackend`.

### 2.4 IME composition-root singletons (A7 target, not A1)

Companion-object `INSTANCE` singletons with `getInstance()` mutual recursion:

- `TaigiKeyboard.kt:45` — top-level `var taigikeyboardInstance: TaigiKeyboard?`, `getInstance()` at line 98
- `TextInputManager.kt:81` — `instance`, `getInstance()`, constructed in `TaigiKeyboard.init { }` block
- `SmartbarManager.kt:169` — `instance`, `getInstance()`, looked up by `TextInputManager` + candidate overlay views
- `MediaInputManager.kt:34` — `instance`, `getInstance()`, looked up by `TaigiKeyboard.init`

Reach of `.getInstance()` across the codebase: `InputView`, `TextInputManager`, `SmartbarManager`, `MediaInputManager`, `KeyView`, multiple smartbar views. A7 untangles this into a single IME-scoped `CompositionRoot` holder constructed in `TaigiKeyboard.onCreate`.

### 2.5 Already-DI-shaped (no A1/A7 work needed)

- `PrefHelper.kt` — constructor takes `Context`
- `SubtypeManager.kt:14` — constructor takes `TaigiKeyboard`, `PrefHelper`
- `ComposingManager.kt:23` — constructor takes `InputMode`, `enableDoubleTapOO`, `enableDoubleTapNN`
- `NextWordHandler.kt:20` — constructor takes `CoroutineScope`, `PrefHelper`, `TaigiKeyboard`, callbacks
- `TaigiAutocompleteService.kt:18` — constructor takes `Context`, `inputMode`, `prefs`
- `EnglishAutocompleteService.kt:26` — constructor takes `Context`
- `CandidateUpdateCoordinator.kt:25` — injected `CoroutineScope`

---

## 3. Engine-layer observable state + async inventory

Mirrors iOS Phase I G4/G5 targets — types that mix state + scheduling + platform I/O, eligible for pure/platform split.

### 3.1 `ComposingManager` (`ime/text/composing/ComposingManager.kt`)

- **State** — private `rawInput`, `composingText`, `isComposing`, `displayDirty`, `selectedCandidateIndex`. No `@Published` / `StateFlow` wrapper.
- **Platform I/O** — every state-mutating method takes `InputConnection` as a parameter and calls `ic.setComposingText()` / `ic.finishComposingText()` inline (e.g. `startComposing`, `appendCharacter`, `deleteBackward`, `commitComposition`, `selectSuggestion`, `reset`).
- **Derivation** — `deriveDisplay()` is Foundation-pure (no Android SDK); calls `ToneConverter.convertToToneMarks` + `TPSConverter.containsTPS`. Async derivation lives in `CandidateUpdateCoordinator`, not here.
- **Delta vs iOS G4-impl target** — iOS split produced `ComposingState` (pure) + `ComposingTransition` with neutral `Effect` enum. Android has **no** `Effect` enum; InputConnection calls are inline, which means A4-impl can't just "adopt the same pattern" — it must design the split in Kotlin while honoring the `finishComposingText()` commits-by-default caveat (`ios-exemplar.md` §4.1, flagged there as Android-critical).
- **Marker** — no `// region Shared-Core Candidate` at file head.

### 3.2 `NextWordService` (`ime/dictionary/NextWordService.kt`)

- **State** — stateful `object`. `@Volatile associationReader`, `@Volatile userDatabase`, `@Volatile isInitialized`, `initMutex`, `AtomicInteger recordCounter`.
- **Scoring constants** — lines 57–73 carry a `CROSS-PLATFORM INVARIANT` comment explicitly listing `USER_WEIGHT (50)`, `DICT_WEIGHT (1)`, `DECAY_HALF_LIFE_HOURS (168.0)`, `LEARNING_BONUS (300.0)`, `HIGH_USAGE_DECAY_FLOOR (0.95)`, `LOW_USAGE_DECAY_FLOOR (0.3)`, `HIGH_USAGE_THRESHOLD (3)` — must mirror iOS. Partial parity already documented.
- **Clock / time** — scoring + decay math embedded in the same file as SQLite persistence. No `nowMs` injection at the engine boundary.
- **Delta vs iOS G5-impl target** — iOS split produced `NextWordEngine` (pure enum namespace) + `NextWordOutcome` + `RawNextWordPrediction` DTO + `NextWordController` (platform executor). Android has **none** of these extracted; `NextWordService.Prediction` is a nested data class that leaks to `NextWordHandler`. A5-impl extracts the pure engine.

### 3.3 `NextWordHandler` (`ime/text/smartbar/NextWordHandler.kt`)

- **State** — `lastSelectedWord`, `lastSelectedRoman`, `lastSelectionTime`, `isShowingNextWord`.
- **Clock** — `System.currentTimeMillis()` read directly at line 56 (`handleNextWordPrediction`) and line 193 (`updateLastSelectedWord`). No `nowMs` injection.
- **Scheduling** — uses injected `CoroutineScope` (good — no owned Timer or Handler). Compares against `CONTEXT_TIMEOUT_MS = 30_000L` and `ASSOCIATION_TIMEOUT_MS = 10_000L` constants. These correspond to iOS `NextWordEngine` `contextTimeoutSeconds (30)` / `associationTimeoutMs (10_000)` constants — **no `CROSS-PLATFORM INVARIANT` comment yet** (iOS exemplar §5.3 lists these for the invariant marker requirement).
- **Race prevention** — no generation counter. iOS G5-impl introduced `currentGeneration` on `queryPredictions` to drop stale results; Android would need the equivalent in A5-impl to match the invariant set documented in `nextword-engine-boundary.md`.

### 3.4 `CandidateUpdateCoordinator` (`ime/text/CandidateUpdateCoordinator.kt`)

- Already clean: 50ms debounced Job cancellation, off-main display derivation on `Dispatchers.Default`. No pure/platform split needed.

### 3.5 Handlers / Timers / Dispatchers in engine-adjacent code

- `TaigiKeyboard.osHandler` (`Handler(Looper.getMainLooper())` for IME lifecycle cleanup) — platform, fine.
- `TextInputManager.Handler` (double-space → period timing logic) — platform, fine.
- `CapsStateManager.Handler` (caps-lock timeout + repeat-delete) — platform, fine.
- `KeyView.Handler` (key repeat) — platform, fine.

No Timer-equivalent leaks into the `ime/dictionary/` package.

### 3.6 Settings access (live-read vs snapshot)

- `PrefHelper.observeInputMode()` / `observeKeyboardLayoutType()` return `Flow<String>` — partial `EngineSettingsProvider` analog (collection-based, not `get()`-based).
- `TaigiKeyboard.onCreate` collects these Flows and routes changes via `onInputModeChanged` / `onKeyboardLayoutTypeChanged`.
- `PrefHelper` also exposes synchronous cached getters (`prefs.inputMode`, `prefs.associationRecordingEnabled`). These **are** live-read per call (re-reads cached Preferences) — matching iOS §3 live-read invariant.
- **Delta** — no named `EngineSettings` interface exists. Engine code (`NextWordService`, `NextWordHandler`, `TaigiAutocompleteService`) takes `PrefHelper` as a parameter, which is richer than iOS's engine contract. A2 defines the slim interface and has `PrefHelper` implement it.

### 3.7 Marker + invariant status

- `// region Shared-Core Candidate` markers: **0 files** (iOS has 43). Purely a documentation gap — no code change. A8 delivers the marker convention + sweep.
- `CROSS-PLATFORM INVARIANT` comments: 4 files — `NextWordService` (scoring), `TaigiUnicode`, `AssociationBinaryReader`, `DictionaryBinaryReader`. iOS `ios-exemplar.md` §5.3 lists 4 surfaces requiring the marker: NextWordScorer, CandidateProcessor, NextWordEngine timing, TaigiUnicode preprocessing. Android has 3 of 4 (missing `CandidateProcessor` scoring constants); the two binary-reader comments are an Android-specific addition useful to keep.

**Progress note (2026-04-20 post-A8-sweep)** — this section captures the audit-day snapshot. Current state: 28 Shared-Core markers landed (A1/A5-impl/A6/A8-skeleton/A8-sweep), all 4 §5.3 INVARIANT surfaces covered (CandidateProcessor expanded in A8-sweep; NextWordEngine timing added by A5-impl). Living counts live in `android-exemplar.md` §5 — defer to that doc, not this snapshot.

---

## 4. Settings access — DataStore / SharedPreferences semantics (Android-specific)

This section exists because `ios-exemplar.md` §3 warns about `val`-vs-`get()` live-read bugs assuming SharedPreferences. Android reality is **DataStore plus a cache**, which changes the semantics and must be audited separately.

- `PreferenceDataStore.kt` — single app-wide `Context.preferencesDataStore` delegate.
- `PrefHelper.kt` — constructor-injected wrapper around that DataStore. Holds a `@Volatile cachedPrefs: Preferences?` and a `pendingKeys` map guarded by `synchronized(lock)`. Reads are synchronous (return cached value or empty); writes are async via `CoroutineScope(Dispatchers.IO)`.
- `PrefHelper.warmUp()` — called once from `TaigiKeyboard.onCreate` to pre-populate the cache.
- Migration path — `migrateFromSharedPreferences()` still active (runs on every `onCreate`). DataStore is the authoritative store; legacy SharedPreferences reads drain into DataStore on first launch.
- Cross-process sync — no Darwin-notification analog; DataStore file is shared via file-system and observed via Flow. The `observeInputMode`/`observeKeyboardLayoutType` Flows serve the live-push role iOS achieves via notifications.

**A2 implications**:

1. `EngineSettings` interface on Android must be backed by the cached `PrefHelper` synchronous reads (not a Flow collection inside engine code).
2. `EngineSettingsProvider.current` semantics must be defined for DataStore: the accessor returns `this` (or an adapter), and each field-level `get()` re-reads `cachedPrefs`. No `val x = prefs.getX()` in the initializer — matches `ios-exemplar.md` §3 warning verbatim.
3. Freshness guarantee: "cached value at last flush or warm-up" — documented, not "read-through to disk every access". Engine code must not assume per-call disk read; the iOS `EngineSettings` protocol already avoids this (live read refers to value-freshness relative to current in-memory state).
4. Pending-write semantics: writes land in `pendingKeys` immediately, then drain to DataStore; a `get()` after a `set()` returns the pending value. This matches user-observable behavior on iOS (`SharedSettings` writes see their own reads instantly). No divergence.

---

## 5. Shared-core candidate roster — Android mapping to iOS 43

iOS roster groups from `shared-core-readiness.md`. Android files mapped below. "Shape match" = no architectural split needed; "needs split" = file exists but mixes pure + platform; "missing" = no Android counterpart.

### Phonetics (iOS 10 files)

| iOS | Android | Shape match? |
|---|---|---|
| `TaigiPhonetics.swift` (facade) | `ime/dictionary/TaigiPhonetics.kt` (object) | ✓ — both are thin facades |
| `Phonetics/Tables/PhoneticsTables.swift` | merged into `TaigiPhonetics.kt` | ⚠ — Android does not split tables into a separate file |
| `Phonetics/Parser/SyllableParser.swift` | merged into `TaigiPhonetics.kt` | ⚠ — parser not extracted |
| `Phonetics/Formatter/TLFormatter.swift` | merged into `TaigiPhonetics.kt` | ⚠ |
| `Phonetics/Formatter/POJFormatter.swift` | merged into `TaigiPhonetics.kt` | ⚠ |
| `Phonetics/Converter/PhoneticsConverter.swift` | merged into `TaigiPhonetics.kt` | ⚠ |
| `Phonetics/Converter/RomanizationConverter.swift` | merged into `TaigiPhonetics.kt` | ⚠ |
| `Phonetics/ToneRestoration.swift` | `ime/dictionary/ToneRestoration.kt` | ✓ |
| `Phonetics/ToneUtilities.swift` | `ime/dictionary/ToneUtilities.kt` | ✓ |
| `Phonetics/ToneConverter.swift` (now takes `ToneToggles`) | `ime/dictionary/ToneConverter.kt` (takes `enableDoubleTapOO`, `enableDoubleTapNN` directly as params) | ⚠ — Android does not have a `ToneToggles` value type; toggles passed as Booleans |

Plus `ToneConverterModels.kt` (Android-only, holds `InputMode` enum + helper types).

### Input (iOS 9 files)

| iOS | Android | Shape match? |
|---|---|---|
| `Input/CharacterInputPipeline.swift` | (no direct analog) | ⚠ — pipeline orchestration happens in `TextInputManager`, which is platform-coupled |
| `Input/CaseTransformer.swift` | `ime/dictionary/SuggestionCaseTransformer.kt` | ✓ — same role, slightly different name |
| `Input/TPS/TPSConverter.swift` | `ime/dictionary/TPSConverter.kt` (single file, table inlined) | ⚠ — Android does not split TPS tables into a separate file |
| `Input/TPS/TPSTables.swift` | folded into `TPSConverter.kt` | ⚠ |
| `Input/TPS/TPSInputAdjuster.swift` | folded into `TPSConverter.kt` | ⚠ |
| `Input/TPS/TPSToTL.swift` | folded into `TPSConverter.kt` | ⚠ |
| `Input/TPS/TLToTPS.swift` | folded into `TPSConverter.kt` | ⚠ |
| `Input/Composing/ComposingState.swift` (G4-impl) | **missing** — state + I/O in `ComposingManager.kt` | ✗ — A4-impl target |
| `Input/Composing/ComposingTransition.swift` (G4-impl) | **missing** | ✗ — A4-impl target |

_Progress note (post-A4-impl + post-A8-sweep)_: `ComposingState.kt`, `ComposingTransition.kt` and `ComposingDelegate.kt` were landed by A4-impl (PR #152). A8-sweep marked `ComposingTransition.kt` as Shared-Core; `ComposingState.kt` is held back (transitive `ToneConverter` import); `ComposingDelegate.kt` carries a `// NOTE: Not shared-core — platform adapter` header. Living status in `android-exemplar.md` §5.

### Lexicon (iOS 11 files)

| iOS | Android | Shape match? |
|---|---|---|
| `Lexicon/Models/TaigiWord.swift` | `ime/dictionary/TaigiWord.kt` | ✓ |
| `Lexicon/Models/InputType.swift` | `ime/dictionary/InputType.kt` (sealed class vs enum — behaviorally identical) | ⚠ shape |
| `Lexicon/Models/DictionarySource.swift` | `ime/dictionary/DictionarySource.kt` | ✓ — ordering is authoritative bitmask on both |
| `Lexicon/Models/CustomDictionaryEntry.swift` | ? | need verification — likely nested inside `CustomDictionaryService` |
| `Lexicon/Models/FrequencyData.swift` | **missing as top-level** — nested inside `UserFrequencyService` | ⚠ — iOS hoisted this in PR #133; Android TODO |
| `Lexicon/Models/LexiconConstants.swift` | `ime/dictionary/DictionaryConstants.kt` | ✓ name differs |
| `Lexicon/Models/LexiconError.swift` | `ime/dictionary/DictionaryError.kt` | ✓ |
| `Lexicon/Utils/TaigiUnicode.swift` | `ime/dictionary/TaigiUnicode.kt` — already has `CROSS-PLATFORM INVARIANT` header | ✓ |
| `Lexicon/Utils/CandidateProcessor.swift` | `ime/dictionary/CandidateProcessor.kt` | ✓ shape; **missing** scoring-constant invariant comment (iOS §5.3 requires it) |
| `Lexicon/Trie/InputNormalizer.swift` | `ime/dictionary/InputNormalizer.kt` | ✓ |
| `Lexicon/Database/CustomDictionaryDerivation.swift` | likely embedded in `CustomDictionaryService` | ⚠ — not extracted |

### NextWord (iOS 6 files — post-G5-impl)

| iOS | Android | Shape match? |
|---|---|---|
| `NextWord/EnginePrediction.swift` | `NextWordService.Prediction` (nested data class) | ⚠ — not a top-level type |
| `NextWord/NextWordScorer.swift` | scoring inlined in `NextWordService` | ⚠ — extract target |
| `NextWord/AutocompleteContextBooster.swift` | need verification on Android | — |
| `NextWord/NextWordEngine.swift` (G5-impl) | **missing** — decide/score intermixed with I/O in `NextWordService` | ✗ — A5-impl target |
| `NextWord/NextWordOutcome.swift` (G5-impl) | **missing** | ✗ — A5-impl target |
| `NextWord/RawNextWordPrediction.swift` (G5-impl) | **missing** | ✗ — A5-impl target |

### Autocomplete (iOS 2 files)

| iOS | Android | Shape match? |
|---|---|---|
| `Autocomplete/Services/AutocompleteInputClassifier.swift` | logic inside `TaigiAutocompleteService.kt` | ⚠ — classifier not extracted |
| `Autocomplete/Services/AutocompleteProviders.swift` (protocols) | no `AutocompleteProviders` equivalent — providers are concrete classes | ⚠ — A6 cleanup target |

### Settings (iOS 4 files)

| iOS | Android | Shape match? |
|---|---|---|
| `Settings/EngineSettings.swift` (protocol) | **missing** — no engine-facing interface; engine takes `PrefHelper` directly | ✗ — A2 target |
| `Settings/EngineSettingsProvider.swift` (protocol) | **missing** | ✗ — A2 target |
| `Settings/InputMode.swift` | folded into `ToneConverterModels.kt` (`object ToneConverterModels { enum class InputMode }`) | ⚠ — Android enum lives inside a wrapping object, not at top level |
| `Settings/ToneToggles.swift` (G4-impl) | **missing** — Booleans passed directly | ✗ — A2 / A4-impl prerequisite |

### Common (iOS 1 file)

| iOS | Android | Shape match? |
|---|---|---|
| `Common/LoggerBackend.swift` (protocol + NullLoggerBackend + LoggerFactory) | **missing** — `android.util.Log` called directly in 4+ engine files | ✗ — A1 adjacent, or split into its own group |

### Roster counts

- **Shape match (1:1)** on Android today: ~13 files (pure-utility `object`s + value types).
- **Needs split / marker / refactor** to match iOS shape: ~15 files.
- **Missing entirely** (no Android counterpart): 5 types — `EngineSettings`, `EngineSettingsProvider`, `LoggerBackend`, `ComposingState`, `NextWordEngine` (with `Outcome` / `RawNextWordPrediction` / `ToneToggles` satellites).

Roster-target for Phase II exit: ≥ 40 files (mirror of iOS ≥ 40 gate in G8/exemplar §9).

### Package-layout divergence

iOS uses directory boundaries (`Lexicon/`, `NextWord/`, `Phonetics/`, `Autocomplete/`, `Settings/`, `Input/`). Android uses `com.siansiansu.taigikeyboard.ime.dictionary.*` as a catch-all for what iOS splits into Lexicon + NextWord + Phonetics + part of Input. Not behavioral, but affects readability and future `:shared-core` module carve-out. A7/A8 decides whether to preserve current package or re-split. Package moves are refactor-freeze compatible (no behavior change) but invalidate deep-links, so either do them all at once or not at all.

---

## 6. Gradle module / R8 / JNI / assets portability

Flagged as audit dimension on Codex pre-review (2026-04-19).

### 6.1 Gradle module shape

- Single `android/app` module. `settings.gradle` includes only `:app`.
- `build.gradle.kts` applies `com.android.application` (not a library module).
- `minSdk = 28`, `targetSdk = 35`, `compileSdk = 36`.
- No `:shared-core` module exists yet. Phase III/IV-A is when the Kotlin-pure core would be carved out into its own module (likely `android-lib` or `com.android.library`).

**Implication for A1/A7**: a future `:shared-core` Kotlin module can only consume files that are stdlib-only. Current `object` services import `android.content.Context`, `android.database.sqlite.*`, `android.util.Log`, `kotlinx.coroutines.*`. A1 + LoggerBackend introduction are prerequisites to carving a pure module out.

### 6.2 R8 / ProGuard

- `release` build has `isMinifyEnabled = true` + `isShrinkResources = true` with `proguard-android-optimize.txt` + `proguard-rules.pro`.
- ProGuard implications for shared-core extraction:
  - JNI method signatures (`trie_jni.cpp`) must be kept via `-keep class ... { native <methods>; }` — already the case for trie-related classes; audit when Rust/FFI arrives.
  - Reflective serialization (Moshi for `KeyboardColorSettings`) already has Moshi rules; same pattern applies to any future shared-core DTOs.
  - Model / value types crossing FFI boundaries (Phase IV-A) need explicit `-keep` — not relevant in Phase II, flagged here for Phase III prep.
- No immediate Phase II action; tracked as a Phase III readiness note.

### 6.3 JNI and native symbols

- `android/app/src/main/cpp/trie_jni.cpp` binds the MARISA trie via JNI.
- `CMakeLists.txt` drives the NDK build; `debugSymbolLevel = "FULL"`.
- A `trie_jni.cpp` equivalent does **not** exist on iOS — iOS uses a Swift bridge over the same C++ library. This is a real portability risk for Rust/FFI (Phase IV-A): will Rust bind MARISA directly, or will both platforms go through a common C layer? Already tracked in `data-artifacts-portability.md` (iOS G10 output) as D1.
- No Phase II action.

### 6.4 Assets

Binary + JSON assets under `android/app/src/main/assets/`:

- `dictionary.bin`, `association.bin`, `dictionary.trie` — the data-artifact portability set (G10 → `data-artifacts-portability.md` D1/D2).
- `ime/text/characters/*.json`, `ime/text/symbols/*.json`, `ime/text/layout/*.json` — layout JSONs. **iOS equivalents are code-defined, not JSON-asset-defined.** Layout-asset portability is not a shared-core concern (layout is platform-UI), but the format divergence is worth noting for A10.
- `ime/media/emoji/*.txt` — emoji data.

A10 (data-artifacts Android addendum) captures per-file load-path + mmap strategy + update semantics on Android, complementing iOS G10.

---

## 7. Phase II task groups — A0–A10

A0–A10 mirror the **intent and gates** of iOS G0–G10. Individual scopes differ where Android architecture differs. Estimates assume single-phase focus and are rougher than iOS G-group estimates because Android has fewer reference points.

### A0 · Invariants test wiring + qualitative perf gate (M — ~4–5 hr)

Mirror of iOS G0 + G9. Android already has 10 test files under `app/src/test/java/.../ime/dictionary/`, but they are not wired to `INVARIANT_*` labels from `behavioral-invariants.md`.

**Deliverable**:
- Audit existing tests against the 12 invariants in `behavioral-invariants.md` §1–12.
- Add / rename tests so each invariant has at least one Android-side `INVARIANT_*` test (equivalent to the iOS G9 requirement — **deferred to A9 as of PR #TBD 2026-04-19** because A1 / A2 / A5-impl reshape the files the tests would target; the A0 matrix at `android-g9-coverage-matrix.md` enumerates the 36 labels A9 must deliver).
- Qualitative perf gate on real device (S1 POJ, S2 TPS, S3 Hanji scroll) — same three sequences as iOS, ported to Android dogfooding script.

**Risk**: low (tests + qualitative gate).

### A1 · Stateful `object` services → class + constructor DI (M-L — ~4–6 hr)

**Rename from iOS G1** — Android's `object` pattern makes this broader than iOS's single BackupService outlier. Excludes pure-utility `object`s (those stay).

**Scope**: convert `LexiconService`, `TrieService`, `NextWordService`, `CustomDictionaryService`, `UserFrequencyService`, `BackupService` from `object` to `class` with constructor DI. Introduce `LoggerBackend` interface + `android.util.Log` adapter as part of this round (engine files drop direct `Log.d` / `Log.e` calls — includes the `CandidateProcessor` / `InputNormalizer` logging cleanup from §2.3). Also introduce a minimal `CompositionRoot` class instantiated in `TaigiKeyboard.onCreate` that owns these service instances so call-sites take them via constructor/property injection rather than `LexiconService.search(...)` global calls.

**Out of scope** (separate groups): IME manager-graph `getInstance()` unwind (A7), Effect-enum refactor (A4), NextWordEngine extract (A5).

**Risk**: medium. Touching every `object` service changes every call-site by removing the implicit `.INSTANCE` qualifier. Refactor-freeze constraint applies — behavior unchanged, call sites pass the service instance from the `CompositionRoot` graph instead of calling `LexiconService.search(...)`. Dogfooding S1/S2/S3 required.

**Sequencing note** (Codex post-review 2026-04-19): A1 introduces the service-side `CompositionRoot`; A7 later extends it with IME managers. This split avoids churning service call-sites twice.

### A2 · EngineSettings interface + PrefHelper-backed provider (M — ~3–4 hr)

Mirror of iOS G2, adapted for DataStore.

**Deliverable**:
- `EngineSettings` interface + `EngineSettingsProvider` interface (see iOS exemplar §3 shape).
- `PrefHelper` implements `EngineSettingsProvider` (or a thin adapter wraps it).
- Each field on `EngineSettings` is a `get()` property that re-reads `PrefHelper.cachedPrefs` — the `val x = prefs.getX()` snapshot anti-pattern must be avoided per `ios-exemplar.md` §3.
- Engine code currently taking `PrefHelper` directly (`NextWordService.predict(..., prefs: PrefHelper)`, `TaigiAutocompleteService.ctor(..., prefs: PrefHelper)`) migrates to `EngineSettings`. Platform executor code can still take `PrefHelper` for write access.
- Audit for any `prefs.observeInputMode()` Flow collection inside engine files (should not exist — collection is a platform concern).

**Risk**: medium — touches NextWord path and autocomplete path. Dogfood required.

### A3 · Dictionary tab Views → ViewModels (M — ~4–5 hr)

Mirror of iOS G3.

**Prerequisite** (Codex post-review 2026-04-19 + §8 #1 decision 2026-04-20): the app-tab graph ownership decision resolved as "yes, add Application subclass", folded into A7. **A3 now blocks on A7**, not on §8 #1 directly — ViewModels resolve service instances via `(application as TaigiKeyboardApplication).composition` (or `CompositionRoot.shared(application)` as a back-compat path). A3 starting before A7 lands would invent a temporary factory and force rework when A7 relocates warmup to Application.

**Deliverable**: 4 new ViewModels — `CustomDictionaryViewModel`, `FrequencyDataViewModel`, `AssociationDataViewModel`, `DataManagementViewModel`. Each extends `androidx.lifecycle.ViewModel` with `MutableStateFlow<State>` (private) + `asStateFlow()` (public). Pattern from `ios-exemplar.md` §6 Kotlin example. Constructor DI — services injected via VM factory.

**Call sites to migrate**: 19 in `ui/tabs/tab3/` (6 in `CustomDictionaryScreen`, 6 in `FrequencyDataScreen`, 6 in `AssociationDataScreen`, 1 partial in `DataManagementScreen`) + 3 `DiagnosticService.gather(context)` call sites in `ui/tabs/tab4/InputSettingsScreen.kt` (lines 334 / 350 / 368, all inside one screen's Copy / Share / other-action callbacks — one VM migration covers all three sites).

**Risk**: medium — async state (import/export loading states) must be preserved. No behavior change.

### A4-design · Composing boundary Android binding addendum (S — ~1 hr)

**Do not skip design** (per Codex pre-review). iOS boundary doc `composing-state-boundary.md` is platform-neutral in intent; Android binding needs a short addendum covering:

- `InputConnection.finishComposingText()` commits by default — bindings MUST zero the region via `setComposingText("", 1)` before issuing it, to honor `clearPreeditWithoutCommit` semantics. (Already flagged in iOS §4.1.)
- IME main-thread rule — all `InputConnection` calls run on `Dispatchers.Main.immediate`.
- StateFlow vs direct Effect execution — Android does not need `StateFlow` inside the platform executor; the executor interprets `Effect` values synchronously on the IME main thread, same as iOS.
- Lifecycle-scoped cancellation — the executor's `CoroutineScope` is cancelled in `onDestroy`.

Target file: `docs/architecture/composing-state-boundary.md` §3 (Android Binding) — append to existing iOS doc, not a separate file.

### A4-impl · ComposingState extract (L — ~4–6 hr)

Mirror of iOS G4-impl.

**Deliverable**: split `ComposingManager.kt` into `ComposingState.kt` (pure, Kotlin stdlib only — state + `apply(intent)` returning `ComposingTransition`) + `ComposingManager.kt` (platform wrapper, implements `Effect` execution via `InputConnection`). Effect enum mirrors iOS `composing-state-boundary.md` §2.2.

**Precondition**: `ToneConverter` takes a `ToneToggles` value type instead of two Booleans (iOS G4 precondition also required this parameterization).

**Parity-correction flag** (Codex post-review 2026-04-19): iOS `Effect.clearPreeditWithoutCommit` mapping to Android requires `ic.setComposingText("", 1)` *before* `ic.finishComposingText()` to avoid the default-commit behavior. Android `ComposingManager` today calls `ic.finishComposingText()` directly in `commitComposition` / `reset`. If the current Android code ever reaches a "clear without commit" path (e.g. user-invoked cancel, subtype switch with pending composition), the new binding may commit differently. Verify the two paths against current behavior and call out any divergence as a parity correction in the PR description.

**Risk**: high — hottest path. Dogfood S1/S2/S3 required.

### A5-design · NextWord boundary Android binding addendum (S — ~30 min)

Same as A4-design, for `nextword-engine-boundary.md`. Focus on Kotlin clock injection (`nowMs: Long` parameter), generation counter in Kotlin style, coroutine-based reschedule vs iOS Timer.

### A5-impl · NextWordEngine extract (L — ~3–5 hr)

Mirror of iOS G5-impl.

**Deliverable**: extract `NextWordEngine` (stateless functions), `NextWordOutcome`, `NextWordPersistedState`, `NextWordDecisionInput`, `RawNextWordPrediction` from `NextWordService.kt`. `NextWordService` becomes thin I/O layer; `NextWordHandler` takes `nowMs` from caller.

**Parity-correction flag** (Codex post-review 2026-04-19): iOS G5-impl introduced a generation counter on `queryPredictions` + `clearPredictionsUI` to drop stale predictions (late async result after user has moved on). Android `NextWordHandler` currently has no generation counter — if users today see stale predictions in edge cases, introducing one is a parity correction, not purely refactor-freeze compatible. Treat as part of this round, explicitly noted in PR description with before/after description; tests must cover the late-callback drop.

**Risk**: high — user-learning math is subtle. A0 invariants audit (`android-g9-coverage-matrix.md` §7 / §8 rows) names the test labels A9 must land after this extract; pre-merge gating for A5-impl is qualitative dogfooding (S1 / S2 / S3) plus a manual walkthrough of the invariants audit — grep-visible `INVARIANT_*` labels do not exist until A9.

### A6 · Autocomplete cleanup (S — ~1–2 hr)

Mirror of iOS G6. Smaller on Android since autocomplete services are already constructor-injected classes (not singletons). Scope: extract `AutocompleteInputClassifier` as a pure object/class, clean up any `LexiconService.INSTANCE` reach-ins post-A1.

### A7 · IME manager-graph `getInstance()` unwind + Application subclass (M+ — ~3.5–4.5 hr)

**Split from iOS G7** — the service-graph composition root lands earlier (see A1 note below); A7 handles the `TaigiKeyboard ↔ TextInputManager ↔ SmartbarManager ↔ MediaInputManager` `getInstance()` cycle plus — bundled per §8 #1 decision — the `TaigiKeyboardApplication : Application` subclass that owns app-tab composition.

**Deliverable**:
- Unwind `.getInstance()` reach (sites enumerated in §2.4). Each site takes its collaborators via init instead.
- Extend the `CompositionRoot` introduced in A1 to hold the IME managers (`TextInputManager`, `SmartbarManager`, `MediaInputManager`) in place of the `companion object instance` pattern.
- Introduce `TaigiKeyboardApplication : Application` (new ~30-LOC file) + `android:name=".TaigiKeyboardApplication"` on the Manifest `<application>` tag (one-line Manifest edit).
- Relocate the warmup chain (`prefs.warmUp()`, `migrateFromSharedPreferences`, `UserFrequencyService.init`, `CustomDictionaryService.init`) from `TaigiKeyboard.onCreate` to `TaigiKeyboardApplication.onCreate`. Keep services idempotent for a safe migration.
- `Application.onCreate` stays cheap — no synchronous heavy I/O; dictionary trie / binary loads continue firing lazily on first engine call, per A1 design.

**Risk**: medium — mechanical but touches every smartbar view + key view. Application bundling adds one Manifest line + a new file with negligible runtime footprint.

**Sequencing note** (Codex post-review finding, 2026-04-19 + §8 #1 decision, 2026-04-20): A1 introduces a minimal `CompositionRoot` holding the service graph so injected services have an owner; A7 expands that root to include the IME managers AND wraps the whole graph in a `TaigiKeyboardApplication` subclass. Having A7 create `CompositionRoot` from scratch after A1 was already injecting via defaults would churn every service call-site twice; keeping Application as its own standalone round would be a ~30 LOC + 1 Manifest-line PR with no test surface of its own.

### A8-skeleton · `android-exemplar.md` + marker convention (S — ~1 hr)

Mirror of iOS G8 — **split from full marker sweep** (Codex post-review 2026-04-19): doing the full marker sweep now would churn files that A1 / A4-impl / A5-impl are about to split or rename.

**Deliverable**:
- `docs/architecture/android-exemplar.md` companion doc — Android-specific deviations only (IME service vs Application scope, DataStore semantics, Dispatchers/coroutine choice, InputConnection binding). Not a full duplicate of `ios-exemplar.md`.
- Document the `// region Shared-Core Candidate` marker syntax + verification grep (from iOS §5.2).
- Apply marker + `CROSS-PLATFORM INVARIANT` comment *only* on files known to be terminal (won't move in A1/A4/A5): `TaigiUnicode.kt`, `TaigiPhonetics.kt`, `SuggestionCaseTransformer.kt`, `TPSConverter.kt`, `ToneRestoration.kt`.

### A8-sweep · Full Shared-Core Candidate marker + invariant comment sweep (S — ~1 hr)

Runs **after** A1, A4-impl, A5-impl land — when file moves/splits have settled.

**Deliverable**:
- `// region Shared-Core Candidate` marker on every remaining candidate file post-split.
- `CROSS-PLATFORM INVARIANT` comment on `CandidateProcessor.kt` (scoring constants — missing per §3.7), and any new extracted files from A4/A5 impl rounds.
- Re-run the five verification greps from `shared-core-readiness.md` Android-translated; report passes.

**Risk**: none (documentation only).

### A9 · Engine test coverage baseline (M — ~3–5 hr)

Mirror of iOS G9.

**Deliverable**: ≥ 70% line coverage on the top-10 most-depended-on shared-core candidates — `TaigiPhonetics`, `InputNormalizer`, `CandidateProcessor`, `ToneConverter`, `SuggestionCaseTransformer`, `ToneRestoration`, `TPSConverter`, `TaigiUnicode`, scoring helpers, `CustomDictionaryService` derivation. Tests wired to `INVARIANT_*` labels (may overlap with A0 work).

**Risk**: low (tests only), but may expose pre-existing bugs.

### A10 · Data-artifacts Android addendum (S — ~1–2 hr)

Mirror of iOS G10. Already have `data-artifacts-portability.md` authored from iOS side (PR #141). A10 adds Android-side sections: asset copy semantics (`LexiconService` copies `dictionary.bin` from assets to `filesDir` on first run), DataStore storage path, SQLite schema migration strategy (`migrateUserDb` at `NextWordService.kt:200`), Android update-in-place strategy (app update overwrites assets).

**Risk**: documentation only.

### Recommended order (Android)

```
A0 (invariants + perf gate)                    ← deferred to end acceptable if A9 coverage overlap high
   ↓
A4-design + A5-design (binding addenda)
   ↓
A8-skeleton (android-exemplar.md + marker convention, apply to 5 terminal files only)
   ↓
A1 (stateful object services → class DI + LoggerBackend + minimal CompositionRoot)
                                              ← unblocks everything downstream
   ↓
Decide §8 #1 Application-subclass path         ← prerequisite for A3
   ↓
A2 (EngineSettings protocol)
   ↓
A6 (Autocomplete cleanup)
   ↓
A3 (Dictionary tab ViewModels)
   ↓
A5-impl (NextWordEngine)
   ↓
A4-impl (ComposingState) — hottest path, fresh context
   ↓
A7 (IME manager-graph getInstance unwind)
   ↓
A8-sweep (full marker + invariant-comment sweep, post-splits)
   ↓
A9 (test baseline)
   ↓
A10 (data-artifacts Android addendum) — parallelizable with A9
```

Total estimate: **30–42 hours focused work** (up from 28–40 after A7 / A8 split). Realistic over 7–9 sessions.

---

## 8. Key decisions flagged for user

These are decisions that must be made during Phase II proper but would benefit from user input early to avoid rework:

1. **Application subclass** — introduce `class TaigiKeyboardApplication : Application` for app-tab graph composition, or keep app-tab state ephemeral (current pattern — `PrefHelper(context)` per Activity)? Recommendation: **yes, add one**. Isolates tab-side VM graph from IME-service graph (§2.2 three-scope problem). Behavior-neutral. Small round of its own, or folded into A7.
   - **DECISION 2026-04-20** (Claude + Codex joint, auto-mode): **YES, add `TaigiKeyboardApplication : Application`. Fold into A7** (not a standalone round). Rationale: A7 already touches `CompositionRoot` structure for the IME manager-graph unwind; adding Application as a thin shell there avoids a round consisting of ~30 LOC + one Manifest line. Class name locked to `TaigiKeyboardApplication` (matches Manifest intent, avoids generic `TaigiApplication`). Warmup chain (`prefs.warmUp()` + `migrateFromSharedPreferences` + `UserFrequencyService.init` + `CustomDictionaryService.init`) relocates from `TaigiKeyboard.onCreate` to `TaigiKeyboardApplication.onCreate` — services must stay idempotent so a belt-and-suspenders duplicate call during the migration is safe. Codex-flagged risk: keep `CompositionRoot.shared(this)` construction in `Application.onCreate` cheap — service constructors are already cheap per A1 (dictionary trie / binary loads trigger lazily inside `LexiconService.search`); do NOT add synchronous heavy I/O to the Application boot path. Single-process assumption (no `android:process` split in Manifest) remains valid; if later split, re-audit. Unblocks A3 (tab ViewModels can use `(application as TaigiKeyboardApplication).composition` or the existing `CompositionRoot.shared(application)`). Manifest delta: one line `android:name=".TaigiKeyboardApplication"` on `<application>`.
2. **Kotlin `object` migration pattern** — A1 converts stateful objects to classes with constructor DI. Default pattern — `companion object { @Volatile lateinit var INSTANCE }` initialized once from composition root, or plain class with no singleton? Recommendation: plain class, instances held by the `CompositionRoot`. No `INSTANCE` resurrection.
3. **`FrequencyData` hoist** — iOS hoisted from `UserFrequencyService` (nested) to top-level in PR #133. Android should mirror. Decision: timing — bundle with A1 or separate round? Recommendation: A1 (same refactor pass).
4. **LoggerBackend Android equivalent** — introduce `LoggerBackend` interface + `AndroidLogLoggerBackend` adapter wrapping `android.util.Log`. Engine files use `LoggerFactory.make(category:)` instead of calling `Log.d` directly. Decision: bundle with A1 or ship as its own A0.5? Recommendation: A1 precondition (carved out as the first PR of A1).
5. **Gradle module split timing** — a `:shared-core` Kotlin module is the clean carve-out boundary for Phase IV-A. Does Phase II include the module split (high signal, matches iOS SPM-target parity), or defer to Phase III? Recommendation: defer. Phase II scope is refactor-only within the current module; moving files across modules invalidates the freeze rule's observability claim (R8 rules + release APK layout change).
6. **Package-layout re-split** — match iOS `Lexicon/ + NextWord/ + Phonetics/` package tree vs keep current `ime.dictionary.*` catch-all. Decision: mechanical but churny. Recommendation: defer until A8-sweep is done and shared-core marker is on every file. Then a single rename PR relocates files into iOS-matching packages.

7. **`CandidateProcessor.sortByScore` data-access split** (Codex post-review 2026-04-19) — Android `CandidateProcessor.calculateScore` reaches `UserFrequencyService.frequencyDataBatch(...)` directly (§2.3 "stateless but not shared-core-clean"). iOS mirror takes `frequencyData: [FrequencyData]` as a method parameter from the caller. Decision: in A1 (bundled with the logging cleanup) vs defer to A8-sweep. Recommendation: A1 — the call-site already batch-fetches the frequency data; moving the call from inside `calculateScore` to the caller is a one-file change that unblocks the shared-core claim on `CandidateProcessor.kt` and mirrors the iOS signature.

8. **Service-graph composition timing vs IME manager-graph unwind** (Codex post-review 2026-04-19) — explicit decision to split A1 (introduces minimal `CompositionRoot` holding services) from A7 (extends it to hold managers) rather than one big round. Captured in both A1 and A7 deliverables above; surfaced here so future rounds don't re-litigate it.

---

## 9. Phase II gating signals

Advance to the Phase II end decision point (see `rules/cross-platform-alignment.md` §4) when ALL of:

1. **Engine purity** — no `LexiconService.INSTANCE` / `NextWordService.INSTANCE` / `TrieService.INSTANCE` reach inside Android engine-layer files. Every stateful service is injected via constructor or accessed via `CompositionRoot`. `android.util.Log` calls inside engine-layer files route through `LoggerBackend`.
2. **UI purity** — `ui/tabs/tab3/*` Compose screens contain zero direct service calls. All DB work lives in a ViewModel. Same holds for `ui/tabs/tab4/InputSettingsScreen.kt`.
3. **Exclusions shrunk** — `ComposingManager`, `NextWordService` are split. Android shared-core roster ≥ 40 files (mirror of iOS count).
4. **Doc parity** — `android-exemplar.md` exists. `composing-state-boundary.md` + `nextword-engine-boundary.md` have Android-binding sections. `// region Shared-Core Candidate` marker on every candidate Kotlin file. `CROSS-PLATFORM INVARIANT` comments on all four surfaces listed in `ios-exemplar.md` §5.3.
5. **Test baseline** — top-10 candidate coverage ≥ 70% on Android, `INVARIANT_*` labels mirrored from iOS tests.
6. **Latency dogfooding pass** — S1/S2/S3 on real Android device(s) show no user-perceptible typing latency regression after A2 / A4-impl / A5-impl land.
7. **Memory leak / stability pass** — no IME service kills observed during extended typing session. Refactor-only: steady-state capacity unchanged.
8. **Data artifact audit addendum** — `data-artifacts-portability.md` has Android sections filled in (A10).

---

## 10. Out of scope for Phase II

- Shared-core module extraction (Phase IV-A / IV-B).
- Rust FFI design / build (Phase IV-A).
- FFI POC (Phase III).
- Rust module carve-out / Gradle `:shared-core` module split (Phase III or later).
- `/shared-core-confidence` skill build (to be developed later in Phase II once A1–A3 expose the surface area).
- Feature additions / bug fixes outside the narrow emergency tier defined in `rules/cross-platform-alignment.md` §1.
- Performance optimization rounds (qualitative gate only).
- Phase II end decision point — happens **after** Phase II closes, not inside it.

---

## 11. Cross-references

- iOS exemplar contract: `ios-exemplar.md` (the alignment target).
- iOS Phase I plan: `ios-exemplar-plan.md` (G0–G10 closed 2026-04-19).
- Shared-core readiness roster: `../engine/shared-core-readiness.md`.
- Behavioral invariants (both platforms): `behavioral-invariants.md`.
- Composing boundary: `composing-state-boundary.md` (A4-design appends Android §).
- NextWord boundary: `nextword-engine-boundary.md` (A5-design appends Android §).
- Data-artifact portability (iOS authored): `data-artifacts-portability.md` (A10 appends Android §).
- Cross-platform alignment rules: `../../rules/cross-platform-alignment.md`.
- Roadmap phase context: `memory/project_shared_core_roadmap.md`.
