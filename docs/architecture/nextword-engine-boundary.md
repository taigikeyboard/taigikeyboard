# NextWord Engine / Platform Boundary

> **Type**: Reference (contract). Section numbers are cited by code (`nextword.proto` and its generated Swift / Java, `decide.rs`, `filter.rs`, both `NextWordController`s, both `RustEngineBridge`s, `SmartbarManager.kt`) and by `../engine/rust-core-proto.md` / `../engine/nextword.md` — never renumber. Gaps are removed design sections.

The next-word state machine is Rust `engine/nextword` (wire shape: `engine/protos/proto/nextword.proto`). It owns validation, the record / reset / predict decision, scoring and filtering (`scorer.rs`, constants in `behavioral-invariants.md` §7–§8), the query generation, and — through `engine/dispatch` (`user_data::handle_nextword`) — the learned bigrams in `user_association.db`. Learning decisions: `behavioral-invariants.md` §40.

Executors own the clock, the context-timeout timer, the main-thread hop and the settings snapshot, and never see a user row:

- iOS `NextWord/NextWordController.swift`
- Android `ime/text/nextword/NextWordController.kt` (§13)
- macOS / Windows / Linux `taigi-desktop-core` `composing/next_word.rs` — handshakes only; desktops show no predictions.

## 2. Contract shape

Each intent (`WordSelected`, `Backspace`, `ContextTimeoutFired`, `ClearForNewComposing`, `ResetAll`, `UpdateLastSelectedWord`, `SetPredictionsVisible`) carries a `DecisionInput` (`now_ms` + settings snapshot) and returns a `DecideResult`: `NextWordEffect`s to run in order, plus a state echo (`current_generation`, `predictions_visible`). Effects:

| Effect | Executor does |
|---|---|
| `RescheduleContextTimeout(after_ms)` | cancel the one stored timer, start a new one; on fire send `ContextTimeoutFired` |
| `CancelContextTimeout` | cancel the timer |
| `QueryPredictions(word, roman, generation, now_ms)` | send `PredictNext` off the main thread (§2.4) |
| `ClearPredictionsUI(generation)` | clear the prediction row |

The engine records bigrams itself; there is no record effect (tags 3 / 4 reserved).

The request envelope also carries a per-IME-session generation (iOS `bumpEnvelopeGeneration`, Android `resetContext`); a mismatch makes the engine reset state before applying the request, and an executor drops an answer that returns after its session ended.

### 2.4 Prediction query

`PredictNext` (`word`, `roman`, toggles, `query_generation`, `now_ms`, `limit`): dispatch reads the bundled `association.bin` rows and the user's rows after `word` (`behavioral-invariants.md` §24), then runs `FilterPredictions` in the same call. `RawNextWordPrediction` is an engine-internal row. The executor gets a `FilterResult` and shows `predictions` unless `was_stale`, then reports visibility back with `SetPredictionsVisible` (mutates visibility only, no generation bump). Showing predictions is executor work, not an effect.

## 3. Scheduling and races

- The engine never reads a clock; the executor reads it once per intent and passes `now_ms` (`INVARIANT_nextword_no_clock_read_in_engine`).
- One stored timer / `Job`; reschedule = cancel then start (`INVARIANT_nextword_rescheduling_leaks_no_timer`). Context timeout 30 s, `CONTEXT_TIMEOUT_MS` in `decide.rs`.
- Every intent that invalidates a pending query bumps `current_generation` (`WordSelected`, `Backspace`, `ClearForNewComposing`, `ResetAll`, `ContextTimeoutFired`). A query answer whose generation no longer matches comes back `was_stale` and is dropped (`INVARIANT_nextword_late_prediction_is_discarded`).

## 5. Association window

A selection records `prev → text` only when `0 <= now_ms - last_selection_time_ms < 10_000` (`ASSOCIATION_TIMEOUT_MS`, `decide.rs::should_record_association`): strict `<`, and a negative delta (clock skew) records nothing.

## 10. Invariant test labels

Pinned in `engine/nextword` tests unless noted:

- `INVARIANT_nextword_association_window_strict_lt_10s` — 9 999 → record, 10 000 → not, negative delta → not.
- `INVARIANT_nextword_backspace_does_not_record` — `Backspace` re-predicts from the last character and records no association.
- `INVARIANT_nextword_sentence_end_resets_context` — `。` cancels the timer, clears state and bumps the generation.
- `INVARIANT_nextword_clause_mark_breaks_context` — the same for every clause mark `，、；：,;:`, as the commit itself and inside `preceding` (`behavioral-invariants.md` §40).
- `INVARIANT_nextword_compound_pairs_are_sequential` — `"a b c"` gives `[(a, b), (b, c)]` in order; split on whitespace only, never a hyphen.
- `INVARIANT_nextword_no_clock_read_in_engine` — review gate: no `SystemTime` / `Instant::now` / `chrono::` in `engine/nextword/src`. No test yet.
- `INVARIANT_nextword_prediction_filter_hides_empty_tl_in_roman_mode` — empty-`tl` rows dropped when Hanji-first is off.
- `INVARIANT_nextword_late_prediction_is_discarded` — query at generation N, timeout bumps to N+1, the answer is not shown (`filter.rs`; iOS `RustEngineBridgeNextWordTests`).
- `INVARIANT_nextword_generation_bumps_on_invalidating_intents` — every invalidating intent raises `current_generation`.
- `INVARIANT_nextword_rescheduling_leaks_no_timer` — executor-side: 100 rapid reschedules leave one live timer. No test yet.

## 13. Android binding

Same contract as iOS; the Android specifics:

### 13.3 One clock per intent

`System.currentTimeMillis()` is read once at intent entry. The `now_ms` on `QueryPredictions` goes unchanged into `RustEngineBridge.nextwordPredictNext`, so the association window and the user-row decay score use the same instant.

### 13.5 Context timeout

The timer is one `Job` on `TaigiKeyboard.serviceScope` (cancelled in `onDestroy`): `scheduleContextTimeout` launches `delay(afterMs)` then `onContextTimeoutFired()`, cancel-then-launch on reschedule. A scope that outlives the IME would leak timeouts across sessions.

### 13.6 Generation

`current_generation` is a wrapping `Long` on the bridge (`UInt64` on iOS). The query coroutine also checks the envelope generation after the engine call and returns if the IME session changed.

### 13.10 Prediction values

`EnginePrediction.subtitle` is `""` on the wire when there is no romanization; both bridges map it to `nil` / `null` (`RustEngineBridge+NextWord.swift`, `RustEngineBridge.kt`).

## Cross-references

- Composing counterpart: `composing-state-boundary.md`.
- Must not regress: `behavioral-invariants.md` §§7, 8, 11, 24, 25, 40.
- Engine notes `../engine/nextword.md`; wire shape `../engine/rust-core-proto.md`; glue map `system-overview.md` §4.
