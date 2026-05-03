# Composing Manager

> **Type**: Feature
> **Keywords**: `Composing`, `rawInput`, `composingText`, `ComposingManager`
> **Related**: autocomplete.md, tone.md

---

## Summary

- Maintains "dual-state": `rawInput` (for search) + `composingText` (for display)
- Input `gua2` → rawInput=`gua2`, composingText=`guá`

---

## Core Concepts

### Dual-State Model

| State | Purpose | Example |
|-------|---------|---------|
| `rawInput` | Trie search (numeric tone) | `gua2` |
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
| `selectSuggestion` | Select candidate, replace composition |

### appendCharacter Flow

1. Update rawInput (preserve original ASCII)
2. Check character combinations (POJ: `oo`→`o͘`, `nn`→`ⁿ`)
3. Check tone conversion (number → diacritic)
4. Update composingText
5. Sync to input field

### deleteBackward Flow

1. Attempt tone restoration (`guá`→`gua`)
2. Success: rawInput deletes number, composingText updates
3. Failure: both delete last character
4. Special: `ⁿ` corresponds to `nn` in rawInput (2 characters)

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
| FFI singleton + generation guard | Rust `engine/composing::EngineHandle` |
| Tone-mark application + POJ doubletap (`oo→o͘`, `nn→ⁿ`) | Rust `engine/phonetics` |
| iOS bridge + 12 op surface | `Engine/RustEngineBridge.swift` (composingStart / Append / AppendHyphen / ReplaceLast / DeleteBackward / CommitDerived / CommitRaw / SelectSuggestion / CommitPreeditThenInsertExternal / Reset / SetSelectedCandidateIndex / QueryState) |
| iOS platform wrapper | `Input/Composing/ComposingManager.swift` (Combine + KeyboardKit context wiring) |
| iOS effect interpreter | `Input/Composing/ComposingDelegate.swift` (`UITextDocumentProxy`) |
| Android bridge | `engine/RustEngineBridge.kt` (matching 12-op surface) |
| Android platform wrapper | `ime/text/composing/ComposingManager.kt` |
| Android effect interpreter | `ime/text/composing/ComposingDelegate.kt` (`InputConnection`; **must zero composing region via `setComposingText("", 1)` before `finishComposingText()`** to honor `clearPreeditWithoutCommit` semantics) |

For per-pub-item descriptions in 台灣華語, see `migration-inventory.csv` (filter `area=composing`). Architectural contract — including Effect ordering rules + Android binding addendum — lives in `architecture/composing-state-boundary.md`.

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
