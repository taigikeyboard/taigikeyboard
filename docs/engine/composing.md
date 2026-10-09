# Composing Manager

> **Type**: Feature
> **Keywords**: `Composing`, `rawInput`, `composingText`, `ComposingManager`
> **Related**: continuous-candidate-display.md, tone.md

---

## Summary

- Maintains "dual-state": `rawInput` (for search) + `composingText` (for display)
- Input `gua2` → rawInput=`gua2`, composingText=`guá`

---

## Core Concepts

### Dual-State Model

| State | Purpose | Example |
|-------|---------|---------|
| `rawInput` | fst search (numeric tone) | `gua2` |
| `composingText` | UI display (diacritics) | `guá` |

### Why Dual-State?

- Trie lookup uses prefixed keys: `tl:gua2`, `poj:goa2` (numeric tone, ASCII)
- Users expect to see diacritics, not numbers

---

## Core Operations

| Operation | Description |
|-----------|-------------|
| `appendCharacter` | Append character, update dual-state |
| `deleteBackward` | Delete character, handle tone restoration |
| `commitComposition` | Confirm composition, output text |
| `commitContinuous` | Pick a candidate: nail a prefix or commit the composition |

### appendCharacter Flow

1. Update rawInput (preserve original ASCII)
2. Check character combinations (POJ: `oo`→`o͘`, `nn`→`ⁿ`)
3. Check tone conversion (number → diacritic)
4. Update composingText
5. Sync to input field

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

| Tone | Handling | Example |
|------|----------|---------|
| 1, 4 | Keep number | `gua1`→`gua1` |
| 2,3,5,6,7,8,9 | Convert to diacritic | `gua2`→`guá` |

---

## Ownership

Composing engine state machine lives in Rust `engine/composing` (since v3.5.4 / PR #197). Platform side holds the wrapper + effect interpreter; tone math sits in `engine/phonetics`.

| Item | Location |
|------|----------|
| State machine (`Phase × Intent → (state', Effect[])`) | Rust `engine/composing` (`api.rs`, `transition.rs`, `derived.rs`) |
| FFI singleton + generation guard | Rust `engine/composing::EngineHandle` (read-only intents never reset — see below) |
| Tone-mark application + POJ doubletap (`oo→o͘`, `nn→ⁿ`) | Rust `engine/phonetics` |
| iOS bridge (8 text-input + 2 continuous-input ops) | `Engine/RustEngineBridge+Composing.swift` (composingStart / Append / AppendHyphen / ReplaceLast / DeleteBackward / CommitRaw / CommitPreeditThenInsertExternal / Reset + FetchAtPos / CommitContinuous; SelectCandidate removed 2026-10-09 (round A1a)) — one engine call per keystroke (R12) |
| iOS platform wrapper | `Input/Composing/ComposingManager.swift` (Observation + KeyboardKit context wiring) |
| iOS effect interpreter | `Input/Composing/ComposingDelegate.swift` (`UITextDocumentProxy`) |
| Android bridge | `engine/ComposingBridge.kt` (same 8 + 2 ops) |
| Android platform wrapper | `ime/text/composing/ComposingManager.kt` |
| Android effect interpreter | `ime/text/composing/ComposingDelegate.kt` (`InputConnection`; **must zero composing region via `setComposingText("", 1)` before `finishComposingText()`** to honor `clearPreeditWithoutCommit` semantics) |

**Read-only intents and threading.** `Intent::is_read_only()` (only `FetchAtPos`) never mutates: `EngineHandle` answers them from a clone of the engine with the mutex released, and a generation mismatch on one returns `Engine::idle_snapshot` **without** resetting state or recording the generation — only the next mutating intent resets. This lets Android run the candidate fetch on `Dispatchers.Default` (`CandidateUpdateCoordinator`): a stale worker fetch can never wipe a newer context, and the coordinator re-validates `ComposingManager.stateToken()` (raw buffer + generation) on Main before touching the strip. iOS / macOS / Windows fetch synchronously on their main threads and mirror the returned snapshot; the mismatch response they see is unchanged (Idle). `lexicon::EngineHandle` state is an `RwLock`, so a worker fetch's dictionary scan and the main thread's `Append` display render (compound-hyphen oracle) read concurrently.

For per-pub-item descriptions in Taiwanese Mandarin, see `migration-inventory.csv` (filter `area=composing`). Architectural contract — including Effect ordering rules + Android binding addendum — lives in `architecture/composing-state-boundary.md`.

## Tests

| Platform | File | Coverage |
|----------|------|----------|
| Rust engine | `engine/composing/tests/{intent_coverage,invariants,lifecycle,proptest_sequences}.rs` | State machine semantics + property-based tests |
| iOS wrapper | `TaigiKeyboardTests/ComposingManagerTests.swift` | Effect-execution order per intent |

---

## Test Cases

| Input sequence | rawInput | composingText |
|----------------|----------|---------------|
| g u a | `gua` | `gua` |
| g u a 2 | `gua2` | `guá` |
| g u a 1 | `gua1` | `gua1` |
| h o o 2 (POJ) | `hoo2` | `hó͘` |

| Before delete | After rawInput | After composingText |
|---------------|----------------|---------------------|
| `guá` / `gua2` | `gua` | `gua` |
| `Phiaⁿ` / `Phiann` | `Phia` | `Phia` |
