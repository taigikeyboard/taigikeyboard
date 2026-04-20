# G5-design — NextWord Engine / Platform Boundary

**Status**: design-only deliverable for Phase I G5, authored 2026-04-19 alongside G4-design. Revised same day after Codex + Gemini review of the G4/G5/G8 docs cycle. Implementation (G5-impl) runs after G4-impl — this doc commits the scheduling contract early so later groups do not encode Timer-specific or DTO-leak assumptions.

**Goal**: split `NextWord/NextWordController.swift` into

1. **`NextWordEngine`** — pure Foundation-only logic (validate input, decide whether to record / reset / predict, normalize romanization, filter compound-word emissions), shared-core candidate.
2. **`NextWordController`** — iOS platform executor: owns the `Timer` for context timeout, `@MainActor` hops for UI updates, `DispatchQueue.main` dispatch, settings snapshot, and query-generation bookkeeping.

…without changing decay math, association-window semantics, or the user-perceived prediction lifecycle.

**Non-goal**: do not implement here. G5-impl takes this sketch, writes code, and G9 wires scheduling-parity tests against it.

---

## 1. What exists today

`NextWordController.swift` mixes:

| Concern | Evidence |
|---|---|
| **Pure decision logic** | `isNoiseText`, `isSentenceEndPunctuation`, `shouldRecordAssociation`, `splitCompoundWord`, `recordCompoundWordAssociations` plumbing, `makePredictions` filter/convert. |
| **Romanization normalization** | `RomanizationConverter.pojToTL` usage, already a candidate. |
| **Platform time source** | `Self.currentTimestampMs = Int64(Date().timeIntervalSince1970 * 1000)` — read at **two** places per `process` call (once inside `shouldRecordAssociation`, once when assigning `lastSelectionTime`). G5-impl normalizes these into one intent timestamp. |
| **Platform scheduling** | `Timer.scheduledTimer` for 30 s context timeout, `Task { @MainActor … }` for prediction query, `DispatchQueue.main.async` on timeout fan-out. |
| **Platform settings read** | `settingsProvider.current` inside `process` and `makePredictions`. |
| **Service I/O** | `nextWordService.recordAssociation`, `nextWordService.predict` (SQLite + binary mmap). Raw prediction rows are currently typed as `NextWordService.Prediction` — platform-bound DTO that leaks into any pure code consuming it. G5-impl introduces a shared-core DTO (see §2.4). |
| **UI fan-out** | `contextUpdater?.setNextWordPredictions / resetNextWordSuggestions`. |

Only the first two columns are Foundation-pure. The rest block `NextWordController` from the roster (see Exclusions: *Timer, DispatchQueue.main, @MainActor, SharedSettings.shared*).

**Timing contracts that must survive the split** (non-obvious):

- **10 s association window** (`associationTimeoutMs`): currently read with two separate `currentTimestampMs` calls per `process`. G5-impl intentionally normalizes to a single `nowMs` captured at intent entry. This is a **behavior clarification**, not strict preservation — the 1–2 ms gap between the two reads was never observable, but the tests in §10 pin the behavior at boundaries.
- **30 s context timeout**: fires on the Timer, resets state, optionally clears UI suggestions. The Timer is rescheduled at every successful `process` / `triggerPrediction`. Keep the rescheduling order: stop → start. A missed stop leaks timers.
- **`@MainActor` hop for prediction** — the actual prediction query happens off-main (`NextWordService` is an async API); the UI update happens on main. G5-impl must preserve this threading — pure engine must not force main-thread usage.
- **Compound-word associations fire inside a single `Task`** — today `recordCompoundWordAssociations` loops `await nextWordService.recordAssociation` sequentially. Keep sequential ordering; parallel `Task`s would race on the SQLite UNIQUE constraint.
- **Stale-prediction race** — a prediction Task in flight may resolve *after* a context timeout or `resetFull` clears state. Today the code accepts this race (late predictions leak onto the UI). G5-impl **eliminates** the race via query generations, not deferred (§3).

---

## 2. Target shape

```
┌──────────────────────────────────────────────────────────────────────┐
│  Platform executor (iOS): NextWordController                         │
│  - Owns the Timer (context timeout) + invalidate / reschedule        │
│  - Owns @MainActor Task dispatch for UI updates                      │
│  - Owns settingsProvider.current reads                               │
│  - Owns currentGeneration counter (monotonic)                         │
│  - Per intent: snapshot settings, clock, generation → DecisionInput  │
│  - Calls NextWordEngine.decide(intent:state:input:)                  │
│  - Interprets engine-returned Effects                                │
│  - After prediction Task resolves: compare resolved generation       │
│    against current; drop if stale.                                   │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│  NextWordEngine (Foundation-only, shared-core candidate)             │
│  - decide(intent:state:input:) → NextWordOutcome                     │
│  - filterPredictions(_:[RawNextWordPrediction], settings:) → [EP]    │
│  - Pure validation + pure association-window check                   │
│  - Pure compound-word split                                          │
│  - Takes currentTimeMs + generation from caller; never reads clock,  │
│    never mutates the counter.                                        │
│  - Takes EngineSettings snapshot by value (no provider protocol).    │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.1 Intent values

```swift
public enum NextWordIntent: Equatable {
    case wordSelected(text: String, roman: String, requireRomanMode: Bool, triggerPrediction: Bool)
    case backspace(lastChar: String)
    case contextTimeoutFired
    case clearForNewComposing   // user started typing again
    case resetFull              // sentence-end / textDidChange empty
}
```

### 2.2 State + decision input (split, per review)

`NextWordPersistedState` is the long-lived state the engine mutates; `NextWordDecisionInput` is the per-call snapshot the executor supplies. The previous draft conflated these into one `NextWordContext`, which invited the executor to persist stale `nowMs` / `settings`.

```swift
public struct NextWordPersistedState: Equatable {
    public var lastSelectedWord: String?
    public var lastSelectedRoman: String?        // normalized to TL already
    public var lastSelectionTimeMs: Int64
    public var isShowing: Bool
    public var currentGeneration: UInt64         // monotonic; bumped on every state-mutating intent
}

public struct NextWordDecisionInput: Equatable {
    public let nowMs: Int64                      // supplied by executor at intent entry
    public let settings: EngineSettings          // value snapshot
}
```

### 2.3 Outcome values

```swift
public struct NextWordOutcome: Equatable {
    public enum Effect: Equatable {
        case rescheduleContextTimeout(after: TimeInterval)
        case cancelContextTimeout
        case recordAssociation(prev: String, prevTl: String, next: String, nextTl: String)
        case recordCompoundAssociations([(prev: String, prevTl: String, next: String, nextTl: String)])
        case queryPredictions(word: String, roman: String, generation: UInt64)
        case clearPredictionsUI(generation: UInt64)
    }

    public let newState: NextWordPersistedState   // lastSelected* / isShowing / currentGeneration updated
    public let effects: [Effect]                  // order matters; executor runs sequentially
}
```

**`generation` on `queryPredictions` and `clearPredictionsUI`** is the mechanism that eliminates the stale-prediction race. When the executor receives the async query result back, it compares the result's generation against `persistedState.currentGeneration`. A mismatch = the intent that started this query has been superseded; drop the result.

### 2.4 Prediction-result step (separate entry point) — with shared DTO

Prediction query happens asynchronously on the platform side. The service returns rows in a platform-bound type (`NextWordService.Prediction`); the executor must convert to a shared-core DTO at the boundary, then pass that to the engine. **Defining this DTO in shared-core is what eliminates the previous Codex finding that `filterPredictions` depended on a platform-service type.**

```swift
// Foundation-only, shared-core candidate — new file.
public struct RawNextWordPrediction: Equatable {
    public let hanzi: String
    public let tl: String
    public let score: Double     // merged dict + user signal, opaque to engine
}
```

**Impl revision (G5-impl, 2026-04-19)**: the original sketch above proposed
`count: Int` + `source: Source (.user | .dict)` so shared-core ports could
re-run `NextWordScorer` themselves. Implementation kept `score: Double`
instead because today's `NextWordService` already *merges* dict + user
rows with summed scores (dict row + user row on the same `(hanzi, tl)`
key become a single row with `score = dictScore + userScore`). A merged
row has no single `source`, so splitting the DTO back into `count` /
`source` would be lossy. Phase I treats `score` as an opaque ordering
weight; elevating scoring into shared-core (so Kotlin/Rust can compute
scores themselves from raw `count` + `lastUsedMs`) is a Phase IV-B
follow-up on the shared-core roadmap.

Service mapping: in the original G5-design sketch, `NextWordService.Prediction` → `RawNextWordPrediction` happened in `NextWordService` itself (platform side) before results cross into the engine. Both platforms simplified this at impl time — iOS G5-impl (PR #141) and Android A5-impl deleted the intermediate `NextWordService.Prediction` DTO and have `predict` return `List<RawNextWordPrediction>` (`[RawNextWordPrediction]` on iOS) directly.

Engine signature:

```swift
public static func filterPredictions(
    _ raw: [RawNextWordPrediction],
    settings: EngineSettings
) -> [EnginePrediction]
```

This is exactly today's `makePredictions(from:)` extracted as a pure static, but typed on the shared DTO. Platform executor calls it after `await nextWordService.predict(...)` and the generation check, then interprets either `setPredictions` or `clearPredictionsUI(generation:)` based on the returned count.

### 2.5 Setting predictions on the UI (clarifying §2 diagram)

The previous diagram listed a `setPredictions([EnginePrediction])` output but the `Effect` enum omitted it. Reason: prediction results are computed on the platform side (after the async query resolves), so they are not part of an `Outcome.Effect` list returned from `decide`. Instead, the executor has a **second code path** for prediction resolution:

```
executor.handleQueryResult(raw: [RawNextWordPrediction], generation: UInt64) {
    guard generation == state.currentGeneration else { return }   // stale, drop
    let predictions = NextWordEngine.filterPredictions(raw, settings: currentSettings)
    if predictions.isEmpty {
        contextUpdater?.resetNextWordSuggestions()
        state.isShowing = false
    } else {
        contextUpdater?.setNextWordPredictions(predictions)
        state.isShowing = true
        // reschedule context timer — same as today
    }
}
```

`setNextWordPredictions` is **not** an engine-emitted Effect because the engine never sees the prediction list. Diagram updated accordingly.

---

## 3. Scheduling contract + race elimination

`NextWordEngine` never touches time or Timers. The executor owns scheduling. The engine describes *when* via effect values:

- `rescheduleContextTimeout(after: 30)` — executor cancels existing Timer, schedules a new one; on fire, executor re-enters the engine with `NextWordIntent.contextTimeoutFired`.
- `cancelContextTimeout` — executor invalidates, no new Timer.

**Clock source ownership**: the executor is the single place that reads `Date().timeIntervalSince1970 * 1000`. It supplies `nowMs` on every `NextWordEngine.decide` call via `NextWordDecisionInput`. Tests call `decide` with a pinned `nowMs` and assert output — zero flakiness.

**Timer identity**: executor keeps a single `Timer` reference. Rescheduling = invalidate old + store new. No queue of pending Timers — matches current behavior. G9 mandates an explicit test: issue 100 rapid `wordSelected` intents, assert executor ends with exactly one live Timer reference.

**Query generation (race elimination)**: every `wordSelected` / `backspace` intent bumps `currentGeneration`. The `queryPredictions(..., generation:)` Effect carries the bumped value. When the async query resolves, the executor checks current generation before applying results. A timeout firing between query dispatch and query resolution bumps generation (via `contextTimeoutFired` emitting a state change) — the late result is then dropped. This is no longer deferred "if observable" work; it is part of the boundary contract.

Rule: **any state transition that invalidates a pending prediction query must bump `currentGeneration`**. The invariant is local to `decide`:

```
let shouldBumpGeneration = (
    intent is .wordSelected || .backspace || .clearForNewComposing ||
    .resetFull || .contextTimeoutFired
)
var newState = state
if shouldBumpGeneration { newState.currentGeneration &+= 1 }   // wrapping ok; 2^64 is plenty
```

---

## 4. Decision table (engine-side, pure)

For `wordSelected(text, roman, requireRomanMode, triggerPrediction)`:

| Condition | Outcome effects | newState mutation |
|---|---|---|
| `requireRomanMode && settings.isTranslateSwapped` | `[]` | unchanged |
| `text` empty | `[]` | unchanged |
| `text` is noise punctuation, NOT sentence-end | `[]` | unchanged |
| `text` is sentence-end punctuation | `[cancelContextTimeout] + [clearPredictionsUI(gen) if isShowing]` | reset to defaults + bump generation |
| `settings.isAssociationRecordingEnabled && shouldRecordAssociation(state, nowMs) && state.lastSelectedWord != nil` | `[recordAssociation(...), recordCompoundAssociations(...)]` (append) | — |
| Always (for valid text) | append `[rescheduleContextTimeout(30)]` | `lastSelectedWord/Roman = ...`, `lastSelectionTimeMs = nowMs`, bump generation |
| `triggerPrediction == true` | append `[queryPredictions(textTl, romanTl, newGen)]` | — |

For `backspace(lastChar)`:

- `newState.lastSelectedWord = lastChar`, `lastSelectedRoman = nil`, `lastSelectionTimeMs = nowMs`, bump generation.
- `effects = [queryPredictions(lastChar, "", newGen)]`.
- Critically: does **NOT** emit `recordAssociation` or `recordCompoundAssociations`. This is today's `rePredictAfterBackspace` invariant — backspace is not a word selection.

For `contextTimeoutFired`:

- If `state.isShowing` true at time of fire → `[clearPredictionsUI(newGen)]`, else `[]`.
- `newState` resets to defaults; bump generation (so any prediction query still in flight is invalidated).

For `clearForNewComposing`:

- `[clearPredictionsUI(newGen)]` if `isShowing`.
- `newState.isShowing = false`; association state left intact; bump generation (today's `clearDisplay` semantics preserved; the added generation bump drops any in-flight prediction).

For `resetFull`:

- `[cancelContextTimeout] + [clearPredictionsUI(newGen) if isShowing]`.
- `newState` zeroed; bump generation.

---

## 5. `shouldRecordAssociation` as a pure function

```swift
static func shouldRecordAssociation(_ state: NextWordPersistedState, nowMs: Int64) -> Bool {
    guard state.lastSelectedWord != nil else { return false }
    return (nowMs - state.lastSelectionTimeMs) < 10_000
}
```

Strict `<` boundary preserved. 10 s constant lives in `NextWordEngine` alongside 30 s context timeout.

**Boundary tests mandated in G9**:
- `nowMs - lastSelectionTimeMs == 9_999` → `true`
- `nowMs - lastSelectionTimeMs == 10_000` → `false`
- `nowMs - lastSelectionTimeMs < 0` (clock skew / wrapped) → `false` (do not record against a future-relative negative age).

The third case is a new assertion — today's implementation returns `true` for a negative delta, which is almost certainly wrong. G5-impl closes this with an explicit `max(0, nowMs - lastSelectionTimeMs)` or a `nowMs >= lastSelectionTimeMs` precondition, documented in the test.

---

## 6. Compound-word splitting preserved

```swift
static func splitCompound(_ word: String) -> [String] {
    word.isEmpty ? [] : word.split(separator: "-").map(String.init).filter { !$0.isEmpty }
}

static func compoundAssociationPairs(displayText: String, roman: String)
    -> [(prev: String, prevTl: String, next: String, nextTl: String)]
```

Executor's `recordCompoundAssociations` effect feeds straight into a single `Task` that loops `await nextWordService.recordAssociation` in order — same sequential shape as today, preventing UNIQUE-constraint races.

---

## 7. Settings access — snapshot-per-intent (with live live-read at executor)

`NextWordController` today reads `settingsProvider.current` twice in `process` (for `isTranslateSwapped` and `isAssociationRecordingEnabled`) and once in `makePredictions`. Between those reads, a settings change could technically flip the answer — though in practice settings updates during a single `process` call are not observed.

**Decision**: executor reads `settingsProvider.current` once at the start of `process` and snapshots it into `NextWordDecisionInput.settings`, then passes the value to `decide`. Prediction-filter step gets its own snapshot at query-resolve time (Task boundary). This matches *per-keystroke live* semantics without forcing the engine to query a provider.

Invariant §11 (engine settings are live-read) still holds at the executor level; engine functions receive a snapshot, which is the correct shape for deterministic testing.

---

## 8. What becomes a shared-core candidate

| File | Role | Candidate? |
|---|---|---|
| `NextWord/NextWordEngine.swift` *(new)* | Pure decide + pure filter | **Yes** — adds to roster. |
| `NextWord/NextWordOutcome.swift` *(new)* | `Intent` / `PersistedState` / `DecisionInput` / `Outcome` / `Effect` types | **Yes**. |
| `NextWord/RawNextWordPrediction.swift` *(new)* | Shared-core prediction DTO (replaces platform-bound `NextWordService.Prediction` at the boundary) | **Yes**. |
| `NextWord/NextWordController.swift` *(reduced)* | iOS Timer + @MainActor + settings read + service I/O + UI fan-out + generation bookkeeping | No — platform executor. Stays in Exclusions. |
| `NextWord/Services/NextWordService.swift` *(updated)* | Adds mapping `Prediction → RawNextWordPrediction` at service boundary | No — SQLite + file manager + shared singleton. Stays in Exclusions. |
| `NextWord/NextWordScorer.swift` | Pure ranking constants | Already a candidate. |

Net: roster **+3 files** (NextWordEngine, NextWordOutcome, RawNextWordPrediction).

---

## 9. Risks + mitigations

| Risk | Mitigation |
|---|---|
| Decay math changes because engine recomputes `nowMs` differently from platform `Self.currentTimestampMs`. | Engine never reads clock — executor supplies `nowMs` once per intent. Invariant §7 tests assert decay = f(`lastUsedMs`, `nowMs`). |
| Timer leak if executor's reschedule path doesn't invalidate on every `rescheduleContextTimeout` effect. | G9 adds **mandated** unit test: sequence of 100 `wordSelected` intents → executor state shows exactly one live Timer reference at the end. No longer optional. |
| `recordCompoundAssociations` emitted as pairs could shuffle if executor reconstructs from `splitCompoundWord` twice. | `Outcome.Effect.recordCompoundAssociations` carries the full pair list from the engine; executor loops them verbatim. No re-split on executor side. |
| ~~Context timeout fires during a prediction `Task` → races `setPredictions` with `clearPredictionsUI`.~~ | **Eliminated** by `currentGeneration` in §3. Any invalidating intent bumps the generation; late query results compare against current generation and drop on mismatch. G9 test `INVARIANT_nextword_late_prediction_is_discarded` covers this. |
| `NextWordService.Prediction` DTO leaks into engine signature. | **Eliminated** by `RawNextWordPrediction` in shared-core (§2.4). Service maps at the boundary. |
| Settings change between snapshot and prediction-filter re-read produces inconsistent `isTranslateSwapped` usage. | Executor takes a fresh settings snapshot at query-resolve time; engine never crosses that boundary twice. Documented, no new failure mode. |
| Negative `nowMs - lastSelectionTimeMs` (clock skew) returns `true` today. | New invariant: strict `< 10_000` and `>= 0`. G9 boundary tests cover it. |

---

## 10. Test hooks for G9

- `INVARIANT_nextword_association_window_strict_lt_10s` — boundary tests: 9_999 → true, 10_000 → false, negative delta → false.
- `INVARIANT_nextword_backspace_does_not_record` — `decide(.backspace(...))` never includes `recordAssociation` or `recordCompoundAssociations` effects.
- `INVARIANT_nextword_sentence_end_resets_context` — `decide(.wordSelected(text: "。", …))` yields `cancelContextTimeout` + clears state + bumps generation.
- `INVARIANT_nextword_compound_pairs_are_sequential` — for `text = "a-b-c"`, `compoundAssociationPairs` returns `[(a, b), (b, c)]` in that order.
- `INVARIANT_nextword_no_clock_read_in_engine` — static analysis / code review gate: `NextWordEngine` file must not reference `Date()`, `CFAbsoluteTimeGetCurrent`, `ProcessInfo.systemUptime`, `DispatchTime.now`.
- `INVARIANT_nextword_prediction_filter_hides_empty_tl_in_roman_mode` — `filterPredictions` drops entries with empty `tl` when `settings.isTranslateSwapped == false`.
- `INVARIANT_nextword_late_prediction_is_discarded` — platform-side integration test: dispatch `queryPredictions(gen=N)`, then fire `contextTimeoutFired` (bumps to N+1), then resolve the query → `setNextWordPredictions` is NOT called.
- `INVARIANT_nextword_generation_bumps_on_invalidating_intents` — every invalidating intent produces `newState.currentGeneration > state.currentGeneration`.
- `INVARIANT_nextword_rescheduling_leaks_no_timer` — platform-side: 100 rapid rescheduling intents → exactly one live Timer.

Pure-state tests runnable without simulator; the last two require iOS + Android platform harnesses.

---

## 11. Decisions deferred to G5-impl

- Whether `NextWordEngine` is an `enum` (static-only, pure-function namespace like `NextWordScorer`) or a `struct` holding `state`. Lean **enum** — matches scorer, no state to own.
- Whether the executor wraps engine calls in a `@MainActor` func. Lean **no** — engine is thread-agnostic; executor's main-thread needs are explicit.
- Whether `NextWordOutcome.Effect` should be a sealed hierarchy (class-ish with associated values) or the flat enum above. Lean **flat enum** — matches `ComposingTransition.Effect` in G4-design.

Already decided (moved out of "deferred" after review cycle):

- `NextWordPersistedState` vs `NextWordDecisionInput` split — **not deferred**, design above.
- `RawNextWordPrediction` DTO — **not deferred**, design above.
- Query generation for race elimination — **not deferred**, design above.
- Negative-delta fix in `shouldRecordAssociation` — **not deferred**, boundary test above.

---

## 12. Cross-references

- Phase I plan: `ios-exemplar-plan.md` §G5.
- G4-design counterpart (same pattern for SwiftUI-scheduled state): `composing-state-boundary.md`.
- Behavioral invariants this doc must not regress: `behavioral-invariants.md` §§7, 8, 11.
- Shared-core readiness contract: `../engine/shared-core-readiness.md`.
- Codex review (roadmap-level): `codex-review-2026-04-19.md`.
- Docs-review cycle (2026-04-19, same day): findings incorporated above.

---

## 13. Android Binding Addendum

**Status**: A5-design deliverable for Phase II, authored 2026-04-20 on branch `phase2/a4a5-design-android-binding`. Pairs with `composing-state-boundary.md` §11. Codex pre + post reviewed.

**Purpose**: lock Android-specific binding contract for the `NextWordEngine` / `NextWordOutcome` shape defined in §§2–7 before A5-impl lands. iOS-authored §§1–12 stay platform-neutral in intent; this addendum captures clock injection, Kotlin coroutine scheduling, and the cross-Service clock path the iOS doc could not.

**Scope**: binding contract only. A5-impl writes the code that honors the contract.

### 13.1 Current Android state (pre-A5-impl)

`android/app/.../ime/text/smartbar/NextWordHandler.kt`:

- Constructor-injected `CoroutineScope` (supplied by `SmartbarManager` at line 50 as `CoroutineScope(SupervisorJob() + Dispatchers.Main)`). Note: `SmartbarManager.onDestroy()` does NOT cancel this scope today — A5-impl MUST either inherit from `TaigiKeyboard.serviceScope` (cancelled in IME `onDestroy`) or add explicit cancellation in the wrapper's teardown path.
- Reads `System.currentTimeMillis()` inline at `handleNextWordPrediction` (line 64), `updateLastSelectedWord` (line 187), and `handleBackspaceForNextWord` (line 222).
- Constants `ASSOCIATION_TIMEOUT_MS = 10_000L`, `CONTEXT_TIMEOUT_MS = 30_000L` at the companion object. Neither carries the `// CROSS-PLATFORM INVARIANT` marker today.
- **Context-timeout is passive**: the 30 s window is checked inside `shouldReset` on the NEXT intent entry. No scheduled task proactively fires a timeout.
- **No generation counter**: late `nextWord.predict(...)` results reach `updateCandidatesWithPredictions` even if the user has moved on (typed a new character, committed a different word, or 30 s has elapsed).
- Direct callbacks (`onUpdateCandidates: (List<TaigiWord>) -> Unit`, `onClearCandidates: () -> Unit`) — no `StateFlow`.

`NextWordService.predict(...)` ALSO reads `System.currentTimeMillis()` internally at line 270 for user-row decay scoring. The clock path touches both files — §13.3 covers the full path.

### 13.2 Binding decisions

| Decision | Choice |
|---|---|
| Section placement | Append at end (§13). Audit §7 A5-design's "§3" reference is stale. |
| Context-timeout model | **Option A — active**. Port iOS `Timer.scheduledTimer` to a coroutine-scheduled `delay` (see §13.5). |
| `RawNextWordPrediction` package | **New** `ime/core/nextword/` package — groups shared-core candidates away from platform I/O (`ime/dictionary/`). A5-impl deletes the pre-A5 nested `NextWordService.Prediction` and has `predict` return `List<ime.core.nextword.RawNextWordPrediction>` directly (simpler than the originally-sketched boundary mapping step). |
| Generation counter | Port iOS pattern as a parity correction in A5-impl (see §13.6). |
| `StateFlow` on executor | Not required — same reasoning as composing §11.4. |

### 13.3 Clock injection — full path

`NextWordEngine` takes `nowMs: Long` on every decision entry point. For the prediction-filter path, the clock also reaches the scoring function inside `NextWordService.predict(...)`:

- Executor reads `System.currentTimeMillis()` once per intent entry, stores it in `NextWordDecisionInput.nowMs`.
- Executor passes the same `nowMs` into `nextWord.predict(word, roman, settings, nowMs)` — `NextWordService.predict` stops reading the clock internally (line 270 deleted) and uses the supplied value for `calculateUserScore(count, lastUsedMs, nowMs)` / `calculateDecay(lastUsedMs, nowMs)`.
- Result: the engine and the prediction query use ONE consistent `nowMs` per intent — no 1–2 ms drift between "should record association?" check and user-row decay scoring.

Engine-side forbidden calls (per `rules/android-guidelines.md` §1 criterion 3): `System.currentTimeMillis()`, `SystemClock.*`, `Instant.now()`. Note: `kotlinx.coroutines.delay` (top-level suspend function) is also forbidden inside the engine — all scheduling lives in the platform executor.

### 13.4 Settings access

Executor reads `EngineSettingsProvider.current` once at intent entry, passes value to `NextWordEngine.decide(...)` via `NextWordDecisionInput.settings`. Prediction-filter step takes a fresh snapshot at query-resolve time (coroutine boundary after `nextWord.predict` completes), same as iOS §7. Live-read semantics from `EngineSettings.kt` are preserved — see `rules/android-guidelines.md` §6.

### 13.5 Active context-timeout — coroutine binding

Android's current passive model is a documented divergence from the iOS boundary contract (§3: "rescheduleContextTimeout(after: 30)" is an active effect). A5-impl ports to an active model using coroutine scheduling. Binding pattern (illustrative — class layout is A5-impl's choice):

- Executor holds a single `Job` reference. Reschedule = cancel old + `scope.launch { delay(30_000); onContextTimeoutFired() }`.
- Cancel = invalidate and clear the reference. No queue of pending jobs — matches iOS single-Timer identity.
- On fire, executor re-enters `NextWordEngine.decide(.contextTimeoutFired, …)`; engine emits `clearPredictionsUI` effect if `state.isShowing`, resets state, bumps generation.
- Scope choice: IME-lifecycle scope (`TaigiKeyboard.serviceScope` or equivalent cancelled in `onDestroy`). A scope that outlives the IME leaks pending timeouts across input sessions — the `SmartbarManager` own-scope pattern must not be extended here.

A5-impl MAY rename this "delay" / "scheduled coroutine" if a Kotlin-idiomatic name fits better; the boundary contract is "single stored Job, cancel-then-schedule".

### 13.6 Generation counter — parity correction

Today Android lacks the iOS §3 `currentGeneration` mechanism. Late predictions can update UI even after state invalidation. A5-impl lands this as a **parity correction** per `rules/cross-platform-alignment.md` §1b:

- `NextWordPersistedState.currentGeneration: Long` (wrapping `Long` ≈ iOS `UInt64` for practical purposes — 2^63 wall-clock-ms is plenty).
- Every invalidating intent bumps generation (see §3 rule).
- `Outcome.Effect.queryPredictions(... generation: Long)` carries the bumped value; executor passes it to the coroutine issuing `nextWord.predict`.
- On `predict` resumption, executor compares against current generation; mismatch drops the result silently.
- A5-impl ships pure-engine tests pinning the generation-bump rule (every invalidating intent produces `newState.currentGeneration > state.currentGeneration`). The end-to-end `INVARIANT_nextword_late_prediction_is_discarded` from §10 needs a wrapper harness with a fake `NextWordService` + coroutine-test dispatcher; the `kotlinx-coroutines-test` dependency is not yet on the Android test classpath, so that wrapper-level test is **deferred to A9** per §13.11. Pre-merge gating for A5-impl is S1/S2/S3 dogfooding + the pure-engine coverage already landed.

### 13.7 Cross-platform invariant constants

After A5-impl lands `NextWordEngine.kt` / equivalent, A8-sweep adds Kotlin `// CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/NextWord/NextWordEngine.swift:<line>. Drift causes silent divergence.` comments to:

- `ASSOCIATION_TIMEOUT_MS = 10_000L`
- `CONTEXT_TIMEOUT_MS = 30_000L` (or whatever the post-split constant name becomes)

per `rules/cross-platform-alignment.md` §3a + `rules/android-guidelines.md` §2. A5-design does not move these constants; A8-sweep applies the marker. Listed here for round-to-round hand-off.

### 13.8 Strict-less-than boundary + negative-delta guard

Port iOS §5 verbatim:

```kotlin
internal fun shouldRecordAssociation(state: NextWordPersistedState, nowMs: Long): Boolean {
    val last = state.lastSelectedWord ?: return false
    val delta = nowMs - state.lastSelectionTimeMs
    return delta in 0 until 10_000L   // strict <10_000, and >= 0 drops negative clock skew
}
```

A5-impl test covers: `delta == 9_999 → true`, `delta == 10_000 → false`, `delta == -1 → false`. Today's Kotlin code returns `true` for a negative delta (same bug as iOS pre-fix); A5-impl closes it as part of the parity-correction batch.

### 13.9 Compound-association pair shape

Effect carries `List<AssociationPair>` already assembled by the engine:

```kotlin
data class AssociationPair(val prev: String, val prevTl: String, val next: String, val nextTl: String)
sealed class Effect {
    data class RecordCompoundAssociations(val pairs: List<AssociationPair>) : Effect()
    // …
}
```

Executor runs the pairs sequentially inside a single `scope.launch { pairs.forEach { nextWord.recordAssociation(...) } }` — same shape as today's loop (NextWordHandler lines 107–119). Sequential ordering is mandatory: parallel coroutines would race on the SQLite `UNIQUE(prev_word, next_word, next_tl)` constraint declared in `NextWordService.kt`'s `user_association` table.

### 13.10 Shared-core candidate roster delta (Android-side)

A5-impl adds the following Android files to the roster (mirroring §8 iOS columns):

| iOS file (§8) | Android file (target) | Shared-Core Candidate marker? |
|---|---|---|
| `NextWord/NextWordEngine.swift` | `ime/core/nextword/NextWordEngine.kt` *(new)* | Yes |
| `NextWord/NextWordOutcome.swift` | `ime/core/nextword/NextWordOutcome.kt` *(new — holds `NextWordIntent`, `NextWordPersistedState`, `NextWordDecisionInput`, `NextWordOutcome`, `Effect` types)* | Yes |
| `NextWord/RawNextWordPrediction.swift` | `ime/core/nextword/RawNextWordPrediction.kt` *(new)* | Yes |
| `NextWord/NextWordController.swift` (platform executor) | `ime/text/smartbar/NextWordHandler.kt` (reduced wrapper) | No — platform executor. |
| `NextWord/Services/NextWordService.swift` (Prediction → DTO mapping) | `ime/dictionary/NextWordService.kt` (`predict` now returns `List<RawNextWordPrediction>` directly; the pre-A5 nested `NextWordService.Prediction` DTO is deleted. `nowMs: Long` added to the `predict` signature per §13.3) | No — SQLite + file manager. |

**Post-A5-impl state (2026-04-20)**: the four new files ship the `// region Shared-Core Candidate` header inline — landing them without the header would have required reformatting them again in A8-sweep. The `// CROSS-PLATFORM INVARIANT` comments on `ASSOCIATION_TIMEOUT_MS` + `CONTEXT_TIMEOUT_MS` also land in A5-impl (§13.7 below). A8-sweep remains responsible for retro-fitting markers on pre-existing files that A5 did not touch, and for the broader §5.3 surface audit (CandidateProcessor scoring constants, any additional §11 divergence comments).

### 13.11 Out of scope for A5-design

- `NextWordEngine` as `object` (static namespace) vs `class` — A5-impl decides, same as iOS G5-impl §11 leans `enum` (Kotlin `object`).
- Exact Kotlin names for `Outcome.Effect` variants — A5-impl aligns case-for-case with iOS §2.3.
- Concrete executor class layout (owned-Job field name, `handleQueryResult` entry-point naming) — A5-impl picks.
- `NextWordService.predict` full signature change (adding `nowMs`) — folded into A5-impl PR as the clock-injection step, not a separate PR.
- StateFlow migration — deferred per §13.2 table.
- A9 `INVARIANT_*` test wiring — deferred by A0 decision; A5-impl dogfoods S1/S2/S3 per `android-g9-coverage-matrix.md`.

### 13.12 Cross-references

- iOS boundary contract: §§1–12 above.
- A5-impl deliverable: `android-state-audit.md` §7 A5-impl.
- Parity-correction policy: `rules/cross-platform-alignment.md` §1b.
- Android guidelines (shared-core purity, clock, coroutines): `rules/android-guidelines.md` §§1, 5.
- Engine settings live-read rule: `rules/android-guidelines.md` §6 + `ios-exemplar.md` §3.
- Behavioral invariants (decay half-life + user>dict weighting): `behavioral-invariants.md` §§7, 8.
