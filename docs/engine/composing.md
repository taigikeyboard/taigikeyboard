# Composing Manager

> **Type**: Feature
> **Keywords**: `Composing`, `rawInput`, `composingText`, `ComposingManager`, `Intent`
> **Related**: [continuous-commit-and-display.md](continuous-commit-and-display.md), [continuous-candidate-display.md](continuous-candidate-display.md), [tone.md](tone.md)

---

## Summary

- The engine keeps the typed ASCII buffer (`raw`, numeric tones) and derives the displayed pre-edit from it: `gua2` → `guá`.
- The whole composition (nailed segments + pending tail) is one pre-edit — [continuous-commit-and-display.md §10.2](continuous-commit-and-display.md#102-display-contract).

---

## Core Operations

`Intent` ([`engine/composing/src/api.rs`](../../engine/composing/src/api.rs)) — one per `ComposingRequest` method in [`composing.proto`](../../engine/protos/proto/composing.proto):

| Intent | Description |
|---|---|
| `Start`, `Append`, `AppendHyphen`, `ReplaceLast` | Edit the pending `raw` at the caret; the display is re-derived |
| `DeleteBackward` | See below |
| `CommitRaw` | Enter: commit the whole composition |
| `CommitPreeditThenInsertExternal` | Commit the composition, then an external text |
| `Reset` | Abort: drop the composition, nothing written |
| `FetchAtPos` | Read-only candidate fetch |
| `CommitContinuous` | Pick a candidate: nail a prefix or finalize the composition |
| `CommitPrediction` | Mobile: a NextWord prediction tap — the document text it writes and its usage; the composition is untouched |
| `MoveCaret` | Desktop: step the caret inside the pending tail, or jump to its start / end; with a Hanji conversion, step over a converted word or re-open the last nailed segment |
| `TelexKey`, `TpsKey` | Desktop Telex key; TPS key at the caret |
| `CommitAsShown`, `CommitAsTyped` | Desktop, under a Hanji conversion |

### deleteBackward Flow

Engine `delete_backward_continuous` (`engine/composing/src/transition.rs`) edits `raw` only; the display text is re-derived, never edited.

1. Pending tail non-empty: drop the raw char before the caret (a tone digit is one raw char, so `gua2`→`gua` renders `guá`→`gua`; `nn`/`ⁿ` is two raw chars) and re-render; pending and nailed both empty → exit to Idle
2. Pending empty, nailed non-empty: unnail the last segment — its `raw_text` becomes the new pending tail
3. Pending and nailed empty: exit to Idle

---

## Character Combinations (POJ Only)

| Input | Output | Unicode |
|-------|--------|---------|
| `oo` | `o͘` | U+006F + U+0358 |
| `nn` | `ⁿ` | U+207F |

- TL mode: no conversion, keeps original

---

## Tone Handling

TL / POJ pre-edit (`phonetics::api::normalize_tone`, permissive placement):

| Tone | Handling | Example |
|------|----------|---------|
| 1, 4 | Digit consumed, no mark (hidden) | `gua1`→`gua` |
| 2,3,5,6,7,8,9 | Convert to diacritic | `gua2`→`guá` |

The hidden digit stays in `raw` and in the learning key (`literal_learning_text`): `a1i1` and `ai1` both show `ai` but keep separate keys.

---

## Ownership

| Item | Location |
|------|----------|
| State machine (`Phase × Intent → (state', Effect[])`) | Rust `engine/composing` (`api.rs`, `transition.rs`, `derived.rs`) |
| FFI singleton + generation guard | Rust `engine/composing::EngineHandle` (read-only intents never reset — see below) |
| Tone-mark application + POJ doubletap (`oo→o͘`, `nn→ⁿ`) | Rust `engine/phonetics` |
| iOS bridge | `Engine/RustEngineBridge+Composing.swift` — one engine call per keystroke |
| iOS platform wrapper | `Input/Composing/ComposingManager.swift` (Observation + KeyboardKit context wiring) |
| iOS effect interpreter | `Input/Composing/ComposingDelegate.swift` (`UITextDocumentProxy`) |
| Android bridge | `engine/ComposingBridge.kt` |
| Android platform wrapper | `ime/text/composing/ComposingManager.kt` |
| Android effect interpreter | `ime/text/composing/ComposingDelegate.kt` (`InputConnection`; **must zero composing region via `setComposingText("", 1)` before `finishComposingText()`** to honor `clearPreeditWithoutCommit` semantics) |

**Read-only intents and threading.** `Intent::is_read_only()` (only `FetchAtPos`) never mutates: `EngineHandle` answers them from a clone of the engine with the mutex released, and a generation mismatch on one returns `Engine::idle_snapshot` **without** resetting state or recording the generation — only the next mutating intent resets. This lets Android run the candidate fetch on `Dispatchers.Default` (`CandidateUpdateCoordinator`): a stale worker fetch can never wipe a newer context, and the coordinator re-validates `ComposingManager.stateToken()` (raw buffer + generation) on Main before touching the strip. iOS / macOS / Windows fetch synchronously on their main threads and mirror the returned snapshot; the mismatch response they see is unchanged (Idle). `lexicon::EngineHandle` state is an `RwLock`, so a worker fetch's dictionary scan and the main thread's `Append` display render (compound-hyphen oracle) read concurrently.

Architectural contract — including Effect ordering rules + Android binding addendum — lives in [`architecture/composing-state-boundary.md`](../architecture/composing-state-boundary.md).

## Tests

| Platform | File | Coverage |
|----------|------|----------|
| Rust engine | `engine/composing/tests/{intent_coverage,invariants,lifecycle,proptest_sequences,continuous_phase}.rs` | State machine semantics + property-based tests |
| iOS wrapper | `TaigiKeyboardTests/ComposingManagerTests.swift` | Effect-execution order per intent |

| Input sequence | raw | Pre-edit |
|----------------|-----|----------|
| g u a | `gua` | `gua` |
| g u a 2 | `gua2` | `guá` |
| g u a 1 | `gua1` | `gua` |
| h o o 2 (POJ) | `hoo2` | `hó͘` |
