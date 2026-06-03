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
17. [Continuous input — explicit-tone candidate filtering](#17-continuous-input--explicit-tone-candidate-filtering)
18. [Continuous input — longest-match prefix suppression](#18-continuous-input--longest-match-prefix-suppression)
19. [Candidate strip + overlay — first-candidate keycap-color hint](#19-candidate-strip--overlay--first-candidate-keycap-color-hint)
21. [Composing input — leading 輕聲 `--` marker is a document literal](#21-composing-input--leading-輕聲----marker-is-a-document-literal)
22. [Continuous input — slot-0 respects the dictionary separator form](#22-continuous-input--slot-0-respects-the-dictionary-separator-form)
23. [Auto-space — attaching punctuation swaps the trailing space](#23-auto-space--attaching-punctuation-swaps-the-trailing-space)
24. [NextWord — prev-Hanji lookup + read-layer reading-variant dedup](#24-nextword--prev-hanji-lookup--read-layer-reading-variant-dedup)
25. [Continuous input — commit carries canonical TL (write-side identity)](#25-continuous-input--commit-carries-canonical-tl-write-side-identity)

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

---

## 17. Continuous input — explicit-tone candidate filtering

### `INVARIANT_CONTINUOUS_EXPLICIT_TONE_FILTER`

In continuous input — the **sole** keyboard candidate source since Item 13 (PR #279, v3.5.8); the toned `lexicon::search` path is Tab3-only — the FST lookup body for a span is tone-selected:

1. A **fully-toned** romanization span (every syllable carries an ASCII tone digit — `tai5`, `tai5gi2`, `kak4`) looks up the **toned** key family (`tl:<tl_num>` / `poj:<poj_num>`) and surfaces **only candidates whose reading is that tone**.
2. A span with **no** tone digit (`tai`, `taibak`) looks up the **toneless** fused key (`tl:<tl_notone>`) and surfaces **all tones** — the deliberate no-tone typing affordance.
3. A **mixed / partial-tone** multi-syllable span (`tai5gi` = toned + toneless) has no fully-toned FST family, so it stays on the toneless key (no regression).

This is a **conditional** contract, NOT "always filter": absent tone ⇒ all tones (intended), present tone ⇒ that tone only. Do not "fix" the toneless-shows-all behavior — it is the feature.

**Why**: typing `tai5` must never surface `tai2` / `tai3`. The continuous path strips tone digits to build a toneless fused key so the no-tone affordance works; before this invariant the strip was **unconditional**, so an explicitly-typed tone was silently discarded and every tone of the syllable appeared (critical bug, 2026-05-30, PR #367). The toned `tl:<tl_num>` keys already exist in `dictionary.fst` (`dictionary/build/create_fst.py:127-130`); they were simply never queried.

**Scope**: TL and POJ only. **English** has no tone semantics (a trailing digit is not a tone) and keeps the toneless strip. **TPS** tones are Bopomofo scalars, not ASCII digits, so TPS always takes the toneless branch (out of this slice). The selection rule `composing::shadow::fst_body_for_span` (→ `span_is_fully_toned_ascii`, the `([a-z]+digit)+` grammar) is applied at all three continuous key-build sites: `left_anchored_keys_from_lattice` (span-local), `build_partial_prefix_key` (Step 4b / empty-keys partial), and the walker edge (`fetch_walker_slot0_inner`). The walker's **custom-dictionary** override stays toneless-keyed (custom matching is tone-insensitive by design). Engine is the single source — iOS / Android inherit via FFI; there is no platform-side tone filtering.

**Relationship to §15**: §15 (`INVARIANT_LEX_INPUT_CLASSIFICATION_NUMERIC_TONE_SET`) defines whether input *carries* a tone; §17 pins what that tone must *do* to the continuous candidate set. The bug lived in the gap between the two — input was classified `RomanWithTone`, but the candidate lookup ignored the tone.

**Tests**:
- **Rust engine** — `engine/composing/tests/continuous_explicit_tone.rs`: `explicit_tone_filters_to_typed_tone` (`tsua2` → 紙 only, not 蛇; symmetric `tsua5` → 蛇 only) + `toneless_input_still_surfaces_all_tones` (`tsua` → both). Unit: `engine/composing/src/shadow.rs::tests` pin `span_is_fully_toned_ascii` / `fst_body_for_span` / the partial-prefix tone policy. Golden `tl_numeric_single` / `tl_numeric_multi` (`engine/composing/tests/golden_fetch_at_pos.rs`) freeze the toned-key wire vector (fixture emits toned `tl:<tl_num>` / `poj:<poj_num>` keys for production parity).
- **Dogfood (real-device, production gate)** — see `.claude/rules/taigi-incidents.md` § Qualitative perf gate dogfood checklist (item: explicit tone filters; toneless shows all tones).

---

## 18. Continuous input — longest-match prefix suppression

### `INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`

In continuous input, the span-local candidate strip surfaces, among the **single-syllable** spans anchored at buffer offset 0, **only the longest**. A shorter single syllable that is a strict prefix of a longer valid single syllable at the same anchor is **suppressed**:

1. `tai` → `台/代/大/…` (`tai`, 3 letters) only; the 2-letter `ta` (`乾/焦/大/…`) is NOT surfaced.
2. `tai5` → `台/臺/抬/…` (`tâi`, tone 5) only; `ta` is NOT surfaced. (Compounds with §17: the longest single syllable is also tone-filtered.)
3. `tsua` → `紙/蛇/…` (`tsua`) only; the shorter `tsu` (`珠`) is NOT surfaced.
4. A bare short syllable with no longer single-syllable extension typed still surfaces: `ka` → `共/家/加/…`, `m` / `ng` (syllabic consonants) — no `kah`/`kak` letters present, so `ka` is itself the longest.

Applies to **both toned and toneless input** — the suppression keys on span length, not tone (USER 2026-05-31「免調也壓制」). TL / POJ / TPS alike (the rule lives in the mode-agnostic span-local key builder).

**Multi-syllable phrase spans are NOT suppressed.** A left-anchored phrase span (`tai`+`bak` → `taibak` → 台北; the sub-word `台`/`台語` of `taigikhipuann`) is a different word, kept alongside the longest single syllable. Only **shorter single-syllable prefixes** are dropped, never phrase candidates — `taigikhipuann` still surfaces `台` (span 0–3), `台語` (span 0–5), and the whole-buffer `台語齒盤`.

**Why**: typing a complete syllable must not pollute the candidate strip with shorter prefix syllables. `tai` is one 3-letter syllable; a 2-letter `ta` candidate is noise (critical bug reported 2026-05-31 — `tai5` surfaced the full `ta` tone family). The shorter syllable IS phonotactically valid, but at the anchor the user committed to the longer reading by typing its extra letters.

**Scope — display layer only, NOT segmentation**: the rule lives in `composing::continuous::left_anchored_keys_from_lattice`, which recomputes the single-syllable ends via the `max_syllables = 1` primitive (`syllabifier::valid_span_endings_lowered`) and emits a key only for the longest single end + every phrase end. The segmentation **lattice keeps every edge** — the whole-sentence walker (`fetch_walker_slot0_inner`) and the min-hop `shadow::span_min_syllable_count` still see every split, so a span segmentable only via a non-greedy shorter-first cut (`ta`+`nia` when `tan`+`ia` dead-ends — Codex PR #290 P1) is unaffected. Engine is the single source — iOS / Android inherit via FFI.

**Best-practice alignment**: longest-match at the surfaced syllable matches khiin-rs `references/khiin-rs/khiin/src/data/segmenter.rs` (longest-match DP). Peer Mandarin/Bopomofo IMEs (McBopomofo, librime) do not face this because their syllables are inherently toned/atomic; the suppression is the price of Taigi's optional-tone affordance (`tai` = all tones, `tai5` = tone 5). Deliberately NOT adopted: suppressing inside the lattice (would reintroduce the PR #290 greedy dead-end) and "keep only the single longest end" (would drop the phrase sub-word candidates).

**Relationship to §17**: §17 selects the tone (toned vs toneless FST key family) for a given span; §18 selects which span lengths surface at the anchor. Orthogonal axes — the reported `tai5`→`ta` bug needed §18 (the `ta` span carries no tone digit, so §17's tone filter never touched it).

**Tests**:
- **Rust engine** — `engine/composing/tests/continuous_explicit_tone.rs::longest_match_suppresses_shorter_prefix_syllable` (`tsua2` drops 珠; toneless `tsua` keeps 紙+蛇 but drops 珠). Golden `tl_toneless_multi` (`engine/composing/tests/golden_fetch_at_pos.rs`) freezes `tsua` → 紙 + 珠仔 (both longest-span) with 珠 absent. `tl_toneless_long_reach` freezes the phrase-not-suppressed property (台/台語 sub-words retained).
- **Dev harness** — `engine/composing/tests/candidate_dump.rs` (`#[ignore]`) dumps production candidates for any input; run `cargo test -p composing --test candidate_dump -- --ignored --nocapture`.
- **Dogfood (real-device, production gate)** — see `.claude/rules/taigi-incidents.md` § Qualitative perf gate dogfood checklist (item: longest-match prefix suppression).

---

## 19. Candidate strip + overlay — first-candidate keycap-color hint

### `INVARIANT_CANDIDATE_FIRST_KEYCAP_HINT`

The first candidate (engine ranker top, index 0) in **both** the candidate strip **and** the expanded candidate overlay renders with a **filled keycap-color background + rounded corners** (no border) as a visual hint. Every other candidate has a transparent background until pressed/selected. Holds on **both iOS and Android**.

- **Color** — the platform keycap/button background, white in light themes and the dark keycap shade in dark themes:
  - Android: `R.attr.key_bgColor` — `CandidateDisplayParams.themeKeyBgColor` (strip), `CandidateOverlayColors.firstCandidateBackground` (overlay).
  - iOS: `Color.keyboardButtonBackground` (non-Liquid-Glass); `keyboardButtonBackgroundLiquid(for:).opacity(0.4)` (Liquid Glass — kept below the pressed/selected 0.6 so the state hierarchy stays legible).
- **Keying** — literal index 0 (strip: enumerated index; overlay: `RowItem.originalIndex == 0`). NOT the `isComposingText` metadata — that path is dead (no producer since the v3.5.8 continuous redesign removed the composing-text cell).
- **Precedence** — the pressed background wins over the first-candidate hint (and, on iOS, the selected background too; Android candidate cells have no selected-candidate visual state, only pressed).

**Why**: the ranker top is the default-commit candidate; a keycap-color fill signals it without a separate selection cursor. Removed in PR #267 (2026-05-13 "no visual distinction" rule), restored 2026-06-01 per USER — the hint aids continuous-input dogfooding. Dashed-border affordances (the old iOS/Android slot-0 style) are **deliberately not used**: every surveyed highlighting IME (Rime family — trime / Hamster / librime concept) uses a filled background, none use a border. FlorisBoard / aiongtaigi-sushi do not distinguish the first candidate at all, so they are not the model here.

**Cross-platform sites**:
- iOS — `CandidateViewStyle.resolvedBackgroundColor(isFirstCandidate:)` ← `CandidateButtonView` (strip) + `ExpandedCandidateGridCell` (overlay).
- Android — `SmartbarCandidateStrip.CandidateCell` (strip) + `CandidateOverlayContent.CandidateCell` (overlay).

**Tests**: pure visual styling — no automated render assertion (Compose / SwiftUI render pins are heavy + brittle; per `code-review-rules.md §9` the gate is qualitative dogfood). Pinned by the **S6 dogfood checklist** item in `.claude/rules/taigi-incidents.md` § Qualitative perf gate.

---

## 21. Composing input — leading 輕聲 `--` marker is a document literal

### `INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL`

A leading ASCII-hyphen run typed when the composing buffer has **no syllable content yet** (engine `Phase::Idle`) is the Taiwanese 輕聲 (neutral-tone) marker `--` (e.g. `--ah` 矣). It is committed to the document as **literal text** and does **not** enter the composing buffer. Only the following syllable (`ah`) composes — so the underlined preedit equals the candidate strip (`ah`), not `--ah`. Holds on **both iOS and Android** (single engine change). Matches the reference IME (MOE): typing `--` shows no underline; only `ah` is underlined; the result is `--ah`.

- **Scope** — leading hyphens only (run typed from `Phase::Idle`). The full run is preserved (single `-` or `--`+). Production keystrokes arrive one char at a time, so each leading `-` is its own `Start{"-"}` → literal insert, stay Idle; the first non-hyphen char starts composing.
- **Internal hyphens unaffected** — a hyphen typed AFTER syllable content (the 連字 in `tai-bak`, or an internal khinsiann `goa--si`) hits the `Append`/`Composing` arm and stays a composing-boundary delimiter. Internal `--` khinsiann compounds (`tso̍h--ji̍t`) remain dictionary-supplied via the candidate roman.
- **`--` is plain document text after insert** — backspace/cursor/editing over it use the host editor's normal behavior (the engine is Idle, no preedit). Picking a candidate after a literal `--` appends to it: `--ah` + tap roman `ah` → `--ah`; + tap **Hanji 矣** → `--矣`. The `--` is kept regardless of candidate kind (roman OR hanji) — this matches MOE, whose hanji mode also retains `--` (`--ah` → 矣 → `--矣`; USER-verified 2026-06-02). 輕聲 `--` is a Taiwanese-romanization marker that the MOE convention preserves even in hanji output, so it is never stripped per candidate kind.
- **Mechanism** — `transition.rs::enter_composing_or_insert_leading_hyphens` emits `CommitTextReplacingPreedit(run)` (a plain insert when there is no active preedit — iOS `clearMarkedText()` no-op + `insertText`; Android `commitText`) and stays Idle, splitting any syllable remainder into `enter_composing`.

**Why**: the engine has no 輕聲 semantics — previously `Intent::Start{"-"}` / `Append{"-"}` in Idle called `enter_composing`, so a leading `--` entered the composing buffer and the whole `--ah` underlined while the candidate was only `ah` (preedit ≠ candidate — dogfood wart 2026-06-02). Treating the leading marker as a document literal (MOE model) makes preedit == candidate and `--` normal editable text. An earlier commit-time re-attach approach (PR #378, closed) fixed only the commit string, not the underline; it was superseded by this input-model fix.

**Engine site**: `engine/composing/src/transition.rs` — `enter_composing_or_insert_leading_hyphens`, wired into the `Start`/Idle and `Append`/Idle arms.

**Tests**: `composing/tests/intent_coverage.rs` (`intent_start_leading_hyphen_inserts_literal_stays_idle`, `intent_append_leading_hyphen_in_idle_inserts_literal`, `intent_start_leading_hyphens_then_syllable_splits`). Plus the **S8 dogfood checklist** item in `.claude/rules/taigi-incidents.md` § Qualitative perf gate.

---

## 22. Continuous input — slot-0 respects the dictionary separator form

### `INVARIANT_CONTINUOUS_SLOT0_RESPECTS_DICT_SEPARATOR`

When the continuous-input best candidate (index 0) is a single lexical dictionary word, its romanization shows the **dictionary's stored separator form** — `-` (連字 compound, `tâi-gí` 台語), `--` (輕聲 khinsiann, `hōo--guá` 予我), or a space (詞組 phrase stored with spaces, `iā sī` 也是) — **not** a fabricated space-join. Holds wherever the full buffer maps to one dict word. Genuine multi-word readings that have **no** single full-span dict word (e.g. `tâi-gí khí-puânn` 台語齒盤) keep the walker's space-joined synthesis.

- **Why the bug existed** — the full-sentence walker synthesizes slot 0 by joining each chosen edge's canonical roman with a **space** (`fetch_walker_slot0_inner` ~`.join(" ")`). That space-join is correct for a real multi-word reading, but when the min-cost path splits a whole-word input into single-syllable edges (`hoogua` → 予/hōo + 我/guá, because the common single chars out-frequency the freq-16 compound 予我), the synth hanji `予我` collides with the dict word 予我 while the synth roman `hōo guá` is a malformed rendering of `hōo--guá`. The slot-0 `(roman, hanji, consumed_span)` dedupe could not collapse the pair because the romans differ **only** in the separator, so the malformed synth won slot 0 and the canonical dict row sank below.
- **Fix location** — the **display/assembly layer**, NOT the cost/segmentation primitive. At the Step-4 slot-0 prepend seam (`composing::continuous::assemble_candidates`), when an existing **FULL-coverage, full-span** dict candidate shares the synth's `hanji` and is the **same reading** (separator-insensitive, tone-preserving — `roman_reading_eq`, which strips ` `/`-` but keeps tone diacritics so `hōo guá` matches `hōo--guá` but never `hōo-guā` tone 7), that dict row's canonical roman is promoted to slot 0 and the synth is dropped. The walker's **reading choice** (which word) is preserved; only the **rendering** changes. This keeps the segmentation/cost primitive untouched per the §18 / diagnosis-discipline §S5 lesson (candidate-strip display fixes must not live in the shared primitive that also feeds the min-hop commit counter).
- **Gates** (all required to promote) — `coverage_kind == COVERAGE_KIND_FULL`; same `consumed_span`; same `hanji`; `x.roman != synth.roman` (only the separator-mismatch case — identical-form readings like 也是 `iā sī` keep the synth verbatim, so slot-0 metadata is unchanged there); `roman_reading_eq`; `!synth.is_custom` (never replace a custom-influenced walker path with a non-custom dict row). Respects **Core Principle #7** word identity (漢字 + canonical 羅馬字): `hōo--guá` (予我) and `hōo-guā` (戶外) stay distinct words.
- **Scope** — engine-internal continuous-input candidate assembly; surfaces identically on iOS and Android (single engine change, no platform mirror).

**Engine site**: `engine/composing/src/continuous.rs` — `roman_reading_eq` + the promote branch in `assemble_candidates` Step 4.

**Tests**: `composing/tests/continuous_slot0_dict_roman.rs` (`slot0_promotes_dict_khinsiann_form_over_space_synth`, `slot0_keeps_space_synth_for_genuine_multiword_reading`) + the `roman_reading_eq` unit test in `continuous.rs`. Empirically verified against production artifacts via the `composing/tests/candidate_dump.rs` dev harness (`DUMP_INPUTS="hoogua"` → `[0]=予我/hōo--guá`). Plus the **S9 dogfood checklist** item in `.claude/rules/taigi-incidents.md` § Qualitative perf gate.

## 23. Auto-space — attaching punctuation swaps the trailing space

### `INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP`

When auto-space is active (`isAutoSpaceEnabled` AND a roman-ish mode that auto-inserts a trailing space — i.e. `!effectiveSwapped || isOutputBothScripts`) and the character immediately before the cursor is a single space, typing **attaching** punctuation deletes that space, inserts the punctuation, then re-inserts the space AFTER it: `guá ` + `?` → `guá? ` (NOT `guá ?`). The trailing space is preserved so the next word stays separated.

- **Attaching set** (mirrored both platforms) — sentence-end `。！？.!?`, clause separators `，,、；;：:`, CLOSING brackets/quotes `)）]】」』`. **Opening** brackets/quotes (`(（[「『`) are deliberately excluded — they want a LEADING space, not attachment. ASCII straight quotes (`"` `'`) are excluded because the same glyph is both opening and closing; attaching would corrupt `guá "…"` into `guá" …`.
- **Why the bug existed** — auto-space is a literal space written to the document (`insertText(" ")` / `commitText(" ", 1)`); there was no smart-punctuation swap. The non-composing punctuation branch inserted the char directly, leaving the auto-space before it → `guá ?`.
- **Gating** — the swap fires only inside the same gate the auto-space insertion sites use, so it never eats a user-typed space in TPS / swapped-Hanji mode or when auto-space is off. Stateless: the document (cursor-preceding char) is the source of truth, not a cross-keystroke "pending space" flag.
- **Scope** — platform-side text-proxy mutation (NOT the Rust engine, which is not in the punctuation/auto-space path). Mirrored on iOS + Android with `CROSS-PLATFORM INVARIANT` comments.

**Platform sites**: iOS `ActionHandler+KeyActions.swift` — `insertNonComposingCharacter` + `isAutoSpaceModeActive`, classifier `Input/AutoSpacePunctuation.swift`. Android `TextInputKeyHandler.kt` — `commitNonComposingCharacter` + `isAutoSpaceModeActive`, classifier `ime/text/AutoSpacePunctuation.kt`.

**Tests**: iOS `AutoSpacePunctuationTests.swift` + Android `AutoSpacePunctuationTest.kt` (`INVARIANT_*` — pin the attaching set on both platforms). The document-mutation swap itself is dogfood-gated (**S10** in `.claude/rules/taigi-incidents.md` § Qualitative perf gate) — no fake-proxy render test by design.

## 24. NextWord — prev-Hanji lookup + read-layer reading-variant dedup

### `INVARIANT_NEXTWORD_PREV_HANJI_LOOKUP`

`user_association.db` next-word lookup keys on `prev_word` (Hanji) ALONE. `prev_tl` (the previous word's romanization) is a **ranking signal, not a hard filter**: query `WHERE prev_word = ? ORDER BY CASE WHEN prev_tl = ? THEN 0 WHEN prev_tl = '' THEN 1 ELSE 2 END, count DESC LIMIT ?`.

- **Recall** — a stored row whose non-empty `prev_tl` differs from the query roman is STILL returned (was silently dropped pre-v3.6.1). This recovers associations learned via a different commit path or app version — most importantly continuous-input commits store a raw `prev_tl` (`taigi`) while normal commits store canonical TL (`tâi-gí`) for the same word, and the old `prev_tl = ? OR prev_tl = ''` hard filter made the learned associations vanish.
- **Why Hanji-only is correct** — the bundled `association.bin` already keys prev on Hanji bytes only (`engine/lexicon/src/association_reader.rs`), and Core Principle #7 (`(hanzi, tl)` pair = word identity) binds the bigram **next** side (`UNIQUE(prev_word, next_word, next_tl)`), NOT the **prev** (context) side. `prev_tl` was never an identity key here.
- **Rank before truncate** — the `CASE` ordering keeps exact-`prev_tl` rows inside the `limit * 2` over-fetch window so a hot `prev_word` cannot starve them at the SQL `LIMIT`. The engine `filter` then applies the final score sort + real limit; final ordering is by score (count/decay), which is intentionally blind to `prev_tl` match quality — recall, not perfect prev-disambiguation, is the contract.
- **No data migration, no schema bump, no DELETE** — relaxing the query alone restores readability of every previously-unreadable ("dead") row. Those rows are NOT corrupt; deleting them would erase learned associations.

### `INVARIANT_NEXTWORD_READ_LAYER_DEDUP`

`engine/nextword/src/filter.rs` collapses separator/tone-only romanization variants of the SAME next word into ONE prediction, AFTER the `(hanzi, tl)` merge and BEFORE shaping/sort/truncate. Rows are grouped by `(hanzi, phonetics::toneless_reading_key(tl))` (separator- AND tone-insensitive). Canonical selection within a group is **separator-based** — a genuine multi-syllable reading is ALWAYS hyphen/space-separated in canonical TL, so a no-separator row sharing the toneless key can only be a fused raw keystroke slice:

1. the unique separator-bearing row (`tâi-gí`) is canonical; every no-separator row (`taigi`) folds its score into it. Separator-bearing rows that are the SAME reading (`hōo-guá` vs `hōo--guá`, differing only `-`/`--`/space — equal after stripping separators, tones kept) fold together too.
2. the group is left UNTOUCHED when (a) NO row has a separator — a bare toneless single syllable may be a genuine tone-1 reading (`當/tang`) byte-identical to a raw, so it must never be folded into a tone-marked sibling (`當/tàng`); tone-mark presence alone is NOT a safe discriminator because tone-1/tone-4 readings are bare; or (b) ≥2 separator-bearing rows are genuinely distinct readings (`tāng-bīn` vs `tàng-bīn`).

This protects genuine 一字多音 (Core Principle #7) — `當/tàng` + `當/tang` and `重/tāng` + `重/tàng` + `重/tang` all stay distinct (no separators, no fold).

- **Why** — the relaxed lookup (above) increases exposure of the duplicate-row class: continuous (`next_tl = taigi`) and normal (`next_tl = tâi-gí`) commits both persist under `UNIQUE(...next_tl)`, and the `(hanzi, tl)` merge keeps them distinct → 台語 would otherwise show twice. This is the read-layer cleanup; the write-side root fix (continuous commit carries canonical TL) is R2.
- **Residuals (both closed write-side in R2)** — (a) a *single-syllable* raw-vs-toned duplicate (`我/gua` vs `我/guá`) is NOT folded, because a bare toneless syllable is indistinguishable from a genuine tone-1 reading; safety (never hide a real reading) is preferred over completeness. (b) `toneless_reading_key` does NOT fold spelling families (`ch↔ts`, `oa↔ua`, POJ↔TL), so a POJ-spelled raw next_tl will not collapse onto its TL canonical.

**Scope**: SQL recall is platform-side (`user_association.db` is native SQLite, not in the engine); the dedup is engine-side (`nextword/filter.rs`), shared by both platforms via FFI.

**Platform sites**: iOS `NextWord/Repository/NextWordRepository.swift::fetchUserRows`; Android `ime/dictionary/NextWordService.kt` user-query SQL (`CROSS-PLATFORM INVARIANT` comment). Dedup: `engine/nextword/src/filter.rs::collapse_reading_variants` + `engine/phonetics/src/api.rs::toneless_reading_key`.

**Tests**: engine `filter.rs` unit tests (`collapse_*`, canonical) + `phonetics/src/api.rs` (`toneless_reading_key_*`). iOS `NextWordRepositoryTests.swift` (in-memory SQLite recall + ranking) + `RustEngineBridgeNextWordTests.swift` (`testFilter_collapses*` / `testFilter_preservesDistinctPolyphones`, FFI dedup parity). Android JVM unit tests cannot load the `.so` or run Robolectric, so the SQL recall is pinned via the shared SQL string + **S11** dogfood (`.claude/rules/taigi-incidents.md` § Qualitative perf gate).

## 25. Continuous input — commit carries canonical TL (write-side identity)

### `INVARIANT_NEXTWORD_CONTINUOUS_CANONICAL_TL`

A continuous-input candidate commit records the **candidate's canonical TL** as the NextWord association romanization (`prev_tl` / `next_tl`), identical to a normal (span-local) candidate commit — NOT the raw typed slice. This is the write-side root fix for the fragmentation §24 mitigated read-side: pre-R2, continuous commit passed `pending[..consumed_bytes]` (e.g. `taigi`) while normal commit passed canonical TL (`tâi-gí`) for the same word, so `user_association.db` accumulated split `(prev_tl, next_tl)` forms.

- **Identity carrier** — each continuous candidate carries a `canonical_tl` sidechannel (`CandidateMessage.canonical_tl`), snapshotted at candidate construction BEFORE the composing-layer recase / POJ-render passes rewrite the display `roman`. `DictionaryRecord.tl` for dict-backed candidates; `phonetics::api::canonical_tl_form(native_roman, mode)` for custom / walker-synth candidates. Populated for hanji-PRESENT candidates too — `(hanji, canonical-TL)` is the word identity (Core Principle #7), not gated on hanji absence.
- **Round-trip** — the platform stores `canonical_tl` on the candidate's tap metadata and forwards it into `CommitContinuous.association_tl` on tap. The engine's `commit_continuous` stores it on `NailedSegment.association_tl` and uses it as the `roman` arg of the `WordSelected` (final) / `UpdateLastSelectedWord` (mid / unnail) NextWord effects.
- **Fallback is byte-identical to pre-R2** — when `association_tl` is empty (legacy callers, the other 12 composing methods, or a TPS-OOV hanji-absent candidate with no recoverable dict TL), the engine falls back to the raw committed slice. `NailedSegment.raw_text` stays the sole authority for span / unnail mechanics — `association_tl` is a separate field, never overloaded onto `raw_text` or `display_text` (the document-commit string).
- **Display untouched** — `canonical_tl` is NEVER consulted for the document commit (that goes through `display_text`); the POJ-render pass mutates only the display `roman`, leaving `canonical_tl` intact.

**Scope**: engine `composing` (proto + `RawCandidate.canonical_tl` + `NailedSegment.association_tl` + `commit_continuous`) and `lexicon` (candidate construction); platform round-trip is iOS / Android tap metadata.

**Platform sites**: iOS `RustEngineBridge+Composing.swift` (`ContinuousCandidate.canonicalTl`, `composingCommitContinuous(associationTl:)`) + `TaigiAutocompleteService.swift` (metadata) + `ActionHandler+Suggestions.swift` (tap reader) + `ComposingManager.swift`. Android `RustEngineBridge.kt` / `ComposingBridge.kt` (`ContinuousCandidate.canonicalTl`, `composingCommitContinuous(associationTl)`) + `TaigiAutocompleteService.kt` + `TaigiWord.MetadataKeys.CANONICAL_TL` + `CandidateClickHandler.kt` + `ComposingManager.kt`.

**Tests**: engine `lexicon/src/continuous.rs` (`record_to_candidate` / `custom_entry_to_candidate` canonical_tl), `composing/src/continuous.rs` (`poj_render_rewrites_roman_but_preserves_canonical_tl`), `composing/src/dispatch.rs` (`raw_to_proto_candidate` propagation), `composing/tests/continuous_phase.rs` (`bug1_*` assert `nw.roman` == canonical TL). iOS `RustEngineBridgeContinuousTests.swift` (final-commit `WordSelected` roman == canonical TL through the real engine) + `TaigiAutocompleteServiceContinuousTests.swift` (metadata sidechannel). Android `ContinuousSuggestionsContractTest.kt` (CANONICAL_TL metadata). Cross-platform device acceptance: **S11** + a new **S12** continuous-vs-normal recall sequence (`.claude/rules/taigi-incidents.md` § Qualitative perf gate).
