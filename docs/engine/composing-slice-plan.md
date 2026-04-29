# v3.5.4 Composing Slice — Implementation Plan

> Branch (proposed): `phase4b/v3.5.4-composing-slice`
> Date: 2026-04-30
> Status: pre-impl plan; awaiting Codex post-plan sandwich review.
> Cross-refs: `composing-slice-audit.md` (scope), `rust-core-proto.md §8` (existing design draft — to be promoted from DESIGN DRAFT to AS-IMPLEMENTED).

## 1. Architectural shift (decided)

**Today**: `ComposingState` (struct/data class) lives platform-side; `phase` + `selectedCandidateIndex` are platform-owned state. `ComposingManager` orchestrates `state.apply(intent, mode, toggles) → ComposingTransition`.

**After v3.5.4**: state lives in a Rust `engine::composing::Engine` struct held inside `Mutex<Engine>` on a singleton `EngineHandle` (per Codex (a) decision). Platform `ComposingManager` becomes a thin orchestrator: forwards intents to `RustEngineBridge.composing*(...)` and dispatches the returned `Effect[]` against `UITextDocumentProxy` / `InputConnection`. `ComposingState` + `ComposingTransition` Swift/Kotlin types DELETED in same PR (path G).

## 2. Proto draft — AS-IMPLEMENTED for v3.5.4

### 2.1 envelope.proto changes (minimal)

```protobuf
// engine/protos/proto/envelope.proto
enum CommandType {
  CMD_UNSPECIFIED = 0;
  CMD_PHONETICS = 1;
  CMD_COMPOSING = 2;   // unreserved (was: reserved 2;)
  CMD_LEXICON = 3;
}

message Request {
  // ...
  oneof payload {
    PhoneticsRequest phonetics = 10;
    ComposingRequest composing = 11;   // unreserved (was: reserved 11;)
    LexiconRequest lexicon = 12;
  }
}

message Response {
  // ...
  oneof payload {
    PhoneticsResponse phonetics = 10;
    ComposingResponse composing = 11;  // unreserved
    LexiconResponse lexicon = 12;
  }
}
```

### 2.2 NEW `engine/protos/proto/composing.proto`

```protobuf
syntax = "proto3";
package taigi.engine;
option java_package = "com.siansiansu.taigikeyboard.engine.proto";
option java_multiple_files = true;

// Intent → Effect state machine. Mirrors iOS ComposingState.Intent /
// Android ComposingState.Intent exactly (1:1 case mapping pre-migration).
//
// Naming: `oneof method` to match Phonetics/Lexicon convention adopted
// in PR #186 (was `oneof intent` in §8 design draft).

message ComposingRequest {
  oneof method {
    Start                            start = 10;
    Append                           append = 11;
    AppendHyphen                     append_hyphen = 12;
    ReplaceLast                      replace_last = 13;
    DeleteBackward                   delete_backward = 14;
    CommitDerived                    commit_derived = 15;
    CommitRaw                        commit_raw = 16;
    SelectSuggestion                 select_suggestion = 17;
    CommitPreeditThenInsertExternal  commit_preedit_then_insert_external = 18;
    Reset                            reset = 19;
    SetSelectedCandidateIndex        set_selected_candidate_index = 20;
    QueryState                       query_state = 21;   // pure read; for `isComposingText` etc.
  }
}

message Start                            { string text = 1; }
message Append                           { string char = 1; }
message AppendHyphen                     {}
message ReplaceLast                      { string replacement = 1; }
message DeleteBackward                   {}
message CommitDerived                    {}
message CommitRaw                        {}
message SelectSuggestion                 { string text = 1; }
message CommitPreeditThenInsertExternal  { string text = 1; }
message Reset                            {}
message SetSelectedCandidateIndex        { int32 index = 1; }
message QueryState                       {}

message ComposingResponse {
  message Preedit {
    string raw_input    = 1;   // numeric tone, ASCII
    string display_text = 2;   // diacritics
  }
  Preedit preedit                    = 1;
  repeated Effect effect             = 2;
  int32 selected_candidate_index     = 3;   // -1 idle, 0 fresh, preserved on ReplaceLast
  bool is_composing                  = 4;   // mirrors iOS `state.isComposing`
}

message Effect {
  oneof kind {
    UpdatePreedit                  update_preedit = 1;
    ClearPreeditWithoutCommit      clear_preedit_without_commit = 2;
    CommitTextReplacingPreedit     commit_text_replacing_preedit = 3;
    DeleteBackwardFromDocument     delete_backward_from_document = 4;
    ResetAutocomplete              reset_autocomplete = 5;
    PerformAutocomplete            perform_autocomplete = 6;
    ResetAutocompleteContext       reset_autocomplete_context = 7;
  }
}

message UpdatePreedit                   { string display = 1; }
message ClearPreeditWithoutCommit       {}
message CommitTextReplacingPreedit      { string text = 1; }
message DeleteBackwardFromDocument      {}
message ResetAutocomplete               {}
message PerformAutocomplete             {}
message ResetAutocompleteContext        {}
```

### 2.3 Notes on additions vs §8 draft

- `oneof intent` → `oneof method` (Phonetics convention).
- Added `SetSelectedCandidateIndex` — UI-driven candidate-bar tap path that exists on `ComposingManager.setSelectedCandidateIndex(_:)`. Was missing from §8 draft.
- Added `QueryState` — replaces platform `manager.isComposingText` getter without holding state on the platform side. Returns current `is_composing` + `preedit` + `selected_candidate_index`.
- Added `is_composing` boolean on `ComposingResponse` (every response carries it; platform stops shadowing this state).

## 3. Rust crate — `engine/composing/`

### 3.1 Layout

```
engine/composing/
├── Cargo.toml          # depends on protos, phonetics (for derived display via Method::NormalizeTone)
├── src/
│   ├── lib.rs          # façade: `pub mod api; pub mod dispatch;`
│   ├── api.rs          # `Engine`, `EngineState`, `Phase` (idle | composing(raw))
│   ├── dispatch.rs     # `pub fn handle(req: &ComposingRequest, engine: &mut Engine, config: &AppConfig) -> Result<ComposingResponse, ComposingError>`
│   ├── transition.rs   # `(state, intent, config) -> (new_state, effects, derived_display)` pure fn
│   └── derived.rs      # `derived_display(raw, config) -> String` — calls NEW `phonetics::api::normalize_tone(input, config)` (oo/nn preprocessing + nasal-case adjustment), returns raw for TPS
└── tests/
    ├── intent_coverage.rs  # one test per intent — round-trip via dispatch
    ├── invariants.rs       # INVARIANT_composing_* parity oracles
    └── lifecycle.rs        # idempotent reset, generation guard, candidate-tap re-entry
```

### 3.2 Why `engine/composing` as a separate crate (not folded into `phonetics`)

- Different concern: state machine vs pure transformations.
- Different dependencies: composing depends on phonetics (one-way edge); phonetics has no composing dep.
- Cohesion goal (per user's brief #1): "rust 模組內聚力高,與其他模組依賴少" — separate crate enforces the boundary.

### 3.2a Phonetics dep — call shape (per Codex P1.2)

`composing::derived::derived_display` MUST run the same preprocessing chain as the platform call to `RustEngineBridge.normalizeTone(input, mode, toggles)`:

1. `oo`/`nn` POJ doubletap preprocessing (driven by `AppConfig.oo_doubletap_enabled` + `nn_doubletap_enabled`).
2. Tone-mark application.
3. Nasal-case adjustment (`adjust_nasal_marker_case`).

Step 1 + 3 live inside `phonetics::dispatch` today (`engine/phonetics/src/dispatch.rs:48-55, 153-165`); plain `phonetics::api::to_tone_marks` skips them.

**Action**: expose `phonetics::api::normalize_tone(input: &str, config: &AppConfig) -> String` as a NEW Rust-internal API that runs the full preprocessing chain. `composing` calls this directly. Do NOT dispatch back through `engine/dispatch::process_request` (would invert the dep direction and reintroduce the §3a domain↔proto boundary anti-pattern).

This is a one-line addition to `phonetics::api` exposing the existing chain as a single fn — no new logic.

### 3.3 Engine ownership model

```rust
// engine/composing/src/api.rs (sketch)
pub struct Engine {
    state: EngineState,
}

#[derive(Clone, PartialEq, Debug)]
pub struct EngineState {
    phase: Phase,
    selected_candidate_index: i32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Phase {
    Idle,
    Composing { raw: String },
}

impl Engine {
    pub fn new() -> Self { ... }
    pub fn snapshot(&self) -> ComposingResponse { ... } // for QueryState
    pub fn apply(&mut self, intent: Intent, config: &AppConfig) -> ComposingResponse { ... }

    // Private helper — NOT public API. Called from `Intent::Reset` handling
    // and the generation-mismatch path inside `dispatch::handle`. External
    // callers always go through `dispatch::handle(...)`.
    fn reset(&mut self) { ... } // idempotent — no-op on Idle, no effects emitted
}
```

**Send/Sync correctness (per Codex P2.1)**: `Engine` MUST remain `Send` (the contained state — `Phase::Idle | Composing { raw: String }` + `i32` index — is `Send` by default since `String: Send`). `Mutex<T>` is `Sync` iff `T: Send`. Add a compile-time assertion in `engine/composing/src/api.rs`:

```rust
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<Engine>();
};
```

The static `OnceCell<EngineHandle>` then satisfies `Sync` via `Mutex<Engine>: Sync`.

### 3.4 EngineHandle (singleton-per-IME-instance, per Codex (a))

The `EngineHandle` is a process-singleton struct in `engine/dispatch` (or `engine/composing` itself):

```rust
pub struct EngineHandle {
    composing: Mutex<composing::Engine>,
}

impl EngineHandle {
    pub fn instance() -> &'static Self { ... } // OnceCell
    pub fn handle_composing(&self, req: &ComposingRequest, config: &AppConfig) -> ComposingResponse {
        let mut engine = self.composing.lock().unwrap();
        composing::dispatch::handle(req, &mut *engine, config).unwrap_or_else(|err| ...)
    }
}
```

`engine/dispatch::process_request` routes `Request.payload = ComposingRequest` through `EngineHandle::instance().handle_composing(...)`.

## 4. Lifecycle invariants (per Codex risk-add)

Encode into the slice:

1. **`reset_composing` is idempotent.** Calling on already-Idle engine is a no-op; emits empty `effect[]`. Verified by `tests/lifecycle.rs::reset_idempotent`.
2. **InputContextGeneration guard on platforms.**
   - iOS: `KeyboardViewController` tracks `inputContextGeneration: UInt` + `lastDocumentIdentifier: UUID?`. On `textWillChange`:
     - If `textDocumentProxy.documentIdentifier != nil`: increment generation iff it differs from `lastDocumentIdentifier`. Update cached identifier.
     - If `textDocumentProxy.documentIdentifier == nil` (per Codex P1.4): conservative path — increment generation ONLY when `!selfCommitInProgress && !manager.isComposing`. If currently composing or in a self-commit window, DEFER the reset (do not increment); rely on the explicit `Reset` intent emitted by `KeyboardViewController+Cleanup.swift::resetComposing()` plus the candidate-tap dogfood test (S2) as the safety net.
   - Android: `TextInputManager` tracks `inputContextGeneration: Long`. Increment in `onStartInputView(restarting=false)` ONLY (not on `restarting=true` re-entry). Pass via request.
   - `generation: u64` field on `Request` is wide enough that wrap is practically infeasible — no wrap handling required.
3. **Self-commit suppression.** During the platform-side dispatch of `CommitTextReplacingPreedit` / `ClearPreeditWithoutCommit`, set a `selfCommitInProgress` flag; subsequent `textWillChange` callbacks during that window do NOT trigger generation increment.
4. **Generation field on `Request`.** Already present (`envelope.proto:Request.generation: uint64`). Currently logged only; v3.5.4 wires it into composing-side reset semantics.

## 5. Bridge surface — iOS + Android

### 5.1 Per-intent wrappers (1:1 with proto methods, 12 ops)

iOS `RustEngineBridge.swift` adds:
- `composingStart(_ text: String, mode:, toggles:, generation:) -> ComposingResponse`
- `composingAppend(_ char: String, mode:, toggles:, generation:) -> ComposingResponse`
- `composingAppendHyphen(mode:, toggles:, generation:) -> ComposingResponse`
- `composingReplaceLast(_ replacement: String, mode:, toggles:, generation:) -> ComposingResponse`
- `composingDeleteBackward(mode:, toggles:, generation:) -> ComposingResponse`
- `composingCommitDerived(mode:, toggles:, generation:) -> ComposingResponse`
- `composingCommitRaw(generation:) -> ComposingResponse`
- `composingSelectSuggestion(_ text: String, generation:) -> ComposingResponse`
- `composingCommitPreeditThenInsertExternal(_ text: String, mode:, toggles:, generation:) -> ComposingResponse`
- `composingReset(generation:) -> ComposingResponse`
- `composingSetSelectedCandidateIndex(_ index: Int, generation:) -> ComposingResponse`
- `composingQueryState(generation:) -> ComposingResponse`

Android `RustEngineBridge.kt` mirrors with same names in camelCase.

### 5.2 Composing response decoding

Both bridges expose a typed `ComposingTransition` Swift/Kotlin struct (re-named slightly to avoid colliding with deleted source-types — actually we DELETE the platform `ComposingTransition` in the same PR, then bridge synthesizes a Swift/Kotlin shape from the proto response. Naming convention: keep `ComposingTransition` on the platform as the bridge return type, since platform call sites already use that name).

## 5b. Documented platform divergences (per Codex P1 review, 2026-04-30)

The Rust state machine is unified across platforms, but TWO documented platform-binding divergences must be encoded on the bridge wrapper side (NOT in Rust):

### 5b.1 Android `deleteBackward` 1-char → `Reset` mapping

Android's `ComposingManager.deleteBackward` picks `Intent::Reset` when `rawInput.length == 1`, where iOS picks `Intent::DeleteBackward`. Reason (`composing-state-boundary.md §11.10`): Android's `ClearPreeditWithoutCommit` already removes the in-document composing region, so emitting an additional `DeleteBackwardFromDocument` would delete a *pre-existing* document char. iOS's floating marked-text model has the opposite need.

**Encoding**: keep the branch on the Android `ComposingManager.kt::deleteBackward` wrapper. The wrapper picks the bridge call:

```kotlin
fun deleteBackward(ic: InputConnection): Boolean {
    // Platform no longer shadows engine state — query first.
    val state = RustEngineBridge.composingQueryState(generation = currentGeneration)
    if (!state.isComposing || state.preedit.rawInput.isEmpty()) return false
    val resp = if (state.preedit.rawInput.length == 1) {
        RustEngineBridge.composingReset(generation = currentGeneration)
    } else {
        RustEngineBridge.composingDeleteBackward(mode, toggles, generation = currentGeneration)
    }
    dispatchEffects(resp, ic)
    return true
}
```

iOS `ComposingManager.swift::deleteBackward` always calls `composingDeleteBackward` (no query, no branch — engine handles the empty-buffer guard internally and returns the appropriate idle transition).

### 5b.2 Lifecycle reset semantics — single table

Three reset paths exist; `Reset` (intent) and engine internal reset are NOT the same:

| Trigger | Wire op | Engine action | Effects emitted |
|---|---|---|---|
| User-initiated reset (mode switch, teardown) | `ComposingRequest.Reset` (method 19) | If composing → set Idle; if Idle → no-op | `[ClearPreeditWithoutCommit, ResetAutocomplete]` if was composing; `[]` if was Idle |
| Focus change with generation mismatch | platform sends NEXT request with new `generation` | Engine compares incoming generation to last-seen; on mismatch → silently drop state to Idle BEFORE applying request | NO effects from the drop itself; the request's own effects then apply against fresh state |
| Idle re-entry / textWillChange same-context | platform suppresses; sends nothing OR sends `QueryState` | no-op | none |

Rust internal `Engine::reset(&mut self)` is NOT public API — it is a private helper called from both `Reset` intent handling and generation-mismatch path. External callers always go through `dispatch::handle(...)`.

## 6. Platform-side changes

### 6.1 Files DELETED (path G)

- `ios/Sources/TaigiKeyboard/Input/Composing/ComposingState.swift` (274 LOC)
- `ios/Sources/TaigiKeyboard/Input/Composing/ComposingTransition.swift` (50 LOC) — replaced by bridge-synthesized type
- `android/.../ime/text/composing/ComposingState.kt` (393 LOC)
- `android/.../ime/text/composing/ComposingTransition.kt` (66 LOC)
- Corresponding test files: `ComposingStateTest.kt`, `ComposingStateTests.swift` (kept as INVARIANT oracles? See §7).

### 6.2 Files MODIFIED (slimmed orchestrators)

- `ios/.../Input/Composing/ComposingManager.swift` — becomes thin adapter; `state.apply(...)` → `RustEngineBridge.composingApply(...)`; effect dispatch loop unchanged.
- `ios/.../Input/Composing/ComposingDelegate.swift` — protocol unchanged (still consumes `ComposingTransition.Effect` enum, now bridge-synthesized).
- `android/.../ime/text/composing/ComposingManager.kt` — same shape.
- `android/.../ime/text/composing/ComposingDelegate.kt` — same shape.

### 6.3 Upstream call sites (12 iOS + 7 Android)

No API change required for callers — `ComposingManager` keeps its public surface. Internal refactor only.

### 6.4 Lifecycle wiring (NEW)

- iOS `KeyboardViewController+TextInput.swift` / `+Setup.swift` / `+Cleanup.swift` — add generation increment + selfCommitInProgress guard.
- Android `TextInputManager.kt::onStartInputView` — add generation increment guard.

## 7. INVARIANT_composing_* parity tests

Two layers:

### 7.1 Rust workspace tests (`engine/composing/tests/invariants.rs`)

One test per intent, oracle from CURRENT iOS/Android Swift/Kotlin behavior captured pre-deletion. Format:

```rust
#[test]
fn invariant_start_then_append_resets_index_to_zero() {
    let mut engine = Engine::new();
    let r1 = engine.apply(Intent::Start("ka".into()), &config_tl());
    assert_eq!(r1.selected_candidate_index, 0);
    let r2 = engine.apply(Intent::Append("u".into()), &config_tl());
    assert_eq!(r2.selected_candidate_index, 0);
    assert_eq!(r2.preedit.raw_input, "kau");
    // ...
}
```

Coverage: every `apply()` branch in current `ComposingState.swift:105-225` (12 cases × 2 phases × edge cases ≈ 30 tests).

### 7.2 Platform parity tests (KEEP from existing test files)

`ios/TaigiKeyboardTests/ComposingStateTests.swift` + `android/.../ComposingStateTest.kt` — retarget to call `RustEngineBridge.composing*(...)` and assert against the bridge-synthesized `ComposingTransition`. These tests were just retargeted in PR #194; they become the platform parity oracle for the bridge synthesis logic. **Retarget lands in commits 9 + 10** (same commits as platform-state deletion) per §8 P1.5 fix.

### 7.3 Property tests (per Codex P2.2)

Add `engine/composing/tests/proptest_sequences.rs`:
- Random `Vec<Intent>` (length 1-12) drawn from `{Append, AppendHyphen, ReplaceLast, DeleteBackward, CommitDerived, CommitRaw, Reset}`.
- Apply sequentially; assert invariants hold at every step:
  - `phase == Idle ⟹ raw_input == "" ∧ selected_candidate_index == -1`
  - `phase == Composing(raw) ⟹ selected_candidate_index >= 0`
  - `selected_candidate_index` only changes on `Start | Append | DeleteBackward(non-empty result) | Reset | SetSelectedCandidateIndex` — preserved on `ReplaceLast`.
  - `is_composing` matches `phase != Idle` on every response.

Catches stale `raw_input`, selected-index drift, and idle invariants cheaply.

## 8. Commit slicing (single PR, ordered commits)

| # | Commit | Description |
|---|---|---|
| 1 | proto: add ComposingRequest/Response + unreserve CMD_COMPOSING | proto-only; regenerates `*.pb.swift` + `*.java` |
| 2 | engine/composing: crate skeleton (api+dispatch+transition+derived) | Rust skeleton; tests stub-only |
| 3 | engine/composing: implement Intent → Effect transition (port from Swift) | full state machine in Rust |
| 4 | engine/composing: INVARIANT_composing_* tests (intent_coverage + invariants) | parity test suite |
| 5 | engine/composing: lifecycle tests (idempotent reset, generation guard) | per Codex risk |
| 6 | engine/dispatch: route CMD_COMPOSING → EngineHandle::handle_composing | dispatch wiring |
| 7 | iOS bridge: add composing* wrappers + ComposingTransition synth | bridge surface only |
| 8 | Android bridge: add composing* wrappers + ComposingTransition synth | mirror commit 7 |
| 9 | iOS: rewire ComposingManager → bridge; DELETE ComposingState.swift + ComposingTransition.swift | path G |
| 10 | Android: rewire ComposingManager → bridge; DELETE ComposingState.kt + ComposingTransition.kt | mirror commit 9 |
| 11 | iOS+Android: lifecycle wiring (InputContextGeneration + selfCommitInProgress) | per Codex risk |
| 12 | docs: rust-core-proto.md §8 DRAFT → AS-IMPLEMENTED; v3.5.3-cleanup-audit.md v3.6.0 → v3.5.4 | doc cleanup absorbed |

**Per-commit compile invariant (per Codex P1.5)**: each commit compiles + tests pass. Commits 9 and 10 BUNDLE the platform-state deletion WITH retarget of `ComposingStateTests.swift` / `ComposingStateTest.kt` so tests never reference deleted types in a transient state. Original §7.2 plan to retarget in a separate later commit is REVOKED — retarget is folded into 9 and 10 respectively.

Commits 7+8 introduce dead bridge surface that becomes live in 9+10. Commit 9 must land before 10 (or vice versa) within the same PR; the in-between cross-platform mismatch is acceptable since no consumer (iOS test or Android test) cuts across platforms.

## 9. Out-of-scope (deferred)

- `TaigiAutocompleteService.kt`, `AutocompleteContextBooster.kt`, `EnglishAutocompleteService.kt` — v3.5.5 NextWord slice.
- `UserFrequencyService.kt`, `AutocompleteInputClassifier.kt` — re-evaluate during v3.5.5 planning.
- `engine/lexicon` crate — v3.5.6.

## 10. Acceptance gates (per cadence template)

1. `engine/composing` workspace tests green; INVARIANT count ≥ 30.
2. iOS `ComposingStateTests` retargeted + green.
3. Android `ComposingStateTest` retargeted + green.
4. Dogfood S1/S2/S3 on real device:
   - S1: type "kau1" → display "kau", select candidate, commit ✓
   - S2: type → tap candidate-bar → commit-via-tap (selfCommit guard) ✓
   - S3: type → switch to a different field (focus change → generation increment → reset) → no leak ✓
5. No keyboard dismiss regression.
6. No memory leaks (qualitative — Mutex<Engine> singleton has no owner cycle).
7. Codex pre-impl APPROVE (THIS DOC).
8. Codex post-impl APPROVE (after PR opened).
9. `/simplify` pass.

## 11. Anticipated risks

- **R1**: iOS `documentIdentifier` nil fallback — RESOLVED §4.2: conservative reset only when `!selfCommitInProgress && !manager.isComposing`; defer otherwise.
- **R2**: `phonetics` dep from `composing` crate forms one new edge in workspace graph; verify no cycle (`phonetics` has no `composing` dep, so safe).
- **R3**: Generated proto files in `engine/protos/proto/composing.proto` cascade to `phonetics.pb.swift` / Java equivalents — must regenerate same PR.
- **R4**: `ComposingTransition.Effect` enum order matters (effects executed in proto-list order); proto repeated-field ordering is preserved by prost — verified.
- **R5**: Android delete-to-empty divergence — RESOLVED §5b.1: kept on Android wrapper; iOS wrapper unchanged.

## 12. Doc updates absorbed in this PR

- `rust-core-proto.md §8` DESIGN DRAFT → AS-IMPLEMENTED + remove "v3.6.0" / "D9.3 deferred" wording.
- `v3.5.3-cleanup-audit.md:42,109` v3.6.0 reference → v3.5.4.
- `ranking-slice-audit.md:154` `// CMD_COMPOSING for D9.3` → reflect filled.
- `envelope.proto:18,53` reserved comments dropped (unreserve action speaks for itself).
- `phonetics.proto:24,162` D9.x history wording — keep (per r3 deferred list).
