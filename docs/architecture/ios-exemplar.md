# iOS Exemplar — Architectural Contract for Android Phase II

**Status**: authored 2026-04-19 as Phase I G8 deliverable (ahead of G1–G7 / G9 implementation, so the pattern guides that work). Revised same day after Codex + Gemini review cycle; the Android-side sections (§1 mapping, §3 live-read, §5.2 marker, §5.3 constants, §6 ViewModel pattern) are now concrete enough to port from directly. This is the doc Android Phase II copies from — "look at iOS and copy it" is not a plan; this is.

**Audience**: whoever is doing Android Phase II alignment (and later, the Rust Phase IV-A author who needs to know what behavioral surface survives).

**Scope**: architectural pattern only. Behavioral contracts live in `behavioral-invariants.md`; the live Rust / native ownership inventory lives in `../engine/migration-inventory.csv`; data-artifact portability (`dictionary.fst` / `dictionary.bin` / SQLite) lives in `data-artifacts-portability.md` — all three are referenced but not duplicated here.

**Contract of this doc**: if Android's Phase II code diverges from the pattern below, fix Android (not this doc) unless the divergence is justified by a platform constraint documented inline.

---

## 1. Layer map

```
┌──────────────────────────────────────────────────────────────────┐
│  View (SwiftUI / Compose)                                        │
│   • Reads @Published / StateFlow from ViewModel                  │
│   • Forwards user actions to ViewModel                           │
│   • Does NOT call repositories / services / SharedSettings       │
└─────────────────────────┬────────────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────────────┐
│  ViewModel (ObservableObject / AndroidX ViewModel)               │
│   • @Published / StateFlow state                                 │
│   • Owns async boundaries (Task / viewModelScope)                │
│   • Calls Services / Platform engine executors                   │
│   • Reads EngineSettingsProvider.current (live, per access)      │
└─────────────────────────┬────────────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────────────┐
│  Platform engine executor (iOS / Android specific)               │
│   • Timer, @MainActor, settings snapshot, Effect dispatch        │
│   • Delegates (UITextDocumentProxy / InputConnection)            │
│   • Owns query-generation counter (NextWord race elimination)    │
│   • Forwards intents to Pure Engine                              │
│   • Interprets Outcome.Effect values                             │
└─────────────────────────┬────────────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────────────┐
│  Pure Engine (Foundation-only / Kotlin stdlib only)              │
│   • No Combine / UIKit / SwiftUI / OSLog / Timers                │
│   • No Android SDK / Compose / coroutines Dispatchers            │
│   • No singletons (no *.shared / companion-object state)         │
│   • No clock reads (caller supplies nowMs)                       │
│   • decide(intent, state, input) → Outcome                       │
└─────────────────────────┬────────────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────────────┐
│  Service                                                         │
│   • Coordinates side effects (DB read/write, file I/O, logging)  │
│   • Accepts dependencies via init (DI); default = .shared        │
│   • Returns engine-layer value types (no KeyboardKit / Android   │
│     framework types; maps platform rows → shared-core DTOs       │
│     at the boundary, e.g. Prediction → RawNextWordPrediction)    │
└─────────────────────────┬────────────────────────────────────────┘
                          │
┌─────────────────────────▼────────────────────────────────────────┐
│  Repository                                                      │
│   • SQLite / binary mmap access                                  │
│   • Platform-specific (FileManager / Context, App Group / app-   │
│     private storage)                                             │
│   • Never exposed above Service                                  │
└──────────────────────────────────────────────────────────────────┘
                          │
                          ▼
                 SQLite / dictionary.bin / association.bin
```

**Android mapping (concrete)**:

| iOS concept | Android equivalent | Concrete rule |
|---|---|---|
| SwiftUI `View` | Compose `@Composable` or Android `View` | Observes state via `collectAsStateWithLifecycle()` (Compose) or `LiveData`/`Flow` observer. |
| SwiftUI `ObservableObject` ViewModel | Android `androidx.lifecycle.ViewModel` (app-tab VMs) or lifecycle-scoped state holder (IME-scoped VMs — see §6 caveat) | `viewModelScope` for async work. |
| `@Published` property | `StateFlow<State>` backed by a private `MutableStateFlow` | Private mutable, public immutable: `val state: StateFlow<State> = _state.asStateFlow()`. |
| `Task { @MainActor … }` | `viewModelScope.launch(Dispatchers.Main.immediate)` for UI updates; `viewModelScope.launch(Dispatchers.IO)` for DB reads | Dispatcher chosen per effect. |
| `EngineSettingsProvider.current` (live, no snapshot) | Settings facade backed by `SharedPreferences` — MUST re-read per call (no `val` caching). | See §3. |
| Pure engine (Foundation-only) | Pure Kotlin (stdlib only — no Android SDK, no Compose, no coroutines Dispatchers). | Enforced by dependency inspection in CI. |
| `ComposingDelegate` (UITextDocumentProxy wrapper) | `InputConnection` wrapper implementing the same neutral `ComposingTransition.Effect` enum | See §4.1 and `composing-state-boundary.md` §2.2 for the Effect → InputConnection mapping table (including the `finishComposingText` commits-by-default caveat). |
| `NextWordController` (Timer + @MainActor executor) | IME-service-scoped executor with `android.os.Handler` or coroutine `delay()` on `Dispatchers.Main` | Holds a single `Job` for the context timeout; cancel-before-reschedule. |
| `OSSignposter` | `Trace.beginSection` / `Trace.endSection` for performance baselining | See `../perf/keyboard-baseline-2026-04.md`. |

**Key rule**: Pure-engine files must compile with **Foundation only on iOS** and **Kotlin stdlib only on Android**. If an Android port drags in `android.util.Log`, `android.os.SystemClock`, `kotlinx.coroutines.*`, or anything from `android.content`, that is a shared-core failure — route through `LoggerBackend` / injected `nowMs`.

---

## 2. DI composition root

**iOS**: composition lives at the process entry points — there are two:

1. **Host app** — `App/TaigiKeyboardApp.swift` (`@main struct TaigiKeyboardApp: App`).
2. **Keyboard extension** — `KeyboardExtension/KeyboardViewController.swift` (`viewDidLoad`).

Each entry point constructs the object graph once:

```swift
// Pseudocode — exact shape after G7 (singleton stripping):
@main struct TaigiKeyboardApp: App {
    let root: CompositionRoot = .app()       // constructs all services for the host app
    var body: some Scene { WindowGroup { ContentView().environmentObject(root) } }
}

// KeyboardViewController (extension):
override func viewDidLoad() {
    super.viewDidLoad()
    self.root = .keyboardExtension()         // lighter graph, no BackupService etc.
    // pass pieces of root into KeyboardView init
}
```

**CompositionRoot** (planned under G7) is a plain struct holding the service instances; not a framework. Views reach its pieces via plain init parameters or `@EnvironmentObject`. No `@Environment` magic beyond what SwiftUI already provides.

**What must NOT be in the composition root**:
- Side-effect initialization (loading `dictionary.bin`, starting Timers) — services own their own lazy load.
- Feature toggles — those live in `EngineSettingsProvider`.
- Global singletons — `static let shared` persists only for `SharedSettings` (cross-process store) after G7.

**Android parallel**:
- Application-level DI in `Application.onCreate`.
- IME service-level DI in `TaigiInputMethodService.onCreate`.
- Hilt / manual DI both acceptable; the pattern is "construct once per entry point, inject via init".

---

## 3. Settings access pattern

**Protocol** (already in roster, file `Settings/EngineSettings.swift` + `Settings/EngineSettingsProvider.swift`):

```swift
// Foundation-only, shared-core candidate
public protocol EngineSettings {
    var inputMode: InputMode { get }
    var isAutoCapitalizationEnabled: Bool { get }
    var isTranslateSwapped: Bool { get }
    var isAssociationRecordingEnabled: Bool { get }
    var toneToggles: ToneToggles { get }     // added by G4 Precondition
    // … other read-only engine-visible flags
}

public protocol EngineSettingsProvider {
    var current: EngineSettings { get }      // LIVE read, not snapshot
}
```

**Rules**:

1. **Live read, not snapshot** — every call to `.current` returns fresh values. Invariant §11 in `behavioral-invariants.md`.
2. **Engine never reads directly** — engine functions take `EngineSettings` as a value parameter; platform executor reads `.current` at intent boundaries and passes the snapshot to the engine.
3. **Only one concrete implementation binds the provider** — on iOS, `SharedSettings.shared` conforms to `EngineSettingsProvider`. The composition root hands it to services at init.
4. **Cross-process sync stays platform-side** — iOS uses Darwin notifications to bump a version counter; Android uses `SharedPreferences` listeners. The engine is oblivious.

**Android-specific live-read warning** (flagged by reviewers): Kotlin makes it easy to accidentally snapshot. A common bug:

```kotlin
// WRONG — `val` captures a snapshot at construction time; subsequent SharedPreferences
// updates are invisible until the object is re-created.
class AndroidEngineSettings(prefs: SharedPreferences) : EngineSettings {
    override val inputMode: InputMode = prefs.getString("input_mode", "tl").toInputMode()
    // …
}
```

```kotlin
// RIGHT — property accessor re-reads on every access.
class AndroidEngineSettings(private val prefs: SharedPreferences) : EngineSettings {
    override val inputMode: InputMode
        get() = prefs.getString("input_mode", "tl").toInputMode()
    // …
}

class AndroidSettingsProvider(private val prefs: SharedPreferences) : EngineSettingsProvider {
    override val current: EngineSettings
        get() = AndroidEngineSettings(prefs)      // or cache the inner object — but never its fields
}
```

Code review must check that every field on `EngineSettings` is either a `get()` property or a method call. `val x = prefs.getX()` in the initializer is a bug.

---

## 4. Engine vs platform split (planned Phase I pattern)

Two instances of this pattern ship during Phase I implementation (G4-impl + G5-impl); both design docs land before the code does:

### 4.1 Composing pipeline — see `composing-state-boundary.md`

- **Pure** — `ComposingState` (struct), `ComposingTransition` (struct + `Effect` enum), `ToneToggles` (struct).
- **Platform** — `ComposingManager` (`ObservableObject`, `@Published`, owns `ComposingDelegate` implemented by `KeyboardViewController` on iOS and by an `InputConnection` wrapper on Android).
- **Contract** — `Effect` enum names are platform-neutral (`updatePreedit`, `clearPreeditWithoutCommit`, `commitTextReplacingPreedit`, `deleteBackwardFromDocument`, `resetAutocomplete`, `performAutocomplete`, `resetAutocompleteContext`). iOS and Android bindings interpret the same enum. **Critical Android caveat**: `finishComposingText()` commits the composing region by default — bindings MUST zero the region via `setComposingText("", 1)` before issuing it, to honor `clearPreeditWithoutCommit` semantics. See the Effect → platform mapping table in `composing-state-boundary.md` §2.2.

### 4.2 NextWord pipeline — see `nextword-engine-boundary.md`

- **Pure** — `NextWordEngine` (enum, static `decide(intent:state:input:)`), `NextWordIntent` / `NextWordPersistedState` / `NextWordDecisionInput` / `NextWordOutcome` / `Outcome.Effect`, `RawNextWordPrediction` (shared DTO replacing the platform-service `Prediction` type at the engine boundary).
- **Platform** — `NextWordController` (owns `Timer`, `@MainActor` dispatch, settings snapshot, query-generation counter).
- **Contract** — Engine never reads clock; executor supplies `nowMs`. Scheduling = effect values (`rescheduleContextTimeout(after:)`, `cancelContextTimeout`). Prediction races eliminated by `currentGeneration` on both `queryPredictions` and `clearPredictionsUI` — late query results drop on generation mismatch.

**Generalizable rule (for future splits)**:

> When a type mixes state + scheduling + platform I/O, split into two files. Keep the state transition as a pure function returning an `Effect`-list value. Let the platform file execute effects sequentially. Tests target the pure function; integration tests target the executor. If the state is accessed concurrently across async boundaries, add a monotonic generation counter to invalidate stale results.

---

## 5. Naming + file layout rules

### 5.1 Directory conventions

```
Sources/TaigiKeyboard/
├── App/                      # Host-app target code (tabs, views)
│   ├── TaigiKeyboardApp.swift
│   ├── ContentView.swift
│   └── Tabs/
│       ├── <Feature>/
│       │   ├── <Feature>View.swift
│       │   └── <Feature>ViewModel.swift
├── KeyboardExtension/        # Extension-only code (controller, setup)
├── Actions/                  # KK ActionHandler seams (platform-side)
├── Autocomplete/             # Autocomplete service + views + VMs
├── Callouts/, Emojis/, Layout/, Overlays/, Styling/    # UI only
├── Logging/                  # Cross-cutting: LoggerBackend
├── Input/                    # Input pipeline (shared-core heavy)
│   ├── Composing/
│   │   ├── ComposingState.swift         # PURE (G4-impl)
│   │   ├── ComposingTransition.swift    # PURE (G4-impl)
│   │   ├── ToneToggles.swift            # PURE (G4-impl, may colocate with EngineSettings)
│   │   ├── ComposingManager.swift       # PLATFORM
│   │   └── ComposingDelegate.swift      # PLATFORM
│   └── TPS/
├── Lexicon/                  # Candidates + DB
│   ├── Models/, Utils/, Trie/            # shared-core candidates
│   ├── Services/                         # platform
│   └── Database/                         # platform
├── NextWord/
│   ├── NextWordScorer.swift              # PURE
│   ├── NextWordEngine.swift              # PURE (G5-impl)
│   ├── NextWordOutcome.swift             # PURE (G5-impl)
│   ├── RawNextWordPrediction.swift       # PURE (G5-impl)
│   ├── NextWordController.swift          # PLATFORM
│   ├── Services/                         # platform
│   └── Repository/                       # platform
├── Phonetics/                # PURE (incl. parameterized ToneConverter after G4 precondition)
├── Settings/                 # EngineSettings + provider (PURE protocols)
└── Strings/                  # localized strings (platform)
```

### 5.2 Marker comment (required at top of every shared-core candidate)

**Swift**:

```swift
// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
```

**Kotlin** (Android mirror):

```kotlin
// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
```

Android Studio's `// region` / `// endregion` pair gives the same code-folding hint as Swift's `// MARK:`. Verification greps look for the literal string `Shared-Core Candidate` on a comment line — both syntaxes satisfy it. Per-file ownership now lives in `../engine/migration-inventory.csv`; the historical roster doc has been retired.

### 5.3 Cross-platform invariant comment (required when a constant must mirror Android)

```swift
/// CROSS-PLATFORM INVARIANT — constants MUST mirror Android
/// `<path/to/KotlinFile.kt>`. Drift causes silent divergence.
```

Already used in `NextWordScorer.swift` and `TaigiUnicode.swift`. Required wherever a numeric constant is duplicated in Kotlin. Explicit list of constants that need this marker after Phase I implementation:

- **Scoring constants** (`NextWordScorer.swift`): `userWeight (50.0)`, `dictWeight (1.0)`, `decayHalfLifeHours (168.0)`, `learningBonus (300.0)`, `highUsageDecayFloor (0.95)`, `lowUsageDecayFloor (0.3)`, `highUsageThreshold (3)`.
- **Candidate scoring** (`CandidateProcessor.swift`): recency window (`1 hour`), `cappedUserFreq` cap (`100`), user-freq multiplier (`100`), completion penalty (`-1000`), closeness max (`500`), exact bonus (`100`), recency bonus (`200`), **tier multipliers `SOURCE_TIERS` (`kautian=15, taigitv=13, stti=12, kungge=11`) + `TIER_DENOMINATOR (10)`** — bit positions mirror `dictionary/build/10_create_dictionary_bin.py`; first-match-wins traversal.
- **NextWord timing** (`NextWordEngine.swift` after G5-impl): `associationTimeoutMs (10_000)`, `contextTimeoutSeconds (30)`.
- **Taigi Unicode preprocessing** (`TaigiUnicode.swift`): `U+207F` / `U+1D3A` / `U+0358` codepoint handling.

### 5.4 Naming

- **Protocols** — describe capability, not identity. `ComposingStateProvider` ✅, `ComposingManagerProtocol` ✗.
- **Enums as pure namespaces** — `enum NextWordScorer { static func scoreDict(…) }` ✅ (no instances, pure static). Matches the style already in the roster.
- **Struct value types** — prefer for state (`ComposingState`, `NextWordPersistedState`, `FrequencyData`, `TaigiWord`).
- **Verb-style methods** — `calculateScore`, not `scoreFor`. Matches Swift API design guidelines.

### 5.5 Access levels

- `public` only when the type crosses a target boundary (extension ↔ host app) or is genuinely part of the engine's external contract. Shared-core types are `public` because a future shared module imports them.
- `internal` default.
- `private` inside implementation helpers.

Do NOT leave `public` on platform executor internals "just in case" — that is how singletons bled back in before.

---

## 6. ViewModel pattern (Android copy target)

After G3 (Dictionary tab Views → ViewModel), every tab view follows this shape on iOS:

```swift
@MainActor
final class <Feature>ViewModel: ObservableObject {
    @Published private(set) var state: <Feature>State = .idle

    private let service: <Feature>Service
    private let settings: EngineSettingsProvider

    init(service: <Feature>Service, settings: EngineSettingsProvider) {
        self.service = service
        self.settings = settings
    }

    func onAppear() { Task { await load() } }

    func <userAction>() { Task { … } }

    private func load() async { … await service.… }
}
```

**Kotlin equivalent** (concrete, not hand-wavy — reviewers flagged this):

```kotlin
class <Feature>ViewModel(
    private val service: <Feature>Service,
    private val settings: EngineSettingsProvider,
) : androidx.lifecycle.ViewModel() {

    // Private mutable, public immutable — the canonical StateFlow idiom.
    private val _state = MutableStateFlow<<Feature>State>(<Feature>State.Idle)
    val state: StateFlow<<Feature>State> = _state.asStateFlow()

    fun onAppear() {
        viewModelScope.launch { load() }
    }

    fun <userAction>() {
        viewModelScope.launch(Dispatchers.IO) { … }
    }

    private suspend fun load() {
        _state.value = <Feature>State.Loading
        try {
            val data = withContext(Dispatchers.IO) { service.fetch() }
            _state.value = <Feature>State.Loaded(data)
        } catch (t: Throwable) {
            _state.value = <Feature>State.Error(t)
        }
    }
}
```

Rules (both platforms):

1. View owns *no* `Task` / `launch`, *no* repository reference, *no* `SharedSettings` read.
2. ViewModel owns `Task` / `viewModelScope.launch` launches; View invokes ViewModel methods.
3. State is a sealed enum / sealed class: `Idle | Loading | Loaded(T) | Error(Throwable)`. No nullable fields representing "not yet loaded".
4. Constructor DI — no `.shared` / `companion object` default inside the ViewModel. Defaults live at the composition root.
5. **Dispatcher ownership** (Android): DB reads on `Dispatchers.IO`, UI state updates on `Dispatchers.Main.immediate` (or let StateFlow collection in Compose handle the hop). Never leak `Dispatchers` into shared-core code.
6. **Collection on the View side** (Compose): `val state by viewModel.state.collectAsStateWithLifecycle()` so collection pauses when the lifecycle stops; unnecessary recomposition avoided.

**IME ViewModel caveat** (Android-specific): `androidx.lifecycle.ViewModel` is lifecycle-scoped to `ViewModelStoreOwner`. The IME service (`InputMethodService`) is not a `ViewModelStoreOwner` by default. For keyboard-extension-side state (the Android equivalent of `KeyboardViewController` state), either:

- use the IME-service's own scope (`lifecycleScope` if extending `LifecycleService`, else a manual `CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)` canceled in `onDestroy`), or
- implement a plain state-holder class owning a `MutableStateFlow` + cancellation hook — no AndroidX `ViewModel` inheritance required.

The AndroidX `ViewModel` class is for tab / settings app code, not for IME-internal state.

**IME thread rule**: all `InputConnection` calls (the Android counterpart of `UITextDocumentProxy`) MUST run on the IME's main thread. The platform executor enforces this by dispatching `Outcome.Effect` execution through `Dispatchers.Main.immediate`. Engine-returned Effect lists are thread-agnostic; the executor is the thread gate.

---

## 7. Immutable contract surface — 36 shared-core candidates (→ 43 after G4 + G5 impl)

Live Rust / native roster lives in `../engine/migration-inventory.csv`. The historical Phase I categorisation is preserved below for context; most items now belong to Rust crates.

- **Phonetics** (9): `TaigiPhonetics`, `PhoneticsTables`, `SyllableParser`, `TLFormatter`, `POJFormatter`, `PhoneticsConverter`, `RomanizationConverter`, `ToneRestoration`, `ToneUtilities`.
- **Input** (7): `CharacterInputPipeline`, `CaseTransformer`, TPS (`TPSConverter`, `TPSTables`, `TPSInputAdjuster`, `TPSToTL`, `TLToTPS`).
- **Lexicon** (11): models + `TaigiUnicode` + `CandidateProcessor` + `InputNormalizer` + `CustomDictionaryDerivation`.
- **NextWord** (3): `EnginePrediction`, `NextWordScorer`, `AutocompleteContextBooster`.
- **Autocomplete** (2): `AutocompleteInputClassifier`, `AutocompleteProviders`.
- **Settings** (3): `EngineSettings`, `EngineSettingsProvider`, `InputMode`.
- **Logging** (1): `LoggerBackend`.

Planned additions after Phase I implementation:

- **G4-impl (+4)**: `ComposingState`, `ComposingTransition`, `ToneToggles`, promoted `ToneConverter` (after Precondition parameterization).
- **G5-impl (+3)**: `NextWordEngine`, `NextWordOutcome`, `RawNextWordPrediction`.

Roster target by end of Phase I: **43 candidates**. Phase II gating signal #3 asks for ≥ 40 — met.

**Contract surface rule**: adding to the roster requires the five-grep verification to pass. Removing from the roster requires a written rationale + Codex review.

---

## 8. Anti-patterns explicitly banned

The refactors in Phase I exist because these previously slipped in. They must not return:

1. **`.shared` read inside an engine-layer file.** Use DI with `= .shared` default at the boundary only.
2. **`@Published` / `ObservableObject` / `StateFlow` on a type that has no view.** If a type isn't driving UI, don't pay Combine's / Flow's cost — use plain properties + a notify hook.
3. **Clock read inside a pure function.** `Date()`, `CFAbsoluteTimeGetCurrent`, `DispatchTime.now`, `System.currentTimeMillis()`, `android.os.SystemClock.*` → caller passes `nowMs`.
4. **Direct repository call from a View.** Always a ViewModel in between.
5. **Singleton resurrection.** If a service is `static let shared` / `companion object INSTANCE`, it must also accept DI; if DI defaults are gone after G7, do not reintroduce `shared`.
6. **Combine / Flow publishers crossing target boundaries.** Extension and host app each own their own object graph.
7. **`UIKit` / `SwiftUI` / Android SDK / Compose imports inside `Lexicon/Models/`, surviving `Autocomplete/Services/` shells, or any Foundation-only Lexicon utility file.** (Phonetics, TPS, Composing engine, NextWord engine, and case-transform are all in Rust now.) Enforced by `Foundation`-only import greps + the Rust-side `forbid(unsafe_code)` lint per `.claude/rules/rust-best-practices.md`.
8. **Platform-bound DTOs in engine signatures.** `NextWordService.Prediction` leaking into `filterPredictions` was the Codex finding that motivated `RawNextWordPrediction`. Rule: services map their rows to shared-core DTOs at the service boundary.
9. **`SharedPreferences` snapshot via `val`.** See §3. Every settings field must be re-read on access.

---

## 9. Phase II expectations (Android mirror)

Phase II delivers an Android target in the same shape:

- **File-level parity** — every pure iOS file has one Kotlin twin in the same layer position. Identical constants. Identical invariant comments cross-referencing each other.
- **Behavioral parity** — `behavioral-invariants.md` tests exist on both platforms (xcodebuild test + gradle test), with identical labels.
- **DI root parity** — Android has one composition root per entry point (Application + InputMethodService); no `.shared` / `companion object` singletons reach into engine files.
- **ViewModel parity** — Kotlin `ViewModel` + `StateFlow` shape for every Dictionary tab view (per §6); repositories reachable only via services called from VMs.
- **Effect-enum parity** — `ComposingTransition.Effect` and `NextWordOutcome.Effect` implemented verbatim on Android. The InputConnection binding contract (§4.1 + `composing-state-boundary.md` §2.2) is the only place Android deviates in behavior, and the deviations are documented there.

**Phase II entry gate**: Android state audit doc (pre-phase deliverable) documents the current deltas between Android code and this exemplar. Without that audit, Phase II is translation work without a map.

---

## 10. Cross-references

- Live Rust / native ownership inventory: `../engine/migration-inventory.csv`.
- Phase I G0–G10 plan: closed and removed; see git history for the historical task list.
- Behavioral invariants: `behavioral-invariants.md`.
- Engine/platform split exemplars: `composing-state-boundary.md`, `nextword-engine-boundary.md`.
- Latency + memory gates that protect this architecture: `../perf/keyboard-baseline-2026-04.md`, `../perf/extension-memory-2026-04.md`.
- Data-artifact portability (G10, pending): `data-artifacts-portability.md`.
- Codex strategic review motivating Phase 0 / G0 / G8 / G10: `codex-review-2026-04-19.md`.
- Docs-review cycle (2026-04-19) that reshaped the Effect enum, added RawNextWordPrediction, mandated generation-based race elimination, and concretized the Android mapping: findings incorporated throughout.
