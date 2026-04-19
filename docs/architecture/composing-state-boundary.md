# G4-design — Composing State / Platform Boundary

**Status**: design-only deliverable for Phase I G4, authored 2026-04-19. Revised same day after Codex + Gemini review (see `codex-review-2026-04-19.md` for the earlier strategic pass; this revision captures the G4/G5/G8 docs-review cycle). Implementation (G4-impl) happens late in Phase I — this doc front-loads the boundary so G1 / G2 / G3 / G6 do not lock in assumptions that contradict it (per Codex finding I1).

**Goal**: split `Input/Composing/ComposingManager.swift` into

1. **`ComposingState`** — pure Foundation-only state machine, shared-core candidate.
2. **`ComposingManager`** — iOS platform wrapper: `ObservableObject` + `@Published` fan-out + `UITextDocumentProxy` side effects.

…without regressing any behavior captured in `behavioral-invariants.md` or changing the user-perceived composing UX.

**Non-goal**: do NOT implement in this doc. G4-impl takes the sketch here, writes code, and wires G9 golden-text regression tests against it.

**Precondition** (resolved before G4-impl begins): `ToneConverter.preprocessPojInput` must be parameterized (take `isDoubleTapOOEnabled` / `isDoubleTapNNEnabled` as arguments instead of reading `SharedSettings.shared`). The parameterization itself is a small one-file change listed in the Phase I plan's "Out of scope" section; G4-impl either folds it in or lands it as an independent prerequisite PR. Without this precondition, `ComposingState.derivedDisplay` cannot be Foundation-pure.

---

## 1. What exists today

`ComposingManager.swift` mixes four concerns:

| Concern | Evidence |
|---|---|
| **Pure state machine** | private `enum ComposingState { idle, composing(raw:) }`, transition rules, ordering contract (`startComposing` / `appendCharacter` / `deleteBackward` / `commit*` / `selectSuggestion` / `reset`). |
| **Derivation** | `deriveDisplay(raw:)` → `TPSTables.containsTPS` / `ToneConverter.convertToToneMarks`. **Important**: `ToneConverter` today reads two `SharedSettings` booleans (`isDoubleTapOOEnabled`, `isDoubleTapNNEnabled`). Non-Foundation-pure until parameterized — see Precondition above. |
| **Combine publication** | `ObservableObject`, four `@Published` properties, `syncStateToProperties()` idle/no-op guards. |
| **Platform side effects** | `ComposingDelegate` calls (`insertText`, `deleteBackward`, `setMarkedText`, `clearMarkedText`, `resetAutocomplete`, `performAutocomplete`, `resetAutocompleteContext`) + `ComposingContextSink.isComposingText`. |

Only the first two are Foundation-pure (once Precondition lands). The other two are what blocks `ComposingManager` from joining the 36-file shared-core roster (see `shared-core-readiness.md` Exclusions).

**Ordering contracts that must survive the split** (non-obvious):

- `deleteBackward` empties raw → **idle transition + `clearSelectionAndSuggestions` run before `delegate?.deleteBackward()`**. Reversed, `markedText` leaks past the text-document deletion.
- `selectSuggestion` → `clearMarkedText → insertText → state=.idle → clearSelection → resetAutocomplete → resetAutocompleteContext`. Routing through `updateComposingState(.idle)` is deliberately skipped to avoid re-emitting `clearMarkedText` after `insertText`.
- `commit*` → `updateComposingState(.idle) → clearSelectionAndSuggestions → delegate.insertText → delegate.resetAutocompleteContext`. Must capture `text` before the idle transition (which zeroes `composingText` / `rawInput`).
- `syncStateToProperties` guards every `@Published` write with an inequality check — idle→idle transitions must NOT fan out `objectWillChange`. This prevents SwiftUI over-redraw during `reset`.
- **Idle implies `selectedCandidateIndex == -1`**. The pure state must enforce this so Android's candidate-bar recycler does not keep a stale highlight after `reset()` / `selectSuggestion(...)`. Today this is incidental (platform sets it); the split makes it an explicit invariant.

---

## 2. Target shape

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

`ToneToggles` lives alongside `EngineSettings` so Android's `EngineSettings` implementation exposes the same two flags. `ToneConverter.preprocessPojInput(_, toggles: ToneToggles)` replaces the current `SharedSettings.shared` read after the Precondition lands.

---

## 4. ComposingDelegate fate

`ComposingDelegate` today is a protocol over iOS-shaped operations (`setMarkedText`, `insertText`, …). After G4-impl, the wrapper no longer calls those methods directly — it calls `delegate.execute(_: Effect)`. `ComposingDelegate` becomes a thin iOS interpreter of the neutral `Effect` enum.

**Decision**: `ComposingDelegate` stays **platform-side**, stays excluded from the shared-core roster. Its only method becomes `execute(_: ComposingTransition.Effect)` (or a small set of methods, one per effect case — pick in G4-impl). Android's Phase II mirror defines its own platform-side delegate that interprets the same `Effect` enum against `InputConnection` per the table in §2.2.

`ComposingTransition.Effect` is the **shared behavioral contract**. `ComposingDelegate` is the **iOS translator**. Android has its own translator. The contract sits above both — neutral.

---

## 5. ComposingContextSink fate

`ComposingContextSink { var isComposingText: Bool { get set } }` — today, `KeyboardContext` conforms. This is a KeyboardKit-shaped hook.

**Decision**: stays platform-side. The wrapper owns the reference and assigns `isComposingText` on every transition (Phase 3). `ComposingState` does not know about it.

---

## 6. What becomes a shared-core candidate

After G4-impl:

| File | Role | Candidate? |
|---|---|---|
| `Input/Composing/ComposingState.swift` *(new)* | Pure state machine + transitions | **Yes** — adds to roster. |
| `Input/Composing/ComposingTransition.swift` *(new)* | `Transition` + `Effect` types | **Yes**. |
| `Input/Composing/ToneToggles.swift` *(new, may colocate with `EngineSettings.swift`)* | Two-bool value type | **Yes**. |
| `Phonetics/ToneConverter.swift` *(updated — Precondition)* | Parameterized POJ preprocess | **Yes** (moves from Exclusions into roster). |
| `Input/Composing/ComposingManager.swift` *(reduced)* | iOS wrapper, `ObservableObject`, delegate routing | No — platform-specific. Stays in Exclusions. |
| `Input/Composing/ComposingDelegate.swift` *(reduced to `execute`)* | iOS Effect interpreter | No — iOS-shaped. |
| `Autocomplete/Services/AutocompleteProviders.swift` | `ComposingStateProvider` protocol | Unchanged; wrapper continues to conform. |

Net: roster **+4 files** (ComposingState, ComposingTransition, ToneToggles, promoted ToneConverter). Android Phase II mirrors must implement the same four.

---

## 7. Risks + mitigations

| Risk | Mitigation |
|---|---|
| Android binding implements `clearPreeditWithoutCommit` via raw `finishComposingText()` and accidentally commits the preedit. | Binding contract in §2.2 mandates `setComposingText("", 1)` first, then `finishComposingText()`. G9 adds `INVARIANT_composing_clear_preedit_does_not_commit` as a platform-side integration test on both iOS and Android. |
| `commitTextReplacingPreedit` on Android double-commits if the binding issues `finishComposingText()` before `commitText()`. | Binding contract forbids pre-finish before commit; `commitText()` atomically replaces the composing region on Android. G9 test covers this sequence explicitly. |
| `@Published` fan-out order changes and UI flashes between transitions. | Phase 1 of §2.4 applies all `@Published` writes atomically inside one synchronous call — SwiftUI batches. Keep the no-op inequality guards. G9 adds a golden-text regression test for `commitRawInput`-at-index-0 (Enter-on-English) path. |
| `selectSuggestion`'s intentional skip of `updateComposingState(.idle)` gets re-introduced as a bug during refactor. | Expressed as the explicit single-effect `commitTextReplacingPreedit` in `ComposingState.apply(.selectSuggestion)`. Unit test asserts the Effect list contains `commitTextReplacingPreedit` and NOT the two-step pair. |
| `derivedDisplay` reads `TPSTables` / `ToneConverter` — both must be Foundation-pure after G4. | Precondition parameterizes `ToneConverter.preprocessPojInput`. Shared-core verification greps (`shared-core-readiness.md` §Verification) must pass on both files at G4-impl merge time. |
| Live settings change mid-composition renders stale display. | `apply` is called per-intent; the wrapper passes `settingsProvider.current.inputMode` and `settingsProvider.current.toneToggles` at call time. A settings change between keystrokes takes effect on the next keystroke — matches invariant §11 (live read, no snapshot). |
| Platform keeps an obsolete `selectedCandidateIndex` highlight after idle transition. | Pure state enforces `selectedCandidateIndex = -1` in idle. G9 test `INVARIANT_composing_idle_has_no_selected_candidate` covers `reset`, `deleteBackward`-to-idle, `commit*`, `selectSuggestion`. |

---

## 8. Test hooks for G9

G9 must cover at minimum:

- `INVARIANT_composing_delete_order` — `deleteBackward` at raw length 1 emits exactly `[clearPreeditWithoutCommit, resetAutocomplete, deleteBackwardFromDocument]`.
- `INVARIANT_composing_select_suggestion_is_atomic_commit` — `selectSuggestion(text)` emits `[commitTextReplacingPreedit(text), resetAutocomplete, resetAutocompleteContext]` and transitions to `.idle` with `selectedCandidateIndex == -1`.
- `INVARIANT_composing_commit_captures_text_before_idle` — `commitComposition` inserts derived text, then transitions to `.idle`; never inserts empty string after idle transition.
- `INVARIANT_composing_replace_last_preserves_selected_index` — `replaceLastCharacter` does NOT reset `selectedCandidateIndex` (TPS auto-correct contract).
- `INVARIANT_composing_idle_to_idle_is_noop` — repeated `reset()` on idle state produces a `Transition` with empty `effects` and no state changes.
- `INVARIANT_composing_idle_has_no_selected_candidate` — every idle-producing intent returns `newSelectedIndex == -1`.
- `INVARIANT_composing_clear_preedit_does_not_commit` — **platform integration test on both iOS and Android**: the `clearPreeditWithoutCommit` effect, as executed by the platform binding, leaves the backing text unchanged.

All pure-state tests runnable without a simulator; the last one requires iOS + Android instrumentation harness.

---

## 9. Decisions not taken in G4-design

Deferred to G4-impl (can change there without breaking this boundary):

- Whether `ComposingState` is a `struct` (copy-on-apply, pure functional) or a `class` owning mutable state. Lean **struct** — enables `Equatable` snapshot diffing in tests. G4-impl confirms.
- Naming: `ComposingState.apply(intent:mode:toneToggles:)` vs `reduce(intent:mode:toneToggles:)`. Pick in code review.
- Whether `ComposingDelegate.execute(_: Effect)` is one method or one method per effect case. Lean **single method + switch** for a tight contract surface; G4-impl confirms after seeing binding code.

Already decided (moved out of "deferred" after review cycle):

- `ToneConverter.preprocessPojInput` parameterization — **Precondition**, not deferred.
- `selectedCandidateIndex` lives inside `ComposingState`, invariantly `-1` in idle — confirmed.
- Effect names — platform-neutral (`updatePreedit` etc.) not iOS-named (`setMarkedText` etc.) — confirmed.

---

## 10. Cross-references

- Phase I plan: `ios-exemplar-plan.md` §G4.
- G5-design counterpart (same pattern for Timer-driven decay): `nextword-engine-boundary.md`.
- Shared-core readiness contract: `../engine/shared-core-readiness.md`.
- Behavioral invariants this doc must not regress: `behavioral-invariants.md` §§1–3, 9, 11.
- Codex review (roadmap-level, 2026-04-19): `codex-review-2026-04-19.md`.
- Docs-review cycle (2026-04-19, same day): findings incorporated above.
