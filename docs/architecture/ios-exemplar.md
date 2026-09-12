# iOS Exemplar — Cross-platform Architectural Pattern

**Type**: Reference. **Status**: authored 2026-04-19 as the alignment target for the Android port; Android converged on it in 2026-04 (the A-rounds), and since the Rust extraction (v3.5.1 → v3.5.9) the pure-engine layer lives in `engine/<crate>` crates rather than in Swift / Kotlin. The pattern below still governs how every platform's glue (iOS, Android, macOS, Windows) is shaped around the engine. Android-specific deviations are in §9.

**Audience**: anyone aligning a platform's structure to iOS, or deciding what behavioral surface the Rust engine must preserve.

**Scope**: architectural pattern only. Behavioral contracts live in `behavioral-invariants.md`; the live Rust / native ownership inventory lives in `../engine/migration-inventory.csv`; data-artifact portability (`dictionary.fst` / `dictionary.bin` / SQLite) lives in `data-artifacts-portability.md` — all three are referenced but not duplicated here.

**Contract of this doc**: if a platform's code diverges from the pattern below, fix the platform (not this doc) unless the divergence is justified by a platform constraint documented inline (§9 for Android).

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
| `OSSignposter` | `Trace.beginSection` / `Trace.endSection` for performance baselining | |

**Key rule**: Pure-engine files must compile with **Foundation only on iOS** and **Kotlin stdlib only on Android**. If an Android port drags in `android.util.Log`, `android.os.SystemClock`, `kotlinx.coroutines.*`, or anything from `android.content`, that is a shared-core failure — route through `LoggerBackend` / injected `nowMs`.

---

## 2. DI composition root

**iOS**: composition lives at the process entry points — there are two:

1. **Host app** — `App/TaigiKeyboardApp.swift` (`@main struct TaigiKeyboardApp: App`).
2. **Keyboard extension** — `KeyboardExtension/KeyboardViewController.swift` (`viewDidLoad`).

Each entry point constructs the object graph once:

```swift
// Pseudocode — shape since the singleton-stripping round:
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

**CompositionRoot** (`Composition/CompositionRoot.swift`) is a plain struct holding the service instances; not a framework. Views reach its pieces via plain init parameters or `@EnvironmentObject`. No `@Environment` magic beyond what SwiftUI already provides.

**What must NOT be in the composition root**:
- Side-effect initialization (loading `dictionary.bin`, starting Timers) — services own their own lazy load.
- Feature toggles — those live in `EngineSettingsProvider`.
- Global singletons — `static let shared` persists only for `SharedSettings` (cross-process store).

**Android parallel**:
- Application-level DI in `TaigiKeyboardApplication.onCreate`.
- IME service-level DI in `TaigiKeyboard.onCreate` (`LifecycleInputMethodService`). See §9.1.
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
    var toneToggles: ToneToggles { get }
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

## 4. Engine vs platform split

Two instances of this pattern exist; both state machines now live in Rust (`engine/composing`, `engine/nextword`) and the platform files below are the executors. The boundary docs describe the contract each platform binds to:

### 4.1 Composing pipeline — see `composing-state-boundary.md`

- **Pure** — `engine/composing` (`ComposingState`, `ComposingTransition`, the `Effect` list); `ToneToggles` on the settings side.
- **Platform** — `ComposingManager` (`ObservableObject`, `@Published`, owns `ComposingDelegate` implemented by `KeyboardViewController` on iOS and by an `InputConnection` wrapper on Android).
- **Contract** — `Effect` enum names are platform-neutral (`updatePreedit`, `clearPreeditWithoutCommit`, `commitTextReplacingPreedit`, `deleteBackwardFromDocument`, `resetAutocomplete`, `performAutocomplete`, `resetAutocompleteContext`). iOS and Android bindings interpret the same enum. **Critical Android caveat**: `finishComposingText()` commits the composing region by default — bindings MUST zero the region via `setComposingText("", 1)` before issuing it, to honor `clearPreeditWithoutCommit` semantics. See the Effect → platform mapping table in `composing-state-boundary.md` §2.2.

### 4.2 NextWord pipeline — see `nextword-engine-boundary.md`

- **Pure** — `engine/nextword` (`decide(intent, state, input) → Outcome`, `NextWordIntent` / `NextWordPersistedState` / `NextWordDecisionInput` / `NextWordOutcome` / `Outcome.Effect`), `RawNextWordPrediction` (shared DTO replacing the platform-service `Prediction` type at the engine boundary).
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
│   └── Tabs/<Feature>/{<Feature>View,<Feature>ViewModel}.swift
├── KeyboardExtension/        # Extension-only code (controller, setup)
├── Composition/              # CompositionRoot (DI, §2)
├── Engine/                   # RustEngineBridge + RustEngineBridge+<Area>.swift (proto FFI)
├── Actions/                  # KK ActionHandler seams (platform-side)
├── Autocomplete/             # Autocomplete service + views + VMs
├── Callouts/, Emojis/, Layout/, Overlays/, Styling/    # UI only
├── Logging/                  # Cross-cutting: LoggerBackend
├── Input/                    # CharacterInputPipeline, AutoSpacePunctuation
│   └── Composing/            # ComposingManager + ComposingDelegate (PLATFORM executor)
├── Lexicon/                  # Models/, Utils/ (shared-core candidates) · Services/, Database/ (platform)
├── NextWord/                 # NextWordController (PLATFORM executor) · Services/, Repository/
├── Settings/                 # EngineSettings + provider (PURE protocols)
└── Strings/                  # localized strings (platform)
```

Phonetics, TPS, composing, next-word scoring / decision, ranking and case-transform have no Swift directory any more — they are Rust crates reached through `Engine/`.

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

Android Studio's `// region` / `// endregion` pair gives the same code-folding hint as Swift's `// MARK:`. Verification greps look for the literal string `Shared-Core Candidate` on a comment line — both syntaxes satisfy it (`grep -rl "Shared-Core Candidate" ios/Sources android/app/src/main`, 16 files each as of 2026-09-13). Per-file ownership lives in `../engine/migration-inventory.csv`.

### 5.3 Cross-platform invariant comment (required when a constant must mirror another platform)

```swift
/// CROSS-PLATFORM INVARIANT — constants MUST mirror Android
/// `<path/to/KotlinFile.kt>`. Drift causes silent divergence.
```

Required wherever a value is duplicated literally across platforms and drift would silently change behavior. Numeric engine constants (NextWord scoring — `USER_WEIGHT` / `LEARNING_BONUS` / decay in `engine/nextword/src/scorer.rs`; candidate `SOURCE_TIERS` / `TIER_DENOMINATOR` in `engine/ranking/src/score.rs`; NextWord timing `CONTEXT_TIMEOUT_MS` in `engine/nextword/src/decide.rs`) live once in Rust, so the marker now binds the surfaces that still have a per-platform copy: settings-key / model tables (`Settings/SettingsModels.swift` ↔ Android `EngineSettings.kt` ↔ macOS `CandidateFontChoice.swift` ↔ Windows `keys.rs`), candidate-strip layout constants, the `mul` keyboard locale tag (`behavioral-invariants.md` §39), and any residual timing constant a platform executor mirrors from the engine (Android `NextWordHandler.kt` `CONTEXT_TIMEOUT_MS`). The policy (constants + tests + docs update together, `INVARIANT_*` test-label prefix) lives in `.claude/rules/cross-platform-alignment.md` §3a.

### 5.4 Naming

- **Protocols** — describe capability, not identity. `ComposingStateProvider` ✅, `ComposingManagerProtocol` ✗.
- **Enums as pure namespaces** — `enum ExternalLookupURLBuilder { static func …(…) }` ✅ (no instances, pure static; `Lexicon/Utils/`).
- **Struct value types** — prefer for state (`ComposingState`, `NextWordPersistedState`, `FrequencyData`, `TaigiWord`).
- **Verb-style methods** — `calculateScore`, not `scoreFor`. Matches Swift API design guidelines.

### 5.5 Access levels

- `public` only when the type crosses a target boundary (extension ↔ host app) or is genuinely part of the engine's external contract. Shared-core types are `public` because a future shared module imports them.
- `internal` default.
- `private` inside implementation helpers.

Do NOT leave `public` on platform executor internals "just in case" — that is how singletons bled back in before.

---

## 6. ViewModel pattern (Android copy target)

Every tab view follows this shape on iOS:

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

## 7. Contract surface — shared-core candidates

The live roster is `../engine/migration-inventory.csv` (one row per symbol; `status` ∈ rust / native / wont_migrate). Historically iOS carried 43 Swift shared-core candidates across Phonetics, Input/TPS, Lexicon, NextWord, Autocomplete, Settings and Logging; the Phonetics, TPS, Composing, NextWord, ranking and case-transform groups migrated to Rust crates and only the Lexicon model / utility files, Settings protocols and `LoggerBackend` remain marked on each platform (16 files each on iOS and Android).

**Contract surface rule**: adding a marker requires the import-purity greps to pass (`.claude/rules/ios-shared-core-candidates.md` §1, `.claude/rules/android-guidelines.md` §1). Removing one requires a written rationale + Codex review.

---

## 8. Anti-patterns explicitly banned

These previously slipped in and were refactored out. They must not return:

1. **`.shared` read inside an engine-layer file.** Use DI with `= .shared` default at the boundary only.
2. **`@Published` / `ObservableObject` / `StateFlow` on a type that has no view.** If a type isn't driving UI, don't pay Combine's / Flow's cost — use plain properties + a notify hook.
3. **Clock read inside a pure function.** `Date()`, `CFAbsoluteTimeGetCurrent`, `DispatchTime.now`, `System.currentTimeMillis()`, `android.os.SystemClock.*` → caller passes `nowMs`.
4. **Direct repository call from a View.** Always a ViewModel in between.
5. **Singleton resurrection.** If a service is `static let shared` / `companion object INSTANCE`, it must also accept DI; do not reintroduce `shared` where DI defaults have been removed.
6. **Combine / Flow publishers crossing target boundaries.** Extension and host app each own their own object graph.
7. **`UIKit` / `SwiftUI` / Android SDK / Compose imports inside `Lexicon/Models/`, surviving `Autocomplete/Services/` shells, or any Foundation-only Lexicon utility file.** (Phonetics, TPS, Composing engine, NextWord engine, and case-transform are all in Rust now.) Enforced by `Foundation`-only import greps + the Rust-side `forbid(unsafe_code)` lint per `.claude/rules/rust-best-practices.md`.
8. **Platform-bound DTOs in engine signatures.** `NextWordService.Prediction` leaking into `filterPredictions` was the Codex finding that motivated `RawNextWordPrediction`. Rule: services map their rows to shared-core DTOs at the service boundary.
9. **`SharedPreferences` snapshot via `val`.** See §3. Every settings field must be re-read on access.

---

## 9. Android deviations from this exemplar

Android has converged on the shape above. Where it *must* diverge — for platform, framework, or language-semantic reasons — the divergence is recorded here so later rounds do not re-litigate it. Kotlin rules (marker criteria, invariant syntax, DI, coroutines, IME lifecycle) are in `.claude/rules/android-guidelines.md`; naming and access levels follow §5.4 / §5.5 with these refinements: `object` only for stateless utilities; shared-core types `public`, platform wiring `internal`; no `inline` / `reified` or cross-boundary extension functions at the shared-core boundary.

### 9.1 Composition scope

iOS composes the engine graph at two entry points (§2). Android also has two live scopes:

- **IME service graph** — `TaigiKeyboard : LifecycleInputMethodService` (`ime/core/TaigiKeyboard.kt`). Created in `onCreate`, torn down in `onDestroy`. Owns per-session state (composing, candidates, next-word history).
- **App graph** — `TaigiKeyboardApplication : Application` owns the warm-up chain (`prefs.warmUp()` + `migrateFromSharedPreferences` + per-service `init(context)`); `CompositionRoot` (`ime/core/CompositionRoot.kt`) is the process-wide service holder both scopes reach into. Services are idempotent, so a duplicate `init` during migration stays safe.

Engine-layer code receives its dependencies via constructor injection from whichever scope constructs it. No `.INSTANCE` reach-ins (`.claude/rules/android-guidelines.md` §4).

### 9.2 Settings live-read

Android stores settings in DataStore behind `PrefHelper`'s cached `Preferences` snapshot; iOS uses `UserDefaults` / `SharedSettings`. §3 warns about the `val x = prefs.getX()` snapshot anti-pattern. The Android mirror is `ime/core/settings/EngineSettings.kt` + `EngineSettingsProvider.kt`: each field on `EngineSettings` is a `get()` property that re-reads the cache on every access, and `EngineSettingsProvider.current` returns a live view. Rule enforced at PR review:

- Inside a shared-core candidate that receives `EngineSettings`, read through the property at the point of decision — never capture into a local `val` at constructor time.
- A caller that needs multi-field consistency within one operation captures a local copy explicitly.

### 9.3 Coroutines and threading

Shared-core candidates on Android must not import `kotlinx.coroutines.*`, `Dispatchers.*`, or own a `CoroutineScope`. Async orchestration lives in the platform wrapper exclusively — matching iOS (no `Task {}` or `DispatchQueue` inside shared-core Swift files). Platform-side conventions:

- `TaigiKeyboard.serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)` — cancelled in `onDestroy`.
- `ViewModel.viewModelScope` — cancelled automatically by AndroidX.
- `Dispatchers.IO` for DB / file / network; `Dispatchers.Default` for pure CPU.
- Clock injection: shared-core callers that need the wall clock take `nowMs: Long` as a parameter. No `System.currentTimeMillis()` inside candidate files.
- Long-running timers (context-timeout, association-timeout) use `kotlinx.coroutines.delay` from the platform wrapper (`ime/text/smartbar/NextWordHandler.kt`) — a straight port of the iOS `NextWordController` timer pattern. See `nextword-engine-boundary.md` §13 (Android binding).

### 9.4 InputConnection binding

`InputConnection.finishComposingText()` **commits** the current composing region by default — it is not a clear-without-commit primitive. The platform wrapper zeroes the composing region with `ic.setComposingText("", 1)` before calling `ic.finishComposingText()` when the intent is `clearPreeditWithoutCommit`. `commitTextReplacingPreedit` atomically replaces the composing region via a single `ic.commitText(text, 1)` — no pre-finish, or the preedit is double-committed. Full binding table and test hooks: `composing-state-boundary.md` §11.2.

### 9.5 Package layout

iOS uses `Lexicon/`, `NextWord/`, `Settings/` as sibling trees (§5.1). Android keeps `ime/dictionary/*` plus `ime/core/{logging,settings}/*` and `ime/text/{composing,smartbar}/*`; the trees are not renamed to match iOS because moving files across packages churns every import in the module for no behavioral gain.

---

## 10. Cross-references

- Live Rust / native ownership inventory: `../engine/migration-inventory.csv`.
- Behavioral invariants: `behavioral-invariants.md`.
- Engine/platform split exemplars: `composing-state-boundary.md` (§11 Android binding), `nextword-engine-boundary.md` (§13 Android binding).
- Data-artifact portability: `data-artifacts-portability.md`.
- Cross-platform policy (refactor-freeze, invariant-discipline, divergence docs): `.claude/rules/cross-platform-alignment.md`.
- Four-platform layout and build map: `system-overview.md`.
