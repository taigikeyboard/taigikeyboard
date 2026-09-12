# Composing State / Platform Boundary

> **Type**: Reference (contract) · **Section numbering**: renumbered? no — gaps are intentional. §2.2, §11.1–§11.10 are cited by code (`ComposingDelegate.swift` / `.kt`, `KeyboardViewController+TextInput.swift`, `TextInputManager.kt`, `MediaInputManager.kt`), by `behavioral-invariants.md` §13 and by `../engine/rust-core-proto.md`, so their numbers are frozen; the pre-Rust design sections (§1, §4–§9) were removed 2026-09-13 and their numbers are not reused.

**What this doc is**: the contract between the composing state machine — Rust `engine/composing` since v3.5.4 (Path G, old #197; `ComposingState.swift` / `.kt` were deleted) — and each platform's effect interpreter: iOS `Input/Composing/ComposingManager.swift` + `ComposingDelegate.swift` (`UITextDocumentProxy`), Android `ime/text/composing/ComposingManager.kt` + `ComposingDelegate.kt` (`InputConnection`, §11), macOS `Composing/ComposingManager.swift` + `ClientEffectExecutor.swift` (IMKit marked text), Windows `taigi-windows-core::composing` + `tsf/src/composition.rs` (TSF edit sessions). The **Effect → platform binding (§2.2)** and the ordering contracts (§2.3, §2.4) are what every interpreter honours; the wrappers are `wont_migrate` glue (`../engine/migration-inventory.csv`, `area=composing`). Originally authored 2026-04-19 as the Phase I G4 design (Codex + Gemini reviewed); iOS G4-impl old #138, Android A4-impl old #152.

---

## 2. Contract shape

The Swift sketches in §2.1–§2.4 are the original design notation; the state machine they describe now runs in Rust (`engine/composing`, `ComposingTransition` carried in the proto response) and the wrapper column is what each platform still implements.

```
┌──────────────────────────────────────────────────────────────────────┐
│  Platform wrapper (iOS): ComposingManager                            │
│  - ObservableObject + @Published isComposing/composingText/          │
│    rawInput/selectedCandidateIndex                                   │
│  - Owns ComposingDelegate wiring (UITextDocumentProxy side effects)  │
│  - Owns ComposingContextSink notification                            │
│  - Reads EngineSettingsProvider.current per intent                   │
│  - Forwards intent calls to ComposingState with settings snapshot    │
│  - Applies ComposingTransition in three phases (see §2.4):           │
│      1. mutate + publish state (guarded-inequality @Published)       │
│      2. execute Effect list sequentially (preedit + document)        │
│      3. notify ComposingContextSink                                  │
└──────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────────────┐
│  ComposingState (Foundation-only, shared-core candidate)             │
│  - enum Phase { idle, composing(raw) }                               │
│  - Intent API: start, append, appendHyphen, replaceLast,             │
│    deleteBackward, commit (→ either composition or raw),             │
│    selectSuggestion(text), reset                                     │
│  - Pure derivation: derivedDisplay(raw, mode, toneToggles)           │
│    via TPSTables.containsTPS / ToneConverter                         │
│  - Emits Transition values with a platform-neutral Effect list       │
│  - No Combine, no @MainActor, no SwiftUI, no UIKit, no singletons,   │
│    no clock reads                                                    │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.1 Pure-state intent API

```swift
// Foundation-only
public struct ComposingState: Equatable {
    public enum Phase: Equatable { case idle; case composing(raw: String) }

    public private(set) var phase: Phase = .idle
    public private(set) var selectedCandidateIndex: Int = -1   // -1 in idle; set to 0 on startComposing

    /// Derivation inputs live at the call site so the pure state never
    /// touches SharedSettings. `toneToggles` is a small value type
    /// carrying the two POJ preprocessing booleans.
    public func derivedDisplay(mode: InputMode, toneToggles: ToneToggles) -> String {
        // TPS symbols are display-ready; POJ/TL go through ToneConverter
        // (parameterized by toneToggles after the Precondition lands).
    }

    public var isComposing: Bool { if case .idle = phase { false } else { true } }
    public var rawInput: String { if case .composing(let r) = phase { r } else { "" } }
}

public struct ToneToggles: Equatable {
    public let isDoubleTapOOEnabled: Bool
    public let isDoubleTapNNEnabled: Bool
}
```

The `selectedCandidateIndex == -1` invariant in idle is asserted in `ComposingState.apply(...)` — every idle-producing path sets it explicitly. Tests verify this for `reset`, `deleteBackward` emptying raw, `commit*`, and `selectSuggestion`.

### 2.2 Transitions + platform-neutral Effects

Instead of the wrapper inferring side effects from property diffs, `ComposingState` returns a `Transition` describing exactly what the platform must do and in what order. Effect names describe **behavior**, not iOS or Android APIs — one of the Critical findings in the review cycle was that `setMarkedText` / `clearMarkedText` were encoded iOS semantics that do not map cleanly onto Android `InputConnection`.

```swift
public struct ComposingTransition: Equatable {
    public enum Effect: Equatable {
        case updatePreedit(String)           // preedit text currently shown to the user
        case clearPreeditWithoutCommit       // wipe preedit; do NOT commit what was there
        case commitTextReplacingPreedit(String)  // atomically replace preedit with given text
        case deleteBackwardFromDocument      // one grapheme from the backing text
        case resetAutocomplete               // engine autocomplete list
        case performAutocomplete             // trigger a new autocomplete pass
        case resetAutocompleteContext        // selection / bigram context
    }

    public let newPhase: ComposingState.Phase
    public let newSelectedIndex: Int          // always set (no "unchanged" sentinel)
    public let effects: [Effect]              // execute in order
    public let derivedDisplay: String         // "" in idle; non-empty in composing
}
```

#### Effect → platform mapping (binding contract)

Android Phase II authors must implement the same Effect enum against `InputConnection`. The mapping is spelled out here, not left to interpretation:

| Effect | iOS — `UITextDocumentProxy` | Android — `InputConnection` |
|---|---|---|
| `updatePreedit(text)` | `setMarkedText(text, selectedRange: end)` | `setComposingText(text, 1)` |
| `clearPreeditWithoutCommit` | `setMarkedText("", selectedRange: 0) + unmarkText()` | `setComposingText("", 1) + finishComposingText()` — **but** `finishComposingText` on Android commits the current composing region, so the Android binding MUST issue `setComposingText("", 1)` first to zero the region before `finishComposingText`. |
| `commitTextReplacingPreedit(text)` | `unmarkText()` is implicit; `insertText(text)` replaces the marked range atomically. | `commitText(text, 1)` — Android commits and clears composing region in one call. |
| `deleteBackwardFromDocument` | `deleteBackward()` | `deleteSurroundingText(1, 0)` (must be issued **after** any pending `commitText` / `finishComposingText`). |
| `resetAutocomplete` | engine-side suggestion clear | engine-side suggestion clear |
| `performAutocomplete` | engine-side query trigger | engine-side query trigger |
| `resetAutocompleteContext` | engine-side context clear | engine-side context clear |

**Critical Android caveat**: `finishComposingText()` is NOT a "clear without commit" — it commits the current composing region. The binding must therefore always zero the composing region via `setComposingText("", 1)` before issuing `finishComposingText()` to honor `clearPreeditWithoutCommit` semantics. Same caveat applies to `commitTextReplacingPreedit` — do not call `finishComposingText` before `commitText` or you will duplicate the text.

### 2.3 Ordering contracts, expressed as effect sequences

The existing ordering contracts live inside `ComposingState.apply(_:)` as **pure tests** rather than property-observer side effects. E.g. `apply(.deleteBackward)` when `raw == "a"` returns:

```
Transition(
  newPhase: .idle,
  newSelectedIndex: -1,
  effects: [
    .clearPreeditWithoutCommit,
    .resetAutocomplete,
    .deleteBackwardFromDocument,   // must come after the two above
  ],
  derivedDisplay: ""
)
```

`apply(.selectSuggestion(text))` returns:

```
Transition(
  newPhase: .idle,
  newSelectedIndex: -1,
  effects: [
    .commitTextReplacingPreedit(text),  // one atomic effect; no separate clear-then-insert
    .resetAutocomplete,
    .resetAutocompleteContext,
  ],
  derivedDisplay: ""
)
```

The former `clearMarkedText → insertText` two-step becomes a single `commitTextReplacingPreedit`, which is unambiguous on both platforms and closes the Android double-commit risk the reviewers flagged.

Unit tests assert `effects` ordering. No SwiftUI required.

### 2.4 Wrapper application phases (explicit)

The wrapper applies a `Transition` in three strictly ordered phases. This is not the generic "effects then publish" loop the first draft implied — reviewers correctly pointed out that today's code publishes state *during* the effect sequence. The explicit three-phase contract is:

```
func apply(_ intent: Intent) {
    let settings = settingsProvider.current
    let transition = state.apply(intent, mode: settings.inputMode, toneToggles: settings.toneToggles)

    // Phase 1 — mutate + publish state (guarded-inequality @Published writes)
    state = transition.newState
    if self.isComposing != state.isComposing { self.isComposing = state.isComposing }
    if self.rawInput != state.rawInput { self.rawInput = state.rawInput }
    if self.composingText != transition.derivedDisplay { self.composingText = transition.derivedDisplay }
    if self.selectedCandidateIndex != transition.newSelectedIndex {
        self.selectedCandidateIndex = transition.newSelectedIndex
    }

    // Phase 2 — execute effects in order (preedit + document + autocomplete)
    for effect in transition.effects { delegate.execute(effect) }

    // Phase 3 — notify composing-context sink
    contextSink?.isComposingText = state.isComposing
}
```

Rationale: SwiftUI batches Phase 1's `@Published` writes inside one synchronous method call, so the UI re-renders once with consistent state before the delegate mutates the text document. The no-op inequality guards keep idle→idle silent.

Per-intent exceptions are expressed in the engine's `Transition.effects` list, not in wrapper logic — the wrapper is intentionally mechanical.

---

## 3. Where settings enter

`derivedDisplay` needs `mode: InputMode` and `toneToggles: ToneToggles`. The split has three options; the design picks **(B)**.

- **(A) Inject at construction.** Simplest, but breaks live settings changes — the user toggles POJ↔TL or the OO/NN preprocess booleans in settings and the current composition renders wrong.
- **(B) Pass per call.** `state.apply(intent, mode: settings.current.inputMode, toneToggles: settings.current.toneToggles)`. `ComposingState` stays settings-agnostic. The wrapper is the only place that reads `EngineSettingsProvider`. *Chosen.* Preserves invariant §11 (engine settings are live-read).
- **(C) Keep `settingsProvider` inside the pure state.** Violates Foundation-only rule (`SharedSettings.shared` default). Rejected.

Implication: every `apply` call on the wrapper side threads `mode` + `toneToggles` through. Cheap — value types, no copy problem.

`ToneToggles` lives alongside `EngineSettings` so every platform's `EngineSettings` exposes the same two flags; the engine receives them on `AppConfig` per request and `phonetics::normalize_tone` reads them there — no platform-side phonetics code remains in the composing path.

---

## 10. Cross-references

- Live Rust / native ownership inventory: `../engine/migration-inventory.csv` (filter `area=composing`).
- Next-word counterpart (same executor / effect pattern): `nextword-engine-boundary.md`.
- Behavioral invariant pin: `behavioral-invariants.md` §13 (composing-buffer reset semantics); must not regress §§1–3, 9, 11.
- Wire shape of the composing requests / effects: `../engine/rust-core-proto.md`; engine internals: `../engine/composing.md`.
- Four-platform glue map: `system-overview.md` §4.

---

## 11. Android Binding Addendum

**Status**: A4-design deliverable for Phase II, authored 2026-04-20 on branch `phase2/a4a5-design-android-binding`. Pairs with `nextword-engine-boundary.md` §13. Codex pre + post reviewed.

**Purpose**: the Android-specific binding contract for the `Effect` enum defined in §2.2. §2–§3 stay platform-neutral in intent; this addendum captures the Kotlin / `InputConnection` / coroutine details.

**Scope**: binding contract only. A4-impl writes the code that honors the contract.

### 11.1 Current Android state (pre-A4-impl)

`android/app/.../ime/text/composing/ComposingManager.kt` is a single `class` with direct `InputConnection` side effects inline. No `Effect` enum, no delegate abstraction, no owned `CoroutineScope`. Background display derivation is driven externally by `CandidateUpdateCoordinator` and applied back via `ComposingManager.applyDerivedDisplay(derivedText, ic)`.

Observable Android ↔ iOS divergence today:

| Path | Today (Android) | Honors `clearPreeditWithoutCommit`? |
|---|---|---|
| `deleteBackward` empty-raw path | `reset(ic)` → `ic.setComposingText("", 1)` + `ic.finishComposingText()` | Yes — pre-zero owned by `reset(ic)`. |
| `reset(ic)` called directly (external, e.g. subtype switch, session end with pending preedit) | `ic.setComposingText("", 1)` + `ic.finishComposingText()` | Yes — corrected by parity PR (see §11.6). |
| `commitComposition(ic)` | sync fallback derive + `ic.setComposingText(composingText, 1)` + `ic.finishComposingText()` | Intended commit path — pre-zero not applicable. The fast/slow split against an externally cleared region (§11.10 divergence #3) no longer surfaces the stale-commit bug: `TextInputManager.onUpdateSelection` → `ComposingManager.onExternalComposingRegionCleared()` zeroes internal state before any later commit runs. |
| `selectSuggestion(text, ic)` | `ic.setComposingText(suggestion, 1)` + `ic.finishComposingText()` | Equivalent to `commitTextReplacingPreedit` — atomic replace. |
| `TextInputManager.resetComposingText()` bare-IC fallback (new-editor session start — a same-editor `restarting=true` keeps the manager and reconciles instead, DELETE / ENTER non-composing, NUMERIC-PHONE key) | delegates to top-level `clearHostComposingRegion(ic)` → `ic.setComposingText("", 1)` + `ic.finishComposingText()` | Yes — corrected by parity PR (see §11.6). |

`reset(ic)` is the canonical owner of the zero-then-finish sequence for composing-aware sites; external callers and the `deleteBackward` empty-raw path now route through it. Bare-`InputConnection` fallback sites in `TextInputManager` (where `composingManager` is null or the current keyboard mode bypasses composing) delegate to the sibling top-level helper `clearHostComposingRegion(ic)` so the zero-then-finish invariant holds at every IC-layer clear site. Pinned by `INVARIANT_composing_clear_preedit_does_not_commit` (see `behavioral-invariants.md` §13).

### 11.2 Effect → `InputConnection` binding rules

Android wrapper's `execute(Effect)` implements §2.2 table column 3 verbatim. Additional Android-only contract:

1. **Zero-then-finish is mandatory** for `clearPreeditWithoutCommit`. `InputConnection.finishComposingText()` commits the current composing region by default; binding MUST issue `ic.setComposingText("", 1)` before `ic.finishComposingText()` or the preedit is silently committed. Same caveat applies at every site that clears preedit — not just the Effect binding.
2. **Atomic commit — do not pre-finish.** For `commitTextReplacingPreedit(text)`, call `ic.commitText(text, 1)` directly. Do NOT call `ic.finishComposingText()` first — `commitText` atomically replaces the composing region and clears it; a prior `finishComposingText` would commit the old preedit and then `commitText` would insert the new text, producing a double-commit.
3. **Ordering: document ops after preedit ops.** `deleteBackwardFromDocument` issues `ic.deleteSurroundingText(1, 0)` AFTER any pending `commitText` / `finishComposingText` for the same intent. Mirrors iOS `UITextDocumentProxy.deleteBackward()` ordering.
4. **Engine-side effects** (`resetAutocomplete`, `performAutocomplete`, `resetAutocompleteContext`) touch only engine state — no `InputConnection` calls — matching iOS.

### 11.3 Threading

- Android IME framework delivers key events on the main thread. `InputConnection` is usable from the IME main thread only.
- The wrapper runs `execute(Effect)` synchronously from the IME main-thread entry point (key press, tap). No dispatcher hop required in the common case.
- Any coroutine boundary inside the wrapper (e.g. async display derivation) MUST `withContext(Dispatchers.Main.immediate) { … }` before touching `InputConnection`. Candidate scoring / trie lookup stays on `Dispatchers.Default`; `InputConnection` work is always main.

### 11.4 Observable state fan-out — no `StateFlow` required

iOS §2.4 uses `@Published` guarded-inequality writes to batch UI updates inside one synchronous call. Android equivalent:

- Wrapper does NOT expose a `StateFlow<ComposingState>`; the executor interprets `Effect` synchronously, same flow as iOS Phase 1 / Phase 2 split.
- Existing callbacks (`onUpdateCandidates`, `onClearCandidates` style — see `NextWordHandler`) stay function types. No migration to `StateFlow` in A4-impl.
- If Compose UI later needs observability, a `StateFlow` wrapper can be layered on top without changing the `Effect` contract. Out of scope for A4-impl.

Reason: adding `StateFlow` inside the executor would force every `Effect` list into an async recomposition cycle, breaking iOS §2.4's "single synchronous call" property.

### 11.5 Lifecycle scope ownership

If A4-impl's `ComposingManager` owns a `CoroutineScope` (e.g. for background display derivation currently co-located in `CandidateUpdateCoordinator`), that scope MUST:

- Be constructed with `SupervisorJob() + Dispatchers.Main.immediate`.
- Be owned by an IME-lifecycle object (`TaigiKeyboard.serviceScope` on the IME service, or a new scope cancelled explicitly in the wrapper's teardown path).
- Be cancelled when the owner's lifecycle ends (service `onDestroy`, or wrapper disposal) — a scope that outlives the IME leaks pending derivations.

Today's state for reference: `ComposingManager` owns no scope; `SmartbarManager` at line 50 owns a `CoroutineScope(SupervisorJob() + Dispatchers.Main)` that is NOT cancelled in its teardown path. A4-impl MUST NOT copy that pattern — new wrapper scopes must be cancellation-bound to an IME lifecycle.

### 11.6 Parity-correction flag — `reset(ic)` pre-zero

**Status**: corrected — see `INVARIANT_composing_clear_preedit_does_not_commit` in `behavioral-invariants.md` §13.

Previously `ComposingManager.reset(ic)` called `ic.finishComposingText()` without a prior `ic.setComposingText("", 1)`. External callers invoking `reset` with a non-empty preedit silently committed that preedit. iOS `ComposingState.apply(.reset)` did not — it emits `clearPreeditWithoutCommit`, bound to `clearMarkedText()` which never inserts text.

The correction landed as an **isolated PR before A4-impl** per `.claude/rules/cross-platform-alignment.md` §1b ("Not be bundled with unrelated refactor work — a parity correction is its own observable change and deserves an isolated review"). What shipped:

- Title prefix `parity:`.
- `ComposingManager.reset(ic)` now does `ic.setComposingText("", 1)` then `ic.finishComposingText()` (`ComposingManager.kt`).
- `ComposingManager.startComposing(...)` mid-composition restart applies the same pre-zero pair (covered by the same INVARIANT label, `behavioral-invariants.md` §13).
- `deleteBackward` empty-raw path's redundant pre-zero removed; routes through `reset(ic)`.
- Regression tests on both platforms: Android `ComposingManagerTest` (JVM, hand-rolled `RecordingInputConnection`); iOS `ComposingManagerTests.testReset_whenComposing_returnsToIdleWithoutInserting` plus the pure-state pin in `ComposingStateTests`.
- A4-impl lands subsequently as pure refactor (Effect-enum split) on top of the corrected behavior.

**Deferred parity follow-ups** (same INVARIANT label, separate PRs):

- ~~`TextInputManager.resetComposingText`~~ — **CLOSED 2026-04-22** (branch `parity/textinputmanager-reset-composing-text`). The four bare-`InputConnection` sites (`TextInputManager.kt:253` / `:477` / `:551` / `:746` — session start, DELETE non-composing fallback, ENTER non-composing fallback, NUMERIC-PHONE key event) now delegate through a new top-level helper `clearHostComposingRegion(ic: InputConnection?)` in `ComposingManager.kt` that issues `setComposingText("", 1)` before `finishComposingText()`. Helper colocates with the sibling `hostReportsNoComposingRegion` policy helper (PR #163 pattern — pure helper unit-tested without Robolectric). Binding pin: `ComposingManagerTest.clearHostComposingRegion zeros then finishes without committing` + `… with null ic is noop`. Reuses the existing `INVARIANT_composing_clear_preedit_does_not_commit` label — no new invariant.
- ~~`MediaInputManager.sendEmojiKeyPress`~~ — **CLOSED 2026-04-21** (branch `parity/emoji-key-press-preedit`). Both platforms now route emoji insertion through `ComposingManager.commitPreeditThenInsertExternal(_:)` / `commitPreeditThenInsertExternal(text, ic)`, which emits a single `CommitTextReplacingPreedit(derived + external)` effect (atomic `commitText` / `clearMarkedText + insertText`) plus autocomplete resets. Pure-state pin: `ComposingStateTest(s)` `INVARIANT_composing_external_insert_commits_preedit_atomically`. Binding pin: Android `ComposingManagerTest.commitPreeditThenInsertExternal when composing emits single atomic commitText`; iOS `ComposingManagerTests.testCommitPreeditThenInsertExternal_whenComposing_commitsAtomicallyWithExternalText`. Same closure also routes iOS emoji-palette backspace through `ActionHandler.handleBackspaceAction` so composing/idle branches match the regular keyboard.

### 11.7 Clock and settings at the boundary

`ComposingState` is clock-free (the state machine has no time-dependent transitions). Settings enter per §3: wrapper reads `EngineSettingsProvider.current.inputMode` + `.toneToggles` at each `apply(intent)` call. `ToneToggles` already exists at `ime/core/settings/ToneToggles.kt` (data class mirroring iOS). `ToneConverter.convertToToneMarks` currently takes two `Boolean` parameters; A4-impl wraps them at the `ComposingState.derivedDisplay(...)` boundary and updates `ToneConverter`'s signature to accept `ToneToggles` directly.

### 11.8 Shared-core candidate roster delta (Android-side)

A4-impl adds the following Android files to the roster (mirroring §6 iOS columns):

| iOS file (§6) | Android file (target) | Shared-Core Candidate marker? |
|---|---|---|
| `Input/Composing/ComposingState.swift` | `ime/text/composing/ComposingState.kt` *(new)* | **Held** — transitively imports `ToneConverter`; marker unlocks when `ToneConverter.kt` is purified (future round) |
| `Input/Composing/ComposingTransition.swift` | `ime/text/composing/ComposingTransition.kt` *(new)* | Yes — landed by A8-sweep |
| `Settings/ToneToggles.swift` | `ime/core/settings/ToneToggles.kt` *(already exists, add marker)* | Yes — landed by A8-sweep |
| `Phonetics/ToneConverter.swift` (parameterized) | `ime/dictionary/ToneConverter.kt` — NOT yet shared-core pure (imports `android.util.Log`, `BuildConfig`). A4-impl signature migration took `ToneToggles`; A8-sweep kept logging intact and applied a `// NOTE: Not shared-core` header. | Deferred to follow-up round (LoggerBackend migration) |
| `Input/Composing/ComposingManager.swift` (reduced wrapper) | `ime/text/composing/ComposingManager.kt` (reduced wrapper) | No — platform. |

A8-sweep applied the `// region Shared-Core Candidate` header per `.claude/rules/android-guidelines.md` §1 to `ComposingTransition.kt` + `ToneToggles.kt`. `ComposingState.kt` marker is held per Codex pre-review (2026-04-20): marking it would leak a transitive platform dependency through `ToneConverter.kt`.

### 11.9 Out of scope for A4-design

- Wrapper API shape (single `execute(Effect)` vs one method per effect) — A4-impl decides, same as iOS G4-impl §9.
- `ComposingState` as `data class` vs `class` — A4-impl picks `data class` by default for `equals()` snapshot testing.
- StateFlow migration — deferred per §11.4.
- `ToneConverter.convertToToneMarks` signature migration — A4-impl precondition, folded into the same PR.

### 11.10 Live-typing preedit text — Android raw vs iOS derived (A4-impl divergence)

**Status**: deferred divergence, locked 2026-04-20 during A4-impl on branch `phase2/a4-composing-state`. Documents a real shared-core contract divergence so it is not silent per `.claude/rules/cross-platform-alignment.md` §3.

For live-typing intents — `Start` / `Append` / `AppendHyphen` / `ReplaceLast` / non-empty-result `DeleteBackward` — the two platforms emit different `Effect.UpdatePreedit` text content:

- **iOS**: `ComposingState.apply(...)` synchronously calls `derivedDisplay(mode, toneToggles)` and emits `.updatePreedit(derivedText)`. The user sees the tone-marked form immediately on every keystroke.
- **Android**: `ComposingState.apply(...)` emits `UpdatePreedit(rawInput)` (the raw keystrokes) AND leaves `transition.derivedDisplay` empty for live-typing paths — no synchronous `ToneConverter.convertToToneMarks` call on the main-thread dispatch path. `ComposingManager.cachedDerivedDisplay` is cleared on every dispatch so `getComposingText()` returns the raw placeholder (matching pre-A4 observable behavior for `handleEnter` / `handleSpace` callers that capture `committedText` before commit). A background derivation loop (`CandidateUpdateCoordinator.scheduleDisplayDerivation`) subsequently calls `ComposingManager.applyDerivedDisplay(derived, ic)` which replaces the preedit on `Dispatchers.Default` → `Dispatchers.Main` AND populates `cachedDerivedDisplay` so subsequent `getComposingText()` calls read the derived form. Commit-path intents (`CommitDerived`) still compute `derivedDisplay` synchronously — the derived text travels inline in the `CommitTextReplacingPreedit(text)` effect and Enter/Space is not a hot path.

**Why the divergence exists** — Android's pre-A4 flow set `composingText = rawInput` as a placeholder, then async-derived. Preserving this flow in A4-impl honors `.claude/rules/cross-platform-alignment.md` §1 refactor-freeze. Collapsing the async path into synchronous derivation (matching iOS) would be a user-visible mid-keystroke behavior change — `.claude/rules/cross-platform-alignment.md` §1b mandates an isolated parity-correction PR for that.

**Why it is not a shared-core contract break** — `Effect.UpdatePreedit(text: String)` carries an arbitrary `String` value. The Effect contract is "show `text` as the preedit," not "show the derived form." Both platforms honor the contract; they pass different text.

`ComposingTransition.derivedDisplay` diverges too: iOS populates it synchronously on every transition (the field feeds `@Published composingText` in the wrapper); Android live-typing paths (`Start` / `Append` / `ReplaceLast` / non-empty-result `DeleteBackward`) intentionally leave it empty to avoid a duplicate main-thread `ToneConverter.convertToToneMarks` call — the wrapper ignores the field and the async loop in `CandidateUpdateCoordinator.scheduleDisplayDerivation` computes the derived form on `Dispatchers.Default`. Commit-path intents (`CommitDerived`) still populate it synchronously because the derived text is carried inline in the `CommitTextReplacingPreedit(text)` effect and Enter/Space is not a hot path.

**Test expectations** —

- Pure-state: `ComposingStateTest.live-typing UpdatePreedit carries raw keystrokes not derived form` pins Android's raw-text contract. iOS `ComposingStateTests.testStart_emitsUpdatePreeditThenPerformAutocomplete` pins iOS's derived-text contract. The two are intentionally NOT cross-platform symmetric — they each assert their own platform's emitted text.
- Binding-side: `ComposingManagerTest.appendCharacter shows raw keystrokes in preedit` asserts the last `setComposingText` call Android emits during keystroke sequence is the raw form.

**Wrapper-level intent routing divergences** (Android only) — three cases where Android's `ComposingManager` dispatches differently than iOS because Android's in-document composing region (vs iOS floating marked text) makes certain emitted effects platform-unsafe:

- `startComposing(char, ic)` on an active preedit: Android wrapper first dispatches `Reset` (emits `ClearPreeditWithoutCommit` → `setComposingText("", 1) + finishComposingText()`), then dispatches `Start(char)`. This pins the pre-zero contract from PR #151 / `behavioral-invariants.md` §13. iOS does not need the pre-Reset because `setMarkedText` atomically replaces the floating mark.
- `deleteBackward(ic)` on 1-char composing buffer: Android wrapper dispatches `Reset` instead of `DeleteBackward`, so the state never emits `DeleteBackwardFromDocument` for this path. On iOS, `DeleteBackwardFromDocument` is the "normal backspace" companion to `ClearPreeditWithoutCommit` (because clearing the floating mark does not touch the document); on Android, `ClearPreeditWithoutCommit` already removes the preedit char from the document, so the extra `deleteSurroundingText(1, 0)` would delete a pre-existing document character.
- `commitComposition(ic)` fast-path vs slow-path split: when `cachedDerivedDisplay` is non-empty (async derivation already replaced the raw preedit with the derived form), wrapper issues `finishComposingText()` ONLY — no `CommitTextReplacingPreedit` dispatch. This matches pre-A4 `displayDirty == false` semantics where `finishComposingText` is a no-op if the host editor has no composing region, avoiding a duplicate text insertion at the new cursor position. When `cachedDerivedDisplay` is empty (async hasn't caught up), wrapper dispatches `CommitDerived` which synchronously derives + atomically commits via `commitText` (matches pre-A4 `displayDirty == true` semantics). **Root cause closed 2026-04-21** by the `onUpdateSelection` reset hook: `TextInputManager.onUpdateSelection` detects `candidatesStart == -1 && candidatesEnd == -1` (host reports no composing region after a tap / selection change) and — since 2026-09-12, only after `ComposingManager.reconcileWithHost(ic)` confirms by re-reading the editor that the report is not older than the IME's own last write — calls `ComposingManager.onExternalComposingRegionCleared()`, which zeroes `state` + `cachedDerivedDisplay` without touching `InputConnection` (`behavioral-invariants.md` §13 extended invariant). A subsequent `commitComposition(ic)` therefore short-circuits via `if (!state.isComposing) return` — no stale-commit. The wrapper's fast/slow split itself stays as refactor-freeze-pure preservation of pre-A4 observable behavior; collapsing it belongs in a future parity: PR per §1b.

All three routes preserve Android's pre-A4 observable behavior. `ComposingState` itself stays iOS-effect-shape compatible — the divergence lives in wrapper intent selection, not in the pure state's effect-emission rules.

**Future parity-correction path** — if we later decide to collapse Android's async derivation into synchronous `state.apply` (matching iOS), the round needs to:

1. Ship as an isolated `parity:` PR per §1b (not bundled with unrelated refactor).
2. Remove `ComposingManager.applyDerivedDisplay` + `deriveDisplay(raw)` and delete `CandidateUpdateCoordinator.scheduleDisplayDerivation` / `displayDerivationJob`.
3. Flip `ComposingState.apply(...)` live-typing intents to emit `UpdatePreedit(derivedDisplay(mode, toggles))`.
4. Update both test suites (pure-state + binding) to assert the derived-text contract on Android too; drop this §11.10 divergence note.
5. Dogfooding S1/S2/S3 covers the before/after comparison (placeholder flash → direct-derived).

Tracked under `.claude/rules/cross-platform-alignment.md` §1b tier; not scheduled in Phase II.

### 11.11 Cross-references

- iOS boundary contract: §§1–10 above.
- A4-impl shipped via Phase II Round A4 (Android binding addendum); engine logic now in Rust `engine/composing` (since v3.5.4).
- Parity-correction policy: `.claude/rules/cross-platform-alignment.md` §1b.
- Android guidelines (DI, coroutines): `.claude/rules/android-guidelines.md` §§4, 5; IME-specific rules: `.claude/rules/android-ime-patterns.md` §2.
- `clearPreeditWithoutCommit` test label: see §8 of this doc and `behavioral-invariants.md` §13 (full Composing-buffer reset semantics + cross-platform test mapping).
