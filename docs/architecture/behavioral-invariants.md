# Phase 0 — Behavioral Invariants

**Status**: authored 2026-04-19 as the Phase 0 gate before the iOS exemplar Phase I plan (`ios-exemplar-plan.md`) begins. Required by Codex strategic review finding C2 to keep the shared-core contract neutral instead of Swift/KeyboardKit-shaped.

**Purpose**: enumerate the cross-platform behaviors the shared-core candidates (36 at authoring; roster grew to 43 after iOS G4-impl PR #138 (2026-04-19) + G5-impl PR #137 (2026-04-19) — see `docs/engine/shared-core-readiness.md`) must uphold on both iOS and Android. Refactors inside Phase I must preserve every invariant in this document. Phase II (Android alignment), Phase III (≥95% + FFI POC), and Phase IV-A (Rust phonetics slice) treat these as the immutable contract.

**Scope boundary**: this doc captures *behavior* only. Architecture purity (DI, ObservableObject, singletons) lives in `shared-core-readiness.md`; data-artifact portability (MARISA / SQLite / `dictionary.bin`) lives in the G10 deliverable.

**Test references**: each invariant ends with one or more `INVARIANT_*` test-case labels. Those labels are placeholders for G9 to wire into `ios/TaigiKeyboardTests` (and the mirror Android suite). An unwired label is allowed in Phase 0; an invariant without a label is not.

**Drift policy**: if iOS and Android behavior diverge on any invariant, treat the divergence as a regression — open an issue, do not adjust this doc to match the code.

---

## Index

1. [Phonetics — TL ↔ POJ round-trip](#1-phonetics--tl--poj-round-trip)
2. [Unicode — NFD preprocessing](#2-unicode--nfd-preprocessing)
3. [TPS — TL ↔ TPS round-trip](#3-tps--tl--tps-round-trip)
4. [Input normalization — mode-agnostic numeric tones](#4-input-normalization--mode-agnostic-numeric-tones)
5. [Candidate dedup — engine vs display tiers](#5-candidate-dedup--engine-vs-display-tiers)
6. [Candidate scoring — determinism + ordering](#6-candidate-scoring--determinism--ordering)
7. [Next-word decay — RIME-style half-life](#7-next-word-decay--rime-style-half-life)
8. [Next-word weighting — user > dict](#8-next-word-weighting--user--dict)
9. [Case transformation](#9-case-transformation)
10. [Custom-dictionary search-key derivation](#10-custom-dictionary-search-key-derivation)
11. [Settings read semantics](#11-settings-read-semantics)
12. [Logger backend neutrality](#12-logger-backend-neutrality)
13. [Composing-buffer reset semantics](#13-composing-buffer-reset-semantics)

---

## 1. Phonetics — TL ↔ POJ round-trip

**Invariant**: for every valid Taigi syllable `s`, converting display form TL → POJ → TL (and POJ → TL → POJ) must yield `s` exactly, including tone marks, combining scalars, and the `o͘` marker. Conversion is **lossless** on valid input.

**Why**: the engine freely rewrites the same syllable across both scripts for UI display and dictionary lookup. Any lossy step silently rewrites user data.

**Scope**: `Phonetics/Parser/SyllableParser.swift`, `Phonetics/Formatter/{TLFormatter,POJFormatter}.swift`, `Phonetics/Converter/PhoneticsConverter.swift`, `Phonetics/Converter/RomanizationConverter.swift`.

**Corner cases that must hold**:
- Tone 1 and tone 4 with no trailing digit round-trip (numeric tone preserved only during composition, not in display forms).
- `oo` ↔ `o͘` (U+0358 combining) both round-trip identically.
- Nasalization `ⁿ` (U+207F), `ᴺ` (U+1D3A), and `nn` are treated as the same nasal marker on parse; emit form is fixed per script (TL = `nn`, POJ = `ⁿ`).
- Leading-case restoration: `Gua2` → `Guá` → `Goá` → `Guá`.

**Test labels**:
- `INVARIANT_tl_to_poj_roundtrip_is_lossless`
- `INVARIANT_poj_to_tl_roundtrip_is_lossless`
- `INVARIANT_oo_combining_form_roundtrips`
- `INVARIANT_nasal_marker_variants_collapse_on_parse`

---

## 2. Unicode — NFD preprocessing

**Invariant**: `TaigiUnicode.nfdPreprocessed(s)` produces the same byte sequence on iOS (Swift `decomposedStringWithCanonicalMapping`) and Android (Kotlin `Normalizer.Form.NFD`) for every input that the keyboard may see — POJ nasal marker substitution, NFD decomposition, and `o͘` collapse to `o`.

**Why**: downstream consumers (`CandidateProcessor.romanToBase`, `InputNormalizer`, `CustomDictionaryDerivation`) depend on the output being identical across platforms. A divergent one-character preprocessing bug silently changes every dedup key, every score calculation, and every trie lookup.

**Scope**: `Lexicon/Utils/TaigiUnicode.swift` + Android mirror.

**Corner cases**:
- Combining tone marks remain decomposed after the function returns; caller strips them.
- `o͘` (U+0358 attached) → `"o"` regardless of whether input arrived pre-composed or pre-decomposed.
- Does not normalize to NFC afterwards; consumers must not assume the output is composed.

**Test labels**:
- `INVARIANT_nfd_preprocessed_platform_parity`
- `INVARIANT_poj_nasal_to_nn_substitution`
- `INVARIANT_o_dot_combining_collapse`

---

## 3. TPS — TL ↔ TPS round-trip

**Invariant**: for every supported TPS syllable sequence (MOE2 layout), `TLToTPS(TPSToTL(x)) == x` and `TPSToTL(TLToTPS(y)) == y`, modulo the documented composition-order adjustments in `TPSInputAdjuster`.

**Why**: TPS mode shares the same candidate/scoring pipeline as POJ/TL via `InputNormalizer`. Divergent TPS conversion between platforms means the same keystroke shows different candidates.

**Scope**: `Input/TPS/{TPSTables,TPSConverter,TPSInputAdjuster,TPSToTL,TLToTPS}.swift`.

**Corner cases**:
- `TPSInputAdjuster` reorders initial/medial/final before conversion; round-trip must account for adjuster output, not raw keystrokes.
- Tone digits on TPS syllables pass through unchanged.
- `containsTPS` must return `false` for any pure Roman input (no false positives on ASCII-only strings).

**Test labels**:
- `INVARIANT_tps_to_tl_roundtrip`
- `INVARIANT_tl_to_tps_roundtrip`
- `INVARIANT_tps_adjuster_is_idempotent`

---

## 4. Input normalization — mode-agnostic numeric tones

**Invariant**: `InputNormalizer.normalize(rawInput, inputMode:)` always emits a `notone`- or `roman_num`-style key (lowercase, hyphens collapsed, diacritics stripped, tones expressed as trailing digits) regardless of the user's input mode (`.poj`, `.tl`, `.tps`, `.english`). The output is the primary MARISA trie lookup key.

**Why**: the trie is stored in a single canonical form; any mode-dependent drift in the key changes which candidates appear.

**Scope**: `Lexicon/Trie/InputNormalizer.swift` + `Input/TPS/TPSToTL.swift` for the TPS path.

**Corner cases**:
- POJ-specific tone marks (`á`, `ê`, `ō`, etc.) normalize to trailing digits.
- Explicit user hyphens are preserved in the `roman_num` variant and stripped in the `notone` variant — two keys for two indexes.
- Empty input returns empty output (never a sentinel character).

**Test labels**:
- `INVARIANT_input_normalizer_is_mode_agnostic`
- `INVARIANT_normalizer_preserves_hyphens_in_roman_num`
- `INVARIANT_normalizer_strips_hyphens_in_notone`

---

## 5. Candidate dedup — engine vs display tiers

**Invariant**: two distinct dedup passes exist and must run in the documented order:

1. **`removeDuplicates(_:)`** runs *before* scoring. Key = `"\(roman)|\(hanzi ?? "")"`. Removes true duplicates.
2. **`removeDisplayDuplicates(_:)`** runs *after* sorting, only in TPS mode. Key = `hanzi`. Keeps the highest-ranked entry per hanzi (entries without hanzi are always kept).

Reversing or merging these two passes changes ordering. Running display dedup before sort drops higher-ranked entries.

**Why**: the keyboard shows TPS symbols to the left of candidates; visually identical hanzi with different roman forms is confusing, but the ranked winner must be retained.

**Scope**: `Lexicon/Utils/CandidateProcessor.swift`.

**Test labels**:
- `INVARIANT_engine_dedup_keys_on_roman_plus_hanzi`
- `INVARIANT_display_dedup_runs_after_sort`
- `INVARIANT_display_dedup_keeps_words_without_hanzi`

---

## 6. Candidate scoring — determinism + ordering

**Invariant**: `CandidateProcessor.calculateScore` is **pure** — same inputs yield the same `ScoreBreakdown`. Given identical `(word, normalizedInput, frequencyData, currentTime)`, iOS and Android return the same total.

Further, the priority ordering is fixed:

```
userFreqScore  (0 … +10000)     dominates
completionPenalty (0 or -1000)   separates exact vs completion tiers
closenessBonus (0 … +500)        orders among completions
recencyBonus   (0 or +200)       tiebreaker within same freq
exactBonus     (0 or +100)       tiebreaker
baseFreqScore  (~0 … +100)       fallback
```

**Why**: the scoring formula is the user-visible ordering of every candidate. Drift means the keyboard ranks differently on iOS vs Android for the same word + same user state.

**Scope**: `Lexicon/Utils/CandidateProcessor.swift` — `calculateScore`, `romanToBase`, `inputToBase`, `sortByScore`.

**Corner cases**:
- `currentTime` is injected at the call site (ms since epoch). No call inside the engine reads the clock.
- `cappedUserFreq = min(count, 100)` — per-word count saturates at 100; never uncapped.
- Recency window is exactly 1 hour (`60 * 60 * 1000` ms); boundary condition `(currentTime - lastUsedMillis) < oneHourMillis` is strict `<`.
- `romanToBase` must strip hyphens, spaces, NFD combining marks, and digits in that order.
- Sort is stable on ties in the sense that the original array order is preserved when `total` ties (Swift `sorted(by:)` is not guaranteed stable — documented weakness; ordering fallback currently relies on the pre-sort input order).

**Test labels**:
- `INVARIANT_score_is_deterministic`
- `INVARIANT_user_freq_dominates_ranking`
- `INVARIANT_completion_penalty_separates_tiers`
- `INVARIANT_recency_window_is_exactly_1_hour`
- `INVARIANT_roman_to_base_strips_tones_hyphens_digits`

---

## 7. Next-word decay — RIME-style half-life

**Invariant**: `NextWordScorer.calculateDecay(lastUsedMs:nowMs:)` computes `exp(-ageHours / 168.0 * ln(2))` — one-week half-life, RIME-derived. Constants are fixed:

| Constant | Value |
|---|---|
| `decayHalfLifeHours` | 168.0 |
| `highUsageDecayFloor` | 0.95 |
| `lowUsageDecayFloor` | 0.3 |
| `highUsageThreshold` | 3 |

**Why**: decay shapes the entire learning curve for user associations. A change here is not caught by scoring tests — it surfaces only after days of use.

**Scope**: `NextWord/NextWordScorer.swift` + Android `NextWordService.kt`.

**Corner cases**:
- `nowMs - lastUsedMs` can be negative if the user rewinds the clock; the formula produces a factor > 1. Callers must not rely on decay ≤ 1.
- `ln(2)` is hard-coded as `0.693` — precision loss vs full `log(2.0)` is acceptable (drift ≈ 0.03% at 1 week); constant must match Android.
- `calculateUserScore` clamps to `max(decayFloor, decay)` — high-usage entries (count ≥ 3) retain ≥ 95% of raw score indefinitely.

**Test labels**:
- `INVARIANT_decay_half_life_is_168_hours`
- `INVARIANT_high_usage_decay_floor_95`
- `INVARIANT_low_usage_decay_floor_30`
- `INVARIANT_scorer_constants_match_android`

---

## 8. Next-word weighting — user > dict

**Invariant**: `scoreDict(count)` = `count * 1.0`. `calculateUserScore(count, lastUsedMs, nowMs)` = `count * 50.0 * effectiveDecay + 300.0`. The `learningBonus = 300.0` guarantees any user entry outranks any dict entry of equivalent count.

**Why**: when the user has selected a word, that signal must dominate cold-start dictionary ranking. Breaking this invariant makes the learning system feel dead.

**Scope**: `NextWord/NextWordScorer.swift`.

**Test labels**:
- `INVARIANT_user_weight_is_50`
- `INVARIANT_dict_weight_is_1`
- `INVARIANT_learning_bonus_is_300`
- `INVARIANT_user_entry_outranks_dict_entry`

---

## 9. Case transformation

**Invariant**: `CaseTransformer.capitalizeCandidate(text, basedOn:input:isAutoCapitalizationEnabled:inputMode:)` is platform-neutral. Given the same inputs, iOS and Android emit byte-identical output. The function does not read global settings — callers forward `isAutoCap` and `inputMode` explicitly.

**Why**: candidate capitalization is visible on every keystroke. Divergence means the keyboard feels inconsistent between devices.

**Scope**: `Input/CaseTransformer.swift`.

**Corner cases**:
- Leading-uppercase input (`"G"`) with `isAutoCap = false` still applies case per `inputMode` rules.
- Uppercase TPS syllables do not exist; `inputMode == .tps` → no-op.
- `.english` mode passes through without romanization-specific adjustments.

**Test labels**:
- `INVARIANT_case_transformer_is_deterministic`
- `INVARIANT_case_transformer_honors_auto_cap_flag`

---

## 10. Custom-dictionary search-key derivation

**Invariant**: `CustomDictionaryDerivation.derive(roman:)` produces three keys (`notone`, `abbrev`, `roman_num`) using the same normalization pipeline as `InputNormalizer`. Keys are lowercase, diacritic-stripped, with documented hyphen behavior per key.

**Why**: custom-dictionary entries must be searchable by the same keys the MARISA trie uses, or the user's added words will not surface.

**Scope**: `Lexicon/Database/CustomDictionaryDerivation.swift`.

**Corner cases**:
- `abbrev` key is one character per syllable (initial letter of each hyphen-separated part).
- Empty `roman` → all three keys empty (never space, never sentinel).
- Keys match the sqlite schema in `CustomDictionaryRepository` — column names `notone`, `abbrev`, `roman_num`.

**Test labels**:
- `INVARIANT_custom_derivation_matches_input_normalizer`
- `INVARIANT_abbrev_key_is_one_char_per_syllable`

---

## 11. Settings read semantics

**Invariant**: `EngineSettingsProvider.current` is read **live**, not snapshotted. Each call returns the settings as of that call. Engine code must not cache the value across calls.

**Why**: settings changes (tone mode, candidate size) take effect instantly from the user's perspective. A snapshot would require reloading the engine to pick up changes.

**Scope**: `Settings/EngineSettings.swift`, `Settings/EngineSettingsProvider.swift`.

**Platform split**:
- iOS binds to `SharedSettings.shared` at the composition root.
- Android binds to its equivalent `SharedPreferences` reader.
- Neither binding leaks into a candidate file.

**Test labels**:
- `INVARIANT_engine_settings_are_live_read`

---

## 12. Logger backend neutrality

**Invariant**: shared-core candidates log via `LoggerFactory.make(category:)` only. No candidate references `DebugLogger`, `os.Logger`, `android.util.Log`, or `println`. The default factory returns `NullLoggerBackend`; platform code installs its concrete backend at startup.

**Why**: logging is the easiest place for a platform dependency to sneak back into the shared core. A stray `os.Logger` import breaks the Foundation-only requirement.

**Scope**: `Common/LoggerBackend.swift` + every candidate that logs (`CandidateProcessor`, `InputNormalizer` today).

**Test labels**:
- `INVARIANT_candidates_only_depend_on_logger_backend_protocol`
- `INVARIANT_null_logger_is_the_default_factory`

---

## 13. Composing-buffer reset semantics

**Invariant**: clearing the composing buffer (`ComposingState.apply(.reset)` on iOS, `ComposingManager.reset(ic)` on Android) MUST NOT commit the active preedit to the backing document. The user-visible effect is identical on both platforms: the preedit disappears, the document is unchanged.

**Why**: external `reset` callers — keyboard subtype switch, session teardown, mode change — assume composing state is discarded, not flushed. A silent commit means tone-marked but un-confirmed text leaks into the editor whenever the keyboard is dismissed mid-composition.

**Scope**:
- iOS engine: `Input/Composing/ComposingState.swift` `.reset` intent emits `[.clearPreeditWithoutCommit, .resetAutocomplete]`.
- iOS binding: `KeyboardExtension/KeyboardViewController+TextInput.swift` `clearMarkedText()` calls `setMarkedText("", …)` + `unmarkText()` — no `insertText`.
- Android binding (composing-aware path): `ime/text/composing/ComposingManager.kt` `reset(ic)` and `startComposing(...)` mid-composition restart both call `ic.setComposingText("", 1)` BEFORE `ic.finishComposingText()`. Pre-zero is mandatory because `InputConnection.finishComposingText()` commits the active composing region by default — see `composing-state-boundary.md` §11.2 rule 1.
- Android binding (bare-IC fallback path): `ime/text/TextInputManager.kt` `resetComposingText()` (4 call-sites: session start, DELETE / ENTER non-composing fallback, NUMERIC-PHONE key event) delegates to the top-level helper `clearHostComposingRegion(ic)` in `ComposingManager.kt`, which issues the same zero-then-finish pair. The helper colocates with the sibling `hostReportsNoComposingRegion` policy helper so both IC-layer invariant safeguards live next to the composing wrapper they protect.

**Extended invariant — `INVARIANT_composing_external_insert_commits_preedit_atomically`**: external insertion surfaces (emoji palette, clipboard paste) MUST commit the active Taigi preedit together with the external text in a single atomic document write — never a `finishComposingText` + `commitText(external)` pair (silent double-commit on Android) nor a bare `insertText(external)` while marked text is live (stale preedit on iOS).

- Pure-state pin: `ComposingStateTest`/`ComposingStateTests` — idle path emits plain insert, composing path emits `CommitTextReplacingPreedit(derived + external)` + `ResetAutocomplete` + `ResetAutocompleteContext`, empty-text path is a no-op.
- Binding pin: Android `ComposingManagerTest` — single `commitText` call, zero `finishComposingText`; iOS `ComposingManagerTests` — mirrored via `DelegateSpy` effect ordering.
- Closed path: `ime/media/MediaInputManager.kt` `sendEmojiKeyPress()` (Android) + `KeyboardExtension/KeyboardViewController+EmojiDelegate.swift` `emojiDidSelect(_:)` (iOS) — both route through `ComposingManager.commitPreeditThenInsertExternal(...)` as of PR #162 (2026-04-21).

**Extended invariant — `INVARIANT_composing_external_region_clear_discards_state`**: when the host editor reports that the composing region no longer exists, the IME binding MUST zero its internal composing state (raw buffer + cached derived display) without issuing any `InputConnection` call. A stale `state.isComposing` or `cachedDerivedDisplay` would let a later commit / reset re-insert preedit text at the new cursor position.

- Android binding: `InputMethodService.onUpdateSelection` delivers `candidatesStart == -1 && candidatesEnd == -1` when the host has no composing region. `TextInputManager.onUpdateSelection` calls `ComposingManager.onExternalComposingRegionCleared()`, which zeroes `state` + `cachedDerivedDisplay` and does not touch `InputConnection`. Closes the root cause of the A4-impl `commitComposition` fast/slow split (`composing-state-boundary.md` §11.10 divergence #3).
- iOS binding: N/A — the floating marked text model has no in-document region for the host to clear externally. Marked text is owned by the keyboard extension; selection / cursor moves leave `ComposingManager.state` intact until the next keyboard-driven intent. The label exists here as an Android-only binding pin; iOS trivially satisfies by construction.
- Binding pin: Android `ComposingManagerTest.test_INVARIANT_composing_external_region_clear_discards_state` (state zeroed, IC untouched) + `…short-circuits subsequent commit via idle state` (regression pin for the §11.10 #3 root cause) + `…clears cachedDerivedDisplay field` (reflection-based pin preventing silent cache leaks if `dispatch()` ever drops its own cache reset). Policy pin: `…hostReportsNoComposingRegion encodes the -1,-1 policy` (pure helper tested without the IME harness).

**Corner cases**:
- Idle reset through the composing-aware path (`ComposingState.apply(.reset)` on iOS, `ComposingManager.reset(ic)` on Android) MUST issue zero `InputConnection` calls / zero `Effect` emissions — observable as a no-op; the wrapper short-circuits on `!state.isComposing`.
- The bare-IC fallback path (`TextInputManager.resetComposingText` → `clearHostComposingRegion(ic)`) ALWAYS issues the zero-then-finish pair when `notifyInputConnection = true`, even when the host has no composing region. This is content-neutral (empty `setComposingText` + region-less `finishComposingText` do not mutate document text) and intentional — the caller does not ask the composing wrapper whether it's composing, so the helper defensively clears any stale host region regardless.
- The `deleteBackward` empty-raw path routes through `reset(ic)` on Android; the same zero-then-finish ordering applies.

**Test labels**:
- `INVARIANT_composing_clear_preedit_does_not_commit` — `ComposingManagerTest` (Android binding, composing-aware path via `reset(ic)` / `startComposing` / `deleteBackward` empty-raw) + `ComposingManagerTest.clearHostComposingRegion zeros then finishes without committing` + `… with null ic is noop` (Android binding, bare-IC fallback path via `TextInputManager.resetComposingText`) + `ComposingManagerTests.testReset_whenComposing_returnsToIdleWithoutInserting` (iOS wrapper, engine-level pin).
- `INVARIANT_composing_reset_when_idle_is_noop` — `ComposingManagerTest` (Android) + `ComposingManagerTests.testReset_whenIdle_emitsNoEffects` (iOS wrapper).
- `INVARIANT_composing_idle_to_idle_is_noop` — `ComposingStateTests.testReset_whenIdle_emitsEmptyEffects` (pure engine, iOS).
- `INVARIANT_composing_external_region_clear_discards_state` — Android `ComposingManagerTest.test_INVARIANT_composing_external_region_clear_discards_state`. iOS N/A (floating marked text — no in-document region for host to clear externally).

**Cross-references**:
- Boundary doc §2.2 (`Effect` enum), §11.1 (Android binding state table), §11.2 rule 1 (zero-then-finish), §11.6 (parity correction history).

---

## Change protocol

- Adding a new invariant: append to this doc + index; create a matching `INVARIANT_*` test stub in G9 immediately.
- Removing an invariant: requires a written rationale in the commit message and a Codex review pass.
- Modifying a constant (decay, weights, thresholds): update iOS source + Android mirror + this doc in the same commit; reject cross-platform drift at review time.

---

## Cross-references

- Architectural surface of the shared-core candidates (43 as of Phase I close): `docs/engine/shared-core-readiness.md`.
- Phase I task plan referencing this doc as prerequisite: `docs/architecture/ios-exemplar-plan.md` (G0).
- Codex review findings C2 / I7 that motivated this doc: `docs/architecture/codex-review-2026-04-19.md`.
- Data-artifact portability (MARISA / SQLite / `dictionary.bin`) tracked separately under G10.
