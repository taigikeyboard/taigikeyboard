# Behavioral Invariants

**Status**: originally authored 2026-04-19 as the Phase 0 gate; remains the immutable cross-platform behavior contract through and beyond the Phase IV-B Rust extraction (slices v3.5.1 → case-transform). Each Rust slice must preserve every invariant in this document.

**Purpose**: enumerate the cross-platform behaviors that the engine — Rust crates plus surviving platform glue — must uphold on both iOS and Android.

**Scope boundary**: this doc captures *cross-platform behavior* only. Architecture purity (DI, ObservableObject, singletons, candidate-purity criteria) lives in `.claude/rules/ios-shared-core-candidates.md` + `.claude/rules/cross-platform-alignment.md`. The current Rust / pending / wont-migrate inventory is `docs/engine/migration-inventory.csv`. Data-artifact portability (`dictionary.fst`, `dictionary.bin`, `association.bin`, SQLite user data) lives in `data-artifacts-portability.md`. Android-only keyboard body invariants (touch, popup, window insets) live in [`keyboard-body-invariants-android.md`](keyboard-body-invariants-android.md).

**Implementation pointer**: most "Scope" lines below name the Rust crate that now owns the behavior plus the cross-platform bridge call. Where a behavior is partly platform-side (UI / SQLite / KeyboardKit / IME glue), both halves are listed.

**Test references**: each invariant ends with one or more `INVARIANT_*` test-case labels. Rust slices wire labels into `engine/<crate>/tests/`; platform tests cover bridge + integration paths.

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
14. [Lexicon — hanzi-input search guard (D-8 parity correction)](#14-lexicon--hanzi-input-search-guard-d-8-parity-correction)
15. [Lexicon — input classification (v3.5.7)](#15-lexicon--input-classification-v357)
16. [Keyboard body — touch + popup + window insets (Android Compose body)](#16-keyboard-body--touch--popup--window-insets-android-compose-body) → moved to [`keyboard-body-invariants-android.md`](keyboard-body-invariants-android.md)

---

## 1. Phonetics — TL ↔ POJ round-trip

**Invariant**: for every valid Taigi syllable `s`, converting display form TL → POJ → TL (and POJ → TL → POJ) must yield `s` exactly, including tone marks, combining scalars, and the `o͘` marker. Conversion is **lossless** on valid input.

**Why**: the engine freely rewrites the same syllable across both scripts for UI display and dictionary lookup. Any lossy step silently rewrites user data.

**Scope**: Rust `engine/phonetics` (`syllable.rs`, `tl.rs`, `poj.rs`, `api::poj_display_to_tl_display` / `tl_display_to_poj_display`). Bridged via `RustEngineBridge.pojToTl` / `tlToPoj` / `normalizeToTl`.

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

**Why**: downstream consumers (Rust `ranking::score::roman_to_base`, `phonetics::normalization::normalize_input`, platform `CustomDictionaryDerivation`) depend on the output being identical across platforms. A divergent one-character preprocessing bug silently changes every dedup key, every score calculation, and every fst lookup.

**Scope**: Rust `engine/phonetics::normalization::taigi_unicode_base_form` (canonical, since v3.5.3 PR #192). Bridged via `RustEngineBridge.nfdPreprocessForLookup`. Platform-side `TaigiUnicode.{swift,kt}` helpers were deleted under Path G.

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

**Scope**: Rust `engine/phonetics::tps` + `tps_adjust` + `api` (since v3.5.1 PR #186). Bridged via `RustEngineBridge.tlNumericToTPS` / `tlDisplayToTPS` / `tpsInputAdjust` / `containsTPS` / `isTPSToneMark`. The TPS → TL direction (`phonetics::tps_to_tl`) is Rust-internal only — consumed by `composing::continuous` for per-span TPS→TL conversion during continuous input. No FFI surface after the dead-surface cleanup (2026-05-04); C-1 (v3.5.9 D) additionally retired the `lexicon::classify_input` consumer when the `tps:` FST family went live. iOS / Android `TPS*.swift` / `.kt` files were deleted under Path G.

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

**Invariant**: `phonetics::normalization::normalize_input(rawInput)` always emits a `notone`- or `roman_num`-style key (lowercase, hyphens collapsed, diacritics stripped, tones expressed as trailing digits) regardless of the user's input mode (`.poj`, `.tl`, `.tps`, `.english`). The output is the primary fst lookup key.

**Why**: the fst is stored in a single canonical form; any mode-dependent drift in the key changes which candidates appear.

**Scope**: Rust `engine/phonetics::normalization::normalize_input` (single source); TPS path delegates to `phonetics::tps::from_zhuyin` first. Bridged via `RustEngineBridge.normalizeInput`.

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
2. **`removeDisplayDuplicates(_:)`** runs *after* sorting, only in TPS mode. Key = `hanzi` (continuous path keys on `(hanzi, consumed_span)` to preserve legitimate distinct partial vs full-buffer surfaces). Keeps the highest-ranked entry per hanzi (entries without hanzi are always kept).

Reversing or merging these two passes changes ordering. Running display dedup before sort drops higher-ranked entries.

**Why**: the keyboard shows TPS symbols to the left of candidates; visually identical hanzi with different roman forms is confusing, but the ranked winner must be retained.

**Scope**:
- Legacy `LexiconService.search` / NextWord ranking pipeline — Rust `engine/ranking::dedup` + `ranking::process_candidates` (single source, since v3.5.2). Bridged via `RustEngineBridge.processCandidates(_:tpsDedupEnabled:...)`.
- Continuous `FetchAtPos` production path (current dogfood) — Rust `engine/composing::continuous::dedupe_display_hanji_for_tps`, gated on `mode == InputMode::Tps` in `assemble_candidates`. Runs after the walker slot-0 prepend + POJ presentation pass. Two `dict.bin` rows sharing the toneless TPS key (e.g. `灣/uan` + `灣/uân` at `tps:ㄨㄢ`) survive the pre-sort `(roman, hanji, consumed_span)` dedupe — distinct romanization is a legitimate TL/POJ UI signal — and only collapse here, where the TPS UI hides romanization entirely.

**Test labels**:
- `INVARIANT_engine_dedup_keys_on_roman_plus_hanzi`
- `INVARIANT_display_dedup_runs_after_sort`
- `INVARIANT_display_dedup_keeps_words_without_hanzi`
- `dedupe_display_hanji_for_tps_collapses_same_hanji_same_span` + `tps_input_collapses_duplicate_hanji` (continuous `FetchAtPos` path, `engine/composing/tests/tps_display_dedup.rs`)

---

## 6. Candidate scoring — determinism + ordering

**Invariant**: Rust `ranking::score` is **pure** — same inputs yield the same `ScoreBreakdown`. Given identical `(word, normalizedInput, frequencyData, currentTime)`, iOS and Android return the same total because both call into the same Rust crate via the FFI seam.

Priority ordering (typical regime):

```
userFreqScore  (0 … +10000)          dominates within typical frequencies
completionPenalty (0 or -1000)        separates exact vs completion tiers
closenessBonus (0 … +500)             orders among completions
recencyBonus   (0 or +200)            tiebreaker within same freq
exactBonus     (0 or +100)            tiebreaker
baseFreqScore  (≈ lengthScore/10 × tier)  fallback (cold-start dominant, uncapped)
```

### Bounded dominance of `userFreqScore`

`userFreqScore` is capped at `USER_FREQ_CAP × USER_FREQ_WEIGHT = 10000` (reached at count = 100), but `baseFreqScore = (lengthScore / 10) × numerator / 10` is **not** capped. For very high-frequency dictionary entries, the base score can exceed user frequency. The invariant is therefore **bounded**, not absolute.

**Pairwise dominance threshold**: `userFreqScore` dominates a competitor's `baseFreqScore` when

```
count × 100  >  (lengthScore / 10) × numerator / 10
⟺  lengthScore  <  count × 10_000 / numerator
```

Concrete thresholds:

| user count | userFreqScore | dominates default-tier up to `lengthScore` | dominates tier-1 (1.5×) up to `lengthScore` |
|---|---|---|---|
| 10  | 1000   | 10_000  | ≈ 6_667  |
| 50  | 5000   | 50_000  | ≈ 33_333 |
| 100 | 10_000 (cap) | 100_000 | ≈ 66_667 |

Outside that regime, raw dictionary frequency becomes the leading factor — this is intentional: heavily-used dictionary entries (e.g., "的" with `lengthScore ≈ 200_000`) should stay on top even for users who haven't formed a habit yet.

**Worked example (typical regime)**: User has used word A (`lengthScore=500`, default tier) 10 times; candidate B (`lengthScore=800`, default tier) is cold. A: `userFreqScore = 1000`, `baseFreqScore = 50`. B: `userFreqScore = 0`, `baseFreqScore = 80`. A wins (~970 lead after other bonuses).

**Worked example (heavy-frequency regime)**: User has used rare word A (`lengthScore=100`, default tier) 100 times; candidate B is "的" (`lengthScore=200_000`, default tier). A: `userFreqScore = 10000`, `baseFreqScore = 10`. B: `userFreqScore = 0`, `baseFreqScore = 20000`. B wins (~9990 lead). A only resurfaces if other bonuses close the gap or if the user enters text where A matches exactly while B is a completion (`-1000` penalty flips the balance).

### Tier-based `baseFreqScore` multiplier

`baseFreqScore = (lengthScore / BASE_FREQ_DIVISOR) × tierNumerator / TIER_DENOMINATOR`. Tiers are selected by first-match-wins traversal of `SOURCE_TIERS`:

| Tier | Dictionary | Bit | Numerator | Effective multiplier |
|---|---|---|---|---|
| 1 | 教育部臺灣台語常用詞辭典 (kautian) | 0 | 15 | 1.5× |
| 2 | 公視台語新詞辭庫 (taigitv) | 1 | 13 | 1.3× |
| 3 | 教育部學科術語辭典 (stti) | 7 | 12 | 1.2× |
| 4 | 台語工藝詞庫 (kungge) | 6 | 11 | 1.1× |
| 5 (default) | 其他來源 / 補充資料 / `nil` bitmask | — | 10 | 1.0× |

Bit positions mirror `dictionary/build/10_create_dictionary_bin.py`. `stti` is in the ranking tier list but **not** in `dictionary/build/01_merge_csv.py:80` OFFICIAL_SOURCES (which governs dedup priority only) — intentional divergence.

**Why**: the scoring formula is the user-visible ordering of every candidate. Drift means the keyboard ranks differently on iOS vs Android for the same word + same user state.

**Scope**: Rust `engine/ranking/src/{score,sort,process}.rs` — `score_candidate`, `roman_to_base`, `input_to_base`, `sort_by_score`, `tier_numerator`, `SOURCE_TIERS` (constants pinned in `score.rs`). Bridged via `RustEngineBridge.processCandidates`.

**Corner cases**:
- `currentTime` is injected at the call site (ms since epoch). No call inside the engine reads the clock.
- `cappedUserFreq = min(count, 100)` — per-word count saturates at 100; never uncapped.
- Recency window is exactly 1 hour (`60 * 60 * 1000` ms); boundary condition `(currentTime - lastUsedMillis) < oneHourMillis` is strict `<`.
- `romanToBase` must strip hyphens, spaces, NFD combining marks, and digits in that order.
- Sort is stable on ties in the sense that the original array order is preserved when `total` ties (Swift `sorted(by:)` is not guaranteed stable — documented weakness; ordering fallback currently relies on the pre-sort input order).
- Tier bonus is bounded at max 1.5× — cannot invert a frequency gap > 1.5× between default- and tier-1 candidates. Proven by `INVARIANT_tier_bonus_preserves_frequency_ordering`.
- Integer math throughout: `rawBase * numerator / denominator` is computed bit-exact across Swift and Kotlin.

**Test labels**:
- `INVARIANT_score_is_deterministic`
- `INVARIANT_user_freq_dominates_ranking` (bounded — see worked examples above)
- `INVARIANT_completion_penalty_separates_tiers`
- `INVARIANT_recency_window_is_exactly_1_hour`
- `INVARIANT_roman_to_base_strips_tones_hyphens_digits`
- `INVARIANT_tier_bonus_preserves_frequency_ordering`
- `INVARIANT_tier_bonus_first_match_wins`

---

## 7. Next-word decay — RIME-style half-life

**Invariant**: `engine::nextword::scorer::calculate_decay(last_used_ms, now_ms)` computes `exp(-age_hours / 168.0 * ln(2))` — one-week half-life, RIME-derived. Constants are fixed:

| Constant | Value |
|---|---|
| `decayHalfLifeHours` | 168.0 |
| `highUsageDecayFloor` | 0.95 |
| `lowUsageDecayFloor` | 0.3 |
| `highUsageThreshold` | 3 |

**Why**: decay shapes the entire learning curve for user associations. A change here is not caught by scoring tests — it surfaces only after days of use.

**Scope**: `engine/nextword/src/scorer.rs` (canonical, post-v3.5.5 swap). Pre-swap mirrors in `NextWord/NextWordScorer.swift` + Android `NextWordService.kt` were deleted in PR for v3.5.5; iOS/Android platforms now route through `RustEngineBridge.nextwordFilter` for the score+merge+sort+limit step.

**Corner cases**:
- `nowMs - lastUsedMs` can be negative if the user rewinds the clock; the formula produces a factor > 1. Callers must not rely on decay ≤ 1.
- `ln(2)` is hard-coded as `LN_2 = 0.693` in the Rust crate — precision loss vs full `log(2.0)` is acceptable (drift ≈ 0.03% at 1 week).
- `calculate_user_score` clamps to `max(decay_floor, decay)` — high-usage entries (count ≥ 3) retain ≥ 95% of raw score indefinitely.

**Test labels** (in `engine/nextword/src/scorer.rs` `tests`):
- `decay_at_zero_age_is_one`
- `decay_at_one_half_life_is_half`
- `decay_at_two_half_lives_is_quarter`
- `decay_is_monotonically_decreasing`
- `high_and_low_floors_diverge_at_threshold`

---

## 8. Next-word weighting — user > dict

**Invariant**: `score_dict(count)` = `count * 1.0`. `calculate_user_score(count, last_used_ms, now_ms)` = `count * 50.0 * max(decay_floor, decay) + 300.0`. The `LEARNING_BONUS = 300.0` guarantees any user entry outranks any dict entry of equivalent count.

**Why**: when the user has selected a word, that signal must dominate cold-start dictionary ranking. Breaking this invariant makes the learning system feel dead.

**Scope**: `engine/nextword/src/scorer.rs` (canonical, post-v3.5.5 swap).

**Test labels** (in `engine/nextword/src/scorer.rs` `tests`):
- `score_dict_count_zero_is_zero`
- `score_dict_scales_linearly`
- `fresh_user_score_includes_learning_bonus`
- `user_outranks_dict_at_equal_count`

---

## 9. Case transformation

**Invariant**: `CaseTransformer.capitalizeCandidate(text, basedOn:input:isAutoCapitalizationEnabled:inputMode:)` is platform-neutral. Given the same inputs, iOS and Android emit byte-identical output. The function does not read global settings — callers forward `isAutoCap` and `inputMode` explicitly.

**Why**: candidate capitalization is visible on every keystroke. Divergence means the keyboard feels inconsistent between devices.

**Scope**: Rust `engine/phonetics::case_transform::capitalize_candidate` + `transform_input_case` (canonical, since case-transform slice / PR #205). Bridged via `RustEngineBridge.capitalizeCandidate` and `RustEngineBridge.transformInputCase`. Platform `CaseTransformer.{swift,kt}` deleted under Path G.

**Corner cases**:
- Leading-uppercase input (`"G"`) with `isAutoCap = false` still applies case per `inputMode` rules.
- Uppercase TPS syllables do not exist; `inputMode == .tps` → no-op.
- `.english` mode passes through without romanization-specific adjustments.

**Test labels**:
- `INVARIANT_case_transformer_is_deterministic`
- `INVARIANT_case_transformer_honors_auto_cap_flag`

---

## 10. Custom-dictionary search-key derivation

**Invariant**: `CustomDictionaryDerivation.derive(roman:)` produces three keys (`notone`, `abbrev`, `roman_num`) using the same normalization pipeline as the engine `phonetics::normalize_input`. Keys are lowercase, diacritic-stripped, with documented hyphen behavior per key.

**Why**: custom-dictionary entries must be searchable by the same keys the engine fst uses, or the user's added words will not surface.

**Scope**: iOS `Lexicon/Database/CustomDictionaryDerivation.swift` + Android `ime/dictionary/CustomDictionaryDerivation.kt` — they live on the user-data SQLite write path (`status=native_pending` / `wont_migrate` per `migration-inventory.csv`). Both platforms call `RustEngineBridge.normalizeInput` for the heavy lifting; only the per-syllable hyphen / abbrev rules stay native.

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

**Scope**: iOS `Logging/LoggerBackend.swift` + Android `ime/core/logging/LoggerBackend.kt`. Rust crates emit via the `log` crate; the platform sinks (`SwiftLoggerSink` / `RustEngineBridge.dispatchLog`) bounce records back into `LoggerBackend`.

**Test labels**:
- `INVARIANT_candidates_only_depend_on_logger_backend_protocol`
- `INVARIANT_null_logger_is_the_default_factory`

---

## 13. Composing-buffer reset semantics

**Invariant**: clearing the composing buffer (`ComposingState.apply(.reset)` on iOS, `ComposingManager.reset(ic)` on Android) MUST NOT commit the active preedit to the backing document. The user-visible effect is identical on both platforms: the preedit disappears, the document is unchanged.

**Why**: external `reset` callers — keyboard subtype switch, session teardown, mode change — assume composing state is discarded, not flushed. A silent commit means tone-marked but un-confirmed text leaks into the editor whenever the keyboard is dismissed mid-composition.

**Scope**:
- Engine: Rust `engine/composing` `Intent::Reset` emits `[ClearPreeditWithoutCommit, ResetAutocomplete]` (canonical, since v3.5.4 / PR #197). Platform `ComposingState.{swift,kt}` files deleted under Path G.
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

- Live Rust / native ownership inventory: `docs/engine/migration-inventory.csv`.
- Per-platform criteria + exclusions: `.claude/rules/ios-shared-core-candidates.md` §1, `.claude/rules/android-guidelines.md` §1.
- Original strategic context: `docs/architecture/codex-review-2026-04-19.md` (Codex review findings C2 / I7).
- Data-artifact portability (`dictionary.fst` / `.bin` / SQLite user data): `docs/architecture/data-artifacts-portability.md`.

---

## Test-label drift snapshot (2026-04-29)

Doc-sweep audit found **18 `INVARIANT_*` labels live in tests but are not enumerated as named invariants in this document**. Per line 9 ("an invariant without a label is not"), the inverse — test labels not bound to a named invariant — points to behaviors that may merit explicit documentation. Backfill is out of scope for the doc sweep; tracked as future work.

Orphan test labels grouped by module (verbatim from `grep -rn "INVARIANT_[a-z_]+" {ios,android}/.../test{,s}/`):

- **Composing** (5): `..._commit_captures_text_before_idle`, `..._delete_order`, `..._idle_has_no_selected_candidate`, `..._replace_last_preserves_selected_index`, `..._select_suggestion_is_atomic_commit`.
- **NextWord** (8): `..._association_window_strict_lt_`, `..._backspace_does_not_record`, `..._compound_pairs_are_sequential`, `..._generation_bumps_on_invalidating_intents`, `..._late_prediction_is_discarded`, `..._prediction_filter_hides_empty_tl_in_roman_mode`, `..._rescheduling_leaks_no_timer`, `..._sentence_end_resets_context`.
- **Unicode** (5 sub-labels of §2 `INVARIANT_nfd_preprocessed_platform_parity`): `..._idempotent_on_ascii`, `..._keeps_tone_combining_marks`, `..._nasal_marker_substitution`, `..._o_combining_dot_collapses`, `..._repeated_o_combining_dot`.

The Unicode bucket is partial coverage of the existing umbrella label and need not be lifted as separate invariants. Composing + NextWord buckets pin behaviors not currently named in §13 / §7-§8 — backfill candidates.


---

## 14. Lexicon — hanzi-input search guard (D-8 parity correction)

**Added**: 2026-05-01 (v3.5.6 lexicon read-path slice). **Re-pointed engine-ward 2026-05-15 (v3.5.8 Item 13)**: the platform-side `LexiconService.search` autocomplete entry — and with it the platform D-8 guard + its iOS/Android parity tests — were retired when the platform lexicon fallback was removed (`continuous-candidate-display.md` §15.4). The keyboard candidate path is now the engine-only Continuous dispatch; the hanzi guard lives entirely in Rust.

**Behavior**: a hanzi (CJK) composing buffer never produces keyboard candidates. Two engine guards enforce this:

1. **Continuous dispatch** (Item 11) — `engine/composing/src/dispatch.rs::handle_fetch_at_pos` checks `is_hanzi(raw)` at the top of dispatch and returns an empty `ContinuousResponse` (carrier present, candidates empty). `is_hanzi` is `.any()`, so a single stray CJK char anywhere fails closed.
2. **Lexicon search API** — `engine/lexicon::api::search` short-circuits `SearchInputType::Hanzi` to `[]` before consulting any reader (retained engine API surface; the keyboard path no longer calls it after Item 13).

**Rationale**: pre-Item-13 the platform `LexiconService.search` hit `lookupCustomDictionary` before the engine guard on iOS, so a custom-dict entry matching the hanzi buffer could leak as a suggestion. That entire platform autocomplete path is now deleted — the custom-dict-on-hanzi layering surface no longer exists outside Rust, so the guard is purely an engine concern.

**Tab3 unaffected**: Tab3 dictionary search accepts hanzi by design and is a fully separate consumer (iOS `DictionarySearchService` → bridge `lexiconSearchByHanzi`; Android `LexiconService.searchByHanzi` / `searchWithSources`). It never went through the retired autocomplete `search()` entry and carries no D-8 guard.

**Tests** (`INVARIANT_LEX_HANZI_GUARD`) — Rust engine only after Item 13:

1. `engine/lexicon/tests/parity.rs::invariant_lex_hanzi_guard_short_circuits` — asserts `search()` returns `[]` for `SearchInputType::Hanzi` regardless of bridge state.
2. `engine/composing/tests/dispatch_continuous.rs::decode_fetch_at_pos_hanzi_buffer_returns_empty_carrier` + `decode_fetch_at_pos_mixed_hanzi_buffer_returns_empty_carrier` (Item 11) — pin the Continuous-dispatch guard incl. mixed-buffer (`"a好b"`) fail-closed.

The prior platform-layer parity tests (iOS `LexiconServiceHanziGuardTests.swift`, Android `LexiconServiceHanziGuardTest.kt`) were deleted in Item 13 per `feedback_path_g_delete_mirrors.md`: the platform regression surface they guarded (custom-dict-before-guard) was removed with the platform lexicon path; algorithm correctness lives in the Rust engine tests above.

---

## 15. Lexicon — input classification (v3.5.7)

**Added**: 2026-05-02 (v3.5.7 classification slice + Tab3 hanzi-range parity correction). Codex sandwich rounds 1+2 + post-draft APPROVED. Implementation in Rust `engine/lexicon::classification::{classify_input, is_hanzi, contains_numeric_tone}`.

This umbrella label has four named subcases. Same `raw` string ⇒ same `(InputType, search_key)` tuple regardless of input mode, settings, or platform.

### `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`

`is_hanzi(text)` returns `true` iff `text` contains at least one Unicode codepoint in any of:

- CJK Unified Ideographs `0x4E00–0x9FFF`
- CJK Extension A `0x3400–0x4DBF`
- CJK Extension B `0x20000–0x2A6DF`
- CJK Extension C `0x2A700–0x2B73F`
- CJK Extension D `0x2B740–0x2B81F`
- CJK Extension E `0x2B820–0x2CEAF`

Extensions F/G/H/I/J are **explicitly excluded** at this slice. Future expansion is a separate behavior change, not part of this parity correction.

**Tab3 parity correction**: pre-v3.5.7, Android `DictionarySearchViewModel.kt:97` used `query.any { it.code in 0x4E00..0x9FFF || it.code in 0x3400..0x4DBF || it.code in 0x20000..0x2A6DF }`. Kotlin `Char.code` is a 16-bit UTF-16 code unit (0–65535), so the `0x20000..0x2A6DF` clause was unreachable; effective coverage was Unified + A only. v3.5.7 routes both platforms' Tab3 through the canonical Rust 6-range check (`LexiconBridge.isHanzi` / `RustEngineBridge.isHanzi`).

### `INVARIANT_LEX_INPUT_CLASSIFICATION_NUMERIC_TONE_SET`

`contains_numeric_tone(text)` returns `true` iff `text` contains at least one ASCII digit in `{'2','3','5','6','7','8','9'}`. Digits `'0'`, `'1'`, `'4'` are NOT numeric tone markers.

**ASCII-only is intentional, parity correction toward this invariant**: pre-v3.5.7, iOS used Swift `Character.isNumber` (Unicode general category N\* — includes full-width digits, Roman numerals, vulgar fractions) and Android used Kotlin `Char.isDigit()` (category Nd — includes full-width digits but not Roman numerals). The two platforms were therefore not aligned on edge inputs, and both had a subtle bug: their `!= '1' / '4' / '0'` exclusion checks compared against ASCII literals, so full-width `１` / `４` / `０` were not excluded and would be misclassified as tone markers. v3.5.7 collapses both platforms onto the ASCII-only contract above. Realistic Taigi IME input only ever produces ASCII tone digits (no IME flips into full-width digit mode for romanization typing), so the contracted set has no observable user impact. Filed under the `cross-platform-alignment.md` §1 "refactor surfaces existing divergence — correct toward documented invariant" allowance; surfaced by Codex on PR #202 (r3176669106) and explicitly accepted.

### `INVARIANT_LEX_INPUT_CLASSIFICATION_PRECEDENCE`

`classify_input(raw)` resolves `InputType` via short-circuit precedence:

1. `is_hanzi(raw)` ⇒ `InputType.Hanzi`
2. `phonetics::has_tone_marks(raw)` ⇒ `InputType.RomanWithTone`
3. `contains_numeric_tone(raw)` ⇒ `InputType.RomanWithTone`
4. otherwise ⇒ `InputType.RomanNoTone`

### `INVARIANT_LEX_INPUT_CLASSIFICATION_SEARCH_KEY`

`classify_input(raw).search_key == raw` (identity passthrough) for every input.

**History**: pre-C-1 this field carried `phonetics::tps_to_tl(raw)` when `contains_tps(raw)` was true, so the platform could feed a TL-form key into a `tl:`-only FST. C-1 (v3.5.9 D = TPS 三索引) gave TPS its own `tps:` FST family populated by C-0, so the `SearchRequest{input_mode=Tps}` path now hits TPS keys directly via `key_normalizer::build` and no per-keystroke pre-conversion is required. The `search_key` field is kept on the wire for backward compatibility but is now a verbatim echo of `raw`.

### Tests

- **Rust engine unit** — `engine/lexicon/src/classification.rs::tests` covers all four subcases (28 tests).
- **iOS** — bridge round-trip exercised through `RustEngineBridge.classifyInput` (called by `TaigiAutocompleteService` continuous path) end-to-end on real keystrokes; future explicit invariant test deferred.
- **Android** — same posture as iOS; `LexiconBridge.classifyInput` tested via `TaigiAutocompleteService` end-to-end on real device.

---

## 16. Keyboard body — touch + popup + window insets (Android Compose body)

**Moved 2026-05-26 →** [`keyboard-body-invariants-android.md`](keyboard-body-invariants-android.md) — 11 `INVARIANT_keyboard_*` labels (Android-only by design; iOS keyboard is a separate KeyboardKit contract). Kotlin source comments referencing the labels are unchanged; see the extracted file for the full list.
