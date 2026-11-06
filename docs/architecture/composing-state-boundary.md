# Composing State / Platform Boundary

> **Type**: Reference (contract). Section numbers are cited by code (both `ComposingDelegate`s, `KeyboardViewController+TextInput.swift`, `KeyboardViewController+EmojiDelegate.swift`, `TextInputManager.kt`, `MediaInputManager.kt`) and by `behavioral-invariants.md` §13 / `../engine/rust-core-proto.md` — never renumber. Gaps are removed design sections.

The composing state machine is Rust `engine/composing`; every `ComposingResponse` carries an ordered `Effect` list (`engine/protos/proto/composing.proto`, `message Effect`). Each platform interprets that list and nothing else:

| Platform | Interpreter |
|---|---|
| iOS | `Input/Composing/ComposingManager.swift` → `KeyboardViewController+TextInput.swift` `execute` → `Actions/HostTextWriter.swift` (`UITextDocumentProxy`) |
| Android | `ime/text/composing/ComposingManager.kt` → `ComposingDelegate.kt` (`InputConnection`, §11) |
| macOS / Windows / Linux | `taigi-desktop-core::composing`, bound by `TaigiInputController.replay` + `ClientWriter.swift` / `tsf/src/composition.rs` / `taigi-desktop-core::session` recorder |

The wrappers are `wont_migrate` glue (`../engine/migration-inventory.csv`, `area=composing`). Behaviour a platform needs goes into the engine's effect list, not into wrapper logic.

## 2. Contract shape

### 2.2 Effect → platform binding

Effect names describe behaviour, not an OS API. `UpdatePreedit.display` is the engine-derived (tone-marked) text with `caret_utf16`; every platform shows it as is.

| Effect | iOS — `HostTextWriter` | Android — `InputConnection` |
|---|---|---|
| `UpdatePreedit(display)` | `setMarkedText(display, selectedRange: end)` | `setComposingText(display, 1)` |
| `ClearPreeditWithoutCommit` | `setMarkedText("", selectedRange: 0)` + `unmarkText()` | `setComposingText("", 1)` **then** `finishComposingText()` (§11.2 rule 1) |
| `CommitTextReplacingPreedit(text)` | over a marked region: `setMarkedText(text, …)` + `unmarkText()`, never `insertText` — Flutter hosts before flutter/flutter#191062 insert at the caret and keep the preedit. Inside a user event the commit is held and written once with the literals that follow (`INVARIANT_composing_host_commit_one_write_per_event`). No marked region: `insertText(text)` | `commitText(text, 1)` — no `finishComposingText` first (§11.2 rule 2) |
| `ClearCandidates` / `RefreshCandidates` / `ResetCandidateContext` | candidate / autocomplete state only | no `InputConnection` call |
| `NextWordUpdateLastSelectedWord` / `NextWordWordSelected` / `NextWordClearForNewComposing` | `NextWordController` | `NextWordEffectRouter` → `NextWordController` |

### 2.3 Ordering

Effects run in list order. Engine tests in `engine/composing/src/transition.rs` pin each sequence, for example:

- Backspace to an empty composition → `ClearPreeditWithoutCommit`, `ClearCandidates`, `NextWordClearForNewComposing` (nothing is deleted from the document; the text lived only in the preedit).
- Enter (`CommitRaw`) → `CommitTextReplacingPreedit(whole composition)`, `ClearCandidates`, `ResetCandidateContext`, `NextWordWordSelected`.

A commit is always one atomic `CommitTextReplacingPreedit`, never clear-then-insert.

### 2.4 Wrapper phases

The wrapper applies a response in three steps: (1) update its observable mirror (`rawInput`, display text, `isComposing` — written only when the value changes, so idle → idle emits nothing); (2) execute the effects in order; (3) iOS notifies its `ComposingContextSink`. The mirror settles before the document changes, so the UI renders once with consistent state.

## 3. Where settings enter

The wrapper reads `EngineSettingsProvider.current` once per intent and sends it on the request (`AppConfig`, including the POJ marker options `phonetics::normalize_tone` reads). The engine keeps no settings between requests, so a settings change applies to the next keystroke (`behavioral-invariants.md` §11).

## 10. Cross-references

- Next-word counterpart: `nextword-engine-boundary.md`.
- Composing reset invariants: `behavioral-invariants.md` §13; must not regress §§1–3, 9, 11.
- Wire shape `../engine/rust-core-proto.md`; engine internals `../engine/composing.md`; glue map `system-overview.md` §4.

## 11. Android binding

### 11.2 `InputConnection` rules

1. **Zero, then finish.** `finishComposingText()` commits the composing region. Every clear without commit — the `ClearPreeditWithoutCommit` binding, `ComposingManager.reset(ic)`, and the bare-`InputConnection` sites through `clearHostComposingRegion(ic)` — issues `setComposingText("", 1)` first (`INVARIANT_composing_clear_preedit_does_not_commit`).
2. **Atomic commit.** `CommitTextReplacingPreedit` is one `commitText(text, 1)`; a preceding `finishComposingText()` would commit the old preedit and double the text.
3. **Main thread.** `InputConnection` calls run on the IME main thread; a coroutine hops back with `withContext(Dispatchers.Main.immediate)` first.

### 11.6 External text and reset

Text inserted from outside the composition (emoji palette) goes through `commitPreeditThenInsertExternal`: one `CommitTextReplacingPreedit(preedit + external)` instead of finishing the preedit and inserting separately (`INVARIANT_composing_external_insert_commits_preedit_atomically`). Android: `MediaInputManager.sendEmojiKeyPress`; iOS: `ActionHandler.insertExternalText`. `reset(ic)` clears without committing per §11.2 rule 1 on both.

### 11.10 Host-cleared composing region

The live-preedit divergences once listed here (Android raw keystrokes vs iOS derived text, the `commitComposition` fast / slow split) are gone: the engine sends the derived display to every platform. What remains Android-only is the host reporting no composing region (`onUpdateSelection` with `candidatesStart == candidatesEnd == -1`, the former "divergence #3"): `TextInputManager.onUpdateSelection` → `ComposingManager.onHostSelectionUpdate` → `reconcileWithHost(ic)` re-reads the editor before `onExternalComposingRegionCleared()` drops state, because the report can be older than the IME's own last write. Full rule: `behavioral-invariants.md` §13.
