# Continuous Mode — Commit Behavior & Display Split (Model B)

> **Status**: **Model B — normative, implemented (engine P1).** Rewritten 2026-05-16 (v3.5.8 Phase 9 Bug 3 closeout). Codex design co-review PASS.
> **Scope**: v3.5.8 Phase 9. The commit/display contract is an **engine effect-model** decision (`engine/composing/src/transition.rs`), not a UI-layer one; bindings are thin effect translators.
> **Replaces / supersedes**: the pre-2026-05-16 "nailed segments are literal document text; composing buffer = pending tail only" model (clarification β). Under **Model B** the whole composition (nailed segments + pending tail) lives in **one** marked / composing region until a hard finalize; nailed segments are **not** in the host document. This is the mainstream-IME-unanimous model (librime / khiin-rs / MOE / azooKey — see §10.1.1) and it eliminates the iOS mid-commit tail-leak (former Bug 3) by construction rather than by compensation.

> **Provenance**: extracted 2026-05-26 from [`continuous-input-ranking.md`](continuous-input-ranking.md) §10 as part of the P3 doc-size split. Section numbering (`§10.1`, `§10.2`, …) is **preserved** so that existing references in code comments, other docs, and historical archives continue to resolve. `§1–§9` references inside this file point at the parent doc's sections of the same number.

### 10.1 Motivation

§1–§9 (parent doc) address **which candidates appear and in what order**. They do not specify **what the user sees being typed** versus **what gets committed** when the user presses Enter or taps a candidate. Two dogfood findings (2026-05-11) converged on the same root cause — an unspecified split between the composing buffer and candidate slot 0:

1. The position-0 candidate cell carries a dashed border that visually conflates it with the composing buffer.
2. The lexicon path and the continuous path each build candidates independently; the lexicon path emits `subtitle=nil`, producing inconsistent slot-0 rendering when both paths fire.

This section establishes a single normative contract for **display split + commit dispatch** in Continuous mode.

### 10.1.1 Mainstream IME source — Model B is unanimous

The contract in §10.2–§10.4 is **Model B**: during continuous input the whole composition — already-**nailed** segments **plus** the pending raw tail — stays inside **one** preedit / marked / composing region until a hard finalize (Enter / final-commit / external-suggestion commit). Nailed segments are **never** written to the host document mid-composition; a candidate tap *nails* a segment inside the composition, and only a hard finalize writes literal text. This is the **unanimous** behavior of every mainstream IME surveyed (research 2026-05-16) — no surveyed IME does immediate partial literal commit mid-composition:

| IME | Evidence (file:line) | Mechanism |
|---|---|---|
| librime / RIME | `references/librime/src/rime/context.h:48-61`, `composition.cc:24-119` | One `Context::GetPreedit()` over the whole `Composition`; selecting a candidate only raises segment `status >= kSelected`; literal text leaves only via `GetCommitText()` on hard `Commit()`. |
| khiin-rs | `references/khiin-rs/swift/osx/src/controller/InputController.swift:103-275`, `IMKTextInputExtensions.swift:14-33` | One `client.mark(currentDisplayText())` over all segments; `client.insert()` only at hard commit; **re-marks the remainder** after a partial commit rather than leaving a bare tail. |
| MOE Tâi-gí | `decompiled/.../KeySectionsModel.java` (via `docs/references/moe-taigi-reference.md:68-124`) | `KeySectionsModel { composedCharacters; composingCharacters }` — **both** fields live inside the Android composing region (`setComposingText`); `composedCharacters` = nailed text, `composingCharacters` = pending raw. Literal text reaches the document only via the explicit `PopFront` / `GetConfirmedCandidates` commit API. |
| azooKey | `references/azooKey-Desktop/Core/Sources/Core/InputUtils/SegmentsManager.swift:1009-1062`, `azooKeyMacInputController.swift:753-792` | One `setMarkedText` whose attributed string carries `.focused` (nailed prefix) + `.unfocused` (pending tail) runs; `insertText` only at hard finalize. |

**MOE re-interpretation (corrects the pre-2026-05-16 reading).** The earlier spec read MOE's `composedCharacters` as text already committed to the document and treated the composing buffer as the pending tail only. The decompile evidence is the opposite: `composedCharacters` + `composingCharacters` **both** sit in MOE's composing region (`setComposingText`), exactly Model B. The `CandidateModel.spanUnits` per-candidate segmentation still maps to slot-0's "segmented version" (§10.2, unchanged). Adopting Model B therefore strengthens — not weakens — alignment with the de-facto Taigi baseline.

**The iOS / macOS technique (the crux).** azooKey (iOS/macOS) and every RIME front-end keep the whole composition in a **single** `setMarkedText(_, selectedRange:)` call; nailed-vs-pending is encoded as attribute runs *within* that one string. The host never confirms a sub-region into literal text because the document is never told a sub-region is final. Our former approach — `insertText(nailed_prefix)` then `setMarkedText(tail)` per mid-commit — is the inverse and has **zero precedent** in any surveyed IME; it is the root cause of the iOS tail-leak (former Bug 3, §10.6).

**Why this is the right call:**

- **Convention familiarity** — matches the IME many Taiwanese users already know (MOE) and the cross-IME consensus; satisfies CLAUDE.md rule 16 (mainstream-comparison-driven design) with a four-IME "Project X already does Y" cite.
- **Decouples "what I typed" from "what the engine guessed"** — preserves user agency under uncertain segmentation (`.claude/rules/cross-platform-alignment.md`; G3 in [parent §7](continuous-input-ranking.md#7-long-term-goals--align-with-mainstream-ime)).
- **Eliminates the iOS leak by construction** — nothing is literal-committed mid-flow, so there is no sub-region for the host to confirm; the iOS arm/detect/compensate workaround is deleted, not extended.
- **Maps onto the existing engine** — no proto / wire change; `Phase::Continuous` already retains every nailed segment's `display_text`, so the engine simply emits one combined `UpdatePreedit` instead of an eager per-segment `CommitTextReplacingPreedit`.

### 10.1.2 Supersedes — slot-0 model unification (Codex co-review clarification α, 2026-05-13)

Prior to §10, [`continuous-candidate-display.md`](continuous-candidate-display.md) modeled slot 0 as a **dedicated composing-text cell** with `isComposingText="true"` metadata — visually distinct (dashed border, rounded background, vertical inset) and **separate from ranker-produced candidates**. Specifically:

- `continuous-candidate-display.md` §4.6 "Slot-0 stays single-line. Pending preedit has no hanji. **This is intentional**."
- `continuous-candidate-display.md` §15.4 "`buildContinuousSuggestions` already always insert slot-0 composing-text cell — so the 'no candidates' UX is automatically preserved (strip shows slot-0 only)."

**§10 supersedes both for Continuous mode**: slot 0 is the **engine ranker's top candidate rendered with segmentation** (§10.2). There is no separate composing-text cell in the strip; the only composing-text surface is the inline host-app pre-edit (`markedText` / `setComposingText`), which under **Model B** carries the **whole composition** — `Σ nailed[i].display_text` + the pending-tail derived form ([`Phase::composing_display`](../../engine/composing/src/api.rs)) — **not** `rawInput` (pending-tail only). `rawInput` ([`Phase::raw_input`](../../engine/composing/src/api.rs)) is demoted to an internal *component* of that surface.

| Aspect | Pre-§10 legacy model | §10 (current spec for Continuous mode) |
|---|---|---|
| Slot 0 content | Composing-text cell (raw, no hanji) | Engine ranker top candidate (segmented; may have hanji) |
| Slot 0 visual | Dashed border + rounded bg + vertical inset | Identical to slots 1..n (no visual distinction — maintainer call 2026-05-13) |
| Tap slot 0 | Commit raw | Commit `candidate[0].display_text` (segmented; see §10.3 + clarification γ) |
| `isComposingText` metadata | Drives visual affordance + click routing | Click-routing only (visual gone in commit `224a8aa3`) |

**Non-Continuous mode** (legacy lexicon path) retains the pre-§10 slot-0 model. §10.5 Mode Gating is the boundary. `continuous-candidate-display.md` §4.6 / §15.4 wording therefore remains accurate **for non-Continuous mode only** — both will carry inline "superseded by §10 in Continuous mode" notes after Item 1 (this section) ships.

Implementation status: visual unification shipped in `224a8aa3` (Item 14, slot-0 dashed border + Android inset/bg/corner removed). Click-routing semantics shipped in Item 4 — the Continuous path no longer inserts a composing-text cell at slot 0; `candidate[0]` is the engine ranker top and Tap-0 routes through `commitContinuous(candidate[0].display_text)` (clarification γ). Segmented content of slot 0 (visual dual-line roman/hanji rendering) lands in Items 5+6.

### 10.2 Display Contract

| Element | Content | Source |
|---|---|---|
| **Composing buffer** (inline pre-edit in the host app — iOS `markedText` / Android `InputConnection.setComposingText`) | **The whole composition** (Model B): `Σ nailed[i].display_text` joined with the pending-tail **`rawInput`** by the **§10.2 word-boundary separator** (see *Segmented-spacing contract* below — **v3.5.8 change**, was "no inter-segment spaces" through v3.5.7). The nailed prefix is the accepted candidates' `display_text` (already swap/TPS/both-scripts-formatted); the tail is the derived display of the pending raw — hyphen-delimited chunks NFC-normalized and tone-marked where convertible, unhyphenated input passed through verbatim. **No** engine syllabification of the tail. | [`Phase::composing_display`](../../engine/composing/src/api.rs) (= [`api::nailed_prefix`](../../engine/composing/src/api.rs) `Σ display_text` separator-joined + [`Phase::raw_input`](../../engine/composing/src/api.rs) → [`derived::derived_display`](../../engine/composing/src/derived.rs)) |
| **Candidate strip, index 0** | **Segmented version** — segmenter + ranker top candidate over the same raw input bytes, with word boundaries inserted by the segmenter (**roman line gets word-boundary spaces; hanji line rendered as-is** — see segmented-rendering rule below). Independent of the inline-preedit `rawInput` contract above; produced from dictionary records, not from the `Phase::raw_input` derived string. | Segmenter + ranker top candidate |
| **Candidate strip, index N ≥ 1** | As defined by [parent §1–§7](continuous-input-ranking.md) (existing ranker output). | Existing path; unchanged |

**Composing-buffer surface (Model B):** the host's single marked / composing region renders [`Phase::composing_display`](../../engine/composing/src/api.rs) = the nailed prefix (`Σ nailed[i].display_text`, verbatim — already formatted at nail time) followed by the pending-tail `rawInput` defined below. The nailed prefix is **not** in the host document; it is part of the marked region until a hard finalize (§10.3). The platform caret/`selectedRange` sits at the **end** of this combined string.

**Segmented-spacing contract (v3.5.8, `output_both_scripts` AppConfig field).** The Model B composing buffer joins adjacent `nailed[i].display_text` — and the nailed prefix ↔ pending tail — with a single ASCII space **iff the rendered script is roman-ish**: roman-first (`!is_translate_swapped`), or both-scripts (`hit (彼)`). Hanji-first (`is_translate_swapped` without `output_both_scripts`) and TPS render hanji/bopomofo as-is with **no** inter-segment space. Predicate (single-sourced in [`api::continuous_word_space`](../../engine/composing/src/api.rs), mirrored by the platform `appendAutoSpaceIfApplicable`): `!(effective_swapped && !output_both_scripts)` where `effective_swapped = is_translate_swapped || input_mode == "tps"`; the separator is additionally suppressed after a hyphen-continuation segment (`tai-`). This corrects the v3.5.7 dogfood bug `Hittui → HitTui` (expected `Hit tui` in roman-first). `is_translate_swapped` alone could not distinguish hanji-first (no space) from both-scripts (space) — both set it `true` — hence the dedicated `output_both_scripts` flag (Codex pre-impl 2026-05-18). The separator is a presentation/commit-render concern — **dictionary-informed** for known two-syllable compounds (see *Compound-hyphen* below) — and is **never** stored in `NailedSegment.display_text` (backspace-pop restores the editable tail from `NailedSegment.raw_text`, which must stay separator-free). It applies identically to the hard-finalize `CommitTextReplacingPreedit` document write (Model B finalizes the same combined string).

**Compound-hyphen (v3.5.8 §10.2 Option A, Codex pre+post sandwich 2026-05-18).** On the **manual single-syllable tap path** the user nails each syllable as an independent `NailedSegment{syllable_count==1}`, so two syllables that together are a dictionary compound (查某 → `tsa-bóo`) lost the internal hyphen and rendered space-joined (`tsa bóo`). Fix: when joining the nailed prefix and the script is roman-ish, an adjacent pair of single-syllable segments whose `canonical_text` concatenation is a **known 2-syllable dictionary word** is joined with an internal `-` instead of the word-boundary space. Existence is decided by [`lexicon::compound_hanji_exists`](../../engine/lexicon/src/continuous.rs) — exact `prefix_index.lookup_exact("hanzi:<a><b>")`, scanning **all** rowids (not the ranker), `true` iff some record has `syllable_count == 2` **and** `hanzi == Some(<a><b>)`. The `syllable_count == 2` gate is load-bearing (many two-CJK-codepoint entries are not two TL syllables, e.g. 先生 / 新婦 — existence alone would over-hyphenate). Single-sourced in [`api::nailed_prefix`](../../engine/composing/src/api.rs) (one `LexiconHandle::with_state` per join after a cheap roman-ish + ≥1 single-syllable-pair pre-gate; `Err`/no-dictionary → graceful pure space-join, byte-identical to pre-Option-A). Pairing is **left-to-right non-overlapping bigram**: a user-typed trailing `-` or an auto `-` blocks the next boundary, so 查某人 tapped as three single syllables stays `tsa-bóo lâng` (3+-syllable compounds only partially hyphenate — accepted scope) and `A-B-C` over-gluing is impossible. Pure-TAILO segments (no hanji → `canonical_text` is roman) do not match a `hanzi:` key and stay space-joined (accepted limitation). Engine-only: no proto / no platform / no `Intent` / no `NailedSegment` field — the hyphen is derived fresh on every render from the segments' own `canonical_text` + `syllable_count`, preserving the backspace-restore invariant.

**Per-segment case (v3.5.8, 2A).** A continuous candidate's presentation `roman` is cased to mirror the user's *raw input for that candidate's own byte span* (`Hittui` → `Hit` then `tui`), engine-side at candidate construction ([`dispatch::recase_roman`](../../engine/composing/src/dispatch.rs); span-local by `consumed_span`, slot-0 per-edge via `shadow_to_raw_end`). The legacy platform `SuggestionCaseTransformer` is **bypassed for continuous candidates** (its global-caps + typed-prefix model is invalid under Model B), so the engine is the single casing source. Canonical sidechannel (`display_text` → `user_frequency.db`/NextWord key) and `hanji` are untouched.

**Precise pending-tail `rawInput` definition** (the tail component of the surface above; option (c) of the three considered, amended 2026-05-13 to match actual engine behavior):

- Source: [`Phase::raw_input(&self, &AppConfig)`](../../engine/composing/src/api.rs) — delegates to [`derived::derived_display`](../../engine/composing/src/derived.rs) which runs the same POJ doubletap → tone-mark → nasal-case chain that builds `Preedit.display_text` today. Operates on `Phase::Continuous.raw` (the still-editable pending tail), not the nailed prefix.
- Transformations applied: tone-marker rendering, NFC normalization, POJ doubletap pre-processing, nasal-marker case adjustment.
- Transformations **NOT** applied: word-boundary inference (no spaces), candidate matching, ranking, **engine-driven syllable segmentation** (user-typed `-` is the only syllable boundary signal).

Rationale for (c) over (a) raw keystrokes / (b) syllabified-without-normalization: keystrokes (`goa2 ai3 li2`) are not human-readable in the host app; normalization is the minimum to make the composing buffer faithfully echo "what the user typed in displayable form" without inferring word groupings.

**Engine-driven auto-hyphenation is out of scope for v3.5.8** (Codex pre-impl consult 2026-05-13, Item 2). `Intent::AppendHyphen` (api.rs) is evidence the keyboard treats `-` as a user-typed character; `phonetics::api::to_tone_marks` splits on `-` but does not insert hyphens. If a user types `goa2ai3li2` with no hyphens, `rawInput` returns `goa2ai3li2` verbatim — no tone marks, no auto-segmentation. Adding syllabifier-driven hyphen insertion would be a follow-up enhancement (likely paired with §10.2 segmented dual-line rendering in Item 6) rather than part of the Item 2 contract. **Distinct from the *Compound-hyphen* contract above** (2026-05-18): that inserts a `-` between two already-**nailed** single-syllable segments whose dictionary identity is *known* (a `hanzi:` exact hit), which is not syllabifier-driven inference of the pending raw tail; the pending-tail `rawInput` rule here is unchanged.

**Scope of the `rawInput` rule** (Codex post-impl review 2026-05-13): the contract above governs **only the inline composing buffer** in the host app. Candidate strip rendering (slots 0..N) draws from dictionary records via the segmenter and ranker; the roman/hanji pair on a candidate cell is unaffected by whether the user typed hyphens in their raw input. See [`continuous-candidate-display.md`](continuous-candidate-display.md) §4 (dual-line carrier) and §15 (fallback retire) for the candidate-rendering path.

**Segmented version rendering rule (Codex co-review clarification γ, 2026-05-13)**

When `candidate[0]` is dual-line (HANT / MIXED — proto carries both `roman` and `hanji` per §4 of [`continuous-candidate-display.md`](continuous-candidate-display.md)):

| Line | Content | Word-boundary spaces? |
|---|---|---|
| Roman line (`CandidateMessage.roman`) | Segmented romanization | **YES** — visible spaces between word groups, e.g., `goa ai-li` |
| Hanji line (`CandidateMessage.hanji`) | Hanzi vocabulary tokens, **rendered as-is** | **NO** — e.g., `我愛你`, not `我 愛你` |

Rationale: Roman/Latin script tokenization conventionally uses spaces; hanji as a logographic script does not. Inserting visible spaces in the hanji line breaks convention and risks character-rendering oddities. Word segmentation is implicit in the hanji line through the candidate's `consumed_span` and `spanUnits` metadata, not visual whitespace.

When `candidate[0]` is single-line (TAILO — roman only, no hanji): roman line shows segmented romanization with word spaces; no hanji line exists.

### 10.3 Commit Contract

Under **Model B** there is exactly **one** literal document write per continuous session — at a hard finalize. Mid-composition candidate taps *nail* segments **inside** the marked region (no document write).

| Trigger | Effect |
|---|---|
| **Tap candidate, pending tail remains** (mid-commit / *nail*) | **No document write.** The candidate's swap/TPS/both-scripts-formatted string (exactly what the legacy lexicon path would commit — clarification γ) becomes the new segment's `NailedSegment.display_text` and is appended to the marked region; the canonical key (`hanji.unwrap_or(roman)`) is carried on `CommitContinuous.canonical_text` and fires `NextWordUpdateLastSelectedWord` (learning at nail time). The host sees one re-rendered `UpdatePreedit` of the **whole composition** — never `CommitTextReplacingPreedit`. |
| **Tap candidate, consumes the rest** (final-commit) | One `CommitTextReplacingPreedit` of the **whole composition** = `Σ nailed[i].display_text` (including this final segment), then exit. Single terminal `NextWordWordSelected(canonical, raw_text, true)` for the final segment (earlier nails already fired `NextWordUpdateLastSelectedWord`; not replayed). |
| **Enter** | One `CommitTextReplacingPreedit` of the **whole composition** = `Σ nailed[i].display_text` + the derived display of the pending raw tail (`Phase::composing_display`). This is exactly what the user saw inline — the entire marked region becomes literal text. Terminal `NextWordWordSelected` for the last "word": the pending tail when one exists, else the last nailed segment (canonical key). |

**Clarification β — REWRITTEN (Model B, 2026-05-16): Enter commits the whole composition, not the pending tail**

> The pre-2026-05-16 β said Enter commits only the pending tail because
> nailed segments were already literal document text. **Under Model B that
> premise is false**: nailed segments are inside the marked region, never in
> the document. Enter therefore commits `Σ nailed[i].display_text +
> derived(pending tail)` — the whole `Phase::composing_display`. "What you
> see is what you typed" still holds, because the marked region *is* the
> whole composition (§10.2). There is no `CommitRaw`-commits-literal-
> keystrokes gap anymore: `commit_raw_continuous` builds the combined string.

**Clarification γ — Tap formats the segment as the swap-aware string; `canonical_text` is the canonical key (v3.5.8 Phase 9 Bug 1; Model-B-adjusted)**

The roman-with-spaces rendering in slot 0 (§10.2 segmented rule) is **display-only**. On tap, the platform formats the candidate's `roman` / `hanji` through the **same swap/TPS/both-scripts formatter the legacy lexicon path uses**; that formatted string becomes the segment's `NailedSegment.display_text` and (Model B) joins the marked region immediately and the single combined `CommitTextReplacingPreedit` at hard finalize — so Continuous and lexicon produce identical document text for the same candidate under the same settings. The canonical key (`hanji.unwrap_or(roman)`) is sent **separately** on `CommitContinuous.canonical_text` and used only for the NextWord/frequency effects. This preserves:

- Frequency-recording keys (platform records on the canonical sidechannel — `ActionHandler.handleSuggestionSelection` continuous branch / `CandidateClickHandler.handleContinuousCandidateClick`)
- NextWord association keys (engine routes `canonical_text` to `NextWordWordSelected` / `NextWordUpdateLastSelectedWord`, including the backspace **unnail** correction — §10.7)
- Canonical word boundary of dictionary vocabulary tokens
- Mode-independent learning: frequency / NextWord do not fork by display mode

Empty `canonical_text` (legacy callers) falls back to `display_text` — unchanged.

Design intent:

- A tap = accept the engine's segmentation for that span; the segment is *nailed* into the composition (visible, editable via backspace-unnail) but **not** committed to the document until the user finishes.
- Enter / final-commit = the single hard finalize; the whole composition becomes literal text in one write. The underline disappears **only** here — matching every mainstream IME (§10.1.1) and the maintainer requirement.

**Commit side effects (Codex B1 item 4; Bug 1 2026-05-15; Model B 2026-05-16)** — frequency recording (`user_frequency.db`) and NextWord are **payload-orthogonal** to the document write and key off `canonical_text`. They fire **at nail time** (`NextWordUpdateLastSelectedWord` per nailed segment) and once at hard finalize (`NextWordWordSelected`), independent of when/whether literal text is written. No double-count: already-nailed segments are not replayed at final commit (Codex risk (i)).

### 10.4 Data-Flow Invariant

```
keystrokes
  → syllabifier (Rust)
    → Phase::Continuous { nailed[], raw }
        composing_display = Σ nailed[i].display_text + derived(raw)
          ───────────────────────────────► ONE marked / composing region
                                            ▲ hard finalize (Enter / final-
                                              commit / external) commits THIS
                                              whole string, once
      → segmenter + ranker (Rust)
        → candidates[]
            [0]   ─────────────────────────► strip slot 0 (display)  — tap nails / finalizes
            [N≥1] ─────────────────────────► strip slot N (display)  — tap nails / finalizes
```

Invariants (every edge case in §10.7 falls out from these — no per-case branching):

| ID | Invariant |
|---|---|
| **I1** | Composing-buffer content = `Phase::composing_display` = `Σ nailed[i].display_text` (verbatim, formatted at nail time) **followed by** the derived display of `Phase::Continuous.raw` (NFC + tone-mark across user-typed `-`; unhyphenated passes through verbatim; no engine syllabification of the tail). Nailed segments are part of this single marked region, **not** the host document. `raw` is the still-editable pending tail (bytes after the last nail), not the original keystroke history. Backspace, append, nail, and unnail all mutate `Phase::Continuous`; I1 re-establishes the whole combined string. |
| **I2** | `candidate[0]` display content = ranker top output rendered with segmentation. (Unchanged.) |
| **I3** | Any mutation of input (insert / backspace / nail / unnail) re-runs syllabifier → segmenter → ranker for the pending tail and re-renders the combined composition; nailed segments are preserved or unnailed per the §10.7 boundary rules. I1 and I2 re-establish automatically. |
| **I4** | Platform performs no re-ranking, no candidate-order rewriting, no exact-match injection — preserves G3 ([parent §7.1](continuous-input-ranking.md#71-goal-axes)). Output-mode formatting of the *chosen* candidate at nail/commit time (swap / TPS / both-scripts → the segment's `display_text`, canonical key on `canonical_text`) is **not** a violation — it formats one chosen candidate exactly as the legacy lexicon path does (clarification γ). **Additionally (Model B): a platform MUST NOT split a mid-composition candidate tap into `commitText(prefix)` / `insertText(prefix)` + a new preedit.** A nailed prefix is rendered inside the single marked region via one `UpdatePreedit`; literal document text is written only by the engine's single hard-finalize `CommitTextReplacingPreedit`. This is the invariant whose violation caused the iOS tail-leak (§10.6). |

### 10.5 Mode Gating

| Mode | Display split active? | Slot-0 = composing? |
|---|---|---|
| Continuous input (Phase 9 path, `fetch_via_continuous`) | **YES** | No — slot 0 differs from composing whenever ≥ 2 syllables |
| Non-Continuous lexicon (legacy `fetch_via_lexicon`) | **NO** | Yes — current behavior preserved |

The split is bound to the engine-side dispatch branch in [`engine/composing/src/dispatch.rs`](../../engine/composing/src/dispatch.rs) — there is no separate platform-side toggle.

### 10.6 Cross-Platform Touch Points

| Layer | iOS | Android |
|---|---|---|
| Composing-buffer write | `KeyboardInputViewController` → `markedText` | `InputConnection.setComposingText` |
| Slot-0 render | `CandidateButtonView.swift` — **remove dashed border** | `CandidatesView` / `CandidateButtonView.kt` — **remove `composingDashedBorder`** |
| Enter commit dispatch | `ActionHandler+Suggestions.swift` | `CandidateClickHandler.kt` (or IME keyboard view) |
| Tap-0 / Tap-N dispatch | `ActionHandler+Suggestions.swift` | `CandidateClickHandler.kt` |
| Swap/TPS/both-scripts segment formatting (γ, Bug 1) | `parseRomanAndHanzi` + `formatOutputText` (shared with legacy branch) → `commitContinuous(displayText:canonicalText:)` | legacy `bracketRoman` + when-expr → `commitContinuous(displayText, canonicalText, …)` |
| Canonical key wire | `CommitContinuous.canonical_text` (= sidechannel `displayText`) | `CommitContinuous.canonical_text` (= `TaigiWord.MetadataKeys.DISPLAY_TEXT`) |
| Mid-commit (nail) | One `UpdatePreedit(whole composition)` → `setMarkedText(combined, caret=end)`. **No `insertText`.** | One `UpdatePreedit(whole composition)` → `setComposingText(combined, 1)`. **No `commitText`.** |
| Hard finalize (Enter / final-commit / external) | One `CommitTextReplacingPreedit(whole composition)` → `clearMarkedText()` + `insertText` | One `CommitTextReplacingPreedit(whole composition)` → `commitText(combined, 1)` |

`.claude/rules/cross-platform-alignment.md` §3a applies: I1–I4 hold identically on both platforms.

**Convergence — Model B removes the former Bug-3 divergence (2026-05-16).** Under the pre-2026-05-16 model a mid-commit emitted `commit_text_replacing_preedit(segment)` + `update_preedit(tail)`; iOS hosts confirmed the small re-marked tail into literal text during their `textWillChange→textDidChange` settle (real-device trace), losing the underline mid-composition. iOS carried an arm/detect/compensate workaround that hit a 3-strike circuit-breaker. **Model B eliminates this by construction**: a mid-commit emits **no** `CommitTextReplacingPreedit` — only one `UpdatePreedit` of the whole composition — so there is no sub-region for the host to confirm. The iOS `armContinuousMidCommitTail` / `detectContinuousMidCommitTailLeak` / `compensateLeakedContinuousMidCommitTail` layer is **deleted** (P2), not kept. There is no longer any platform-specific divergence here; both platforms run the identical effect sequence and `behavioral-invariants.md` needs no entry.

**External-composing-region clear (field/app switch, host clears mid-composition).** Policy (single, documented per Codex risk (iii)): treat it as a **hard abort** — drop all nailed + pending, exit Idle, no document write (mirrors `reset_continuous`). Not a compensation attempt. iOS field-switch path + Android `onUpdateSelection → onExternalComposingRegionCleared` both route to this abort.

**Engine-side coupling — resolved**: the `subtitle=nil` symptom was closed by fix-plan Item 5/6 (`CandidateMessage` `roman` + optional `hanji`; dual-line render). v3.5.8 Phase 9 Bug 1 added `CommitContinuous.canonical_text` so the segment's formatted display and the canonical freq/NextWord key are decoupled on the wire. §10.2/10.3/10.4 hold under Model B with no proto change.

### 10.7 Edge Cases (fall out from §10.4 invariants)

| Case | Behavior | Why |
|---|---|---|
| Single syllable | composing == `candidate[0]` | Segmenter inserts zero word boundaries → identity |
| Continuous disabled | Split inactive | §10.5 mode gating |
| Backspace, pending tail non-empty | Last char of pending tail drops; the combined composition (`Σ nailed.display_text` + shrunk tail) re-renders; `candidate[0]` recomputes. **No** `DeleteBackwardFromDocument` (nothing is in the document). | I3 |
| Backspace, pending tail empty, ≥1 nailed (**unnail**) | The last nailed segment is popped; its `raw_text` becomes the new pending tail; combined composition re-renders. NextWord last-selected rolls back to the prior nailed segment (or clears). **No** `DeleteBackwardFromDocument` — authority is `raw_text`, never a display-char count (swap/TPS/both-scripts display can desync from raw). | I3 + Codex risk (v) |
| Empty buffer | No composing display, no candidates | Pre-Phase-9 behavior; unchanged |
| Syllabifier partial-parse failure | `rawInput` shows partial parse + raw tail; candidates may be empty | Existing syllabifier error path; this section does not modify it |
| **Partial prefix below first syllable ending** (e.g., `raw = "gu"`, no completed syllable) | Composing buffer shows `gu` literally. Slot 0 is **not** the "segmented version" rule:<br/>• **Pre-§15.3.D state** (current code, before fix-plan item 10): strip is empty.<br/>• **Post-§15.3.D state** (after fix-plan item 10 lands): strip shows engine-prefix candidates per `continuous-candidate-display.md` §15.3.D, ranked below full-syllable candidates via `coverage_kind` (§15.5).<br/>In both states, Enter commits literal `gu`. | Segmenter has no word boundaries to insert when `raw` is sub-syllable; the "candidate[0] = segmented version" rule is **undefined** below first valid syllable ending. Codex co-review clarification (B3, 2026-05-13). |
| **Enter after segments already nailed** | Commits the **whole composition** = `Σ nailed[i].display_text` + derived(pending tail), in one `CommitTextReplacingPreedit`. The marked region (which *was* the whole composition) becomes literal text and the underline clears. | Clarification β REWRITTEN (Model B): nailed segments were in the marked region, not the document. |
| **Abort / reset mid-composition** | The whole marked region is cleared and all nailed segments dropped; nothing reaches the document (mirrors `reset_continuous`). Not "keep nailed, drop pending". | Codex risk (ii): nailed were never in the document. |
| **Hanji accidentally in composing buffer** | Composing shows the hanji literals; engine returns no candidates (per §15.3.E of `continuous-candidate-display.md`); Enter commits the hanji literals (as the pending tail of the combined string). | Engine hanzi guard short-circuits to empty `ContinuousResponse`; composing surface unaffected. Fix-plan item 11. |

### 10.8 Regression Test Hooks

Minimum coverage to declare §10 closed (Model B acceptance criterion):

1. **Single syllable** — type `goa` → assert composing == `candidate[0].display_text`.
2. **Three-syllable two-word** — `candidate[0]` roman line shows word-boundary spaces, hanji line shows the hanzi **without** added spaces (clarification γ; unchanged).
3. **Tone-marker spanning syllables** — NFC + tone rendering apply to the pending-tail component without affecting segmentation.
4. **Mid-commit emits no document write** — type ≥2 syllables, tap `candidate[0]` for a prefix (pending tail remains) → assert effects are `[UpdatePreedit(combined), NextWordUpdateLastSelectedWord, PerformAutocomplete]`; assert **no `CommitTextReplacingPreedit`**; assert composing buffer == `Σ nailed.display_text + derived(pending)` (still one marked region / underlined).
5. **Primary Bug-3 acceptance** — type `taiuantaigi`, tap 「臺灣」 (nails `taiuan`, `taigi` pending) → composing shows `臺灣` + derived(`taigi`), **still underlined**, no `CommitTextReplacingPreedit`. Then Enter → one `CommitTextReplacingPreedit("臺灣" + derived("taigi"))`, exit Idle, underline clears **only here**. (Never commits just `taigi`.)
6. **Final-commit commits whole composition** — nail "tsu"→珠 (pending "a"), then tap final candidate consuming "a"→仔 → one `CommitTextReplacingPreedit("珠仔")` (not "仔"), exit; single terminal `NextWordWordSelected`.
7. **Backspace never writes/deletes document** — across pending-shrink, unnail (pop), and empty-out: assert **no `DeleteBackwardFromDocument`** is ever emitted in Continuous; unnail restores the popped segment's `raw_text` as pending and re-renders combined.
8. **Abort / reset clears whole region** — with ≥1 nailed segment, `Reset` / `ResetContinuous` → `[ClearPreeditWithoutCommit, …]`, exit Idle, **no `CommitTextReplacingPreedit`**, all nailed dropped.
9. **`select_suggestion` / `commit_preedit_then_insert_external` under Continuous** — committed string = `Σ nailed.display_text` + (text | derived(raw)+external); with zero nailed, unchanged from legacy.
10. **No-nailed parity** — pure Composing→EnterContinuous→Enter (no tap) commits `derived(raw)` exactly as before (combined == derived(raw) when nailed empty); translate-swapped / TPS pass-through unchanged.
11. **Hanji line no added spaces** — HANT/MIXED multi-word `candidate[0]`: roman line has word spaces, hanji line is the exact dictionary hanzi (clarification γ; unchanged).

Cross-platform: every case must pass identically on iOS and Android per `.claude/rules/cross-platform-alignment.md` — including the I4 "no `commitText(prefix)` / `insertText(prefix)` mid-composition" assertion.

### 10.9 Relationship to §1–§9 (parent doc)

| § (parent) | Connection |
|---|---|
| §1–§3 (ranking gap) | Orthogonal. Ranking determines candidate **order**; §10 determines **display and commit**. Both ship in Phase 9. |
| §4 (Codex co-confirm) | Separate co-confirm pass required for §10 after quota recovery. |
| §5 (mainstream IME) | §10.2–§10.4 is **Model B**, the unanimous pattern across librime / khiin-rs / MOE / azooKey — see §10.1.1 for the four-IME cite-and-trace. Our former eager partial-literal-commit had zero precedent and caused the iOS leak; Model B closes that parity gap and the bug together. |
| §6 (architectural classification) | §10 adds no new Gap; it is a normative UI/IME contract. |
| §7 (long-term goals) | Reinforces G3 (engine is ranking authority). Compatible with G1/G2/G4 trajectory. |
| §8 (v3.5.8 decision) | §10 is part of the expanded Phase 9 scope per the 2026-05-11 revision. |
| §9 (open questions) | Adds implicit Q8: should §10's display split extend to non-Continuous mode if Continuous becomes default in a future release? Currently NO per §10.5; revisit when default flips. |

### 10.10 Codex Co-Review Log (2026-05-13)

Codex spec co-review pass against §10 + [`continuous-candidate-display.md`](continuous-candidate-display.md) (transcript not retained). Outputs synthesized into §10 inline:

| Codex finding | Landed in |
|---|---|
| A1: §10.2/10.3/10.4 directionally consistent; no-candidate boundary unpinned | §10.7 partial-prefix row + hanji-in-buffer row |
| A2/B2: §10 vs §4.6/§15.4 cross-spec contradiction on slot-0 model | §10.1.2 Supersedes notice + reciprocal notes in `continuous-candidate-display.md` |
| A3: segmentation target ambiguous between `display_text` / roman / hanji | §10.2 segmented-rendering rule + §10.3 clarification γ |
| A4 + B3: Enter-after-nail / partial-prefix semantics | §10.3 clarification β + §10.7 new rows |
| C1: MOE Enter evidence weaker than spec claims | §10.1.1 clarification δ |
| D fix plan (14 items, ordered) | **ALL 14 items shipped.** Items 1–6 + 14 = spec foundation + dual-line carrier; Items 7–12 closed every engine syllabification gap; **Item 13 (v3.5.8 capstone) retired the platform lexicon fallback** so the Continuous engine is the single candidate source. v3.5.8 feature-complete after Item 13. |

**Clarifications recorded inline** (β/δ superseded by the Model B rewrite, 2026-05-16 — see §10.10a):
- **α (slot-0 supersedes)** → §10.1.2 (still valid)
- **β** — REWRITTEN: Enter commits the **whole composition** (`Phase::composing_display`), not the pending tail → §10.3 + §10.4 I1 + §10.7
- **γ (tap formats the segment as the swap-aware string; `canonical_text` is the canonical key, `display_text` is the formatted marked-region/document string)** → §10.2 + §10.3 (Model-B-adjusted: the formatted string is the segment's display inside the marked region, joined into the single hard-finalize commit; the canonical key rides `CommitContinuous.canonical_text`)
- **δ** — SUPERSEDED: the "MOE Enter→raw" black-box claim is no longer load-bearing; Model B rests on the four-IME source cite-and-trace (§10.1.1), not on MOE Enter behavior

### 10.10a Model B rewrite (2026-05-16, Bug 3 closeout)

Codex design co-review PASS (auto-mode batched consult). The pre-2026-05-16 model (nailed = literal document text; composing buffer = pending tail; mid-commit emits `CommitTextReplacingPreedit`) was the root cause of the iOS mid-commit tail-leak (former Bug 3) and had no mainstream precedent. Model B (whole composition in one marked region; single literal write at hard finalize) is mainstream-unanimous and removes the bug by construction. Codex-surfaced design points landed: (i) per-segment learning fires at nail, single terminal `NextWordWordSelected`, no replay; (ii) `reset_continuous` clears the whole region + drops all nailed; (iii) external-region-clear = single hard-abort policy (§10.6); (iv) caret at end of combined marked text; (v) backspace unnails via `raw_text`, never display-char count, never `DeleteBackwardFromDocument`; (vi) Model B eases the future lattice/walker (§ "整句 lattice + walker" — orthogonal). Engine landed in P1 (`engine/composing/`); iOS converges + deletes the Bug-3 workaround in P2; Android converges in P3. The former §10.6 platform divergence is **deleted** (convergence).

Fix-plan ordering **(historical log — the Item 3 Enter/commit semantics below are SUPERSEDED by §10.10a Model B 2026-05-16: `Intent::CommitRaw` in `Phase::Continuous` now commits `Phase::composing_display` (the whole composition), NOT `derived_display(pending)`; mid-commit emits no `CommitTextReplacingPreedit`)**: item 14 (visual unification) shipped 2026-05-13. Item 1 (doc reconcile) + Item 2 (`Phase::raw_input` accessor + invariant tests + §10.2 amendment for actual engine behavior) shipped on branch `v358-continuous-display-spec`. Item 3 (`Intent::CommitRaw` in `Phase::Continuous` commits `derived_display(pending)`, mid-commit preserves nailed segments) shipped 2026-05-14 **(Enter/commit semantics later superseded by Model B — see §10.10a)**. Item 4 (Tap-0/Tap-N commit semantics — slot-0 cell removed from Continuous path, strict-required `displayText` sidechannel) shipped 2026-05-14. Item 5 (`CandidateMessage` `roman` + `hanji` proto fields + bridge decode + defensive `roman.isEmpty → displayText` fallback) shipped 2026-05-14. Item 6 (pin segmented dual-line rendering at platform UI — iOS `buildContinuousSuggestions` + Android `buildContinuousSuggestionsForCandidates` swap `text/title` ← `c.roman`, `subtitle/hanzi` ← `c.hanji`, present-empty hanji collapses to nil) shipped in PR #271. Multi-word chain segmentation (§10.7 #2 case) is **out of scope** — each `RawCandidate` today maps to exactly one dictionary record, so chain segmentation requires a future lattice/chain-producer slice. Items 7–12 (engine syllabification gaps) shipped PR #272–#278; **Item 13 (capstone) retired the platform lexicon fallback** (`continuous-candidate-display.md` §15.4) — v3.5.8 feature-complete.

`(roman, hanji)` ranker dedupe is **deferred to Item 12** (custom-dict integration) — the first slice that can emit cross-source duplicates. Today the default `dict.bin` builder collapses duplicates via `dictionary/build/merge_csv.py`'s `groupby(["hanzi", "_tl_key"])`, so no realistic input surfaces a duplicate `(roman, hanji)` pair into the Continuous ranker. Locking the winner policy (lowest `source_tier_rank` vs SortKey winner) without real custom-dict plumb context would be premature; Item 12 picks it then.

Durable re-grounding of the fix plan in a future session: re-run a Codex ANALYSIS-ONLY consult against the latest spec. Plan does not need to live in a separate doc — the spec itself now carries enough structure for an implementer to plan from.

---

## Cross-references

- Parent doc (ranking gap §1–§9 + cross-references §11): [`continuous-input-ranking.md`](continuous-input-ranking.md).
- Candidate-strip rendering + dual-line carrier: [`continuous-candidate-display.md`](continuous-candidate-display.md).
- Cross-platform parity invariants: [`../architecture/behavioral-invariants.md`](../architecture/behavioral-invariants.md).
- Engine source: [`engine/composing/src/api.rs`](../../engine/composing/src/api.rs), [`engine/composing/src/transition.rs`](../../engine/composing/src/transition.rs), [`engine/composing/src/dispatch.rs`](../../engine/composing/src/dispatch.rs).
