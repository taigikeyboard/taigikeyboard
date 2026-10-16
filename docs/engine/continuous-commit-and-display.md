# Continuous Mode — Commit Behavior & Display Split (Model B)

> **Type**: Specification (normative, implemented)
> **Scope**: the commit / display contract is an engine effect-model decision ([`engine/composing/src/transition.rs`](../../engine/composing/src/transition.rs)); platform bindings are thin effect translators.
> **Related**: [continuous-input-ranking.md](continuous-input-ranking.md) (candidate order), [continuous-candidate-display.md](continuous-candidate-display.md) (candidate carrier), [composing.md](composing.md)

## 10. Model B

The whole composition — already-**nailed** segments plus the pending raw tail — lives in **one** marked / composing region until a hard finalize (Enter, final-commit, external-suggestion commit). A candidate tap *nails* a segment inside the composition; only a hard finalize writes literal text to the host document. The engine has two phases, `Phase::Idle` and `Phase::Continuous { raw, caret, nailed, conversion }` ([`api.rs`](../../engine/composing/src/api.rs)).

### 10.1.1 Mainstream IME source — Model B is unanimous

| IME | Evidence | Mechanism |
|---|---|---|
| librime | `references/librime/src/rime/context.h`, `composition.cc` | One `Context::GetPreedit()` over the whole `Composition`; a selection only raises segment status; text leaves only via `GetCommitText()` on `Commit()` |
| khiin-rs | `references/khiin-rs/swift/osx/src/controller/InputController.swift` | One `client.mark(...)` over all segments; `client.insert()` only at hard commit |
| MOE Tâi-gí | [`moe-taigi-reference.md`](../references/moe-taigi-reference.md) § Nail Mechanism | `composedCharacters` (nailed) and `composingCharacters` (pending) both sit in the Android composing region |
| azooKey | `references/azooKey-Desktop/Core/Sources/Core/InputUtils/SegmentsManager.swift` | One `setMarkedText` with focused (nailed) + unfocused (pending) attribute runs; `insertText` only at finalize |

Writing a nailed prefix as literal text mid-composition (`insertText(prefix)` + `setMarkedText(tail)`) has no precedent, and on iOS hosts it leaks the tail into the document.

### 10.1.2 Slot 0 is the ranker's top candidate

There is no composing-text cell in the candidate strip. Slot 0 is the engine's top candidate (the walker's whole-sentence path, rendered segmented), styled like every other slot; tapping it commits that candidate like any other. The only composing-text surface is the inline host pre-edit, which carries the whole composition (`Phase::composing_display`), not just the pending tail (`Phase::raw_input`).

### 10.2 Display contract

| Element | Content | Source |
|---|---|---|
| Inline pre-edit (iOS `markedText`, Android `setComposingText`) | `Σ nailed[i].display_text` joined to the derived display of the pending `raw` by the word-boundary separator | `Phase::composing_display` = `api::nailed_prefix` + `Phase::raw_input` → `derived::derived_display` |
| Candidate strip, slot 0 | Walker best path: roman line with word-boundary spaces, hanji line as-is | `continuous::fetch_walker_slot0_inner` |
| Candidate strip, slot N ≥ 1 | Ranked candidates | [continuous-input-ranking.md](continuous-input-ranking.md) |

**Pending tail.** `derived_display` applies POJ doubletap pre-processing, tone-mark rendering, NFC normalization and nasal-marker case. It does **not** syllabify, insert hyphens, match candidates or infer word boundaries; user-typed separators are kept (`goa2ai3li2` → `goáàilí`, behavioral-invariants §30). While a Hanji `conversion` is shown, the pre-edit carries the converted tail instead, and `composing_display` stays the text a commit writes.

**Segmented-spacing contract.** Adjacent nailed segments, and the nailed prefix ↔ pending tail, are joined by one ASCII space **iff the rendered script is roman-ish**: `api::continuous_word_space` = `!config.renders_hanji_first() || config.output_both_scripts` (`renders_hanji_first` = hanji-first output or the TPS layout). Hanji-first and TPS join with nothing. No separator follows a hyphen-continuation segment (`tai-`). The separator is never stored in `NailedSegment.display_text` — backspace restores the tail from the separator-free `raw_text` — and the hard finalize writes the same joined string.

**Compound-hyphen.** When roman-ish, the longest run (n ≥ 2) of adjacent single-syllable nailed segments whose concatenated `canonical_text` is a known n-syllable dictionary word (`lexicon::compound_hanji_exists`: an exact `hanzi:` key with a record of that `syllable_count` and that `hanji`) is joined with the syllable joiner instead of a space — 查某 tapped as two syllables renders `tsa-bóo`. The `syllable_count` gate is load-bearing (先生 is two CJK characters but not this compound). Hanji-less segments never match. Derived on every render; no state.

**Per-segment case.** A candidate's `roman` is recased to the user's typed bytes for its own span (`continuous::recase_roman`; `Hittui` → `Hit`, `tui`). The engine is the only casing source; `display_text` and `hanji` are untouched.

**Segmented rendering.** A dual-script candidate's roman line carries word-boundary spaces (`goa ai-li`); its hanji line is the dictionary hanji with no added spaces (`我愛你`). A roman-only candidate shows the roman line only.

### 10.3 Commit contract

| Trigger | Effect |
|---|---|
| Tap, pending tail remains (**nail**) | No document write. The engine resolves the segment text and appends a `NailedSegment`; the host gets one `UpdatePreedit` of the whole composition plus `NextWordUpdateLastSelectedWord` |
| Tap consuming the rest (**final-commit**) | One `CommitTextReplacingPreedit` of the whole composition, exit to Idle, one terminal `NextWordWordSelected` |
| Enter | One `CommitTextReplacingPreedit` of `Phase::composing_display` (nailed prefix + derived tail), exit to Idle |

The **engine** formats the committed text: the platform sends `CommitContinuous.script` with the pick's `roman` / `hanji`, and `composing::commit_text` resolves the swap / TPS / both-scripts string under the live `AppConfig` (behavioral-invariants §23 / §28), answering `ComposingResponse.commit` (`CommitResolution`). With the user-data stores open, the engine also records the pick's usage itself (`UserDataStores::record_usage`); the platform sends no `RecordUsage` for it.

`CommitContinuous.canonical_text` (= `hanji.unwrap_or(roman)`) is the key frequency and NextWord learn under, so learning is mode-independent. An empty `canonical_text`, or `COMMIT_SCRIPT_UNSPECIFIED`, makes the commit **ignored**: no state change, no usage, `CommitOutcome::IGNORED` ([`composing.proto`](../../engine/protos/proto/composing.proto) `CommitContinuous`; `transition.rs` `commit_continuous`).

**Clarification β.** Enter commits the whole composition, not the pending tail: nailed segments were never in the document.

NextWord learns only at the hard finalize: `NextWordWordSelected` carries the nailed segments as `preceding` and the composition is learned as one sequence (behavioral-invariants §40 `INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING`). `NextWordUpdateLastSelectedWord` still fires per nail as the platform's nail signal; an abandoned composition teaches nothing.

For the Show Typed Text First literal, `roman` mirrors the permissive preedit while `display_text` / `canonical_tl` keep the learning key computed before permissive rendering (§34): `a1i1` and `ai1` both commit `ai` but keep separate keys.

### 10.4 Data-flow invariant

```
keystrokes → Phase::Continuous { nailed[], raw }
               composing_display = Σ nailed[i].display_text + derived(raw)  ──► ONE marked region
                                                                                 (hard finalize writes it once)
             → syllabifier → lattice / span-local fetch → candidates[0..N]  ──► strip (tap nails / finalizes)
```

| ID | Invariant |
|---|---|
| **I1** | Composing-buffer content = `Phase::composing_display`. Nailed segments are part of the marked region, not the host document; `raw` is the still-editable tail after the last nail. Every mutation re-renders the whole string. |
| **I2** | `candidate[0]` = ranker top output rendered with segmentation. |
| **I3** | Any mutation (insert / backspace / nail / unnail) re-runs the fetch for the pending tail and re-renders the composition. |
| **I4** | The platform never re-ranks, rewrites candidate order or injects exact-match bonuses (G3), and never splits a mid-composition tap into `commitText(prefix)` / `insertText(prefix)` + a new preedit. Literal text is written only by the engine's hard-finalize `CommitTextReplacingPreedit`. |

### 10.6 Effects → host API

| Effect | iOS (`HostTextWriter`) | Android (`ComposingManager`) |
|---|---|---|
| `UpdatePreedit` (nail, keystroke) | `setMarkedText(display, caret)` | `setComposingText(display, 1)` |
| `CommitTextReplacingPreedit` (hard finalize) | `setMarkedText(text + same-event literals)` + `unmarkText()` — one host write (`INVARIANT_composing_host_commit_one_write_per_event`) | `commitText(text, 1)` |
| `ClearPreeditWithoutCommit` (reset / abort) | `setMarkedText("")` + `unmarkText()` | `setComposingText("", 1)` + `finishComposingText()` |

An external clear of the composing region (field / app switch) is a hard abort: drop all nailed and pending state, exit Idle, no document write — the same as `transition::reset`.

### 10.7 Edge cases

| Case | Behavior |
|---|---|
| Single syllable | Pre-edit equals `candidate[0]` (no boundary to insert) |
| Backspace, pending tail non-empty | The char before the caret drops; the composition re-renders. Nothing is deleted from the document |
| Backspace, pending empty, ≥ 1 nailed (**unnail**) | The last segment pops; its `raw_text` becomes the pending tail (never a display-char count, which can desync under swap / TPS). NextWord last-selected rolls back to the prior segment, or clears |
| Partial prefix below the first syllable ending (`gu`) | Pre-edit shows `gu`; the strip shows partial-prefix candidates ([continuous-candidate-display.md](continuous-candidate-display.md) §15.3.D, ordered per §15.5); Enter commits `gu` |
| Enter after nails | Commits `Σ nailed.display_text` + derived tail in one write |
| Reset / abort | `[ClearPreeditWithoutCommit, ClearCandidates, NextWordClearForNewComposing]`, all nailed dropped, nothing reaches the document (`transition::reset`) |
| Hanji in the composing buffer | Pre-edit shows it; the engine returns no candidates (§15.3.E); Enter commits it as the tail |

Tests: `engine/composing/tests/continuous_phase.rs`, `engine/composing/tests/dispatch_continuous.rs`.

### 10.10 Candidate dedupe

`lexicon::continuous::candidate::dedupe_by_roman_hanji_span` collapses candidates equal on `(reading, hanji, consumed_span)` — first with `roman` as the reading, then, when any custom row is present, with `canonical_tl`. The lowest `source_tier_rank` survives (custom = rank 0, so a custom entry always beats a `dict.bin` duplicate); an equal rank keeps the earlier insertion. Custom entries are synthesized full-buffer (`consumed_span = (0, raw_len)`) and appended after the FST hits; the walker's edge pick runs the same dedupe on an edge's homophones when custom rows sit under its key.
