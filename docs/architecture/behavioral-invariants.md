# Behavioral Invariants

The cross-platform behavior contract every Rust slice and platform change must preserve.

**Purpose**: enumerate the cross-platform behaviors that the engine — Rust crates plus surviving platform glue — must uphold on every platform: iOS, Android, macOS (IMKit), Windows (TSF), Linux (Fcitx5 + IBus).

**Scope boundary**: this doc captures *cross-platform behavior* only. Architecture purity (DI, ObservableObject, singletons, candidate-purity criteria) lives in `docs/contributing/ios-architecture.md` §5 + `docs/contributing/cross-platform-alignment.md`. The current Rust / pending / wont-migrate inventory is `docs/engine/migration-inventory.csv`. Data-artifact portability (`dictionary.fst`, `dictionary.bin`, `association.bin`, SQLite user data) lives in `data-artifacts-portability.md`. Android-only keyboard body invariants (touch, popup, window insets) live in [`keyboard-body-invariants-android.md`](keyboard-body-invariants-android.md).

**Implementation pointer**: most "Scope" lines below name the Rust crate that owns the behavior plus the cross-platform bridge call. Where a behavior is partly platform-side (UI / OS backup policy / KeyboardKit / IME glue), both halves are listed. The user-data SQLite stores are engine-owned (`engine/userdata`, `docs/architecture/user-data-engine-roadmap.md`).

**Test references**: each invariant ends with one or more `INVARIANT_*` test-case labels. Rust slices wire labels into `engine/<crate>/tests/`; platform tests cover bridge + integration paths. Device acceptance items (`Sn`) live in [`dogfood-checklist.md`](dogfood-checklist.md).

**Drift policy**: if any two of the five platforms diverge on an invariant, treat the divergence as a regression — open an issue, do not adjust this doc to match the code. UI-only invariants name the platforms they bind.

---

## Index

1. [Phonetics — TL ↔ POJ round-trip](#1-phonetics--tl--poj-round-trip)
2. [Unicode — NFD preprocessing](#2-unicode--nfd-preprocessing)
3. [TPS — TL ↔ TPS round-trip](#3-tps--tl--tps-round-trip)
4. [Input normalization — mode-agnostic numeric tones](#4-input-normalization--mode-agnostic-numeric-tones)
5. [Candidate dedup — engine vs display tiers](#5-candidate-dedup--engine-vs-display-tiers)
6. [Candidate scoring — retired additive formula](#6-candidate-scoring--retired-additive-formula)
7. [Next-word decay — RIME-style half-life](#7-next-word-decay--rime-style-half-life)
8. [Next-word weighting — user > dict, always](#8-next-word-weighting--user--dict-always)
9. [Case transformation](#9-case-transformation)
10. [Custom-dictionary search-key derivation](#10-custom-dictionary-search-key-derivation)
11. [Settings read semantics](#11-settings-read-semantics)
12. [Logger backend neutrality](#12-logger-backend-neutrality)
13. [Composing-buffer reset semantics](#13-composing-buffer-reset-semantics)
14. [Lexicon — hanzi-input search guard](#14-lexicon--hanzi-input-search-guard)
15. [Lexicon — input classification](#15-lexicon--input-classification)
16. [Keyboard body (Android)](#16-keyboard-body-android) → [`keyboard-body-invariants-android.md`](keyboard-body-invariants-android.md)
17. [Continuous input — explicit-tone candidate filtering](#17-continuous-input--explicit-tone-candidate-filtering)
18. [Continuous input — longest-match prefix suppression](#18-continuous-input--longest-match-prefix-suppression)
19. [Candidate strip + overlay — first-candidate keycap-color hint](#19-candidate-strip--overlay--first-candidate-keycap-color-hint)
21. [Composing input — leading neutral-tone `--` marker is a document literal](#21-composing-input--leading-neutral-tone----marker-is-a-document-literal)
22. [Continuous input — slot-0 respects the dictionary separator form](#22-continuous-input--slot-0-respects-the-dictionary-separator-form)
23. [Auto-space — attaching punctuation swaps the trailing space](#23-auto-space--attaching-punctuation-swaps-the-trailing-space)
24. [NextWord — prev-Hanji lookup + read-layer reading-variant dedup](#24-nextword--prev-hanji-lookup--read-layer-reading-variant-dedup)
25. [Continuous input — commit carries canonical TL (write-side identity)](#25-continuous-input--commit-carries-canonical-tl-write-side-identity)
26. [Custom dictionary — cross-input-mode search via side table](#26-custom-dictionary--cross-input-mode-search-via-side-table)
27. [Custom dictionary — row-count capacity](#27-custom-dictionary--row-count-capacity)
28. [User frequency — `(Hanji, canonical-TL)` pair-key identity](#28-user-frequency--hanji-canonical-tl-pair-key-identity)
29. [User-data backup — excluded from OS automatic backup](#29-user-data-backup--excluded-from-os-automatic-backup)
30. [Composing display is literal — no spelling conversion (TL + POJ)](#30-composing-display-is-literal--no-spelling-conversion-tl--poj)
31. [TPS space is a soft syllable separator (first-tone continuous input)](#31-tps-space-is-a-soft-syllable-separator-first-tone-continuous-input)
32. [TPS stop coda is phonotactically gated (no-space first-tone continuous input)](#32-tps-stop-coda-is-phonotactically-gated-no-space-first-tone-continuous-input)
33. [TPS nasal coda is phonotactically gated (no-space first-tone continuous input)](#33-tps-nasal-coda-is-phonotactically-gated-no-space-first-tone-continuous-input)
34. [Continuous literal-roman candidate (Hanji-with-romanization fast input)](#34--continuous-literal-roman-candidate-hanji-with-romanization-fast-input)
35. [TPS ambiguity-aware lookup — every reading of the pressed keys reaches the dictionary](#35-tps-ambiguity-aware-lookup--every-reading-of-the-pressed-keys-reaches-the-dictionary)
36. [Key-press feedback — app toggle gates sound + haptics (intentional OS-master divergence)](#36-key-press-feedback--app-toggle-gates-sound--haptics-intentional-os-master-divergence)
37. [App UI display-language production roster](#37--app-ui-display-language-production-roster)
38. [Main-app tab titles follow the display-language picker](#38--main-app-tab-titles-follow-the-display-language-picker)
39. [Keyboard locale tag is the neutral `mul`, not a Chinese-bearing tag](#39--keyboard-locale-tag-is-the-neutral-mul-not-a-chinese-bearing-tag)
40. [NextWord learning decisions are platform-neutral](#40--nextword-learning-decisions-are-platform-neutral)
41. [TPS keyboard space pins the unmarked tone (1 open rime / 4 stop coda)](#41--tps-keyboard-space-pins-the-unmarked-tone-1-open-rime--4-stop-coda)
42. [A candidate cell shows both scripts](#42--a-candidate-cell-shows-both-scripts)
43. [A candidate never carries a syllable the user has not typed into](#43--a-candidate-never-carries-a-syllable-the-user-has-not-typed-into)
44. [Romanization Only cells collapse rows that read the same](#44--romanization-only-cells-collapse-rows-that-read-the-same)
45. [The `o͘ⁿ` spelling of the nasal final is respelled per syllable, never per buffer](#45--the-o͘ⁿ-spelling-of-the-nasal-final-is-respelled-per-syllable-never-per-buffer)
46. [An acronym-shaped buffer reaches the words it abbreviates](#46--an-acronym-shaped-buffer-reaches-the-words-it-abbreviates)
47. [The macOS symbol picker stands on a placeholder marked region](#47--the-macos-symbol-picker-stands-on-a-placeholder-marked-region)
48. [Hanji leads out of the box; the desktop General pane owns the output script and its own reset](#48--hanji-leads-out-of-the-box-the-desktop-general-pane-owns-the-output-script-and-its-own-reset)
49. [The Syllable Separator rewrites the romanization the user sees, never the keys the engine learns by](#49--the-syllable-separator-rewrites-the-romanization-the-user-sees-never-the-keys-the-engine-learns-by)
50. [A phrase composed segment by segment becomes a whole-buffer candidate](#50--a-phrase-composed-segment-by-segment-becomes-a-whole-buffer-candidate)
51. [A separator typed between two picks reaches the document as typed](#51--a-separator-typed-between-two-picks-reaches-the-document-as-typed)
52. [A typed hyphen is a syllable boundary](#52--a-typed-hyphen-is-a-syllable-boundary)
53. ["Nasal mark in POJ capitals" decides the case of the POJ nasal marker, everywhere the user reads it](#53--nasal-mark-in-poj-capitals-decides-the-case-of-the-poj-nasal-marker-everywhere-the-user-reads-it)
54. [Every composing request renders under the settings it carries](#54--every-composing-request-renders-under-the-settings-it-carries)
55. [The separator at every typed boundary is the one the user typed](#55--the-separator-at-every-typed-boundary-is-the-one-the-user-typed)
56. [The previous word re-ranks the continuous candidates, never the segmentation](#56--the-previous-word-re-ranks-the-continuous-candidates-never-the-segmentation)
57. [Every dictionary switched off offers no dictionary candidates](#57--every-dictionary-switched-off-offers-no-dictionary-candidates)
58. [A desktop user-data list sends its filter as typed, reloads after every write, and selects only what is on screen](#58--a-desktop-user-data-list-sends-its-filter-as-typed-reloads-after-every-write-and-selects-only-what-is-on-screen)
59. [Under TPS the desktop preedit shows the Hanji conversion, and only a pick teaches](#59--under-tps-the-desktop-preedit-shows-the-hanji-conversion-and-only-a-pick-teaches)
60. [The desktop input indicator shows the script and what a commit writes](#60--the-desktop-input-indicator-shows-the-script-and-what-a-commit-writes)
61. [On macOS an activation makes a session eligible; a delivered key makes it the owner](#61--on-macos-an-activation-makes-a-session-eligible-a-delivered-key-makes-it-the-owner)
- [Change protocol](#change-protocol) · [Cross-references](#cross-references)

---

## 1. Phonetics — TL ↔ POJ round-trip

**Invariant**: for every valid Taigi syllable `s`, converting display form TL → POJ → TL (and POJ → TL → POJ) must yield `s` exactly, including tone marks, combining scalars, and the `o͘` marker. Conversion is **lossless** on valid input.

**Why**: the engine freely rewrites the same syllable across both scripts for UI display and dictionary lookup. Any lossy step silently rewrites user data.

**Scope**: Rust `engine/phonetics` (`syllable.rs`, `tl.rs`, `poj.rs`, `api::poj_display_to_tl_display` / `tl_display_to_poj_display`). Bridged via `RustEngineBridge.tlToPoj`; POJ→TL runs in-process only. `phonetics::normalize_to_tl` is Rust-internal.

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

**Invariant**: `phonetics::normalization::taigi_unicode_base_form(s)` is the one NFD preprocessing every platform reaches (through the bridge), so the byte sequence is identical on every platform for every input the keyboard may see — POJ nasal marker substitution, NFD decomposition, and `o͘` collapse to `o`.

**Why**: downstream consumers (`phonetics::normalization::normalize_input`, the custom-dictionary search keys `phonetics::custom_search`) depend on the output being identical across platforms. A divergent one-character preprocessing bug silently changes every dedup key, every score, and every fst lookup.

**Scope**: Rust `engine/phonetics::normalization::taigi_unicode_base_form` (canonical). The external-lookup URLs reach it inside the `ExternalLookupDigitForm` op (`phonetics::external_lookup`, `RustEngineBridge.externalLookupDigitForm`). No platform-side copy exists.

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

**Why**: TPS mode shares the same candidate/scoring pipeline as POJ/TL. Divergent TPS conversion between platforms means the same keystroke shows different candidates.

**Scope**: Rust `engine/phonetics::tps` + `tps_adjust` + `api`. Bridged via `RustEngineBridge.tlDisplayToTPS` / `tpsInputAdjust` / `isTPSToneMark`. `phonetics::api::tl_numeric_to_tps`, `phonetics::api::contains_tps` and the TPS → TL direction (`phonetics::tps_to_tl`, consumed by `composing::continuous` for per-span conversion) are Rust-internal only. No platform-side TPS converter exists.

**Corner cases**:
- `TPSInputAdjuster` reorders initial/medial/final before conversion; round-trip must account for adjuster output, not raw keystrokes.
- Tone digits on TPS syllables pass through unchanged.
- `phonetics::api::contains_tps` must return `false` for any pure Roman input (no false positives on ASCII-only strings).

**Test labels**:
- `INVARIANT_tps_to_tl_roundtrip`
- `INVARIANT_tl_to_tps_roundtrip`
- `INVARIANT_tps_adjuster_is_idempotent`

---

## 4. Input normalization — mode-agnostic numeric tones

**Invariant**: `phonetics::normalization::normalize_input(rawInput)` always emits a numeric key (lowercase, hyphens and spaces dropped, diacritics stripped, tones expressed as trailing digits) regardless of the user's input mode (`.poj`, `.tl`, `.tps`, `.english`). The output is the primary fst lookup key.

**Why**: the fst is stored in a single canonical form; any mode-dependent drift in the key changes which candidates appear.

**Scope**: Rust `engine/phonetics::normalization::normalize_input` (single source, in-process only); TPS path delegates to `phonetics::tps::from_zhuyin` first.

**Corner cases**:
- POJ-specific tone marks (`á`, `ê`, `ō`, etc.) normalize to trailing digits.
- Explicit user hyphens are dropped: `normalize_input` splits on `-` / space and concatenates; the `notone` derivation strips them too.
- Live TPS lookup keys go through `phonetics::KeyFamily::search_key`, not `normalize_input`.
- Empty input returns empty output (never a sentinel character).

**Test labels**:
- `INVARIANT_input_normalizer_is_mode_agnostic`
- `INVARIANT_normalizer_strips_hyphens_in_notone`

---

## 5. Candidate dedup — engine vs display tiers

**Invariant**: two distinct dedup passes exist and must run in the documented order:

1. **Engine dedup** runs *before* sorting — the Continuous pre-sort dedupe on `(hanji, consumed_span)` + (`roman` or the word identity `canonical_tl`, Core Principle #6). Removes true duplicates, including a custom row whose stored roman differs from the dictionary's only in form (NFD import, POJ or digit-tone spelling: `li2` must not list 李 twice).
2. **Display dedup** runs *after* sorting, only in TPS mode. Key = `(hanzi, consumed_span)` to preserve legitimate distinct partial vs full-buffer surfaces. Keeps the highest-ranked entry per hanzi (entries without hanzi are always kept).

Reversing or merging these two passes changes ordering. Running display dedup before sort drops higher-ranked entries.

**Why**: the TPS keyboard hides romanization; visually identical hanzi with different roman forms is confusing, but the ranked winner must be retained.

**Scope**:
- `engine/composing::continuous::dedupe_display_hanji_for_tps`, gated on `mode == InputMode::Tps` in `assemble_candidates`, after the walker slot-0 prepend + POJ presentation pass. Two `dict.bin` rows sharing the toneless TPS key (`灣/uan` + `灣/uân` at `tps:ㄨㄢ`) survive the pre-sort dedupe — distinct tones are distinct words — and only collapse here.
- **Candidate Display = Romanization Only (§44) is the mirror-image display pass for TL/POJ**, `composing::requests::dedupe_display_roman`, keyed on the **rendered roman alone**. It runs in `requests::handle_fetch_at_pos` AFTER the §34 literal prepend — not inside `assemble_candidates`, because the literal is inserted later and a pass there would leave `台/tâi` beside the bare literal `tâi`. Under Romanization Only an entry without hanji (the §34 literal, a roman-only custom entry) DOES participate and, being first-seen, survives; "entries without hanzi are always kept" holds for the TPS pass only. Both display passes: after sort, first-seen wins.

**Test labels**:
- `dedupe_display_hanji_for_tps_collapses_same_hanji_same_span` + `tps_input_collapses_duplicate_hanji` (`engine/composing/tests/tps_display_dedup.rs`)

---

## 6. Candidate scoring — retired additive formula

Retired; live candidate ranking is the Continuous `FetchAtPos` sort key — see `docs/engine/continuous-input-ranking.md` and §28.

---

## 7. Next-word decay — RIME-style half-life

**Invariant**: `engine::nextword::scorer::calculate_decay(last_used_ms, now_ms)` computes `exp(-age_hours / 168.0 * ln(2))` — one-week half-life, RIME-derived. Constants are fixed:

| Constant | Value |
|---|---|
| `DECAY_HALF_LIFE_HOURS` | 168.0 |
| `HIGH_USAGE_DECAY_FLOOR` | 0.95 |
| `LOW_USAGE_DECAY_FLOOR` | 0.30 |
| `HIGH_USAGE_THRESHOLD` | 3 |

**Why**: decay shapes the entire learning curve for user associations. A change here is not caught by scoring tests — it surfaces only after days of use.

**Scope**: `engine/nextword/src/scorer.rs` (single source). Every platform routes through `RustEngineBridge.nextwordPredictNext` (engine-side bundled lookup + score/merge/sort/limit, `engine/dispatch/src/predict.rs`).

**Corner cases**:
- `nowMs - lastUsedMs` can be negative if the user rewinds the clock; the formula produces a factor > 1. Callers must not rely on decay ≤ 1.
- `ln(2)` is hard-coded as `LN_2 = 0.693` — precision loss vs full `log(2.0)` is acceptable (drift ≈ 0.03% at 1 week).
- `calculate_user_score` clamps to `max(decay_floor, decay)` — high-usage entries (count ≥ 3) retain ≥ 95% of raw score indefinitely.

**Test labels** (in `engine/nextword/src/scorer.rs` `tests`):
- `decay_at_zero_age_is_one`
- `decay_at_one_half_life_is_half`
- `decay_at_two_half_lives_is_quarter`
- `decay_is_monotonically_decreasing`
- `high_and_low_floors_diverge_at_threshold`

---

## 8. Next-word weighting — user > dict, always

**Invariant**: `score_dict(count)` = `count * 1.0`. `calculate_user_score(count, last_used_ms, now_ms)` = `count * 50.0 * max(decay_floor, decay) + 300.0`. A merged prediction ranks by `combined_score(dict, user)` = `min(dict, DICT_SCORE_CAP) + user`, where `dict` / `user` are the summed dictionary / user contributions after the `(hanzi, tl)` merge and the reading-variant fold (§24). `DICT_SCORE_CAP = 250.0` sits below the lowest user score the scorer can produce (`1 × 50 × 0.30 + 300` = 315), so **every prediction with user evidence outranks every bundled-only prediction**, whatever the corpus count (`association.bin` word keys carry unbounded corpus counts). Bundled rows arrive count-desc and the score sort is stable, so capped bundled ties keep their corpus order.

**Why**: when the user has selected a word, that signal must dominate cold-start dictionary ranking. Breaking this invariant makes the learning system feel dead.

**Scope**: `engine/nextword/src/scorer.rs` (constants, `combined_score`) + `engine/nextword/src/filter.rs` (`MergedRow` keeps the two contributions apart until shaping).

**Test labels** (in `engine/nextword/src/scorer.rs` / `filter.rs` `tests`):
- `score_dict_count_zero_is_zero`
- `score_dict_scales_linearly`
- `fresh_user_score_includes_learning_bonus`
- `user_outranks_dict_at_equal_count`
- `dict_cap_below_weakest_user_score`
- `combined_score_caps_dict_only`
- `learned_prediction_outranks_any_bundled_prediction`

---

## 9. Case transformation

**Invariant**: case transformation (`transform_input_case` for keystrokes; `raise_case` / `match_case` for candidates, inside the engine) is platform-neutral. Given the same inputs, every platform emits byte-identical output. The functions do not read global settings — callers forward `inputMode` explicitly.

**Why**: candidate capitalization is visible on every keystroke. Divergence means the keyboard feels inconsistent between devices.

**Scope**: Rust `engine/phonetics::case_transform` (canonical). Keystrokes bridged via `RustEngineBridge.transformInputCase`; candidates are cased by the engine itself (`composing::continuous` `recase_roman`). No platform-side case transformer exists.

**Corner cases**:
- Uppercase TPS syllables do not exist; `inputMode == .tps` → no-op.
- `.english` mode passes through without romanization-specific adjustments.
- **Candidate casing only raises letters** (`raise_case`, behind `recase_roman`): CapsLock → all upper, shifted first key → first letter upper, plain lowercase → the roman **as stored**. A custom entry keeps its own capitals: `klsi` / `Klsi` → `Keng-lâm Su-īⁿ`, `KLSI` → `KENG-LÂM SU-Īᴺ` (marker per §53). Only the keystroke path `transform_input_case` lowercases. The POJ render (`recase_tl_as_poj_display`, step 5) re-raises **per `-` / space token with `match_case`**, never from a whole-string `LetterCase`: the rewriter title-cases each token and can shrink it (`NN` → `ⁿ`), and the stored lowercase `ⁿ` would make a whole-string read misclassify Caps Lock `KENG-LÂM SU-Īⁿ` as title case (`engine/composing/tests/nasal_marker_case.rs::caps_lock_keeps_a_custom_entry_with_a_nasal_marker_all_caps_in_poj_mode`).
- **Caps Lock raises every letter, Shift only the first — keys and callouts too**: the tone-letter table stores first-letter upper (TL `óo` → `Óo`), so `full_uppercase_tone_string` raises the rest on a table hit — Caps Lock `óo` → `ÓO`, Shift `Óo`. The callout tables carry both shapes: `Oo` / `Ng` (Shift) and `OO` / `NG` (Caps Lock), `phonetics::tone_variations`. Tests: `case_transform.rs` `caps_lock_raises_every_letter_of_a_tl_oo_table_entry` / `shift_keeps_a_tl_oo_table_entry_first_letter_only`, `tone_variations.rs` tests.
- The engine API takes no auto-cap flag (`case_transform.rs` accepts only `LetterCase` + `InputMode`); platforms read auto-cap for logging only.

**Test labels**:
- `INVARIANT_case_transformer_is_deterministic`

---

## 10. Custom-dictionary search-key derivation

**Invariant**: a custom-dictionary entry's search keys (forms `num`, `notone`, `abbrev`) are derived by the engine from its `roman`, using the same normalization pipeline as `phonetics::normalize_input`. Keys are lowercase, diacritic-stripped, with documented hyphen behavior per form.

**Why**: custom-dictionary entries must be searchable by the same keys the engine fst uses, or the user's added words will not surface.

**Scope**: engine `phonetics::derivation` (`derive_notone`, `derive_abbrev`) + `phonetics::custom_search::derive_custom_search_keys` (§26), called in-process by `engine/userdata` (`userdata::derive_custom_search_keys`, the `CustomDictionaryStore` / `LearnedPhraseStore` deriver) on every write and by `CustomDictionaryStore::rederive_search_keys_if_needed` on takeover. No platform derives a key.

**Corner cases**:
- `abbrev` key: the §46 face — one leading spelling unit per syllable, `ph` / `th` / `kh` / `tsh` whole — derived by `phonetics::derive_abbrev`.
- Empty `roman` → no keys (never space, never sentinel); the store refuses to write an entry whose `roman` derives none (`CustomDictionaryRefusal` `UNSEARCHABLE`).
- The legacy per-row columns `notone`, `abbrev`, `roman_num` survive on files the phones wrote, but nothing writes or reads them: the engine inserts only `id` / `roman` / `hanzi` / timestamps, so a new row leaves them at `''` and an edit leaves them as they were.

**Test labels**:
- `INVARIANT_custom_derivation_matches_input_normalizer`

---

## 11. Settings read semantics

**Invariant**: `EngineSettingsProvider.current` is read **live**, not snapshotted. Each call returns the settings as of that call. Engine code must not cache the value across calls.

**Why**: settings changes (tone mode, candidate size) take effect instantly from the user's perspective. A snapshot would require reloading the engine to pick up changes.

**Scope**: iOS `Settings/EngineSettings.swift`, `Settings/EngineSettingsProvider.swift`; Android `ime/settings/EngineSettings.kt`, `ime/settings/EngineSettingsProvider.kt`.

**Platform split**:
- iOS binds to `SharedSettings.shared` at the composition root.
- Android binds to its equivalent preferences reader.
- Neither binding leaks into a candidate file.

**Test labels**:
- `INVARIANT_engine_settings_are_live_read`

---

## 12. Logger backend neutrality

**Invariant**: shared-core candidates log via `LoggerFactory.make(category:)` only. No candidate references `DebugLogger`, `os.Logger`, `android.util.Log`, or `println`. The default factory returns `NullLoggerBackend`; platform code installs its concrete backend at startup.

**Why**: logging is the easiest place for a platform dependency to sneak back into the shared core.

**Scope**: iOS `Logging/LoggerBackend.swift` + Android `ime/core/logging/LoggerBackend.kt`. Rust crates emit via the `log` crate; the platform sinks (`SwiftLoggerSink` / `RustEngineBridge.dispatchLog`) bounce records back into `LoggerBackend`.

**Test labels**:
- `INVARIANT_candidates_only_depend_on_logger_backend_protocol`
- `INVARIANT_null_logger_is_the_default_factory`

---

## 13. Composing-buffer reset semantics

**Invariant**: clearing the composing buffer (iOS `ComposingManager` reset, Android `ComposingManager.reset(ic)`) MUST NOT commit the active preedit to the backing document. The preedit disappears; the document is unchanged.

**Why**: external `reset` callers — input-mode switch, session teardown — assume composing state is discarded, not flushed. A silent commit leaks un-confirmed text into the editor whenever the keyboard is dismissed mid-composition.

**Scope**:
- Engine: `engine/composing` `Intent::Reset` emits the abort trio `[ClearPreeditWithoutCommit, ClearCandidates, NextWordClearForNewComposing]` while composing.
- iOS binding: `execute(.clearPreeditWithoutCommit)` (`KeyboardExtension/KeyboardViewController+TextInput.swift`) → `Actions/HostTextWriter.swift` `clear()` — `setMarkedText("", …)` + `unmarkText()`, no `insertText`.
- Android binding: the zero-then-finish rule and its call sites (`ComposingManager.reset(ic)`, `TextInputKeyHandler.resetComposingText` → `clearHostComposingRegion(ic)`) are in [`composing-state-boundary.md`](composing-state-boundary.md) §11.2 rule 1.

**Extended invariant — `INVARIANT_composing_external_insert_commits_preedit_atomically`**: external insertion surfaces (emoji palette, clipboard paste, iOS symbol panel) commit the active preedit together with the external text in a single atomic document write — never `finishComposingText` + `commitText(external)` (Android double-commit) nor a bare `insertText(external)` while marked text is live (iOS stale preedit). Path: `ComposingManager.commitPreeditThenInsertExternal` — Android `MediaInputManager.sendEmojiKeyPress`, iOS `ActionHandler.insertExternalText` (emoji delegate, `TaigiKeyboardView` symbol insert); binding detail in `composing-state-boundary.md` §11.6.
- Pins: `engine/composing/tests/continuous_phase.rs` (`commit_preedit_then_insert_external_under_continuous_combines_pending_and_external`) + `tests/invariants.rs` (`invariant_commit_preedit_then_insert_*`) — idle emits a plain insert, composing emits `CommitTextReplacingPreedit(composition + external)` + `ClearCandidates` + `ResetCandidateContext` + `NextWordClearForNewComposing`, empty text is a no-op. iOS `ComposingManagerTests.testCommitPreeditThenInsertExternal_whenComposing_commitsAtomicallyWithExternalText`. Android: no JVM test yet (to add over the recording host in `ComposingManagerHostReconciliationTest`).

**Extended invariant — `INVARIANT_composing_host_commit_one_write_per_event`** (iOS binding): a commit over the marked text, together with every literal the same user event writes after it (Space, punctuation, auto space, Enter's newline), reaches the host as ONE `setMarkedText(commit + literals)` + `unmarkText()` — never `insertText` over the marked text, and never a commit followed by a separate write in the same main-thread turn.

- **Why**: (1) Flutter before flutter/flutter#191062 inserts `insertText` at the caret and keeps the marked text, leaving the raw preedit in front of the commit; Apple's documented flow (update marked text, then `unmarkText()`) has no such dependency. (2) Observed on iOS 27: proxy calls from one turn reach the host merged (all inserted text, then the last `setMarkedText`, then the unmark), so commit + `insertText(" ")` puts the space before the word on UIKit and duplicates the word on Flutter. azooKey's `insertText` + `setMarkedText("")` order fails on Flutter.
- **Mechanism**: `Actions/HostTextWriter.swift` writes the keyboard's own host text. `ActionHandler.performInputEvent` opens an event at every entry point (key release, backspace repeat, candidate tap, emoji, emoji-palette backspace, symbol panel); a commit over marked text inside it is held, later inserts append to it, and the event's end writes it once (under `ComposingManager.performAsSelfCommit`, so `textWillChange` does not read it as a field switch). A held commit accepts only trailing inserts — `update` / `clear` / `deleteBackward` while one is held is an assertion failure. UIKit hosts answer `unmarkText()` of non-empty marked text with a `textDidChange` ~50 ms later; `KeyboardViewController.textDidChange` drops that echo before `super` and before `nextWordController.resetAndClearUI()` (`HostTextWriter.isEchoOfOwnCommit`: only the first callback after a commit, within 0.5 s, with text before the caret exactly the pre-composition text plus the commit and text after the caret unchanged — azooKey's `ExpectedEditTracker` match; a host that never echoes, such as Flutter, leaves nothing to swallow a later real edit). Otherwise every commit would wipe the fresh NextWord predictions and `last_selected_word`.
- **Binding pin**: iOS `HostTextWriterTests`. Android needs none — `commitText(text, 1)` commits and leaves the composing region in one call.

**Extended invariant — `INVARIANT_composing_external_region_clear_discards_state`** (Android binding; iOS satisfies it by construction — floating marked text has no in-document region the host can clear): when the host editor no longer holds the composing region the IME wrote, the binding zeroes its internal composing state (raw buffer + cached derived display) without any `InputConnection` call. A stale `state.isComposing` would let a later commit / reset re-insert preedit text at the new cursor.

- `onUpdateSelection` reports `candidatesStart == candidatesEnd == -1` when the host has no composing region — but the report can be **older than the IME's own last `setComposingText`** (an app focus / draft-restore selection move, or `onStartInputView(restarting=true)` from an app-side `restartInput`); treating it as current wiped the first key typed after an IME / app switch. So `TextInputManager.onUpdateSelection` → `ComposingManager.onHostSelectionUpdate` → `reconcileWithHost(ic)`: while composing, ONE `ic.getExtractedText` snapshot (text + cursor; the report's own selection is never used as a coordinate). Preedit found right before a collapsed cursor → keep state and **re-assert the composing span** over it (`ic.setComposingRegion(startOffset + selEnd - len, startOffset + selEnd)`, the FlorisBoard shape); mismatch, non-collapsed selection, or `null` → `onExternalComposingRegionCleared()`, which zeroes state without touching `InputConnection`.
- Accepted heuristic: a tap-away landing at the end of identical text elsewhere is indistinguishable and is kept. FlorisBoard's re-derivation of the composing word from document text is not adopted — Taigi raw input is not recoverable from the rendered preedit.
- `bumpGeneration()` zeroes the mirror too, so an input-context invalidation can never leave a mirror the reconciliation would keep after the engine dropped the composition. `onStartInputView(restarting=true)` on a still-composing-eligible editor keeps the live `ComposingManager` (no rebuild, no region wipe, no generation bump) and runs the same reconciliation, re-fetching candidates when the composition survives.
- Binding pin: Android `ComposingManagerHostReconciliationTest` — `reconcile_hostLostPreedit_clearsOnceWithoutWritingHost`, `reconcile_hostStillHoldsPreedit_keepsStateAndGeneration_andReassertsSpan`, `reconcile_partialExtraction_usesStartOffset`, `reconcile_nullRead_clears` / `reconcile_nullConnection_clears`, `reconcile_cursorMovedInsidePreedit_clears` / `reconcile_nonCollapsedSelection_clears`, `reconcile_reentrantFromInsideWrite_seesNewPreedit`, `reconcile_commitAThenStartB_lateCallbackForA_keepsB`, `reconcile_unicodePreedit_keeps`.

**Corner cases**:
- Idle reset through the composing-aware path issues zero `InputConnection` calls / zero `Effect` emissions — the wrapper short-circuits on `!state.isComposing`.
- The bare-IC path (`TextInputKeyHandler.resetComposingText` → `clearHostComposingRegion(ic)`) ALWAYS issues the zero-then-finish pair when `notifyInputConnection = true`, even with no host composing region — content-neutral and intentional.
- The `deleteBackward` empty-raw path routes through `reset(ic)` on Android; the same zero-then-finish ordering applies.

**Test labels**:
- `INVARIANT_composing_clear_preedit_does_not_commit` — iOS `ComposingManagerTests` (wrapper). Android: no JVM test; the zero-then-finish order is pinned only in `ComposingDelegate.kt` / `TextInputKeyHandler.kt` comments and dogfood.
- `INVARIANT_composing_reset_when_idle_is_noop` — `ComposingManagerTests.testReset_whenIdle_emitsNoEffects` (iOS). Android: no JVM test.
- `INVARIANT_composing_idle_to_idle_is_noop` — `engine/composing/tests/invariants.rs::invariant_reset_idle_is_noop`.
- `INVARIANT_composing_external_region_clear_discards_state` — Android `ComposingManagerHostReconciliationTest`.

**Cross-references**: [`composing-state-boundary.md`](composing-state-boundary.md) §2.2 (`Effect` → platform binding), §11.2 (`InputConnection` rules), §11.6 (external text and reset), §11.10 (host-cleared composing region).

---

## 14. Lexicon — hanzi-input search guard

**Invariant**: a hanzi (CJK) composing buffer never produces keyboard candidates. `engine/composing/src/requests.rs::handle_fetch_at_pos` checks `is_hanji(raw)` at the top of dispatch and returns an empty `ContinuousResponse` (carrier present, candidates empty). `is_hanji` is `.any()`, so a single stray CJK char anywhere fails closed.

**Why**: the keyboard candidate path is engine-only Continuous dispatch, so the guard is purely an engine concern; a custom-dictionary or dictionary hit on a hanzi buffer would surface as a bogus suggestion.

**Tab3 unaffected**: Tab3 dictionary search accepts hanzi by design (iOS / Android `DictionarySearchService` → `searchByHanji` / `searchWithSources`); hanji queries go to `search_by_hanji`, so no guard is needed there.

**Tests** (`INVARIANT_LEX_HANJI_GUARD`): `engine/composing/tests/dispatch_continuous.rs::decode_fetch_at_pos_hanji_buffer_returns_empty_carrier` + `decode_fetch_at_pos_mixed_hanji_buffer_returns_empty_carrier` (mixed buffer `"a好b"` fails closed).

---

## 15. Lexicon — input classification

Implementation: `engine/phonetics/src/hanji.rs`, re-exported as `lexicon::classification::is_hanji` so `userdata` reads the same predicate. Explicit-tone handling in the continuous path is pinned by §17.

### `INVARIANT_LEX_INPUT_CLASSIFICATION_HANJI_RANGE`

`is_hanji(text)` returns `true` iff `text` contains at least one Unicode codepoint in `CJK_RANGES`:

- CJK Unified Ideographs `0x4E00–0x9FFF`
- CJK Extension A `0x3400–0x4DBF`
- `0x20000–0x3134F` — Extensions B–G and I, the Compatibility Ideographs Supplement, and the unassigned gaps between them
- CJK Compatibility Ideographs `0xF900–0xFAFF`

Extension H (`0x31350–0x323AF`) and later stay out. The table equals the dictionary pipeline's `dictionary/common/cjk.py` `CJK_RANGES`; `engine/lexicon/tests/cjk_ranges_parity.rs` parses the Python tuple and fails on any drift. A lone Extension F/G or Compatibility character (𰣻 U+308FB, 丸 U+2F801, 嗀 U+FA0D — all `dictionary.csv` rows) is Hanji, so Tab3 takes `search_by_hanji` and the continuous guard returns an empty carrier. A mixed query such as `a𰣻` routes like `a台`: the whole query goes to `search_by_hanji` and the custom dictionary is not consulted. No NFC normalization — a Compatibility codepoint is looked up as itself.

Tab3 on both platforms routes through `RustEngineBridge.isHanji` / `LexiconBridge.isHanji`; no platform-side range check (a Kotlin `Char.code` range check cannot see supplementary-plane codepoints).

### Tests

- **Rust engine unit** — `engine/phonetics/src/hanji.rs::tests` covers the range bounds.
- **Rust engine prod** — `engine/lexicon/tests/cjk_ranges_parity.rs`: table parity with `cjk.py`, and 𰣻 / U+2F801 / U+FA0D reach their `search_by_hanji` rows.
- **Continuous guard** — `engine/composing/tests/dispatch_continuous.rs::decode_fetch_at_pos_mixed_hanji_buffer_returns_empty_carrier` (`a好b`, `a𰣻`, `a\u{2F801}`, `a\u{FA0D}`).

---

## 16. Keyboard body (Android)

The Android-only `INVARIANT_keyboard_*` labels (touch, popup, window insets) live in [`keyboard-body-invariants-android.md`](keyboard-body-invariants-android.md).

---

## 17. Continuous input — explicit-tone candidate filtering

### `INVARIANT_CONTINUOUS_EXPLICIT_TONE_FILTER`

In continuous input — the sole keyboard candidate source; the toned `lexicon::search` path is Tab3-only — the FST lookup body for a span is tone-selected:

1. A **fully-toned** romanization span (every syllable carries an ASCII tone digit — `tai5`, `tai5gi2`, `kak4`) looks up the **toned** key family (`tl:<tl_num>` / `poj:<poj_num>`) and surfaces **only candidates whose reading is that tone**.
2. A span with **no** tone digit (`tai`, `taibak`) looks up the **toneless** fused key (`tl:<tl_notone>`) and surfaces **all tones** — the deliberate no-tone typing affordance.
3. A **mixed / partial-tone** multi-syllable span (`teng5sek`) has no partial-tone FST family, so it looks up the **toneless** key and then **pins every syllable the user did tone**: `teng5sek` keeps 程式 `têng-sek` and drops 等式 `téng-sek` / 中式 `teng-sek`; the un-toned `sek` stays free. The pin is a `lexicon::TonePin::TypedTones { mode, typed }` built by `composing::shadow::span_tone_pin` from the hyphenless typed span, checked per reading by walking the record's numeric-tone face (`phonetics::tl_num_syllable_ends_from_tl` / `poj_num_syllable_ends_from_tl`) syllable by syllable: a syllable's letters must be the next thing typed (the nasal `onn` ↔ `oonn` alias tried per syllable — `si7hoonn` still reaches 是乎 `sī-honnh`), and a typed digit right after them must equal the face's tone. Typed text ending inside a syllable ends the walk with every typed digit honored, so one walk serves the exact, walker, partial-prefix and custom paths; everything else (misaligned letters, typed text the face cannot account for) is a REJECT, never a fail-open. A fully-toned span (case 1) carries the same pin beside its verbatim key: the key filters dictionary hits, the pin keeps a toneless-matched **custom** entry honest (custom matching is tone-insensitive by design), on both the lexicon merge and the walker's per-edge pick — every **eligible** entry under the toneless key joins it.

   Known gap: `span_is_fully_toned_ascii` cannot split fused syllables, so a **reverse** partial span whose LAST letter-group is toned (`tengsek4`) reads as fully toned and keys the verbatim `poj:tengsek4`, which no record carries — the direct phrase-key lookup misses (the walker synth may still surface a phrase that passes the pin), never a wrong tone.
4. **TPS**: a TPS mark (ˋ ˪ ˊ ˇ ˫ ˙ ˆ) or a keyboard space is a typed **syllable close**, and every candidate must honour each one. A partial-tone span keys the toneless body as in case 3, then `lexicon::TonePin::TpsTyped { body, closes }` (built by `composing::shadow::tps_span_closes`) re-applies them per reading through `phonetics::tps_notone_prefix_boundary_tone`: a **mark** needs a syllable ending right there with that tone (`ㄉㄞ˫ㄨㄢ` keeps 大員 `tāi-uân`, drops 台灣 `tâi-uân`); a space **inside** the span needs a syllable end of any tone (§31 / §35 keep `ㄍㄠ`␣`ㄉㄞ˪` 交代 and `ㄇ`␣`ㄒㄧ` 毋是); the space closing an unmarked tail keeps tone 1 / 4 (§41). The boundary half stops a §35 Final reading of the next onset joining the closed syllable: `ㄌㄧˋㄫㆤ˫` does not compose 靈會 `lîng ē`, `ㄍㄚ`␣`ㄍㄚ` does not compose 角仔 `kak-á`. Closes sit in toneless-body offsets and align against the **matched** key on the exact / walker / partial-prefix paths (§35 substitutions are byte-length preserving), against the typed body on the custom / learned merges. A syllable with no mark and no space stays free. `span_is_fully_toned_tps` still reads a body ending in a mark as fully toned (`ㄍㄠㄉㄞˊ`), so its verbatim key keeps pinning the leading unmarked syllable to its no-mark tone.

This is a **conditional** contract, NOT "always filter": absent tone ⇒ all tones (intended), present tone ⇒ that tone only, per syllable. Do not "fix" the toneless-shows-all behavior — it is the feature.

**Why**: typing `tai5` must never surface `tai2` / `tai3`. The continuous path strips tone digits to build a toneless fused key for the no-tone affordance; an unconditional strip silently discards an explicitly-typed tone. The toned `tl:<tl_num>` keys exist in `dictionary.fst` (`dictionary/build/create_fst.py::romanization_keys`).

**Scope**: TL and POJ for cases 1–3, TPS for case 4 (its fully-toned key selection is `span_is_fully_toned_tps`, §41). **English** has no tone semantics (a trailing digit is not a tone) and keeps the toneless strip. The selection rule `composing::shadow::fst_body_for_span` (→ `span_is_fully_toned_ascii`, the `([a-z]+digit)+` grammar) is applied at all three continuous key-build sites: `left_anchored_keys_and_restrictions` (span-local), `build_partial_prefix_key`, and the walker edge (`fetch_walker_slot0_inner`). The walker's custom-dictionary rows stay toneless-keyed. Engine is the single source; there is no platform-side tone filtering.

**Tests**:
- **Rust engine** — `engine/composing/tests/continuous_explicit_tone.rs`: `explicit_tone_filters_to_typed_tone` (`tsua2` → 紙 only, not 蛇; `tsua5` → 蛇 only) + `toneless_input_still_surfaces_all_tones`. Case 3: `engine/composing/tests/continuous_partial_tone.rs` (`ting5sik` → 程式 only across span-local / walker slot 0 / partial-prefix `ting5s` / both custom paths, 豬 / 鎮 / 是 as strict-prefix controls) + `lexicon::continuous::typed_tone_pin_tests`. Case 4: `engine/composing/tests/tps_typed_tone.rs` + `walker_fixed_inputs_prod::tps_typed_marks_and_spaces_pin_the_reading` + `lexicon::continuous::tps_close_pin_tests` + `phonetics::tps` `tone_mark_number_*`. Unit: `engine/composing/src/shadow.rs::tests`. Golden `tl_numeric_single` / `tl_numeric_multi` (`engine/composing/tests/golden_fetch_at_pos.rs`) freeze the toned-key wire vector.
- **Dogfood** — S4 (cases 1 / 2), S46 (case 3), S121 (case 4).

---

## 18. Continuous input — longest-match prefix suppression

### `INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`

In continuous input, the span-local candidate strip surfaces, among the **single-syllable** spans anchored at buffer offset 0, **only the longest**. A shorter single syllable that is a strict prefix of a longer valid single syllable at the same anchor is **suppressed**:

1. `tai` → `台/代/大/…` (`tai`) only; the 2-letter `ta` (`乾/焦/大/…`) is NOT surfaced.
2. `tai5` → `台/臺/抬/…` (`tâi`) only; `ta` is NOT surfaced (compounds with §17).
3. `tsua` → `紙/蛇/…` only; the shorter `tsu` (`珠`) is NOT surfaced.
4. A bare short syllable with no longer extension typed still surfaces: `ka` → `共/家/加/…`, `m` / `ng`.
5. `iah8` → `抑/役/也/葉/…` (`ia̍h`) only; the `ia` family is NOT surfaced, even though `ia` also reads as `i`+`a`. Same for `ioh8`, `iok8`, `tiah8`, `tsiah8`.

Applies to **both toned and toneless input** — the suppression keys on span length, not tone. TL / POJ / TPS alike, with one TPS-only exception, guard (e).

**Straddled-prefix guard (guard (e), TPS only)**: a shorter single-syllable span survives when one syllable leaving its end runs past the longest single's end — the longest single then ate the start of the next syllable. Toneless `ㄌㄧㄏㄛ` (你好 lí-hó): the longest single `ㄌㄧㄏ` (li̍h) straddles `ㄏㄛ`, so `ㄌㄧ` (你 / 汝) is listed beside 裂; `ㄍㄚㄉㄚ` lists 家 ka beside 結 kat; `ㄍㄧㄨㄧ` keeps ki beside kiu. A syllable typed alone (`ㄍㄚㆵ`, `ㄍㄧㄨ`, mid-typing `ㄍㄚㄉ`) has nothing past it, so the prefix stays suppressed. TL / POJ keep the rule unchanged by maintainer decision.

**Multi-syllable phrase spans are NOT suppressed.** A left-anchored phrase span (`taibak` → 台北; `台`/`台語` inside `taigikhipuann`) is a different word, kept alongside the longest single syllable.

**Phrase-reading guard and its dead-end exception (guard (c) / (d))**: a shorter single-syllable span survives when the same bytes ALSO parse as a phrase (`ai` = `a`+`i` under `ainn`). That rescue does NOT apply when the span is a **closed dead end**: no lattice edge leaves its end AND the remainder after it is already tone-closed (TL / POJ: a typed tone digit `1..=9` anywhere in the remainder; TPS: a tone mark, or a §41 stripped-space barrier strictly after the end). `iah8` → `ia` is suppressed (`h8` can never grow into a syllable); `iah` (no digit) keeps `ia` because `h` may become `hoo`; `iakau3` keeps `ia` because `kau3` continues from it. A phrase span whose remainder is dead (`tai5gi2` + a mistyped `boh7`) is NOT touched.

**Why**: typing a complete syllable must not pollute the strip with shorter prefix syllables; at the anchor the user committed to the longer reading by typing its extra letters.

**Scope — display layer only, NOT segmentation**: the rule lives in `composing::shadow::left_anchored_keys_and_restrictions`, which recomputes the single-syllable ends via the `max_syllables = 1` primitive (`syllabifier::valid_span_endings_lowered`) and emits a key only for the longest single end + every phrase end. The segmentation **lattice keeps every edge** — the walker (`fetch_walker_slot0_inner`) and `shadow::span_min_syllable_count` still see every split, so a span segmentable only via a non-greedy shorter-first cut (`ta`+`nia` when `tan`+`ia` dead-ends) is unaffected (`docs/contributing/known-pitfalls.md` fix-location rule).

**Best-practice alignment**: longest-match matches khiin-rs `references/khiin-rs/khiin/src/data/segmenter.rs`. Deliberately NOT adopted: suppressing inside the lattice (reintroduces the greedy dead-end) and "keep only the single longest end" (drops phrase sub-words).

**Relationship to §17**: §17 selects the tone (key family) for a span; §18 selects which span lengths surface at the anchor. Orthogonal axes.

**Tests**:
- **Rust engine** — `engine/composing/tests/continuous_explicit_tone.rs::longest_match_suppresses_shorter_prefix_syllable`. Golden `tl_toneless_multi` / `tl_toneless_long_reach` (`engine/composing/tests/golden_fetch_at_pos.rs`). Guard (c) / (d): `engine/composing/tests/continuous_keys_tl_lattice.rs::{phrase_reachable_shorter_span_survives_suppression, closed_dead_end_prefix_is_not_rescued_by_its_phrase_reading, phrase_reachable_prefix_survives_while_the_remainder_is_open}`, `continuous_explicit_tone.rs::closed_tone8_syllable_does_not_trail_its_shorter_prefix_family`, `continuous_keys_tps.rs::closed_dead_end_prefix_is_suppressed_under_a_tone_mark`. Guard (e): `continuous_keys_tps.rs::{straddled_shorter_single_survives_beside_the_longest, longest_single_without_a_following_syllable_still_suppresses_the_prefix}`, `continuous_keys_tl_lattice.rs::straddled_prefix_guard_stays_tps_only`, `tps_hanji_conversion_prod.rs::a_toneless_first_syllable_a_longer_one_straddles_is_listed`.
- **Dev harness** — `engine/composing/tests/candidate_dump.rs` (`#[ignore]`): `cargo test -p composing --test prod candidate_dump -- --ignored --nocapture`.
- **Dogfood** — S5, S56, S120 (guard (e)).

---

## 19. Candidate strip + overlay — first-candidate keycap-color hint

### `INVARIANT_CANDIDATE_FIRST_KEYCAP_HINT`

The first candidate (engine ranker top, index 0) in **both** the candidate strip **and** the expanded candidate overlay renders with a **filled keycap-color background + rounded corners** (no border). Every other candidate has a transparent background until pressed/selected. Holds on iOS and Android.

- **Color** — the platform keycap background:
  - Android: `R.attr.key_bgColor` — `CandidateDisplayParams.themeKeyBgColor` (strip), `CandidateOverlayColors.firstCandidateBackground` (overlay).
  - iOS: `Color.keyboardButtonBackground` (non-Liquid-Glass); `keyboardButtonBackgroundLiquid(for:).opacity(0.4)` (Liquid Glass — below the pressed/selected 0.6).
- **Keying** — literal index 0 (strip: enumerated index; overlay: `RowItem.originalIndex == 0`).
- **Precedence** — pressed wins over the hint. iOS order: pressed > first candidate > selected (`CandidateViewStyle.resolvedBackgroundColor`) — the first candidate is always selected while typing and must still read as the hint. Android cells have only a pressed state.

**Why**: the ranker top is the default-commit candidate; a keycap-color fill signals it without a separate selection cursor. Filled background, not a border — every surveyed highlighting IME (Rime family) uses a fill.

**Cross-platform sites**:
- iOS — `CandidateViewStyle.resolvedBackgroundColor(isFirstCandidate:)` ← `CandidateButtonView` (strip) + `ExpandedCandidateGridCell` (overlay).
- Android — `candidateCellBackground` (`CandidateStripState.kt`) ← `CandidateStrip.kt` `CandidateCell` (strip) + `CandidateOverlayContent.CandidateCell` (overlay).

**Tests**: iOS `BuiltInThemesTests.testResolvedBackgroundColor_firstCandidateLightBeatsSelection` and Android `CandidateCellBackgroundTest` pin the colour precedence. No render assertion on either platform; dogfood **S6**.

---

## 21. Composing input — leading neutral-tone `--` marker is a document literal

### `INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL`

A leading ASCII-hyphen run typed when the composing buffer has **no syllable content yet** (engine `Phase::Idle`) is the neutral-tone (khinsiann) marker `--` (e.g. `--ah` 矣). It is committed to the document as **literal text** and does **not** enter the composing buffer; only the following syllable (`ah`) composes — so the underlined preedit equals the candidate (`ah`), not `--ah`. Matches the MOE reference IME.

- **Scope** — leading hyphens only (run typed from `Phase::Idle`). The full run is preserved. Keystrokes arrive one char at a time, so each leading `-` is its own `Start{"-"}` → literal insert, stay Idle; the first non-hyphen char starts composing.
- **Internal hyphens unaffected** — a hyphen typed AFTER syllable content (`tai-bak`, internal khinsiann `goa--si`) stays a composing-boundary delimiter. Internal `--` compounds (`tso̍h--ji̍t`) come from the candidate roman.
- **`--` is plain document text after insert** — editing over it uses the host editor's normal behavior. Picking after a literal `--` appends: `--ah` + roman `ah` → `--ah`; + Hanji 矣 → `--矣` (MOE keeps `--` in hanji output too).
- **Mechanism** — `transition.rs::begin_composition_or_insert_leading_hyphens` emits `CommitTextReplacingPreedit(run)` (a plain insert with no active preedit) and stays Idle, splitting any syllable remainder into `begin_composition`.

**Why**: the engine has no neutral-tone semantics; a leading `--` inside the composing buffer made preedit ≠ candidate. The document-literal model makes preedit == candidate.

**Engine site**: `engine/composing/src/transition.rs` — `begin_composition_or_insert_leading_hyphens`, wired into the `Start`/Idle and `Append`/Idle arms.

**Tests**: `composing/tests/intent_coverage.rs` (`intent_start_leading_hyphen_inserts_literal_stays_idle`, `intent_append_leading_hyphen_in_idle_inserts_literal`, `intent_start_leading_hyphens_then_syllable_splits`); dogfood **S8**.

---

## 22. Continuous input — slot-0 respects the dictionary separator form

### `INVARIANT_CONTINUOUS_SLOT0_RESPECTS_DICT_SEPARATOR`

When the continuous best candidate (index 0) is a single lexical dictionary word, its romanization shows the **dictionary's stored separator form** — `-` (`tâi-gí` 台語), `--` (`hōo--guá` 予我), or a space (`iā sī` 也是) — **not** a fabricated space-join. Genuine multi-word readings with **no** single full-span dict word (`tâi-gí khí-puânn` 台語齒盤) keep the walker's space-joined synthesis.

- **Why** — the walker synthesizes slot 0 by space-joining each edge's canonical roman. When the min-cost path splits a whole-word input into single-syllable edges (`hoogua` → 予 + 我), the synth hanji `予我` collides with the dict word while the synth roman `hōo guá` is a malformed rendering of `hōo--guá`; the `(roman, hanji, consumed_span)` dedupe cannot collapse a pair differing only in separator.
- **Fix location** — the display/assembly layer, NOT the cost/segmentation primitive (`docs/contributing/known-pitfalls.md` fix-location rule). At the Step-4 slot-0 prepend seam (`composing::continuous::assemble_candidates`), when a **FULL-coverage, full-span** dict candidate shares the synth's `hanji` and is the **same reading** (`roman_reading_eq`: strips ` `/`-`, keeps tone diacritics — `hōo guá` matches `hōo--guá`, never `hōo-guā`), that row's canonical roman is promoted to slot 0 and the synth dropped. The walker's reading choice is preserved; only the rendering changes.
- **Gates** (all required) — `coverage_kind == COVERAGE_KIND_FULL`; same `consumed_span`; same `hanji`; **no exact twin** already carrying the synth's roman (a one-edge path's synth IS that edge's dict roman; promoting its separator sibling would swap 囡仔 `gín-á` for `gín--á`); `roman_reading_eq`; `!synth.is_custom`. Word identity (Core Principle #6): `hōo--guá` (予我) and `hōo-guā` (戶外) stay distinct.
- **Scope** — engine-internal; every platform inherits it.

**Engine site**: `engine/composing/src/continuous.rs` — `roman_reading_eq` + the promote branch in `assemble_candidates` Step 4.

**Tests**: `composing/tests/continuous_slot0_dict_roman.rs` (`slot0_promotes_dict_khinsiann_form_over_space_synth`, `slot0_keeps_space_synth_for_genuine_multiword_reading`, `slot0_keeps_walker_pick_over_its_separator_sibling_cold`, `slot0_follows_the_selected_separator_form`) + the `roman_reading_eq` unit test in `continuous.rs`; dogfood **S9**.

## 23. Auto-space — attaching punctuation swaps the trailing space

### `INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP`

When auto-space is active (`isAutoSpaceEnabled` AND the last commit really wrote romanization) and the character immediately before the cursor is a single space, typing **attaching** punctuation deletes that space, inserts the punctuation, then re-inserts the space AFTER it: `guá ` + `?` → `guá? ` (NOT `guá ?`).

**The gate asks what the commit WROTE, not what the mode leads with** (every platform). Auto-space is a property of romanization — `guá beh khì` needs the gaps, 我欲去 does not — so the insertion sites and this swap read a `wroteRomanization` verdict resolved by the branch that picked the document string. A candidate with no Hanji (the §34 literal, an OOV name, a romanization-only custom entry) writes romanization under every mode; the swap key / a §42 split cell commits the script the mode does NOT lead with.

Truth table:

| commit | document string | `wroteRomanization` |
|---|---|---|
| candidate with no Hanji | the romanization | **true** |
| candidate with no Hanji, TPS layout (mobile) | its Bopomofo, what the cell shows (`tl_display_to_tps(roman)`) | false |
| Hanji candidate, Annotate in Brackets ON (mobile only) | `Hanji (romanization)` / `romanization (Hanji)` — on the TPS layout `Hanji (Bopomofo)` | true |
| Hanji candidate, Hanji-led, brackets OFF | the Hanji | **false** |
| Hanji candidate, roman-led, brackets OFF | the romanization | true |
| the other script (Output the Other Script / ⇧ + slot key / swap key / §42 roman cell) | that script alone | that script |
| the preedit AS TYPED (⇧Return, Enter at slot 0, punctuation with no desktop highlight) | the composition verbatim | the LAYOUT: TL/POJ true, TPS false |

**Desktop punctuation confirms the highlight** (macOS, Windows, Linux): document text typed during composition accepts the highlighted candidate in the cell's own script, then commits any remaining tail and appends the text. The candidate and punctuation reach the document in one commit; a finalized pick uses the engine's auto-space verdict. A prefix pick preserves the unconsumed tail as shown, and a TPS pick finishes the converted composition. With no visible highlighted cell, the preedit is committed as shown. A rejected candidate pick keeps the composition for another selection. Mobile candidate strips have no keyboard highlight and keep their explicit-pick behavior. Pins: desktop-core `tests/intent_executor.rs` punctuation cases; macOS `FullWidthPunctuationControllerTests.testPunctuationMidComposition_confirmsTheHighlightedCandidate`.

**The swap asks about PROVENANCE, not the mode.** The arm is set only where a space is actually written, so its existence IS the verdict — the desktop stores the caret position it must re-verify (`armedAutoSpaceCaret` / `ContextState::armed_auto_space`), mobile a bool (`isAutoSpaceArmed`). Only `isAutoSpaceEnabled` is re-read live at swap time, so flipping Hanji-first between the commit and the punctuation key does not turn an auto space stale.

**Every user event consumes the arm before dispatching.** Desktop takes the arm at the top of the key handler; mobile does it in `performInputEvent` at each event entry (keys, candidate taps, symbol-overlay taps, emoji). A keystroke that writes anything else leaves nothing to swap, so a user-typed space is never eaten. **Residual** (accepted, mobile): moving the caret by touch raises no IME event, so an arm can survive a caret move; the swap still requires a space immediately before the caret. The desktop re-verifies the caret position against the client.

- **Attaching set** (one copy: engine `phonetics/src/punctuation.rs`, the `IsAttachingPunctuation` op every platform asks) — sentence-end `。！？.!?`, clause separators `，,、；;：:`, CLOSING brackets/quotes `)）]】」』`. **Opening** brackets/quotes are excluded (they want a leading space). ASCII straight quotes are excluded (same glyph opens and closes).
- **Gating** — the swap fires only inside the auto-space insertion gate, so it never eats a user-typed space when auto-space is off, after a Hanji commit, or after a TPS Bopomofo commit. The cursor-preceding char must still be a space (mobile checks it; the desktop verifies the caret range too). `false` until the first commit.
- **Known approximation** — full-width punctuation still reads the output MODE. When a punctuation key commits raw preedit (mobile, or desktop with no highlighted candidate), Hanji-first gets the full-width glyph while the raw preedit earns the roman space: `taigi？ `. The same holds on the desktop after a prefix pick: the mixed `台gi` commits with its tail and earns the space (`engine/composing/src/transition.rs` `confirm_composition_then_insert` gates on Auto-Space and the layout, not on what the composition holds): `台gi？ `. A full-width punctuation policy question, left open.
- **Scope** — the host executes space insertion and the swap on all five platforms. Desktop punctuation uses the engine's `ConfirmCompositionThenInsert` transition to accept an optional highlighted candidate, finish the tail and include the literal and spacing in one commit; `CommitResolution.leaves_trailing_auto_space` tells the host whether to arm the swap after a successful write. Other insertion sites append their space in the platform binding. For a Continuous candidate commit and a NextWord prediction tap the verdict is the engine's (`engine/composing/src/commit_text.rs`; `CommitContinuous` / `CommitPrediction`). Mirrored with `CROSS-PLATFORM INVARIANT` comments.

Every platform sends which script it wants (`CommitContinuous.script`: LEAD for Return / a slot key / a click / an un-split mobile cell, OTHER for the desktop's Output the Other Script row (unbound since 2026-10-15) and ⇧ + slot key, HANJI / ROMAN for a §42 mobile split cell) plus the pick's `roman` and hanji; the engine answers `ComposingResponse.commit` — `CommitResolution { outcome, document_text, wrote_romanization, earns_auto_space }`, where `earns_auto_space` = FINALIZED ∧ wrote romanization ∧ no trailing `-`; the platform ANDs the live Auto-Space setting (iOS / Android `ContinuousCommitOutcome.finalized(earnsAutoSpace)`, desktop-core `CandidateCommitOutcome::Finalized { earns_auto_space }`). OTHER on a one-script candidate or under Romanization Only is IGNORED: nothing written. The TPS layout ignores Candidate Display (Hanji leads) and renders the lead's romanization as Bopomofo. A NextWord prediction tap sends `CommitPrediction { script, roman, hanji, canonical_tl }` and gets the same `CommitResolution` (FINALIZED with the document text, or IGNORED — nothing written); iOS / Android write `document_text` and AND the live setting into `earns_auto_space` (`PredictionCommit`).

**Platform sites**: iOS `ActionHandler+Suggestions.swift` — `continuousPick(for:)` / `predictionPick(for:)` / `commitScript(for:)`, `rawPreeditWritesRomanization` / `appendAutoSpaceIfEarned`; `ActionHandler.swift` — `performInputEvent` / `armAutoSpaceSwap` / `isAutoSpaceSwapArmed`; `ActionHandler+KeyActions.swift` — `insertNonComposingCharacter`. Android `candidates/CandidateClickHandler.kt` — `continuousPick` / `predictionPick` / `commitScript`, `rawPreeditWritesRomanization` / `appendAutoSpaceIfEarned`; `ime/core/TaigiKeyboard.kt` (the arm); `TextInputKeyHandler.kt` — `commitNonComposingCharacter`; classifier `RustEngineBridge.isAttachingPunctuation` (`engine/PhoneticsBridge.kt`). Engine `composing/src/commit_text.rs` — `resolve_commit_text` / `commit_resolution` / `finalized_composition_resolution` (the desktops' literal-commit verdict, `CommitRaw` / `CommitAsShown`); `transition.rs` / `external_insert.rs` — `confirm_composition_then_insert` / `augment_insert`. macOS, Windows, Linux (`taigi-desktop-core`) `composing/outcomes.rs`, `policies/auto_space.rs` (`is_gate_active` only), `composing/intent_executor.rs` — `auto_space_gate` / `append_auto_space` / `swap_auto_space`; each shell's `IntentSurface` adapter holds the arm (`taigi-windows-tsf/src/session.rs` `TsfIntentSurface`, `taigi-linux-core/src/executor.rs` `KeySurface`); on macOS the arm is `TaigiInputController.armedAutoSpaceCaret`, and `ComposingBackend.panel` asks `RustEngineBridge.isAttachingPunctuation` only where a swap can happen.

**Tests**: engine `phonetics/src/punctuation.rs` (attaching set) + `phonetics/tests/op_coverage.rs`, through each bridge in iOS `RustEngineBridgeTests`, Android androidTest `RustEngineBridgeTest`, macOS `ComposingBackendTests.testTheSwapCheck_isAskedOnlyWhereASwapCanHappen` and desktop-core `policies/auto_space.rs`; the verdict truth table in `composing/src/commit_text.rs` (`truth_table`, `tps_truth_table`, `resolution_earns_auto_space_only_on_a_romanized_final_commit`), `composing/tests/continuous_commit_resolution.rs`, `composing/tests/prediction_commit_resolution.rs` and `composing/tests/confirm_composition_then_insert.rs`; the request each mobile tap sends in iOS `ActionHandlerContinuousPickTests.swift` + `ActionHandlerPredictionSuggestionsTests.swift`, Android `ContinuousPickTest.kt` + `PredictionWordsBuilderTest.kt`; iOS `RustEngineBridgeContinuousTests` (through the real engine); the platform auto-space predicate in iOS `ActionHandlerCommitRulesTests.swift`, Android `AutoSpaceVerdictTest.kt`; macOS `ComposingManagerCandidateTests.testCommitCandidate_writesWhatItsCellShows_andSpaceTheOtherScript` + `AutoSpaceControllerTests.swift`; Windows / Linux `tests/composing_manager.rs` + `tests/intent_executor.rs`. The mobile document-mutation swap stays dogfood-gated (**S10**).

## 24. NextWord — prev-Hanji lookup + read-layer reading-variant dedup

### `INVARIANT_NEXTWORD_PREV_HANJI_LOOKUP`

`user_association.db` separates **three** identities. All three hold on every platform — one engine store (`userdata::UserAssociationStore`, `engine/userdata/src/association.rs`).

**1. Storage identity — `(prev_word, prev_tl, next_word, next_tl)`.** Schema **v6**: the UNIQUE key carries `prev_tl`, so Core Principle #6 binds the bigram's **previous** side as well as its next. 重/tîng → 複 and 重/tāng → 複 are two rows, not one; a narrower key would overwrite `prev_tl` and sum counts across readings.

**2. Recall scope — `prev_word` (Hanji) ALONE.** `prev_tl` is a **ranking signal, not a hard filter**. A stored row whose non-empty `prev_tl` differs from the query roman is STILL returned. That recovers associations learned under the other reading of a polyphonic Hanji and from a different commit path or app version (older continuous commits stored a raw `prev_tl` such as `taigi`). The bundled `association.bin` is keyed differently — word key `(Hanji, TL)` first, character key as backoff (`INVARIANT_NEXTWORD_WORD_KEY_BACKOFF`).

**3. Prediction identity — `(next_word, next_tl)`, at most one user row each.** A query pulls back several rows predicting the same word under different previous readings. `engine/nextword/src/filter.rs` merges by `(hanzi, tl)` and **adds** scores, and `calculate_user_score` contributes a fixed `LEARNING_BONUS` per row — summing would hand one word several bonuses because its context word is polyphonic. **When user evidence exists, each predicted `(next_word, next_tl)` receives exactly one user-row contribution.** Dictionary rows still add on top.

- **The filter holds the rule, the SQL holds the order.** The cap lives in `filter.rs`; it keeps the FIRST user row per `(hanzi, tl)`, so which reading's evidence is used is decided by the store's read order (`UserAssociationStore::rows_following`): `ORDER BY CASE WHEN prev_tl = ? THEN 0 WHEN prev_tl = '' THEN 1 ELSE 2 END, count DESC, last_used DESC, id ASC`. That ordering is a contract — nothing between the SQL cursor and the `FilterPredictions` build may reorder rows (`dispatch::user_data::handle_nextword` reads them; `predict::expand_predict_next` appends them after the bundled rows unchanged); `rows_following` carries a `CROSS-PLATFORM INVARIANT` note. `id` is the final tiebreak, so the v5→v6 rebuild **copies `id`**.
- **Rank before truncate** — the tier ordering keeps exact-`prev_tl` rows inside the `limit * 2` over-fetch window. At `DEFAULT_LIMIT = 30` the window is far wider than the duplicate count a real polyphonic context produces.
- **One index** — `(prev_word, prev_tl)` serves the recall `WHERE` from its left prefix and the tier ordering from the second column.
- **Existing rows are grandfathered** — the v5→v6 rebuild preserves every row (widening a UNIQUE key cannot conflict). A row merged under the old key keeps its last-writer `prev_tl` and summed count; nothing is split or deleted. A v2 stamp is dropped rather than migrated; any other pre-v6 table that carries the row columns is rebuilt with its rows (including an iOS pre-v3 table, which iOS used to drop — NAMED DIVERGENCE, user-data-engine-roadmap U7), and one without them is dropped.

### `INVARIANT_NEXTWORD_WORD_KEY_BACKOFF`

The bundled `association.bin` lookup (`engine/lexicon/src/search.rs::lookup_associations`, reached from `engine/dispatch/src/predict.rs` through `lexicon::api::lookup_associations(previous_word, previous_tl, …)` with `PredictNext.word` / `.roman`) tries the committed word's **word key** `hanji\u{1}tl` (`lexicon::association_reader::word_key`, byte-exact) and backs off to the **character key** only when the word key yields no row under the enabled sources. An empty `roman` (backspace, hanji-only commit) goes straight to the character key. Word → character, one direction, never a merge: a word-key hit is not padded with character-key rows (the n-gram model's own lower-order backoff).

- **Filter before cut** — `AssociationReader::lookup` applies the source mask while reading and stops after `limit` survivors, so a key whose top entries come from disabled sources still yields its enabled ones; a word key counts as absent only when **no** entry passes (`lexicon/tests/parity.rs::lookup_associations_filters_before_limit`).
- **Reading forms** — `roman` is canonical display TL on the candidate-commit paths; a raw continuous tail, a TPS commit or a legacy slice may carry another form, which simply misses the word key and backs off. Learned rows are unaffected (recall scope above).

**Tests**: engine `dispatch/tests/predict_next.rs` (production lexicon) — `known_reading_predicts_from_word_key` (食/tsia̍h → 酒 first; character key 食 → 飯 first), `unknown_reading_backs_off_to_character_key`, `word_key_emptied_by_source_filter_backs_off`, `word_key_hit_is_not_padded_from_character_key`.

### `INVARIANT_NEXTWORD_LOOKUP_KEY_LAST_GRAPHEME`

The bundled character-key lookup (the backoff above, and every empty-`roman` query) keys on the committed word's **last user-perceived character, whole** — and so does the backspace re-predict. A supplementary-plane Hanji (𣍐 U+2334D, a UTF-16 surrogate pair) is one key; a UTF-16-unit read (Kotlin `String.last()`) would key on the low surrogate alone and find no predictions.

**Sites**: the prediction key is engine-side — `engine/lexicon/src/search.rs::lookup_associations` takes the last Unicode scalar of `PredictNext.word` (every `association.bin` key Hanji is one scalar). The backspace re-predict key stays platform-side: iOS `ActionHandler+KeyActions.swift::handleBackspaceForNextWord` (`String.last`), Android `lastGrapheme` (`ime/text/keyboard/TextInputKeyHandler.kt`, ICU `BreakIterator`) in `NextWordController.handleBackspaceForNextWord`.

**Tests**: engine `dispatch/tests/predict_next.rs::supplementary_plane_hanji_is_one_lookup_key` (袂𣍐 → 使), Android `TextInputKeyHandlerTest` `INVARIANT nextword lookup key keeps a supplementary Hanji whole`.

### `INVARIANT_NEXTWORD_READ_LAYER_DEDUP`

`engine/nextword/src/filter.rs` collapses separator/tone-only romanization variants of the SAME next word into ONE prediction, AFTER the `(hanzi, tl)` merge and BEFORE shaping/sort/truncate. Rows are grouped by `(hanzi, phonetics::toneless_reading_key(tl))`. Canonical selection is **separator-based** — a genuine multi-syllable reading is always separated in canonical TL, so a no-separator row sharing the toneless key can only be a fused raw keystroke slice:

1. the unique separator-bearing row (`tâi-gí`) is canonical; every no-separator row (`taigi`) folds into it. Separator-bearing rows that are the SAME reading (`hōo-guá` vs `hōo--guá`) fold together too.
2. the group is left UNTOUCHED when (a) NO row has a separator — a bare toneless single syllable may be a genuine tone-1 reading (`當/tang`), so it must never fold into `當/tàng`; or (b) ≥2 separator-bearing rows are genuinely distinct readings (`tāng-bīn` vs `tàng-bīn`).

This protects genuine polyphonic Hanji (Core Principle #6).

- **Residuals** — (a) a single-syllable raw-vs-toned duplicate (`我/gua` vs `我/guá`) is NOT folded (indistinguishable from a tone-1 reading); (b) `toneless_reading_key` does NOT fold spelling families (`ch↔ts`, `oa↔ua`), so a POJ-spelled raw `next_tl` does not collapse onto its TL canonical. Both are closed write-side by §25.

**Scope**: engine-side throughout (`docs/contributing/rust-migration-policy.md` §6) — schema, migration and SQL recall in `engine/userdata`, the per-prediction pick and dedup in `nextword/src/filter.rs`. A platform sends `PredictNext` (`word`, `roman`, `now_ms`) and never sees a user row.

**Platform sites**: schema + migration — `engine/userdata/src/association.rs` (`apply_schema` / `rebuild_to_v6`). Recall — `UserAssociationStore::rows_following`, read by `engine/dispatch/src/user_data/with_stores.rs::following_rows` (over-fetch `effective_prediction_limit * 2`) for each `PredictNext`; only iOS and Android send `PredictNext` — macOS, Windows and Linux have no prediction surface by design (the engine still records their bigrams). Dedup: `engine/nextword/src/filter.rs::collapse_reading_variants` + `engine/phonetics/src/api.rs::toneless_reading_key`.

**Tests**: engine `filter.rs` — `second_user_row_for_one_prediction_does_not_stack_its_bonus`, `first_user_row_wins_regardless_of_the_later_row_count`, `dict_and_user_scores_still_sum_for_one_prediction`, `different_next_tl_stays_two_predictions`, the `collapse_*` variant tests, `phonetics/src/api.rs` `toneless_reading_key_*`. `engine/userdata/tests/takeover.rs` pins the schema against files built from each platform's shipped DDL (`a_v5_association_table_is_rebuilt_under_the_v6_key_with_its_rows`, iOS v4, v3 backfill, Android v1 kept, v2 dropped, no-row-columns dropped). `engine/userdata/tests/stores.rs` pins the pair key (`bigrams_are_keyed_on_both_readings_and_written_in_order`) and the delivery order (`predictions_read_the_exact_reading_first_then_untagged_then_the_others`); `engine/dispatch/tests/predict_next.rs` the bundled + learned join. `RustEngineBridgeNextWordTests.swift` covers FFI dedup parity. Dogfood **S11**.

## 25. Continuous input — commit carries canonical TL (write-side identity)

### `INVARIANT_NEXTWORD_CONTINUOUS_CANONICAL_TL`

A continuous-input candidate commit records the **candidate's canonical TL** as the NextWord association romanization (`prev_tl` / `next_tl`), identical to a span-local candidate commit — NOT the raw typed slice (`taigi`), which would split `user_association.db` rows by form.

- **Identity carrier** — each continuous candidate carries `CandidateMessage.canonical_tl`, snapshotted at construction BEFORE the recase / POJ-render passes rewrite `roman`: `DictionaryRecord.tl` for dict-backed candidates; `phonetics::api::canonical_tl_form(native_roman, mode)` for custom / walker-synth candidates. Populated for hanji-present candidates too — `(hanji, canonical-TL)` is the word identity (Core Principle #6).
- **Round-trip** — the platform stores `canonical_tl` on the candidate's tap metadata and forwards it as `CommitContinuous.association_tl`. `commit_continuous` stores it on `NailedSegment.association_tl` and uses it as the `roman` of the `WordSelected` effect — the terminal word's and each `preceding` segment's (§40) — and of the `UpdateLastSelectedWord` nail / unnail handshake.
- **Fallback** — when `association_tl` is empty (a caller that sends none, or a TPS-OOV hanji-absent candidate with no recoverable dict TL), the engine uses the raw committed slice. `NailedSegment.raw_text` stays the sole authority for span / unnail mechanics; `association_tl` is never overloaded onto `raw_text` or `display_text`.
- **Display untouched** — `canonical_tl` is never consulted for the document commit (that is `display_text`).

**Scope**: engine `composing` (proto + `RawCandidate.canonical_tl` + `NailedSegment.association_tl` + `commit_continuous`) and `lexicon` (candidate construction); platform round-trip is iOS / Android tap metadata.

**Platform sites**: iOS `RustEngineBridge+Composing.swift` (`ContinuousCandidate.canonicalTl`, `composingCommitContinuous(associationTl:)`) + `TaigiAutocompleteService.swift` + `ActionHandler+Suggestions.swift` + `ComposingManager.swift`. Android `RustEngineBridge.kt` / `ComposingBridge.kt` + `TaigiAutocompleteService.kt` + `TaigiWord.MetadataKeys.CANONICAL_TL` + `CandidateClickHandler.kt` + `ComposingManager.kt`.

**Tests**: engine `lexicon/src/continuous/` (`record_to_candidate` / `custom_entry_to_candidate` canonical_tl), `composing/src/continuous.rs` (`poj_render_rewrites_roman_but_preserves_canonical_tl`), `composing/src/requests.rs` (`raw_to_proto_candidate` propagation), `composing/tests/continuous_phase.rs` (`bug1_*` assert `nw.roman` == canonical TL). iOS `RustEngineBridgeContinuousTests.swift` + `TaigiAutocompleteServiceContinuousTests.swift`. Android `ContinuousSuggestionsContractTest.kt`. Dogfood **S11**, **S12**.

## 26. Custom dictionary — cross-input-mode search via side table

### `INVARIANT_CUSTOM_DICT_CROSS_MODE`

A user-added custom-dictionary entry is findable whichever input mode (TL / POJ / TPS) the user types in. A word stored as a single native `roman` (e.g. POJ `chiah` / 食) surfaces when the user types its TL form (`tsiah`), its POJ form (`chiah`), OR its TPS Bopomofo, in continuous input AND the settings dictionary browser.

- **Search axis = three-family, mirroring the system dictionary** — each entry materializes the full `{tl, poj, tps} × {num, notone, abbrev}` (+ TPS er/or dialect variant) key bundle into a `custom_search_key(entry_id, family, form, key)` side table, the analogue of the system FST's three families (`dictionary/build/create_fst.py::romanization_keys`). One engine function derives it — `phonetics::custom_search::derive_custom_search_keys(roman)` (raw roman → canonical TL → all families), called in-process on every write (`userdata::derive_custom_search_keys`, §10).
- **Query is family-native** — `phonetics::custom_search::derive_custom_query_key(input, input_mode)` returns the key for the **effective family** = `contains_tps(input) ? tps : parse(input_mode)`, mirroring the composing dispatch TPS upgrade (`engine/composing/src/requests.rs::handle_fetch_at_pos`); the TPS layout's `input_mode = "tps"` reads as the TL family, so the family MUST come from the raw input, not settings alone. The store matches `WHERE family=? AND form IN (?, 'abbrev') AND key LIKE ?||'%'` (`CustomDictionaryStore::rows_matching`) — for the settings dictionary search (`UserDataRequest.search_custom_entries`); the keystroke fetch (`dispatch::user_data::with_stores::buffer_rows`) uses `CustomDictionaryStore::keystroke_rows` (abbreviation matched whole, §43).
- **Internal-consistency contract, NOT system-FST parity** — custom keys are matched only within `custom_dictionary.db`; the invariant is WRITE-key == QUERY-key for the same word.
- **Raw `roman` untouched** — the stored `roman` and `composing::shadow::custom_toneless_key` (which reads `roman`, handed over as `composing::UserRows.custom`) are unchanged; the legacy `notone`/`abbrev`/`roman_num` columns are neither written nor queried (§10).
- **Non-destructive migration** — on takeover the engine creates the side table when absent and re-derives every entry's keys once (`CustomDictionaryStore::rederive_search_keys_if_needed`, user-data-engine-roadmap U7); never DROPs user data.
- **Each process opens the user data (lifecycle, load-bearing)** — the engine answers without custom rows until the stores are open in THAT process. The iOS keyboard extension is a separate process from the host app (the App Group shares files, not the engine), so `KeyboardViewController.viewDidLoad` MUST open them (`UserDataOpening.open(in:)`, Full Access only) as the app does in `TaigiKeyboardApp.init`, or the keyboard surfaces NO custom entries in ANY mode. Android opens once in `TaigiKeyboardApplication.onCreate`.

**Scope**: engine `phonetics` (`custom_search.rs` + `tps_num_from_tl` / `tps_abbrev_from_tl`) + engine `userdata` (`custom_dictionary.db` is engine-owned, `rust-migration-policy §6`) — derivation, side table, DDL and the query exist once, in Rust.

**Platform sites**: engine `engine/userdata/src/custom_dictionary.rs` (schema, `upsert` / `batch_import` side-key write, `rows_matching`, takeover re-derivation), `engine/userdata/src/stores.rs` (`derive_custom_search_keys`), `engine/dispatch/src/user_data/with_stores.rs` (`buffer_rows`), `engine/userdata/src/requests.rs` (`search_custom_entries`). Platforms only send ops: the keystroke fetch carries no entries (`FetchAtPos.custom_dictionary_disabled` is the Enable Custom Dictionary gate); the settings search goes through `SearchCustomEntries` — iOS `DictionarySearchService.swift` → `RustEngineBridge+UserData.swift`, Android `DictionarySearchService.kt` → `UserDataBridge.kt`, Windows / Linux `taigi_desktop_core::engine::dictionary_search` → `engine::user_data::search_custom_entries`.

**Tests**: engine `phonetics/src/custom_search.rs`; `engine/userdata/tests/stores.rs` (`an_added_entry_is_found_by_its_key_from_another_romanization_and_by_prefix`, `the_engine_derivation_finds_a_poj_entry_typed_as_tl`, `keys_written_by_an_older_derivation_are_rederived_once`); `engine/userdata/tests/takeover.rs` `every_released_phone_custom_dictionary_is_re_derived_under_a_stamp_its_app_accepts`; `engine/dispatch/tests/user_data_reads.rs::engine_reads_answer_what_the_same_rows_answer`. The per-process open lifecycle is NOT unit-covered — the iOS test host never opens the user data (`TaigiKeyboardApp.init` skips `UserDataOpening.open(in:)` under XCTest) and the store tests open their own stores; dogfood **S13** (type a custom word in TL / POJ / TPS in a freshly-opened keyboard).

## 27. Custom dictionary — row-count capacity

### `INVARIANT_CUSTOM_DICT_CAPACITY`

The custom-dictionary table is bounded by a hard row cap of **30000** on every platform. User-authored entries are NEVER silently evicted (unlike `user_frequency` / `user_association`, learned data pruned oldest-first); the cap **grandfathers** every existing row and **blocks new inserts** beyond the limit. CSV export is the user's escape hatch.

- **Single save** (`UserDataRequest.save_custom_entry` → `CustomDictionaryStore::upsert`) — inserting a NEW id at/over the cap is refused (`CustomDictionaryRefusal` `FULL`). The refusal is swallowed at the call boundary (iOS `CustomDictionaryViewModel.save` `try?`; Android `CustomDictionaryViewModel.save` logs it) — no "dictionary full" UI; the guard is defensive. Updating an **existing** id always bypasses the cap.
- **Bulk import** (`ImportCustomCsv` → `CustomDictionaryStore::batch_import`) — (1) a file whose own row count exceeds the cap is refused outright (`FULL`, iOS `tooManyEntries`); (2) each chunk re-counts inside its transaction, stops at the cap, and reports overflow (with duplicates) in the **skipped** count. A `.taigi` restore (`ImportBackup`) uses the same loop without gate (1) (`import_until_full`): it fills the room left, on every platform.
- **TOCTOU** — check and write run in the same `BEGIN IMMEDIATE` transaction on the store's one serial writer (`immediate_transaction`).
- **Cap is one engine constant** — `CustomDictionaryStore::MAX_ENTRIES` (`engine/userdata/src/custom_dictionary.rs`), also the CSV decoder's row limit (`engine/userdata/src/requests.rs::import_custom_csv`). Changing it requires this doc + the invariant tests in the same PR.

**Scope**: engine `userdata` store policy, reached through the `UserDataRequest` ops. Platforms only word the refusal (iOS `UserDataClient.swift` `tooManyEntries`, Android `CustomDictionaryScreen.kt`).

**Platform sites**: `engine/userdata/src/custom_dictionary.rs` (`MAX_ENTRIES`, `upsert` guard, `batch_import` pre-check, `import_until_full`), `engine/userdata/src/backup.rs` (`import_backup` → `import_until_full`), `engine/userdata/src/requests.rs` (`refusal` → `CustomDictionaryRefusal`).

**Tests**: `engine/userdata/tests/stores.rs` (injectable `entry_limit`) — `the_capacity_refuses_a_new_entry_but_never_an_edit`, `batch_import_skips_duplicates_stops_at_the_cap_and_refuses_an_oversize_file`, `a_restore_fills_the_room_left_past_duplicates`. Dogfood **S14**.

## 28. User frequency — `(Hanji, canonical-TL)` pair-key identity

### `INVARIANT_USER_FREQ_PAIR_KEY`

User-selection frequency is keyed by the **`(display_text, canonical_tl)` PAIR** (Core Principle #6), never `display_text` alone. Polyphonic Hanji (重/tîng 重複 vs 重/tāng 重量) accumulate **separate** counts and rank independently.

- **Identity carrier** — `canonical_tl` is the candidate's canonical-TL reading, snapshotted BEFORE the POJ-render pass (= `RawCandidate.canonical_tl`, §25). Dict hit: `DictionaryRecord.tl`; custom/OOV/walker-synth: `phonetics::api::canonical_tl_form(roman, mode)`.
- **Engine `FrequencyMap`** — nested `display_text → (canonical_tl → FrequencyData)`. `get(display, tl)` is **tolerant**: exact `(display, tl)` first, then the legacy `(display, "")` bucket; it **never sums** — an exact bucket shadows the legacy one. A miss returns `FrequencyData::default()` (neutral cold-start: `user_weight = 0.0`).
- **Legacy `canonical_tl == ""` bucket** — older rows and old-backup imports that cannot be re-keyed carry `tl = ""`. Every reading of that `display_text` falls back to it until re-learned with its own reading. Self-healing; the legacy row is never deleted (its count has no reading attribution). New writes carry a real canonical TL (empty only for TPS-OOV / English rows).
- **Schema** — `user_frequency` identity is `UNIQUE(word, tl)`. The engine store rebuilds any file still without `tl` (gated on the shape, not the stamp) in one `BEGIN IMMEDIATE` transaction (`engine/userdata/src/frequency.rs::migrate_to_pair_key_if_needed`): create → copy with `tl = ''` → drop → rename. `id` / `count` / `last_used` / `created_at` preserved; `idx_word` dropped (the `UNIQUE(word, tl)` autoindex serves `WHERE word`).
- **Write path** — every commit records `(displayText, canonicalTl)`, whichever script it wrote. A Continuous pick is recorded by the engine itself (`CommitContinuous` with a `CommitScript` hands `Applied.usage` to `dispatch::user_data::with_stores::handle_composing`, once per nailed or finalized pick, only with the stores open). So is a NextWord prediction pick (`CommitPrediction` hands `Applied.usage` with no `hanji`, so no learned-phrase touch, D2). Android reports an English word through its `UsageRecorder` as `UserDataRequest.record_usage` (`RecordUsage{display_text, canonical_tl}`). Either way `UserFrequencyStore::record` — `ON CONFLICT(word, tl)`.
- **Read path** — inside the one `FetchAtPos`: `dispatch::user_data::with_stores::frequency_map` loads every `(word, tl)` row for the display texts the neutral fetch offered (`UserFrequencyStore::rows_for_words`) into a `FrequencyMap` (`composing::UserRows.frequency`), resolved via the tolerant `get`.
- **Backup** — `.taigi` carries `tl` per frequency row; restore (`UserFrequencyStore::import_merge`) upserts `ON CONFLICT(word, tl)`, keeping the larger count. A v1 backup has no reading → legacy `tl = ""` bucket. The `.taigi` backup is the only export path; there is no frequency viewer or CSV, and recording is always on.

**Scope**: engine `ranking` (`FrequencyMap`) + `lexicon::continuous` + `composing::continuous` read the pair key; engine `userdata` owns storage, write, migration and backup; `dispatch` builds the map. Platforms only report prediction picks.

**Platform sites**: proto `RecordUsage.canonical_tl` (`user_data.proto`). Engine `ranking/src/score.rs` (`FrequencyMap`), `lexicon/src/continuous/`, `composing/src/continuous.rs`, `userdata/src/frequency.rs`, `userdata/src/backup.rs`, `userdata/src/requests.rs` (`record_usage`), `dispatch/src/user_data/with_stores.rs` (`frequency_map`), `composing/src/transition.rs` `commit_continuous` / `commit_prediction` (`Applied.usage`; the desktops have no prediction strip). English words: Android `ime/text/composing/UsageRecorder.kt` + `CandidateClickHandler.kt`.

**Tests**: engine `ranking/src/score.rs` (`frequency_map_pair_key_separates_homograph_readings`, `frequency_map_legacy_empty_tl_is_tolerant_fallback`, `frequency_map_missing_pair_is_neutral_cold_start`); `engine/userdata/tests/stores.rs::recording_counts_the_pair_and_reads_back_by_word`; `engine/userdata/tests/takeover.rs` `a_pre_pair_key_frequency_table_keeps_its_rows_ids_and_counts` + `a_pair_key_frequency_table_left_at_version_zero_is_not_rebuilt`; `engine/dispatch/tests/user_data_writes.rs::the_engine_writes_what_the_platforms_wrote` + `engine_resolved_picks_are_counted_by_the_engine`, `user_data_reads.rs::engine_reads_answer_what_the_same_rows_answer`. Dogfood **S15**.

## 29. User-data backup — excluded from OS automatic backup

### `INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP`

All four user-data SQLite databases — `user_frequency`, `user_association`, `custom_dictionary`, `learned_phrases` (§50) — and the `<file>.pre-engine` copies the engine's first takeover leaves beside them are **marked excluded from OS automatic backup** (iOS iCloud / Android Auto Backup + device-transfer). Learned behaviour and user-authored words stay on-device; the **manual `.taigi` export/import is the only cross-device path** (product decision). `isExcludedFromBackup` is a system *directive*: the contract is that the data is *marked* excluded.

- **iOS** — `UserDataOpening.open(in:)` marks every store file and pre-takeover copy that exists before the open, then again after a detached repeat open that waits for the takeover (so a brand-new install is marked on first launch). Each process (app, keyboard) runs it; a failed `setResourceValues` is swallowed.
- **Android** — `android:allowBackup="false"` disables Auto Backup + device-transfer; `backup_rules.xml` (API <31) and `data_extraction_rules.xml` (API 31+, `cloud-backup` AND `device-transfer`) exclude the root domain as defense-in-depth. The rules + manifest carry comments citing this invariant.
- **Journal mode** — both phones open the stores with the `DELETE` journal (`OpenUserData.journal`), so the only persistent file per DB is the main `.db`. If WAL is ever enabled, the sidecars need the same exclusion.
- **UX footgun (intentional)** — a reinstall / new device does NOT auto-restore; the user must export a `.taigi` manually. Backup UI copy must not imply OS / iCloud restore.

**Scope**: platform backup policy over engine-owned files; the engine never sees backup policy.

**Platform sites**: iOS `Engine/UserDataOpening.swift` (`excludeFromBackup(in:)` over `RustEngineBridge.userDataFileNames` + `.pre-engine`; `DELETE` journal named in `RustEngineBridge+UserData.swift` `userDataOpen`). Android `AndroidManifest.xml`, `res/xml/data_extraction_rules.xml`, `res/xml/backup_rules.xml`.

**Tests**: iOS `BackupExclusionTests.swift` (`test_INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP_*` — every store file and `.pre-engine` copy reads back `isExcludedFromBackup == true`; a file not yet written is not created). Android: manifest / XML posture, dogfood **S16**.

## 30. Composing display is literal — no spelling conversion (TL + POJ)

### `INVARIANT_TL_INPUT_LITERAL_NO_POJ_SPELLING_FOLD`

(Label kept from the TL-only first slice; the contract spans TL and POJ.) In **both** TL and POJ input modes, the composing **display** — the preedit and the Enter/Space final-commit text — shows exactly what the user typed: only the tone digit becomes a tone mark, NO spelling family conversion (`ch`↔`ts`, `oa`↔`ua`, `oe`↔`ue`, `eng`↔`ing`, `ek`↔`ik` all stay). `ting2`→`tíng`, `goa2`→`goá` (TL) / `góa` (POJ), `teng2`→`téng`.

Permissive tone placement: `normalize_tone` processes each tone digit in sequence; the digit closes the letters typed since the previous digit or separator. When those letters are one valid syllable, the mark lands where `tl::place_tl_tone_mark` / `poj::place_poj_tone_mark` put it (POJ `hoang2` → `hoáng`, `hoannh5` → `hoâⁿh`). Otherwise the same rules mark the last vowel cluster through the segment end (Taigi Telex last-cluster placement); a vowel-less segment takes the tone only on a syllabic `m` / `ng` that ends it, optionally before `h` (`hngm2` → `hngḿ`), else the digit stays (`mp3`, `html5`). A vowel-less syllable marks its final syllabic nasal, as `taigi-converter` does (`nng5` → `nn̂g`, `ngm2` → `ngḿ`). A new tone replaces a pasted tone mark on the letters it is placed among (`goá2` → `góa`); marks on earlier clusters stay (`tâigi2` → `tâigí`). `tai5gi2` → `tâigí`, `goa2ai3li2` → `goáàilí` in TL. In POJ mode a TL-spelled `ua…` / `ue…` cluster takes TL placement (`gua2` → `guá`). Marks remain visible while the next syllable is typed (`tai5g` → `tâig`). Tones 1 and 4 consume their digits with no mark (`tai1` → `tai`, `bak4` → `bak`). Typed separators stay exactly as entered; no boundaries are inferred. A digit without a preceding tone-bearing letter stays literal (`t2`, `tai-2`), and so does every digit of a run of two or more (`covid19`, `win10`, `a23`). English and TPS bypass roman tone placement. POJ double-tap folds and nasal-marker case still run around the placement step.

The preedit, Show Typed Text First candidate, Enter/Space raw-text commits and candidate-tap commits use the same rendered literal, with or without Show Typed Text First, on all five platforms (engine behavior, no setting). Dictionary lookup, spelling conversion and ranking are unaffected.

Three axes, deliberately decoupled:

- **Composing display** (`normalize_tone` → `permissive_tone`): tone placed **directly on the typed literal letters**; the mode selects the glyph and vowel-priority convention. No spelling fold or inferred separator.
- **FST search** (`canonicalize_poj_shadow`): TL / English / TPS use the encoding-only `TL_ENCODING_RULES` (POJ-glyph→ASCII, no spelling fold); POJ stays glyph-only. No `oonn→onn` fold at whole-buffer scope (§45). The TL special finals `eng` / `ek` (`docs/phonetics/taigi-phonetics-reference.md` §3.2.6) are never collapsed to `ing`/`ik`.
- **Cross-mode identity** (`user_frequency.db` + NextWord) — **stays canonical** so learning is mode-independent. Keyed on `canonical_tl_form` + `CommitContinuous.canonical_text` / `association_tl`, separate proto fields from the committed `display_text`: POJ `goa` and TL `gua` both key `guá`. `canonical_tl_form(Tl)` drops only `eng→ing`/`ek→ik` (keeps `ch`/`oa`/`oe` so a POJ-form custom entry `góa` still keys `guá`); `canonical_tl_form(Poj)` keeps the full fold.

`normalize_to_tl` / `NORMALIZE_TO_TL_RULES` (POJ→TL canonicalization, FST inventory build) is unchanged. English / TPS composing is identity; the English search path is literal too. Pasted POJ **diacritic** text in TL mode is not folded to TL spelling.

- **Known gap**: a continuous **OOV-synth candidate** tapped in **POJ** mode still goes through `recase_tl_as_poj_display` (`composing/src/continuous.rs` Step 5), so the strip and a tap-commit of that synth can show a converted spelling (`ting`→`teng`). Fixing it needs distinguishing the user-input synth from dict/custom candidates in the shared recase.

**Scope**: shared Rust engine (`phonetics` + `composing`) — no platform code.

**Engine sites**: `phonetics/src/api.rs` (`normalize_tone`; `consumed_tone_digit_offsets` carries the renderer's digit decisions into caret mapping; `canonical_tl_form`). `phonetics/src/permissive_tone.rs` (segment selection, digit runs, separators, Unicode replacement). `phonetics/src/syllable.rs` (`TL_ENCODING_RULES`; `normalize_to_tl_keep_tl_finals` for the TL identity key). `composing/src/shadow.rs` (`canonicalize_poj_shadow`; `custom_toneless_key` byte-identity).

**Tests**: `phonetics/src/permissive_tone.rs`; `phonetics/src/api.rs` (`to_tone_marks_tl_is_literal`, `to_tone_marks_poj_is_literal`, `to_tone_marks_special_final_and_invalid_tokens`); `phonetics/tests/canonical_tl_form.rs` (`tl_mode_preserves_special_final_eng`, `poj_form_in_tl_mode_folds_canonical_tl`); `composing/src/derived.rs` (`raw_input_composing_tl_preserves_special_final_eng`, `raw_input_composing_tl_is_literal_no_poj_spelling_fold`, `raw_input_composing_poj_is_literal_no_spelling_fold`); `composing/src/shadow.rs` (`canonicalize_poj_shadow_*`, `custom_toneless_key_tl_strips_tone_keeps_literal_spelling`); `composing/src/requests.rs` `literal_roman_candidate_permissive_tones_mirror_preview`; `composing/tests/continuous_keys_tl_poj_diacritic.rs`; `composing/tests/dispatch_continuous.rs`. Dogfood **S17**.

## 31. TPS space is a soft syllable separator (first-tone continuous input)

### `INVARIANT_TPS_SPACE_SOFT_SEPARATOR`

In **TPS** continuous input, the ASCII space (`U+0020`) is the keyboard's tone-1 / syllable-boundary marker — NOT a literal space or word break. First tone carries no Bopomofo mark, so the space is its only delimiter (and what stops the next dual-form consonant being folded into the previous syllable's entering-tone coda — `tps_adjust::adjust_initial_key` treats space as a boundary). The segmenter treats it as a **soft** separator: stripped from the segmentation shadow so the lattice chains the syllables on either side into a cross-space phrase candidate, while the shadow→raw offset map keeps the separator byte inside a full-span commit so the preedit is fully replaced. `ㄍㄠ ㄉㄞ` surfaces **交代** at the full span (raw `(0, 13)`), not just `ㄍㄠ` → 到; `ㄉㄞ ㄍㄧ` surfaces 台機. A mid-commit of `ㄍㄠ` (span `(0, 6)`) leaves the separator pending; the next fetch strips its leading separator and the next commit consumes it.

**The marker is never shown or committed**: `derived::derived_display` hides every `U+0020` in a TPS buffer (§41 for the full seam list), and a TRAILING separator is consumed by a full-span commit as well as an interior one.

**TPS-only** — for TL / POJ / English a space is a real word boundary and stays a HARD segment boundary (`tai uan` stays `tl:tai` only). `build_separator_shadow` returns the identity shadow + map for non-TPS.

**Scope**: shared Rust engine (`composing`). iOS and Android append the boundary space on the `space` key while composing in TPS (`ActionHandler+KeyActions.swift` / `TextInputKeyHandler.kt`). Needs `make build` before device dogfood.

**Engine sites**: `composing/src/shadow.rs` — `build_separator_shadow(input, mode)`, wired into `build_shadow_lattice_with_barriers` (walker + left-anchored projection), `build_partial_prefix_key`, and `custom_toneless_key`; the strip+offset-map loop is `strip_char_shadow(input, skip)` (also backs `build_hyphen_shadow`).

**Tests**: `composing/src/shadow.rs` (`build_separator_shadow_tps_strips_space_and_maps_to_raw`, `build_separator_shadow_non_tps_is_identity`, `build_separator_shadow_tps_keeps_entering_tone_coda`); `composing/tests/continuous_keys_tps.rs` (`tps_lattice_spans_tone1_separator_space_into_phrase_key`); `composing/tests/continuous_keys_tl_lattice.rs` (`tl_space_stays_hard_boundary_not_collapsed`). Dogfood **S18**. The no-space path (`ㄍㄠㄉㄞ` typed directly) is §32.

## 32. TPS stop coda is phonotactically gated (no-space first-tone continuous input)

### `INVARIANT_TPS_STOPCODA_PHONOTACTIC_GATE`

In **TPS** input, a dual-form STOP key (`ㄅ`/`ㄉ`/`ㄍ`/`ㄏ`) typed after a pure vowel converts to its entering-tone coda form (`ㆴ`/`ㆵ`/`ㆻ`/`ㆷ`) ONLY when the resulting final is phonotactically valid (entering-tone finals, `taigi-phonetics-reference.md` §3.2.4 / `tables.rs::TL_FINALS`). When the coda would form a non-syllable, the key stays an INITIAL so it begins the next syllable.

`au` admits no stop coda (only `-h`, `auh` 搯), so `ㄍㄠㄉㄞ` keeps `ㄉ` an initial → the lattice segments it → 交代 (kau-tài) with no space.

- **Valid coda → convert**: `ㄚ`+`ㄉ`=`kat`, `ㄚ`+`ㄅ`=`kap`, `ㄚ`+`ㄍ`=`kak`, `ㄛ`+`ㄅ`=`op`, `ㄛ`+`ㄍ`=`ok`, `ㄍㄨㄚ`+`ㄉ`=`kuat`, `ㄍㄨㄚ`+`ㄍ`=`kuak`; `ㄍㄠ`+`ㄏ`=`kauh` (the gate is coda-specific).
- **Impossible coda → keep initial**: `ㄍㄠ`+`ㄉ`(kaut), `ㄍㄠ`+`ㄅ`/`ㄍ`, `ㄛ`+`ㄉ`(ot), `ㄍㄧㄠ`+`ㄉ`(kiaut), `ㄠ`+`ㄉ`, `ㄧㄛ`+`ㄅ`(iop), `ㄍㄨㄚ`+`ㄅ`(kuap — `uap` is absent from MOE §4, every code table and the dictionary).

**Scope / decisions:**
- **Nasals too** — §33 generalized the helper to `dual_final_form` so one gate handles stops and nasals.
- **No segmentation.** The gate fires only when the boundary-suffix (chars after the last `SYLLABLE_BOUNDARY_CHARS`/space) is exactly ONE valid syllable AND `<suffix><coda>` is invalid. A multi-syllable no-space run (`ㄍㄠㄉㄚ`) falls through to the default convert.
- **Ambiguous-by-default.** When both readings are valid (`ㄚ`+`ㄉ` = `kat` or `ka`+`ta`-), the gate keeps the coda; the alternate is reached via the boundary space (§31).
- **Residuals** (space-disambiguated): chained no-space multi-syllable with a mid impossible coda (`ㄍㄠㄍㄠ`+`ㄉ`); zero-onset second syllable after an open syllable (`ㄍㄠ`+`ㄧ`+`ㄉ`).

**Scope**: shared Rust engine (`phonetics`). iOS and Android call `Method::TpsInputAdjust` per keystroke (gated on the TPS layout). Needs `make build`.

**Engine sites**: `phonetics/src/tps_adjust.rs` — `adjust_initial_key`, `dual_final_form(initial, last)`, `pending_open_syllable`. Validity via `crate::tps_to_tl` → `crate::is_valid_syllable`.

**Tests**: `phonetics/src/tps_adjust.rs` (`stop_after_vowel_with_valid_coda_converts`, `stop_after_vowel_with_impossible_coda_keeps_initial`, `h_coda_after_au_still_converts`, `multi_syllable_no_space_suffix_falls_through_to_convert`, `boundary_and_empty_buffer_keep_initial_as_before`). Dogfood **S19**.

## 33. TPS nasal coda is phonotactically gated (no-space first-tone continuous input)

### `INVARIANT_TPS_NASALCODA_PHONOTACTIC_GATE`

Sibling of §32 for dual-form NASALS. In **TPS** input, `ㄇ`/`ㄋ`/`ㄫ` typed after a pure vowel converts to its nasal-coda form (`ㄇ`→`ㆬ`, `ㄋ`→`ㄣ`, `ㄫ`→`ㄥ` after `ㄧ` else `ㆭ`) ONLY when the resulting final is valid (`taigi-phonetics-reference.md` §3.2.3 / `tables.rs::TL_FINALS`); otherwise the key stays an INITIAL. `u` admits no `-m`, so `ㄍㄨㄇㆦ` keeps `ㄇ` an initial → 龜毛 (ku-môo).

- **Valid → convert**: `ㄚ`+`ㄇ`=`am`, `ㄚ`+`ㄋ`=`an`, `ㄚ`+`ㄫ`=`ang` (`ㆭ`), `ㄧ`+`ㄫ`=`ing` (`ㄥ`), `ㄍㄨ`+`ㄋ`=`kun`, `ㄒㄧ`+`ㄋ`=`sin`.
- **Impossible → keep initial**: `ㄍㄨ`+`ㄇ` (kum), `ㄍㄠ`+`ㄇ`/`ㄋ`/`ㄫ`, `ㄍㄨㄧ`+`ㄫ` (kuing).

**Scope / decisions:**
- **One gate, one glyph source.** `dual_final_form(initial, last)` feeds both the validity test and the emitted glyph; the `ㄫ`→`ㄥ`/`ㆭ` context branch lives once inside it.
- **Rule 2b interaction is benign.** `syllabic_nasal_replacement` (Rule 2b, ungated, `ㄇ`/`ㄫ` only) still folds a kept `ㄇ`/`ㄫ` to syllabic `ㆬ`/`ㆭ` when a tone mark follows; those `<vowel><nasal><tone>` sequences map to no dictionary word either way.
- **No segmentation / ambiguous-by-default / residuals** — as §32.

**Scope**: shared Rust engine (`phonetics`), reached through `Method::TpsInputAdjust`. Needs `make build`.

**Engine sites**: `phonetics/src/tps_adjust.rs` — `adjust_initial_key`, `dual_final_form`, `pending_open_syllable`.

**Tests**: `phonetics/src/tps_adjust.rs` (`nasals_with_valid_coda_convert`, `nasal_after_vowel_with_impossible_coda_keeps_initial`, `nasal_rule2b_interaction_is_benign`). Dogfood **S20**.

## §34 — Continuous literal-roman candidate (Hanji-with-romanization fast input)

### `INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE`

In **TL / POJ** continuous input, when Show Typed Text First is on, the candidate list surfaces at **index 0, whenever composing (tone or no tone)** a **roman-only** candidate equal to the **current composing result** (the preedit literal): `tai` → `tai`, `tai5` → `tâi`, `nng7` → `nn̄g`, `goa2` → `goá` TL / `góa` POJ, `tai5-gi2` → `tâi-gí`. It has **no hanji**, so a tap commits the romanization — Hanji-with-romanization input commits a romanized word in one tap without toggling the 文/A switch, even in Hanji-first mode (PhahTaigi parity, `references/PhahTaigi_iOS`). It is always present while composing, so the list does not jump as the user types toward or past a tone.

- **WYSIWYG**: the candidate `roman ==` the preedit literal (`composing::derived::derived_display`) **exactly** — the typed spelling with tone marks only, NO spelling fold (§30), not re-run through the Step 5 POJ recase. It mirrors the permissive rendering (§30), including unseparated syllables (`tai5gi2` → `tâigí`) and unmarked tones 1/4; typed separators are preserved, none added.
- **Stable learning identity (Core Principle #6)**: `display_text` is derived from raw input by `composing::derived::learning_text` (`phonetics::api::literal_learning_text`, the pre-permissive normalization contract); `canonical_tl` canonicalizes that text with `canonical_tl_form`. Preview and committed text use `roman`. So `a1i3` shows and commits `aì` but keys `(a1i3, a1i3)`, while `ai3` keys `(ài, ài)`; `a1i1` and `ai1` both show `ai` but keep distinct keys. Enter on a pending tail learns the same text (`commit_raw_continuous` → `learning_text`). The §44 identity adoption still applies.

**Gates** — surfaces whenever `mode ∈ {Tl, Poj}` (TPS and English excluded) and `derived_display(raw, config)` is **non-empty**. No tone-digit / residual-digit / trailing-hyphen condition (`tai5-` → `tâi-`).

**Toggle (Show Typed Text First, S22) — all five platforms, default OFF**: when ON, the index-0 forced prepend is added; OFF skips it. The toggle gates **only** this prepend — roman-only / OOV-synth candidates that `assemble_candidates` produces on its own are unaffected. The platform setting is **positive** (`isLiteralRomanCandidateEnabled` iOS / macOS, `literalRomanCandidateEnabled` Android, `is_literal_roman_candidate_enabled` desktop-core) and is inverted into `FetchAtPos.literal_roman_candidate_disabled` (field 6) once per fetch — mobile in `ComposingManager`, the desktops in desktop-core `engine/composing.rs`. Inverted so proto3-default `false` = show: un-wired builds and proto-decoded fixtures keep the candidate. **A stored value always wins over the default**; an install that never touched it follows the default. Surfaces: mobile host-app Settings (Typing section) + in-keyboard quick-settings overlay; desktop General pane under Auto-Space. CROSS-PLATFORM INVARIANT — `EngineSettings.isLiteralRomanCandidateEnabled` (iOS `Settings/EngineSettings.swift` ↔ Android `ime/settings/EngineSettings.kt` ↔ macOS `Settings/EngineSettings.swift` ↔ desktop-core `settings/engine_settings.rs`).

**Desktop notes (macOS / Windows / Linux)**: the candidate window opens with index 0 highlighted and Return = `confirmHighlighted`, so with the setting ON **Return commits the typed literal** and the first dictionary candidate moves to the second cell. **The literal cell takes NO selection key**: the keys start on the cell after it, so the first key of the live set picks the first dictionary candidate — `q` under Standard, `1` under Telex. The shift follows the ROW the keys number: a later page, a vertical column scrolled past the literal, and an expanded grid row not starting at cell 0 are keyed from their own first cell. The page is NOT enlarged, so the row that leads with the literal keys eight candidates and its ninth key picks nothing. Linux: `LookupSelection` carries the unkeyed lead and the table sends the page's labels with the literal's blank (`taigi-linux-core` `selection.rs` / `session.rs`; IBus sends the blank as a space). Under a stacked arrangement the literal cell **centres its single line across the WHOLE cell** (it names no key, so it does not step around the key column; macOS `CandidateItemView.activateStackedConstraints` / `updateStackedTextArea`, Windows `CandidateWindow::draw_cell` `centres_across_cell`); the cell's height still comes from the list (`CandidateMetrics.forContent(hasAnnotations:)` / `for_content`) so the row stays aligned. The inline (vertical) arrangement keeps the column. The literal cell draws **no fill of its own** — it is told apart by having no key.

One seam answers both the drawn labels and the key handler — `CandidateIndexLabel` (`candidateIndex(forKeySlot:leadCellIsUnkeyed:indexForSlot:)` / desktop-core `candidates/index_label.rs` `candidate_index_for_key_slot`, with `key_slot_shift`) — so a key drawn beside a cell is the key that commits it. Whether the lead cell is the literal is decided by desktop-core `composing::presentation::leads_with_literal_roman` from the same settings snapshot: the setting is on AND the leading candidate carries no hanji. ⇧Return (`commitLiteral`, raw `CommitRaw`) is the raw escape hatch that ignores the highlight. Space on the literal cell (one script) answers `.ignored` (§42). Under Romanization Only (§44) the literal collapses a same-roman dictionary row into itself and keeps slot 0; under Hanji with Romanization the literal cell leads. macOS: `literalRomanCandidateEnabled` must stay OUT of `Settings/RetiredSettingsCleanup.swift` (a tombstone there clears the setting at launch). Open gap: the auto-space policy (desktop-core `policies/auto_space.rs`) decides the space from `CandidateScript` + the effective swap, and a one-script literal is presented as `.primary`, so Return on the literal under Hanji-first side-by-side or Hanji with Romanization may not earn the auto-space a romanization commit should.

**Placement / dedup**: prepended at index 0 — the walker slot-0 / dict best moves to index 1. A pre-existing IDENTICAL bare-roman (`hanji = None`, same roman) candidate is dropped; dict rows that carry hanji stay (`tâi`/台 and a bare `tâi` commit differently).

**Scope**: shared Rust engine (`composing`) for the candidate + the toggle gate; thin platform wiring for the setting (settings store + `EngineSettings` + the invert + a UI row). The injection lives in the display-layer seam `composing::requests::handle_fetch_at_pos` (`literal_roman_candidate` helper), NOT in `assemble_candidates`. Every platform commits a roman-only candidate's `roman` (engine `commit_text::resolve_commit_text` for every mobile tap; desktop-core presents a one-script cell `.primary`).

**Engine sites**: `engine/protos/proto/composing.proto` (`FetchAtPos.literal_roman_candidate_disabled` field 6), `composing/src/api.rs` (`Intent::FetchAtPos` field), `composing/src/requests.rs` (`decode_intent` + `handle` thread the flag; `literal_roman_candidate` build; gate + bare-roman dedup in `handle_fetch_at_pos`). Platform wiring: iOS `Settings/SharedSettings.swift` + `Settings/EngineSettings.swift` + `Engine/RustEngineBridge+Composing.swift` + `Input/Composing/ComposingManager.swift` + `App/Tabs/Settings/SettingsTab.swift` + `Overlays/SettingsSelectionOverlay.swift`; Android `ime/settings/PreferenceKeys.kt` + `PrefHelper.kt` + `ime/settings/EngineSettings.kt` + `engine/ComposingBridge.kt` + `engine/RustEngineBridge.kt` + `ime/text/composing/ComposingManager.kt` + `ui/tabs/settings/InputSettingsScreen.kt` + `ime/text/overlays/SettingsOverlayContent.kt`; macOS `Settings/EngineSettings.swift` + `Settings/SettingsStore.swift` + `Settings/GeneralSettingsView.swift`; desktop-core `settings/engine_settings.rs` + `settings/keys.rs` + `settings/document.rs` + `engine/composing.rs`; Windows `taigi-windows-settings/src/winui/pages/general.rs`; Linux `taigikeyboard-settings/src/pages/general.rs`.

**Tests**: `composing/src/requests.rs` (`literal_roman_candidate_*` — toneless / partial / tone-1/4 / unhyphenated blob / trailing-hyphen / POJ doubletap mirror the preedit; empty + TPS/English excluded; identity fold) + `composing/tests/dispatch_continuous.rs` (`fetch_at_pos_literal_roman_toggle_gates_index0_prepend`) + golden `composing/tests/golden/fetch_at_pos.golden` (TL/POJ sections lead with the preedit literal, all cases ON). Dogfood **S22**.

## 35. TPS ambiguity-aware lookup — every reading of the pressed keys reaches the dictionary

### `INVARIANT_TPS_DEFOLD_ENUMERATE`

In **TPS** continuous input, a physical key with dual forms (`ㄍ` = onset `ㄍ` / coda `ㆻ`; `ㄇ` = onset `ㄇ` / syllabic `ㆬ`; …) is resolved at LOOKUP time against the dictionary — the per-keystroke auto-correct (§32/§33) still picks one glyph for the preedit display, but its guess does not decide reachability. Typing 考卷 toneless as `ㄎㄛ`+`ㄍ`+`ㄫ` leaves `ㄎㄛㆻㄫ` in the buffer; the lookup considers every reading of those KEYS and finds the stored `ㄎㄛㄍㆭ`. Every dictionary reading that has a `tps:` FST key is reachable; the only unreachable rows are ≥5-syllable entries `create_fst.py`'s `MAX_SYLLABLES_TL_NUM = 4` gate never emits keys for (a build-pipeline scope fact).

librime Spelling-Algebra / McBopomofo one-key-multi-symbol model: alternates are extra paths ranked after the user's literal text; the raw buffer is never rewritten.

- **Ambiguity families** (`phonetics::tps_ambiguity_family`): exactly the dual-form sets the auto-correct rewrites — `{ㄅㆴ} {ㄉㆵ} {ㄍㆻ} {ㄏㆷ} {ㄇㆬ} {ㄋㄣ} {ㄫㆭㄥ}`, each member tagged `Initial`/`Final`. Deliberately NOT included: palatalization pairs (the keystroke rule is phonologically unconditional — zero readings lost), same-key vowel callouts `ㆤㄝ`/`ㆮㆯ`/`ㆰㆱ` (different phonemes; matching across them would be cross-phoneme fuzzy search), and `ㆪㆳ` (one symbol encoded twice — every lookup folds ㆳ onto ㆪ first, `phonetics::fold_tps_glyph_alias`). All members are 3-byte glyphs → substitutions are byte-length preserving, so lattice spans and offset maps stay valid.
- **Mechanism**: keys stay the user's LITERAL span text. `lexicon::tps_pattern::TpsKeyPattern` is an `fst::Automaton` expanding each family glyph to its (positionally legal) members; one index walk intersects it with the FST — no string enumeration. Three wire contracts: `Exact` (syllables.fst probes), `ExactWire` (`pattern ‖ 0xFF ‖ exactly 4 rowid bytes ‖ EOF`), `StartsWith` (partial-prefix).
- **Consumers — single authority**: the segmenter probe (`syllabifier::tps` → `SyllableInventory::contains_in_tps_readings`), the span-local fetch (`fetch_candidates_for_keys_with_barriers` → `PrefixIndex::lookup_exact_tps_readings`), the walker (`best_candidate_for_key_with_barriers`) and partial-prefix hydration (`lookup_prefix_shortest_first_tps_readings`) all resolve through the same pattern, and every record guard validates against the **matched** key, never the query key (the literal-key guard would reject every recovered reading; it also rejects abbreviation collisions).
- **Ordering**: readings return substitution-count ascending (charwise diff vs the literal key), so the user's own text resolves first and `dedupe_by_roman_hanji_span` keeps it on collisions. Visible order remains `CandidateSortKey` — a high-frequency substituted word MAY outrank a literal one (媽 over 毋仔 on `ㆬㄚ`); no ambiguity penalty. Partial-prefix budget: matched-key length asc → substitution asc → byte order.
- **Barrier contract (§31 interaction)**: the stripped TPS space / hyphen offsets travel with the shadow (`build_shadow_lattice_with_barriers`). (a) No SINGLE syllable may cross a barrier — chains still meet AT it (`ㄍㄠ`␣`ㄉㄞ` → 交代). (b) The glyph immediately before a barrier keeps only `Final`-role readings (plus the literal): `ㄎㄛㆻ`␣`ㄫ` never reconstructs 考卷, while `ㄇ`␣`ㄒㄧ` still reaches 毋是. A slot before a tone mark is Final-only too; buffer-end stays unrestricted.
- **Bare-nasal merge**: a single-glyph TPS buffer whose only span comes from expansion (bare `ㄇ` → `ㆬ` 毋; bare `ㄫ` → `ㆭ` 黃) runs span fetch AND the partial-prefix branch and merges, so the single-char readings surface without a long-press (continuations are limited by §43).
- **§18 applies uniformly**: one lattice, so the §18 suppression of shorter prefix syllables (`ke` under `ㄍㆤㆷㄧㄥ`, `pau` under `ㄅㄠㆷㆦㆻ`) holds for expanded readings too.

**Scope**: shared Rust engine (`phonetics` + `lexicon` + `composing`). Every platform with a TPS surface reaches it via the continuous fetch. TL / POJ / English are untouched: expansion is gated on the `tps:` key family / `InputMode::Tps` at every consumer, pinned by full-vs-base exact-equality tests. **Custom dictionary is out of scope**: custom rows are matched by a single derived key in the store's query, so custom entries are not ambiguity-reachable (documented limitation). Needs `make build` (no `make dict`).

**Open follow-ups**: an ambiguity penalty on substituted readings; whole-word vs single-char ranking cohesion; the preedit following the winning segmentation (display still shows the keystroke guess); custom-dictionary ambiguity parity.

**Engine sites**: `phonetics/src/tps_ambiguity.rs`, `lexicon/src/tps_pattern.rs` (`TpsKeyPattern`, `WireMode`, `substitution_count`), `lexicon/src/prefix_index.rs` (`lookup_exact_tps_readings`, `lookup_prefix_shortest_first_tps_readings`), `lexicon/src/syllable_inventory.rs` (`contains_in_tps_readings`), `lexicon/src/continuous/` (`fetch_candidates_for_keys_with_barriers`, `best_candidate_for_key_with_barriers` — matched-key guards), `composing/src/shadow.rs` (`build_shadow_lattice_with_barriers`, `build_continuous_keys` → `ContinuousKeys`, `left_anchored_keys_and_restrictions`, `key_final_only_offsets`), `composing/src/syllabifier/tps.rs`, `composing/src/lattice/builder.rs` (`build_lattice_with_barriers`), `composing/src/continuous.rs` (bare-nasal merge; walker barrier plumbing), `composing/src/requests.rs` (`build_continuous_keys_with_inventory`).

**Tests**: `phonetics/src/tps_ambiguity.rs`, `lexicon/src/tps_pattern.rs`, `lexicon/tests/tps_readings.rs` (考卷 reachability, literal-first ordering ㆬㄚ, barrier direction both ways, `tl:` namespace isolation, prefix readings with matched keys), `composing/tests/continuous_keys_tps.rs` (expanded segmentation admits 考卷/毋是 spans, 博雅/門 untouched, separator chain vs fusion, bare-nasal literal key, barrier-aware §18 recompute), `composing/src/shadow.rs` (barrier recording, mixed `␠-`/`-␠` projection), `composing/tests/continuous_keys_tl_lattice.rs` (non-TPS full-vs-base exact equality). Dogfood **S23**.

## 36. Key-press feedback — app toggle gates sound + haptics (intentional OS-master divergence)

### `INVARIANT_KEYPRESS_FEEDBACK_APP_TOGGLE_GATE`

The in-app sound and vibration toggles gate key-press feedback on both mobile platforms: toggle OFF → no feedback; toggle ON **and** the relevant system feedback enabled → feedback fires. This much MUST match.

The platforms **intentionally diverge** on the OS master feedback setting — do NOT "fix" the iOS case as a bug:

- **Android (vibration)**: a **direct `Vibrator`** (`KeyPressVibrator`, default `USAGE` — not `USAGE_TOUCH`) drives the haptic, so the in-app toggle is the gate **independent of** `Settings.System.HAPTIC_FEEDBACK_ENABLED` (fixes app-ON + OS-touch-haptic-OFF producing no vibration on Samsung). It does NOT bypass the master `VIBRATE_ON`, battery-saver, OEM policy, or absent hardware (intended).
- **Android (sound)**: `AudioManager.playSoundEffect` remains ANDed with `Settings.System.SOUND_EFFECTS_ENABLED` (industry norm) — app-sound-ON + system-keyboard-sound-OFF → silent is expected.
- **iOS (both)**: feedback is KeyboardKit-managed (`UIImpactFeedbackGenerator` + audio). A keyboard extension has **no** bypass of the **System Haptics** master, so app-ON + System-Haptics-OFF → no vibration is expected platform behavior. `AudioServicesPlaySystemSound(kSystemSoundID_Vibrate)` and a custom CoreHaptics engine were deliberately NOT adopted (full-device buzz / same gate + complexity / Apple-discouraged).

**Net divergence** (intentional, platform-imposed; `docs/contributing/cross-platform-alignment.md` §3): with the toggle ON, Android vibrates when the OS touch-haptic setting is off; iOS does not vibrate when System Haptics is off.

**Scope**: platform-side only. Android `ime/core/KeyPressVibrator.kt` (+ callers `ImeKeyEventDispatcher`, `MediaInputManager`); iOS KeyboardKit `feedbackService` + `state.feedbackContext` (`KeyboardViewController+Setup.swift`), custom-key path `ActionHandler+CustomActions.swift`.

**Tests**: device-matrix dogfood only (haptics + audio are not unit-testable on JVM/sim): Android Samsung + Pixel matrix, iOS checklist.

## §37 — App UI display-language production roster

### `INVARIANT_DISPLAY_LANGUAGE_PRODUCTION_ROSTER`

Two distinct sets, both of which MUST match across iOS, Android, and macOS:

- **Production roster** = the release-selectable languages: exactly **[hanji, english, japanese, tailo, poj]** (tags `["hanji", "en", "ja", "tailo", "poj"]`, in that order). The codegen completeness gate (`tools/i18n` `PRODUCTION_LANGUAGES`) enforces that every key fully authors each, in the SAME order. (`poj` is the Pe̍h-ōe-jī rendering of the same reading as `tailo`; `validate_production_completeness` alone enforces the pair.)
- **Selectable set** = what the picker offers = **[system] + production roster**: **[system, hanji, en, ja, tailo, poj]**. `system` (Automatic) has no authored strings; it is a resolution policy (`INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION`).

`DisplayLanguage.fromTag` maps unknown tags to `hanji`; `fromTag("system")` returns `system`. The persisted default tag is **`system`** — a fresh install follows the device locale. The default is lazy-read, so an install with no persisted tag follows it; any persisted tag wins. Display is **orthogonal to the keyboard input mode**.

**Scope**: platform-side UI only. iOS `Strings/DisplayLanguage.swift` (`productionLanguages` / `selectableLanguages` / `endonym` / `fromTag`); Android `i18n/DisplayLanguage.kt`; macOS `macos/Sources/TaigiInputMethodCore/Strings/DisplayLanguage.swift`. **Intentional storage divergence, behavior identical**: Android resolves Hanji through the default `values/` resource set and iOS resolves en/ja through `.lproj` bundles, while macOS resolves ALL FIVE through the generated `GeneratedStrings` map (the input method is a hand-assembled SwiftPM bundle with no resources, so it also omits iOS's `StringResolution` enum). Production-language option labels are **endonyms** (W3C language-selector guidance), not i18n keys, and MUST match across platforms; the `system` row is the one translated label (i18n key `settings.displayLanguageAutomatic`).

**Tests**: iOS `SettingsKeyTests.swift`, Android `DisplayLanguageTest.kt`, macOS `DisplayLanguageTests.swift` pin the production roster, exact selectable roster, `system` round-trip, unknown-tag → Hanji fallback, and `tailo`/`poj` resolution; macOS `StringResolverTests.swift` pins that all five resolve from the generated map. `tools/i18n/test_i18n.py` pins `PRODUCTION_LANGUAGES == ("hanji","en","ja","tailo","poj")`, missing-`tailo`/missing-`poj` rejection, the platform scope filter, and that a `"macos"` scope leaves every iOS/Android artifact byte-identical. All three platform sources, this doc, and the tests move together (`CROSS-PLATFORM INVARIANT` comments). The picker UI + cross-process live-switch are dogfood-pinned.

### `INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION`

When the selected display language is `system` (Automatic), the **effective** language is resolved from the device locale by a pure mapping, identical on all three platforms:

| Device language subtag | Effective display language |
|---|---|
| `ja*` | japanese |
| `en*` | english |
| anything else / absent (incl. `zh*`) | hanji |

Taiwanese Hanji is the neutral default: a Chinese-locale (or any non-`ja`/`en`) device reads the UI in Hanji.

`system` is a persisted *state*, never a one-time seed. The effective language drives the string resolver AND the English-only plural branch; the **selected** value drives the picker checkmark + the settings-row label. An explicit selection ignores the OS locale (only `system` keys its resolver on the device locale). The source is the **device** locale, immune to app-level overrides: Android `LocaleManager.systemLocales[0]` (API 33+) / `Resources.getSystem().configuration.locales[0]` (API 28–32) — NOT `Locale.getDefault()`; iOS / macOS `Locale.preferredLanguages.first`. Live OS-language changes are picked up at the next refresh point: Android — the device subtag is part of the resolver's memo key for `system`; iOS — `syncFromSettings()` recomputes effective even when the selection is unchanged, driven on `scenePhase == .active` and each keyboard-extension overlay's `.onAppear`.

**macOS** (single process) needs both refresh paths: (a) **selection changes** — key-path KVO on the persisted `displayLanguage` default calls `syncFromSettings()`, covering writes from outside the app (`defaults write`); (b) **OS-language changes under Automatic** — the persisted tag stays `"system"`, so (a) never fires; `SettingsWindowController.show()` and `TaigiInputController.menu()` call `syncFromSettings()` at their own refresh points. Rendering: SwiftUI re-renders the settings window (title = the pane's `navigationTitle`, `sceneBridgingOptions = .all`); `NSApp.mainMenu` is rebuilt from `DisplayLanguageStore.languageDidChange`, invoked from `apply(selected:)` right after the resolver is swapped; `AppDelegate` supplies the callback, the store never imports AppKit.

**Scope**: iOS `Strings/DisplayLanguage.swift` (`resolveAutomatic` / `effectiveLanguage`) + `DisplayLanguageStore.swift`; Android `i18n/DisplayLanguage.kt` + `StringResolver.kt`; macOS `Strings/DisplayLanguage.swift` + `DisplayLanguageStore.swift`.

**Tests**: iOS `SettingsKeyTests.swift`, Android `DisplayLanguageTest.kt`, macOS `DisplayLanguageTests.swift` (`INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION_*` — `ja*→japanese`, `en*→english`, `zh*/fr/absent→hanji`; explicit selection ignores locale). macOS `DisplayLanguageStoreTests.swift` pins the store against an injected defaults suite and device subtag, including the external-write refresh. Cross-process agreement (iOS/Android) + live OS-language refresh are dogfood-pinned.

## §38 — Main-app tab titles follow the display-language picker

### `INVARIANT_NAV_TAB_TITLES_FOLLOW_PICKER`

The five main-app bottom-nav tab titles (Home / Theme / Layout / Dictionary / Settings) are i18n keys in the `nav` namespace and follow the **in-app display-language picker**, live-switching with it — NOT the OS locale. Each key is the tab-strip label and, for Theme / Layout / Dictionary / Settings, also the page title. Home keeps its app-header page title (`homeAppHeaderTitle`), so `navTabHome` is the tab-strip label only.

- **iOS**: `TabType.titleKey: StringKey`, resolved via `DisplayLanguageStore` (`lang.string(tab.titleKey)`) — tab strip in `ContentView.swift`, page chrome via `.navigationTitle(...)` in `SettingsTab` / `LayoutTab` / `ThemePickerView` / `DictionaryTab`.
- **Android**: `TabItem.label: StringKey` (resolved via `stringRes(...)` in `MainSettingsScreen`, applied to the `NavigationBarItem` text AND the icon `contentDescription`); page chrome via `L10n.navTab*` in each screen's `LargeTopAppBar` (Home keeps `L10n.homeAppHeaderTitle`).
- **Out of scope — Android system chrome stays OS/native**: an Activity's recents `android:label` is an OS-locale surface. `ThemeEditorActivity` sets no label → inherits `app_name`.

**Scope**: platform-side host UI only. Strings in `i18n/nav.json` (all five production languages).

**Tests**: the i18n completeness gate (`tools/i18n/test_i18n.py` + `validate_production_completeness`) pins `nav.json`; the enum→key mapping is compile-checked (exhaustive `switch` / `when`). The live switch is dogfood-pinned.

## §39 — Keyboard locale tag is the neutral `mul`, not a Chinese-bearing tag

### `INVARIANT_KEYBOARD_LOCALE_NEUTRAL_TAG`

The IME's OS-declared locale tag is the neutral **`mul`** (BCP-47 "Multiple languages") on both mobile platforms — NOT `nan-*` or `zh-*`. CLDR renders any `nan` tag as "Min Nan **Chinese**" (English) / "閩南語", and the English base name is not overridable from app resources. `mul` renders "Multiple languages / 多種語言", so the OS keyboard list never labels the keyboard as a kind of Chinese.

- **iOS**: `KeyboardExtension/Info.plist` → `NSExtension.NSExtensionAttributes.PrimaryLanguage = "mul"` (the static plist language-declaration field; display name per Foundation `Locale.localizedString(forLanguageCode: "mul")`).
- **Android**: `res/xml/method.xml` subtype → `android:languageTag="mul"`; the deprecated `android:imeSubtypeLocale` is removed, not kept alongside (an OEM preferring the legacy field could re-surface "Min Nan Chinese"). The subtype `android:label` stays `@string/app_name`.

**Scope**: OS-visible keyboard chrome only. Independent of `INVARIANT_DISPLAY_LANGUAGE_AUTOMATIC_RESOLUTION`.

**Unverified**: the iOS per-app language picker (Settings → App → Language) reads `CFBundleLocalizations`, the compiled `.lproj` set and pbxproj `knownRegions`; do NOT claim it is Chinese-free until checked on device.

**Tests**: no unit test. Dogfood: on an English-locale device the keyboard list shows "台語齒盤" / "Multiple languages", never "Min Nan Chinese".

## §40 — NextWord learning decisions are platform-neutral

### `INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`

What a commit teaches NextWord depends on the **writing system it is written in**, never on which OS produced it. All five platforms forward their commits and learn the same associations from the same commit (macOS, Windows and Linux learn without showing predictions, §24). No request names its platform.

The rules, all in `engine/nextword/src/decide.rs`:

- **Word boundary = whitespace only.** `split_compound` never breaks on `-`. In engine-rendered output a hyphen joins the syllables of one word (`tâi-gí`), `--` marks the neutral tone (`hōo--guá`), and a space separates the sub-words of a phrase (`iā sī`) — §22. A user-typed hyphen is a delimiter inside a word too (§21), and raw typed text reaches this crate (`engine/composing/src/transition.rs::commit_raw_continuous` passes the pending tail as `roman`). So `-` is a bigram boundary in neither provenance: `tâi-gí khí-puânn` → one pair 台語 → 齒盤.
- **TPS is one unit.** In TPS an ASCII space is the tone-1 marker (§31), so a TPS payload never splits. `AppConfig.input_mode` does not answer this — the payload does: **Bopomofo content is the detector**, the same source `composing::derived::derived_display` uses.
- **Segmentation disagreement fails closed.** `compound_association_pairs` zips the Hanji and romanization sides by position; when they split into different part counts (Hanji `也是` has no space, `iā sī` does) it records **nothing**. The short side is never padded: a blank TL writes a row no correctly-keyed lookup matches (Core Principle #6).
- **Noise = no word material anywhere.** `is_noise_text` is `!text.chars().any(phonetics::is_word_material)`, where word material is an alphabetic character that is not one of the four `Lm` TPS tone marks (`ˆ ˇ ˊ ˋ`) and not a POJ nasal (`ⁿ` / `ᴺ`). A Unicode property, not a punctuation table, so an unlisted mark is never learned as a word. A commit that merely *starts* with a bracket or space (`hit (彼)`) is kept.
- **The same test applies per word.** A commit mixing a word and a mark (`台語 ˆ`) passes the whole-string gate; a part with no word material disqualifies the whole commit rather than being dropped (dropping it would splice its neighbours into an adjacency the user never typed).
- **Sentence end or clause mark = a noise commit that starts with `。！？.!?` or `，、；：,;:`.** Such a commit (or `preceding` segment) — `is_noise_text` + `breaks_context` over `CONTEXT_BREAK_PUNCTUATION` — drops the context: the last word before the mark is never learned as the predecessor of the first word after it. Other noise (a symbol such as `★`) keeps the context. Test: `decide.rs::macos_sentence_end_punctuation_resets_context`.
- **A clause mark breaks the context like a sentence end** (`INVARIANT_NEXTWORD_CLAUSE_MARK_BREAKS_CONTEXT`). Full-width `，、；：` and half-width `, ; :` clear `last_selected_word` / `last_selected_roman` and record no pair across them, alone or inside a commit's `preceding`; the mark predicts nothing. Why: `食，飯` must not teach 食 → 飯 — the bundled corpus breaks its chain on every punctuation (`dictionary/build/corpus_bigrams.py`), and a user row outranks every bundled row (§8). Half-width because romanization-mode users commit half-width punctuation. Granularity: a whole `10:30` or `1,000` commit (digits are not word material) keeps the context; a lone `:` or `,` breaks it; brackets, quotes and ellipsis are not in the set. Tests: `decide.rs::every_clause_mark_breaks_context_alone_and_inside_a_commit`, `commit_ending_in_clause_mark_clears_context`, control `commit_ending_in_symbol_keeps_last_word_as_context`.
- **Punctuation typed outside a composition reaches the engine** (`INVARIANT_NEXTWORD_PUNCTUATION_OUTSIDE_COMPOSITION_REACHES_ENGINE`). A character committed straight into the document is forwarded as a `WordSelected` with no reading and no prediction (iOS / Android `NextWordController.noteCharacterTypedOutsideComposition`, desktop-core `note_character_typed_outside_composition`), so the context-break rule ends the context on a typed `。` or `，`. The platform forwards every non-letter, non-whitespace character (letters start a composition; whitespace never breaks the context) and lets the engine decide. Tests: Android `TextInputKeyHandlerTest`, iOS `RustEngineBridgeNextWordTests`, desktop `composing_manager.rs::a_comma_is_reported_and_a_letter_outside_is_not`.
- **A selected word that ends in sentence punctuation is still learned** (`INVARIANT_NEXTWORD_TRAILING_SENTENCE_PUNCTUATION_IS_STILL_LEARNED`). A word that merely ends in `。！？.!?` is word material, so selecting `多謝！` after `我` learns 我 → 多謝！ and keeps 多謝！ as context. No platform pre-checks the committed text; the engine is the single decider. Test: `decide.rs::word_selected_ending_in_sentence_punctuation_is_still_learned`; Android dogfood-pinned (JVM tests cannot load the `.so`).
- **The reading is learned as sent** (`INVARIANT_NEXTWORD_READING_LEARNED_VERBATIM`). `WordSelected.roman` / `CommittedWord.roman` become the pair's `prev_tl` / `next_tl` and the context reading unchanged — the sender hands over canonical TL: composing effects carry the candidate's canonical TL (§25) and put the Enter tail through `canonical_tl_form` (`transition.rs` `commit_raw_continuous`); iOS / Android taps send the canonical-TL sidechannel; a TPS or English reading passes as typed. A POJ → TL fold here would rewrite the TL special finals `eng` / `ek` (§3.2.6): 蔣經國 `tsiúnn-keng-kok` must not be learned as `tsiúnn-king-kok`. Rows written by an older folding build stay as they are. The `.taigi` codec follows: version 3 restores readings as written, 1 / 2 still fold (`userdata/src/backup.rs`). Tests: `decide.rs::word_selected_learns_tl_special_finals_verbatim`, `composing/tests/continuous_phase.rs::commit_raw_tail_reading_is_canonical_tl`, `dispatch/tests/nextword_readings.rs`, `userdata/tests/backup.rs` (`a_version_3_backup_restores_canonical_readings_as_written`, `a_version_2_backup_still_folds_its_readings`).

**Why an invariant**: these rules once diverged per platform (split on `-` on one, on whitespace on another, a padded short side, different noise sets); the engine now owns one decision.

**Existing rows are grandfathered.** Older databases keep bigrams the old rules wrote (word-internal pairs, empty-TL rows). No reliable discriminator separates a drift artifact from a legitimately learned association, so no cleanup runs. These rows do not reliably decay away: `calculate_user_score` adds a fixed `LEARNING_BONUS`, and capacity pruning starts above 50 000 rows.

**Scope**: rules in `engine/nextword/src/decide.rs`; platforms only forward commits. No platform writes a bigram: the pairs a decision records come back to `dispatch` beside the response (`nextword::Handled.associations`) and are written to `user_association.db` there.

**Tests**: `decide.rs` — `split_compound_breaks_on_whitespace_but_never_on_hyphen`, `compound_pairs_record_only_a_trustworthy_boundary` (the one shape that earns a pair, plus TPS on either side, segmentation disagreement, no boundary, a part that is not a word), `noise_is_the_absence_of_word_material`, `compound_pairs_are_sequential`, `learning_decisions_do_not_depend_on_the_platform` (the shape platforms once answered three ways gives the two-word pair), and — across the terminal word and a `preceding` word — `hyphenated_single_word_teaches_no_compound_on_either_entry_point` + `noise_records_nothing_and_becomes_no_context_on_either_entry_point`. iOS `RustEngineBridgeNextWordTests.swift` pins the contract through the real FFI. Android dogfood-pinned.

### `INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING`

**One commit is one sequence.** A continuous composition writes its nailed segments and its terminal word in one final commit (Model B, `docs/engine/continuous-commit-and-display.md` §10.3), and NextWord learns that commit as one ordered sequence in one decision — `WordSelected.preceding` (the nailed segments, document order) followed by `WordSelected.text`:

- **Every adjacent pair inside the commit is learned** (`欲 → 食`, `食 → 飯`), once per occurrence (`好 好 好` teaches `好 → 好` twice), whatever time the nails took.
- **The 10 s window gates only the link to the previous commit** (`我 → 欲`), measured from that commit's own time; a nail never refreshes it.
- **A nail or unnail learns nothing and changes no context.** `UpdateLastSelectedWord` is an acknowledgement (platforms read the effect as the nail signal). A composition abandoned after nails teaches nothing and leaves the context it started from.
- Per-word rules apply to each word: compound pairs, the noise gate, a sentence end or clause mark breaks the chain (a commit ending in one clears the context), a symbol does not.
- **Scope**: the final candidate pick and Enter (`composing/src/transition.rs` `commit_continuous` / `commit_raw_continuous`). Commit-then-insert-external finalizes the nailed prefix with `NextWordClearForNewComposing` and learns nothing.

**Tests**: `nextword/src/decide.rs` — `commit_learns_every_adjacent_pair_of_its_sequence`, `commit_outside_window_still_learns_its_own_pairs`, `commit_repeated_word_learns_each_occurrence`, `commit_sentence_end_and_clause_mark_both_break_chain`, `commit_ending_in_clause_mark_clears_context`, `commit_ending_in_symbol_keeps_last_word_as_context`, `update_last_selected_word_learns_nothing_and_keeps_context`; `composing/tests/continuous_phase.rs` — `final_commit_carries_nailed_segments_as_preceding`, `commit_raw_under_continuous_after_mid_commit_commits_whole_composition`. Every platform forwards `preceding` verbatim (iOS / Android bridges, `taigi-desktop-core`).

## §41 — TPS keyboard space pins the unmarked tone (1 open rime / 4 stop coda)

### `INVARIANT_TPS_SPACE_PINS_UNMARKED_TONE`

In **TPS**, pressing space closes the syllable just typed, and a closed syllable with no tone mark can only carry one of the two tones TPS writes unmarked — **1** on an open rime, **4** on a stop coda (ㆴㆵㆻㆷ). So a candidate whose span ENDS on that space must carry that tone; every other tone of the same toneless key is dropped. `ㄒㄧ`␣ surfaces 詩 (si1), not 是 / 時 / 死; `ㄐㄧㆵ`␣ surfaces 這 (tsit4), not the more frequent 一 (tsit8). Without the space nothing changes.

This completes §17 for the two tones that have no mark to type: for tones 1 and 4 `tps_num == tps_notone`, and tone 4's `tps_num` is byte-identical to tone 8's `tps_notone`, so the `tps:` key family cannot express "tone 4 only".

- **Why the space carries the signal**: it is the only delimiter tones 1/4 have (§31). The shadow strips it to zero width so a first-tone phrase still forms one cross-space edge, but records the offset as a **barrier**, so the signal survives to key selection.
- **Span-end only.** A barrier strictly INSIDE a candidate's span stays a plain syllable boundary — §31's `ㄍㄠ`␣`ㄉㄞ˪` → 交代 opens on kau1 and §35's `ㄇ`␣`ㄒㄧ` → 毋是 on m7. The single-syllable span that STOPS at the space is pinned (`ㄍㄠ`␣ → 交 kau1, not 到 / 猴) while the phrase running through it is not.
- **Resolved at lookup, not in the key.** The pinned span queries the toneless key; eligibility is decided per record. `phonetics::tps_notone_prefix_boundary_tone` walks the record's TL syllable tokens, accumulating each token's fused TPS notone surface, and reports the tone of the token whose accumulated prefix ends exactly at the typed body — `None` (no boundary there) is a REJECT. The or→er variant surface is accepted. **No dictionary rebuild** (`make build` only).
- **A marked tail never takes the space close.** `ㄍㄠ˪`␣ still surfaces 到 (kau3): a span ending in a tone mark carries that mark's own close (§17 case 4).
- **Every candidate source is covered, dictionary and custom.** The span-local fetch and the walker validate against their **matched** key (§35). Custom-dictionary and learned entries are synthesized whole-buffer, so they align against the typed buffer body (`ContinuousFetchCtx::tone_pin`, one `TonePin::admits` dispatcher on every candidate path). The space pin is the `SpaceClosed` close of `TonePin::TpsTyped`, beside §17 case 4's marks and interior spaces; the partial-prefix path aligns it on the matched key. The walker's per-edge custom rows carry the same gate, though it cannot fire in TPS today: `composing::shadow::custom_toneless_key` rejects a TPS body that is not all-Bopomofo and `custom_dictionary.db` stores TL / POJ romans.
- **TL / POJ / English unchanged.** Their space is a literal word boundary (no barrier) and their tones are ASCII digits (§17).
- **The marker is never visible, committed, or learned.** The space lives in the raw buffer (the barrier, §35's Final-only restriction and this pin read it there), but `derived::strip_tps_separator_markers` drops every `U+0020` from a TPS buffer at each seam where raw feeds the outside world: the **pre-edit** (`derived_display`) shows `ㄒㄧ`, not `ㄒㄧ␣`; `CommitRaw` writes `ㄍㄠㄉㄞ` through `combined_display`; the terminal **`NextWordWordSelected.roman`** is marker-free (a row keyed `ㄍㄠ␣ㄉㄞ` would never match a later `ㄍㄠㄉㄞ` lookup). TPS glyphs and tone marks pass untouched; a TL/POJ space is unaffected.
- **A trailing marker is consumed by the full span.** `strip_char_shadow_with_barriers` records offset-map entries only for KEPT characters, so a trailing separator falls past the last entry; the composed shadow→raw endpoint is corrected in `lattice_from_canonical_with_barriers` AFTER the barrier merge, so a full-span commit leaves no lone `" "` pending. Barrier offsets, lattice edges and `key_final_only_offsets` are untouched, and a trailing hyphen STAYS pending — the tail rule accepts ASCII spaces only.

**Scope**: shared Rust engine (`phonetics` + `composing` + `lexicon`). Needs `make build`, not `make dict`.

**Engine sites**: `phonetics/src/tps.rs` (`tps_notone_prefix_boundary_tone` over `tps_notone_boundary_tones`; shared `tl_syllable_tokens` / `tps_notone_from_numeric_token`), `composing/src/derived.rs` (`strip_tps_separator_markers`), `composing/src/transition.rs` (`CommitRaw`, terminal NextWord roman), `composing/src/shadow.rs` (`tps_span_closes`, `span_tone_pin` / `whole_buffer_tone_pin`, `ContinuousKeys::tone_pins`, trailing-separator endpoint correction in `lattice_from_canonical_with_barriers`), `composing/src/continuous.rs` (per-key + per-edge plumbing), `lexicon/src/continuous/` (`is_unmarked_tps_tone`, `reading_passes_tps_closes`, and the guard in `fetch_candidates_for_keys_with_barriers` / `best_candidate_for_key_with_barriers` / `fetch_partial_prefix_candidates_unbounded` / both custom merges).

**Tests**: `composing/src/derived.rs` (marker hidden trailing / interior / repeated, tone marks kept, TL literal space kept), `composing/src/shadow.rs` (trailing separator consumed, interior span end unmoved, trailing hyphen still pending, mixed `␠-` / `-␠` / `␠-␠` tails, empty + all-separator buffers; pin predicate at span end vs interior vs absent, marked tail, TPS-only), `composing/tests/continuous_phase.rs` (Enter commits without the marker; terminal NextWord `text` AND `roman` marker-free; full-span commit returns to Idle), `phonetics/src/tps.rs` (boundary-tone unit pins incl. tsit4 vs tsit8, mid-syllable / over-long / empty prefix rejects, or→er), `composing/tests/tps_space_pinned_tone.rs` (`ㄒㄧ`␣ → 詩, `ㄐㄧㆵ`␣ → 這, toneless controls, `ㄍㄠ`␣`ㄉㄞ˪` → 交代 preserved, `ㄍㄠ˪`␣ not pinned, custom-entry cases, strict-prefix control 之, `pinned_candidate_span_consumes_the_trailing_separator`). Dogfood **S24**.

## §42 — A candidate cell shows both scripts

### `INVARIANT_CANDIDATE_CELL_SHOWS_BOTH_SCRIPTS`

Every candidate cell shows the Hanji AND the romanization of the word it offers, on all five platforms, under the default Candidate Display = Hanji–Romanization Pairing. The swap setting decides which one LEADS; it never decides whether the other appears.

**Why**: a Taigi word is the `(Hanji, romanization)` pair (Core Principle #6), so one script alone does not identify a candidate: 重 is two words (tîng / tāng), and tsia̍h is several. A list showing one script offers rows the user cannot tell apart.

**Candidate Display storage contract**: one setting `candidateDisplayMode` (Android `keyboard__candidate_display_mode`) with raw values `sideBySide` (Hanji–Romanization Pairing, the default), `combined` (Hanji with Romanization), `romanOnly` (Romanization Only), identical on every platform so a transferred setting resolves to the same mode; a missing or unknown value reads as `sideBySide` (iOS `Settings/SettingsModels.swift` `CandidateDisplayMode`, Android `ime/settings/CandidateDisplayMode.kt` `storageValue` / `fromStorage`, macOS `Settings/EngineSettings.swift`, desktop-core `settings/engine_settings.rs`). The engine receives it as `AppConfig.candidate_display_mode` (§44). The desktop cycle shortcut and the picker list the modes in this order: side-by-side → combined → Romanization Only.

**Exception 1 — Romanization Only.** A TL/POJ cell renders the romanization ALONE — no subtitle, no annotation — and committing writes the romanization. A deliberate information loss: all-romanization writing does not distinguish homophones, so the user has opted out of this invariant's disambiguation. §44 collapses rows that would read identically. The swap setting and Annotate in Brackets keep their stored values but have no effect (the platform derives `isHanjiFirst = false`, `isOutputBothScripts = false` from the mode); TPS ignores the picker (hanji-first by construction).

**Exception 2 — Hanji with Romanization.** Both scripts appear **as two adjacent one-script candidates, not one cell**: for each `(Hanji, romanization)` the window lists a Hanji cell then a romanization cell, neither with a subtitle; a hanji-less candidate (§34 literal, roman-only custom entry) lists its roman cell alone. Picking a cell commits that cell's script (Enter / slot key / tap); on the desktop Output the Other Script (unbound since 2026-10-15) or ⇧ + a slot key commits the other script of the same candidate. **Cells that read the same are listed once, per script, first-seen wins** — 食/𤆬 share one `tsia̍h`; the literal `tâi` absorbs 台's roman cell; 重/tîng and 重/tāng draw ONE 重 cell and keep both roman cells, so the losing reading stays reachable through its romanization. A one-script cell cannot be told from an earlier one reading the same, so a second one is a defect. Both cells commit the SAME candidate (identity sidechannels unchanged). The platforms project the mode onto the swap pair — effective `isHanjiFirst = true` (`.primary` = hanji, `.alternate` = roman), `isOutputBothScripts = stored` — a **compatibility projection**: auto-space follows the script actually committed, the 選 label / NextWord Enter-raw gate behave as hanji-first, Annotate in Brackets applies to a hanji-cell commit. The split happens in the platform presentation layer; neither §44 pass runs. NextWord prediction rows follow the picker too: under mixed a prediction with romanization is a Hanji cell then a romanization cell (`NextWordController.buildPredictionWords` / `ActionHandler.predictionSuggestions`), same per-script first-seen dedupe; under Romanization Only the roman alone; under side-by-side the dual-script cell.

**Flags under each mode** (every platform): Romanization Only masks every stored script flag (`showsHanji`); mixed forces only the candidate lead (`effectiveHanjiFirst`); everything else — Annotate in Brackets, punctuation width (`effectiveFullWidthPunctuation`), the 文/A key / swap shortcut (`allowsSwapToggle`) — follows the stored value. So under mixed the mobile key (bottom row + overlay) and the desktop `` ` `` shortcut stay live and flip only the punctuation width; under Romanization Only the width is always half (key hidden, shortcut inert). **TPS is always full-width on every page** (character / numeric / symbolic — the derived getter ORs `isTpsLayout` in), and the 文/A key is hidden there.

**The §34 literal inherits the identity of the row it absorbs.** Under a single-script display (Romanization Only via `dedupe_display_roman`, mixed via the platform roman-cell dedupe) the dictionary row that reads like the literal has no cell of its own, so tapping the literal is the only way to commit that word. `composing::requests::adopt_collapsed_dict_identity` copies the absorbed row's `display_text` + `canonical_tl` onto the literal while `roman` / `hanji: None` stay — the cell still reads and writes the exact preedit literal; first-seen (top-ranked) wins among homophones. Consequence: a single-syllable literal that inherits a dictionary word joins the `api::nailed_prefix` compound run (`台` + literal `gí` → `tâi-gí`). Documented exception to "identity is mode-independent": the same literal tap learns `(隻, tsiah)` under Romanization Only / mixed and `(tsiah, tsiah)` under side-by-side, where 隻's own cell was available.

**One-script content is one line tall (desktop).** A list in which NO cell carries an annotation (Romanization Only; combined; a side-by-side list of roman-only rows) is one line tall, recomputed on every show / in-place update; any annotated cell brings the two-line height back for the whole list.

**Desktop keyboard path**: the picker is also a recordable global shortcut, `cycleCandidateDisplayMode`, default **⌃⌘H / Ctrl+Alt+H**, cycling in picker order. It writes the same setting, flashes the new mode's label, and the open bar is fetched again in place (the mode changes which candidates exist — §44). Unlike the swap key it is never inert.

- **Primary / secondary, not a formatted string.** The cell keeps the two scripts in separate visual roles — a smaller, dimmer secondary beside or under the primary. What COMMITTING writes is a separate rendering (engine `composing/src/commit_text.rs`, Continuous picks and predictions alike): the two agree on which script leads, and the document string may bracket the other, but the cell never renders the bracket form.
- **A one-script candidate shows one script.** A romanization-only candidate (`hanji` absent, or present-but-empty) has no second script in either swap direction and reserves no width for it.
- **Per platform**: iOS stacks them (`CandidateButtonView.swift` — title `Text` over the subtitle `Text` in one `VStack`); Android mirrors it in the smartbar; macOS puts the secondary in an annotation column (`CandidateItemView`), aligned on one x in the vertical layout.
- **The slot key beside a cell is the key that picks it.** macOS draws beside each visible slot the key the chosen set gives it (`CandidateIndexLabel`), and the set is the ONLY thing that picks: a bare `1`…`9` is always a tone digit (`tai5` + `2` composes `tai52`). Sets (`CandidateSlotKeySet`, one live at a time, chosen in the Shortcuts pane): `q w d f z x v y ;` (the shipped bare-key set — eight letters no TL/POJ syllable spells plus `;`), or `⇧n` / `⌃n` / `⌥n`. The label column is measured against every form so it never steps as the live key changes; numbering follows the viewport. Upgrade edge (accepted): `⇧` digits are read off the number-row key code, so the recorder refuses them and a global row holding one is cleared at launch; a composing row recorded as the symbol `⇧3` types is left in place and shadowed by the fixed tier.

**Scope**: platform display layers; the engine returns both fields on every candidate (`ContinuousCandidate.roman` / `.hanji`).

**Tests**: macOS `CandidateCellContentTests` (swap flips primary/annotation, romanization-only candidate annotates nothing in either direction, empty Hanji reads as absent, annotation width / font / padding, recycled cell, column floor) and `TaigiInputControllerCandidateTests.testBarCells_carryBothScriptsByDefault` / `testBarCellPrimary_isWhatCommittingLeadsWith`.

## §43 — A candidate never carries a syllable the user has not typed into

### `INVARIANT_CANDIDATE_SYLLABLE_REACH`

A continuous-input candidate is offered only when the typed input reaches INTO its final syllable. Typing `kesi` never surfaces 家私頭仔 `ke-si-thâu-á`; `tsuisi` never surfaces 水社寮 `tsuí-siā-liâu` or 水仙宮 `tsuí-sian-kiong`. The rule is identical on every platform — it lives in the shared engine. *No candidate, dictionary or custom, carries a syllable the input has not reached.*

- **The other half of §18.** §18 suppresses candidates SHORTER than what the user anchored; this is the same principle on the record side. §18 is decidable from the key alone, so it lives in the span-local key builder; "does the typed body reach the record's last syllable" needs `record.tl` (the FST key is fused), so it is answered after `dict.record(rowid)` in the lexicon hydration loop.
- **Where over-length candidates come from.** Only the partial-prefix path: `lexicon::fetch_partial_prefix_candidates_unbounded` hydrates every row whose FST key has the typed body as a **strict prefix**. Its three callers — the `keys.is_empty()` fallthrough (`gu`, `t`, `ㄉ`), the TPS `bare_expanded_only` merge, and Step 4b's whole-input extension scan — inherit the guard because it sits in the fetcher. The span-local path cannot produce one (its key IS the consumed span's toneless); the walker spans `0..raw_len`; the §34 literal is built from the typed raw.
- **Stated per-syllable, not as a count comparison.** "How many syllables were typed" has no single answer (`aia` = `ai`+`a` or `a`+`i`+`a`); a count rule would delete 阿姨仔 `a-î-á`, an EXACT key hit. Measuring how far the typed bytes reach into the record's own syllable chain leaves exact hits untouched.
- **An unfinished trailing syllable counts as reached.** `tai` does not surface 台語 `tâi-gí` (input stops ON the boundary) but `taig` does, and `taigi` still surfaces the strict extension `tâi-gír`.
- **Measured on the key surface the row was found under.** The numeric-tone families (`tl:<tl_num>` / `poj:<poj_num>` / `tps:<tps_num>`) are different coordinate systems from their toneless siblings. `KeyFace` picks the surface from the body (ASCII digit → numeric-tone family, Bopomofo tone mark → `tps_num`, otherwise fused toneless); the `er`↔`or` variant is tried as a second face.
- **The numeric faces are the build pipeline's.** `phonetics::normalize_input` supplies default tone digits only when the reading carries a mark somewhere, so an all-tone-1 reading (`kau-kuan`) derives `kaukuan` while `tl_num` holds `kau1kuan1`; a mirror that disagrees with its own key surface fails open. The mirrors are therefore per-syllable `bare spelling + tone digit`, defaulting to 4 on a stop coda and 1 otherwise, following each column's own convention — `tl_num` keeps the reading's glyphs (`thò͘-sái` → `tho͘3sai2`), `poj_num` is ASCII-folded (`khuànn` → `khoann3`). The toneless columns are these with the digits removed (`notone.py::remove_tone`).
- **Reach against the TYPED key, reading against the MATCHED key.** For §35 families the matched key is a substituted FULL stored key; substitution is charwise, so the typed prefix's byte length is the reach on either face.
- **Custom-dictionary entries answer the same rule.** `lexicon::continuous::custom_reach` measures each custom row through the same `SyllableReach` faces, on the reading the store derived its search keys from (`phonetics::api::custom_search_reading`), with typed spaces and hyphens dropped as the store's query key drops them. It sorts the row into `Exact` (face = typed body: the exact path's FULL block), `Extension` (reaches the final syllable: stamped `COVERAGE_KIND_PARTIAL_PREFIX`) or `Unreached` (dropped). Unlike a dictionary row it **fails closed** (a face that does not cover the typed body is an acronym or longer spelling). An acronym is answered by the abbreviation block only, on the exact face (`tkt`, never `tk`), as a dictionary row is (`lexicon::continuous::merge_custom_abbreviations`); the keystroke store query (`CustomDictionaryStore::keystroke_rows`) matches the abbreviation whole and spends its row limit shortest key first, while the dictionary search page keeps its prefix union (`rows_matching`). A custom 陳建中 `Tân Kiàn-tiong` appears from `tankiant` or `tkt`, never from `t` / `ta`.
- **Dictionary rows fail open on an unreconstructable surface.** A family with no romanization face (`hanzi:`), or a body the face does not cover (acronym faces, rejected by sibling guards), keeps its pre-rule behaviour. Every production romanization face IS reconstructable (pinned against the shipped CSV), so a fail-open means real drift.
- **Rejected rows still spend hydration budget.** `PARTIAL_PREFIX_HYDRATE_CAP` caps records INSPECTED, not eligible records collected; shortest-matched-key-first front-loads single-syllable readings.

**Effect on §35**: a lone `ㄇ` is one syllable, so 毋 and every single-syllable `m-` reading appear, while 毋是 waits for `ㄇㄒ`. Fully typed words (考卷, 毋是, 會記得, 雞胸, 烏白, 包袱) resolve from their own input.

**Scope**: shared Rust engine (`lexicon` + `phonetics`), no `.proto` or key-family change. Needs `make build`, not `make dict`.

**Engine sites**: `lexicon/src/continuous/` (`SyllableReach` + `KeyFace`, the guard in `fetch_partial_prefix_candidates_unbounded`, `custom_reach`), `phonetics/src/syllable.rs` (`tl_num_syllable_ends_from_tl`, `poj_num_syllable_ends_from_tl`), `phonetics/src/tps.rs` (`tps_notone_syllable_ends_from_tl`, `tps_num_syllable_ends_from_tl`). The `*_syllable_ends` entry points exist because a face's derivation is **not prefix-stable under truncation** (default tones are decided per reading), so boundaries are read off one full-reading pass; they return the fused face plus offsets because the guard runs up to `PARTIAL_PREFIX_HYDRATE_CAP` times per keystroke.

**Deliberately not folded in.** The prefix test inside `SyllableReach::syllable_ends` duplicates the `matches_continuous_*_toneless_prefix_key` guards, and `phonetics::tps_notone_prefix_boundary_tone` (§41) walks the same accumulation. Those guards **fail open on a tone-bearing body** while this one must measure it, so unifying would tighten three shipped behaviours — a refactor round with its own behaviour-freeze list.

**Tests**: `lexicon/tests/roman_num_face_parity.rs` — the load-bearing one: reads the shipped `dictionary/output/dictionary.csv` and asserts a byte-match for all romanization columns over every row (a hermetic fixture built from the same mirror cannot catch drift). `lexicon/tests/partial_prefix_syllable_reach.rs` (TL keep/drop either side of a boundary, one letter into the next syllable, lone initial, 4-syllable drop, numeric-tone face for `tai5`, an unmarked all-tone-1/4 reading under a toned key, POJ parity, TPS, `er`↔`or`, custom entries in TL / POJ / TPS on the store's search reading, acronyms off the phonetic path, hydrate-cap flood); `composing/tests/golden/fetch_at_pos.golden`; `lexicon/tests/span_local_fetch.rs::partial_prefix_unbounded_exposes_full_pool_for_cross_batch_dedupe`. Dogfood **S25**, custom entries **S122**.

## §44 — Romanization Only cells collapse rows that read the same

### `INVARIANT_ROMAN_ONLY_CELLS_COLLAPSE_SAME_ROMAN`

When Candidate Display = Romanization Only (`AppConfig.candidate_display_mode = ROMAN_ONLY`), the engine never returns two TL/POJ candidates that would render as the same roman-only cell: continuous candidates are deduped on the rendered roman and NextWord predictions on their rendered `text`, both AFTER their score sort so the survivor is the best-ranked row (or the §34 literal, inserted at index 0 after the sort). Under any other value — `SIDE_BY_SIDE`, the proto3 default `UNSPECIFIED`, or an unknown value — neither pass runs. **The span is not part of the key**: under a single-script display a cell that reads exactly like an earlier one is never listed, even when the two commit different slices (uniform with §42's per-script dedupe).

- **Why the engine, not the cell renderers.** Same-roman rows are distinct words under Core Principle #6 (`食/tsia̍h` vs `𤆬/tsia̍h`), so the identity dedupe `lexicon::dedupe_by_roman_hanji_span` keeps both; only a display-tier pass may collapse them, and it needs the sort order and the POJ-rendered roman — engine-side facts. One implementation keeps every platform's strips and NextWord row identical (§5 precedent, TPS side).
- **Two passes, because predictions bypass the continuous pipeline.** `nextword::filter` merges by `(hanzi, tl)` and `shape_prediction` emits the same `text` with a different `subtitle` for homophones; the platform prediction cell drops the subtitle here, so the prediction pass runs after the stable score sort and before the limit, filling the strip with distinct cells.
- **The literal is the survivor when it is in the group** (`隻/tsiah` collapses into the literal `tsiah`); it inherits the absorbed row's identity (§42).
- **Custom entries survive their group.** `CandidateSortKey`'s `preference_class` puts a never-selected custom row ahead of the never-selected rows of its tier's readings (or, for an abbreviation hit, its abbreviations), so a custom `tsia̍h/X` is the first-seen row of its roman group unless a selected row of the group leads it — and except against rows placed ahead of every sort: the walker's slot 0 and the §34 literal.
- **One normalisation point.** `AppConfig::is_roman_only_display` (`engine/protos/src/lib.rs`) is the only place the raw `i32` is interpreted.
- **Platform side derives, never re-implements.** Each platform keeps the swap (stored as `isTranslateSwapped`) and, on mobile, `outputBothScripts` STORED untouched (the desktops have no `outputBothScripts` key and send `both = false`), and exposes a derived pair (`stored && mode != romanOnly`) at its `EngineSettings` seam; the engine therefore receives `swapped = false, both = false` under Romanization Only, and every reader — `continuous_word_space`, `decide.rs::decide_word_selected`, `filter.rs::shape_prediction`, auto-space, keycap width, 選/suán, full-width punctuation, desktop Output the Other Script (`.ignored` on an annotation-less cell) — behaves as roman-first with no new branch. The 文/A key is hidden on mobile under Romanization Only (the space bar takes the width) and the toggle stays guarded (stored value preserved; desktop shortcut a silent no-op); Annotate in Brackets UI is disabled; TPS/English paths are untouched. Punctuation width reads its own derived value (`effectiveFullWidthPunctuation`, §42).

**Scope**: shared Rust engine (`protos` + `composing::requests` + `nextword::filter`) + per-platform settings/cell arms on all five platforms. Needs `make build`, not `make dict`.

**Engine sites**: `protos/proto/envelope.proto` (`CandidateDisplayMode`, `AppConfig.candidate_display_mode = 9`), `protos/src/lib.rs` (`is_roman_only_display`), `composing/src/requests.rs` (`dedupe_display_roman` after the literal block), `nextword/src/filter.rs` (step 3b).

**Tests**: `composing/tests/roman_only_display_dedup.rs` (keeps top-ranked 食 over 𤆬, literal absorbs 隻/tsiah and keeps slot 0; side-by-side / `0` / `99` keep all rows byte-identically; POJ keys on the rendered `chia̍h`; TPS independent of the picker), `nextword/src/filter.rs` (`roman_only_display_collapses_same_roman_predictions_keeping_the_best`, `roman_only_display_dedupe_runs_before_limit_truncation`). Platform tests pin the derived pair / stored-value preservation / cell arms. Dogfood **S26**.

## §45 — The `o͘ⁿ` spelling of the nasal final is respelled per syllable, never per buffer

### `INVARIANT_ROMAN_NASAL_OO_ALIAS_IS_SYLLABLE_LOCAL`

The nasal final /ɔ̃/ is canonically `onn` (TL) / `oⁿ` (POJ) — `docs/phonetics/taigi-phonetics-reference.md:118` + `:304`, and `taigi-converter/src/tables.js` `POJ_FINAL_SUBS`, which orders `nn`→`ⁿ` ahead of `oo`→`o͘`. Some writers use the 台日大辭典 rendering `o͘ⁿ`, which reaches the engine as ASCII `oonn`; the MOE2 POJ layout has an `o͘` key and an `nn` key, so a user can type it directly.

Those four letters are a nasal final **only inside one syllable**. Across a seam they are an `oo` final meeting the next syllable's onset — 滷卵 `loo|nng`, 芋卵 `oo|nng`, 菜脯卵 `chhai|poo|nng`, 可惡 `kho|onn`, 苦惡鳥 `khoo|onn|chiau`. The invariant is about **scope**:

- **The alias is produced by EXPANDING at build time; no lookup-time fold reconstructs it.** `dictionary/build/create_fst.py` respells `onn`→`oonn` per syllable of `tl_num` / `poj_num` and emits it as an extra key body beside the canonical one, same rowid; `fst-builder`'s `build-syllables` does the same via `phonetics::nasal_oo_alias_spelling`. Expanding is unambiguous because the builder still holds syllable boundaries. (`canonical_tl_form` folds `oonn`→`onn` at runtime per syllable, for the identity key only.)
- **No whole-buffer fold exists.** `phonetics::TL_ENCODING_RULES` — applied to a whole buffer by `composing::shadow::canonicalize_poj_shadow` — must not contain `oonn`→`onn`: at that scope it fires across the seam (`lo͘nng` → `lonng` loses 滷卵; `khooⁿ` → `khonn` loses 可惡). The per-syllable `NORMALIZE_TO_TL_RULES` keeps the fold for the cross-mode identity (Core Principle #6).
- **Indexing is purely additive.** Every added key contains `oonn`; `tps:` and `hanzi:` untouched. `tl:khoonn` / `poj:khoonn` / `tl:khooonn` / `poj:khooonn` collide with existing keys — the genuine 可惡 / 苦惡 ambiguity; both readings surface.
- **A build-time alias needs its runtime counterpart in the face guards.** `lexicon::continuous`'s guards reconstruct a face from the record and compare it to the key the row was found under; 好 `hònn` reconstructs `honn` but is found under `tl:hoonn`. Without an alias arm in both the equality guard (span-local + walker) and the prefix guard (partial-prefix), alias-indexed rows hydrate and are filtered straight back out. Same shape as the `er`↔`or` alias (`matches_continuous_tps_toneless_key` → `tps_notone_or_variant`).
- **The prefix guard's alias arm is gated on the TYPED body.** A respelled face is a strict superstring of the canonical one, so consulting it unconditionally matches every prefix of it (`tl:hoo` would match 好/否/呼/齁 through `hoonn`, and the capped extension pool would displace real rows). The alias applies only once `oonn` is actually typed.
- **Custom entries get the same treatment at map-build time.** `composing::continuous` registers the alias spelling of each custom key alongside it; an alias key is disjoint from every canonical key, so a cross-seam respelling yields a dead lookup, never a wrong hit.
- **Display is untouched.** Composing stays literal (§30); the candidate's roman comes from the dictionary record.

**Scope**: dictionary build + shared Rust engine (`phonetics`, `lexicon`, `composing`). Needs `make dict` **and** `make build`.

**Tests**: `phonetics/tests/invariants.rs` (`invariant_roman_nasal_oo_alias_is_syllable_local`, `invariant_roman_nasal_oo_alias_never_folds_whole_buffer`), `phonetics/src/syllable.rs` (respell unit cases), `lexicon/src/continuous/` `nasal_oo_alias_face_tests` (both guards admit the alias; the prefix guard does not leak into a canonical prefix; a cross-seam face is left alone), `dictionary/tests/test_nasal_oo_alias.py`. Dogfood **S28**.

## §46 — An acronym-shaped buffer reaches the words it abbreviates

### `INVARIANT_CONTINUOUS_WHOLE_BUFFER_ABBREV`

`dictionary/common/abbrev.py` gives every ≥ 2-syllable row an abbreviation face — `tl_abbrev` (`só-sî` → `ss`), `poj_abbrev` (over the POJ display, `chia̍h-pn̄g` → `chp`), `tps_abbrev` (`ㄙㄒ`). A whole buffer that is such a face reaches those rows (`ss` → 鎖匙 `só-sî`), while a **span** key inside a longer buffer never does (`tl:gi` inside `ginalangtsiahpngbesai` is a syllable; 外夷 `guā-î` shares no letters with it).

- **The index, not a heuristic, says what an abbreviation is.** `create_fst.py` emits every row's abbreviation face under its own family — `tl-abbrev:` / `poj-abbrev:` / `tps-abbrev:` (`docs/engine/binary-format.md` §3.2) — and nothing else goes there. A phonetic family (`tl:` …) never holds an acronym key, so the span-local / walker / partial-prefix fetches stay acronym-free with no per-key predicate, and `composing::shadow::abbrev_query_key` is a plain letter screen: ≥ 2 glyphs, TL / POJ ASCII letters only, TPS glyphs without tone marks — no digit, hyphen or space, so no `TonePin` or barrier applies. Vowel-initial abbreviations are reachable (紅嬰仔 `aea`, 育幼院 `iii`, 阿姨 `ai`): a buffer that is also a reading gets both blocks. Step 4c runs after BOTH fetch branches (`mk` has the left-anchored syllable `m` and still abbreviates 物件). Tab3 `lexicon::search` unions both families (main exact, abbrev exact, main prefix, abbrev prefix).
- **The face is one leading spelling unit per syllable.** `dictionary/common/abbrev.py::extract_abbrev` and its runtime mirror `phonetics::derive_abbrev` take, per syllable (lowercased, diacritics stripped), the longest initial it starts with — TL `tsh` / `ts`, POJ `chh` / `ch`, shared `ph` / `th` / `kh` / `ng` / `p m b t n l k g s j h` (`docs/phonetics/taigi-phonetics-reference.md` §2) — or its first letter when it has no initial. 披頭巾 `phi-thâu-kin` → `phthk`, 出去 `tshkh` (POJ `chhkh`), 食飯 `tsp` (POJ `chp`), 鎖匙 `ss`, 紅嬰仔 `aea`, 毋是 `ms`, 黃酸 `ngs` (syllabic `ng` whole), 卵仔 `na`. TPS takes the one initial glyph (`tps_abbrev_from_tl`, 披頭巾 `ㄆㄊㄍ`), so the three families agree. The first-letter face (`ptk`) is not indexed — a dual index would bring the `ph` = `p`+`h` ambiguity back. Custom words follow the same face through `phonetics::custom_search` (§10). The legacy `abbrev` column on the phones' files stays first-letter and is never read.
- **A hit is whatever the family holds.** `lexicon::fetch_abbrev_candidates` does an EXACT `lookup_exact` of the tagged key and keeps every row that passes the source filter; the face is pinned at build time (`tests/roman_num_face_parity.rs`, `tests/tps_abbrev_parity.rs`). Exact, not prefix: `ss` does not reach 總鎖匙. A whole-buffer `gi` lists the `g…-i…` words in its tail like any abbreviation.
- **Ranking: below everything syllabic of the same coverage.** Hits carry `COVERAGE_KIND_ABBREV` (= 2), which `CandidateSortKey` orders after `PARTIAL_PREFIX` within a tier, and are appended after a cross-batch exclude (`retain_absent_from`, same `(canonical_tl, hanji, span)` identity or same recased `(roman, hanji, span)`, shared with Step 4b); Step 4d re-sorts the merged list. `tier` leads (`continuous-input-ranking.md`): a whole-input abbreviation precedes a row covering only the front (`mk` 物件 before `m` 毋), and a custom word leads the block. Within the block: user weight, then dictionary frequency. **Order = sort → recase → exclude → truncate** (`PARTIAL_PREFIX_OUTPUT_CAP`, applied by the composing step): a row reachable as both a Step 4b extension and an abbreviation hit shows once, and the block is still filled to the cap.
- **Metadata is the record's.** `consumed_span` = the whole buffer, `syllable_count` = the record's (鎖匙 = 2), `canonical_tl` = the record's TL, so frequency / NextWord learn the same identity as a syllabic selection. Recase mirrors the buffer (`SS` → `SÓ-SÎ`); POJ mode renders POJ (`cp` → `chia̍h-pn̄g`). Custom entries are not merged again.
- **Cost.** Hydration is uncapped (a rank-blind cap in rowid order would drop the frequent words the sort exists to surface); the largest production key (`tl:tt`, ~8.5k rows) fetches in ~6 ms release on an M-series Mac.
- **The family split keeps single-initial partial-prefix input un-starved**: with no acronym key in the phonetic range, bare `t` / `s` / `m` / `a` / TPS `ㄇ` / `ㄍ` list their frequent single-syllable words. By design, the shortest-first hydration budget (`PARTIAL_PREFIX_HYDRATE_CAP`) fills with ≤ 3-letter keys under a bare initial.
- **Known asymmetry**: the custom-dictionary settings search matches abbreviations by prefix (`CustomDictionaryStore::rows_matching`, `LIKE ? || '%'`) while this dictionary lookup is exact.

**Scope**: dictionary build (`create_fst.py` abbreviation family) + shared Rust engine (`phonetics::derive_abbrev`, `lexicon::fetch_abbrev_candidates` + `COVERAGE_KIND_ABBREV`, `phonetics::abbrev_family_key`, `lexicon::search` union, `composing::shadow::abbrev_query_key` + Step 4c). Needs `make dict` **and** `make build`; every platform inherits through `FetchAtPos`.

**Tests**: `dictionary/tests/test_abbrev_onset.py` + `phonetics/src/derivation.rs`, `dictionary/tests/test_fst_abbrev_family.py`, `composing/src/shadow.rs` (`abbrev_query_key_*`), `lexicon/tests/whole_buffer_abbrev.rs`, `lexicon/tests/tlpoj_partial_prefix_abbrev.rs` + `tps_partial_prefix_abbrev.rs` + `tps_readings.rs` (an acronym flood under `*-abbrev:` never enters a phonetic-family scan), `lexicon/tests/roman_num_face_parity.rs` (`tl_abbrev` / `poj_abbrev` byte-match over every multi-syllable CSV row), `composing/tests/continuous_abbrev.rs` (end-to-end: literal at 0, block order under `ts`, `mk` overlaps deduped with the block still filled, recase, separator / digit / single-glyph negatives, POJ face, TPS byte span, custom dedupe). Production `candidate_dump` reference: `ss` → 先生 … 鎖匙; `tshkh` → 出去; `tsp` → 食飯; `aea` → 紅嬰仔; `ginalangtsiahpngbesai` no 外夷; TPS `ㄙㄒ` → 鎖匙. Dogfood **S50**.

## §47 — The macOS symbol picker stands on a placeholder marked region

### `INVARIANT_MACOS_SYMBOL_PICKER_PLACEHOLDER_MARKED_TEXT`

The desktop symbol picker opens only once the composition is over. A host that is not a Cocoa text view reads "no marked text" as "no input method at work": Chromium — Chrome and every Electron app — runs the input method's turn and then forwards the REAL key event to the page unless marked text was present before or after that turn (`content/app_shim_remote_cocoa/render_widget_host_view_cocoa.mm`, `keyEvent:`). Over a bare insertion point the arrows would walk the list AND move the page's caret, and Return would pick a symbol AND reach the page (a chat box submits). Negative control: the composing candidate bar walks the same `handle` → `navigate` path and never leaks — it has marked text under it.

- **The picker marks a placeholder while it is up.** `presentSymbolPicker` writes one space, underlined like a composition, caret after it (`ClientWriter.setMarkedText(symbolPickerPlaceholder, caretUTF16:)`) BEFORE the caret walk, so the anchor is the placeholder's rectangle; `TaigiInputController.isSymbolPickerPlaceholderMarked` records it. vChewing keeps the same placeholder for the same reason (`IMEStateParsed4Darwin.swift:256-266`, `InputSession_HandleStates.swift:58-64`). A space, not U+200B: a zero-width leftover would be invisible.
- **Never over a selection.** Marked text replaces the selection the way typing does, and clearing it on Escape would not bring the selection back, so `presentSymbolPicker` reads `client.selectedRange()` first and marks nothing over a non-collapsed one — the list still opens over the bare anchor, and a pick replaces the selection. `NSNotFound` reads as a caret.
- **Down with the list, cleared once.** `dismissSymbolPicker` clears the placeholder (`.clearPreeditWithoutCommit`, through `lastClient`) only when one went in, and drops the flag before the write so a re-entrant client callback finds nothing left to clear; a host never gets an empty region it did not have (McBopomofo #346). `activateServer` drops the flag without a client call. `isMarkedTextVisible` stays the composition's: `finishComposition` dismisses the picker before it reads the flag.
- **Cleared BEFORE the picker's key writes.** A pick clears the placeholder, then inserts over a document with no marked region — the auto-space swap (`swapAutoSpace`) measures `selectedRange()` and rewrites with an absolute `replacementRange`, which IMK would read relative to live marked text. A letter that closes the list clears the placeholder, then marks its own composition (`" "`, `""`, `"t"`).
- **Windows is untouched.** TSF's key sink eats the key from `OnTestKeyDown` / `OnKeyDown` with no composition needed (`session.rs::symbol_picker_takes_key` answers TRUE at both phases for a live picker).

**Scope**: macOS only — `Controller/TaigiInputController.swift` (`isSymbolPickerPlaceholderMarked`, `presentSymbolPicker`, `dismissSymbolPicker`, `activateServer`). No engine change.

**Tests**: `TaigiInputControllerSymbolPickerTests` against `RecordingTextInputClient` (whose `setMarkedText` edits the simulated document) — `testTheChord_marksAPlaceholder_beforeAnchoring`, `testThePlaceholder_isInTheDocumentOnlyWhileTheListIsUp`, `testOverASelection_nothingIsMarked_andThePickReplacesIt`, `testClosing_clearsThePlaceholder_once`, `testAClientWithNoCaret_opensNothing` / `testAListTheWindowRefused_leavesNoPickerBehind`, `testALetter_closesThePicker_andStartsComposing`, `testABracketPair_isOneWrite`, `testDeactivate_ofAnotherSession_leavesTheLiveSessionsPickerAlone`, `testAPickedAttachingMark_swapsWithTheAutoSpace`. Not pinned by a mock: Chromium's deferred composition updates between the clear and the swap's `selectedRange()` on one Return. Dogfood **S51**.

## §48 — Hanji leads out of the box; the desktop General pane owns the output script and its own reset

### `INVARIANT_HANJI_FIRST_DEFAULT`

A fresh install on every platform leads side-by-side cells with the Hanji, writes the Hanji on commit, and types full-width punctuation.

- **The stored swap defaults to `true` on all five platforms.** desktop-core `EngineSettings::DEFAULT` (one `const`, read by `keys.rs` and `Default`) names the stored swap once and DERIVES the snapshot's effective `is_hanji_first` / `is_full_width_punctuation` from it under side-by-side, so a defaults snapshot cannot disagree with itself; macOS `EngineSettings.defaults` is the default table `SettingsStore.Keys` reads, pinned to the core's by `DesktopCoreRuntimeTests.testWhitelist_matchesSettingsStoreDefaults`, and its punctuation width (`SettingsStore.isFullWidthPunctuation`) applies the same rule; iOS `SharedSettings.isHanjiFirstKey` default + `resetToDefaults`; Android `PrefHelper.storedIsHanjiFirst` default + reset. The engine-side `AppConfig` struct defaults are wire defaults every platform overrides.
- **A never-stored key follows the default; a stored one does not.** The readers all follow "absent → default" (`SettingsStore.bool`, `SettingsDocument::bool`, `SettingsKey.bool(default:)`, `by preference(…, true)`), so a stored `false` ("I chose romanization") is kept. No migration writes the default through — a written default is indistinguishable from a choice.
- **Derivation is untouched.** §42's `effectiveHanjiFirst` / `effectiveOutputBothScripts` / `effectiveFullWidthPunctuation` still read the stored value through the display mode.
- **The mobile 文/A key lights for the NON-default.** iOS `ButtonImageProvider` draws `character.square.fill` and Android `KeyContent.kt` paints `keyBgActive` when `isFullWidthPunctuation` is FALSE (romanization lead, half-width marks); the shipped state is the plain key, so a fresh install shows no key "on" the user never switched on. Presentation only. The Android theme preview (`KeyboardPreviewPanel.kt`) renders the shipped state.

### `INVARIANT_DESKTOP_GENERAL_PANE_OUTPUT_SCRIPT_AND_RESET`

- **Output Script is a row on the General pane, bound to the stored swap** (macOS, Windows, Linux). Labelled `settings.outputScript`, paired with `settings.inputScript` on the TL / POJ picker (mobile keeps Input Mode — its picker also holds TPS). The pane is one run of rows, no sub-groups, in typing-pipeline order. macOS `GeneralSettingsView` `@AppStorage(Keys.isHanjiFirst)` picker (`Hanji` = `true`); Windows `general.rs` `choice_row` over `OUTPUT_SCRIPTS = [true, false]` writing `Message::SetSwitch(keys::IS_HANJI_FIRST, …)`; Linux `taigikeyboard-settings/src/pages/general.rs` over `keys::IS_HANJI_FIRST` (stored as `isTranslateSwapped`). Same key the `` ` `` chord flips (`TaigiInputController.performShortcutAction .toggleTranslateSwapped`, `session.rs ShortcutAction::ToggleTranslateSwapped`), so a chord press repaints the open pane.
- **Disabled exactly where the chord is inert.** `CandidateDisplayMode.allowsSwapToggle` / `allows_swap_toggle` (§42): greyed under Romanization Only with the stored value still showing; enabled under Hanji with Romanization, where it only decides punctuation width.
- **Reset to Defaults at the foot of the pane, same shape as the other panes.** macOS `WideActionRow(.themeEditorResetAll)` → `SettingsStore.resetGeneralSettings()`; Windows / Linux `reset_row` → `SettingsDocument::reset_general()` over `keys::GENERAL_KEYS`. The roster is spelled out (the two lists name the same keys) and the keys are removed, not written. Not in it: `displayLanguage` (a reset must not switch the UI language under the user), update bookkeeping and the remembered pane, and every other pane's keys.

**Scope**: all five platforms for the default; the three desktops for the pane. No engine change. i18n: `settings.inputScript` / `outputScript` / `outputScriptHanji` / `outputScriptRoman` (desktop scope).

**Tests**: macOS `SettingsStoreTests` `testStore_withNothingStored_isHanjiFirst`, `testResetGeneralSettings_putsEveryRowBackAndLeavesTheRestAlone`; desktop-core `document.rs` `empty_document_reads_every_default`, `reset_general_removes_the_pane_s_keys_and_nothing_else`, `keys.rs` roster dedupe; `composing_manager.rs` fresh-rig commit writes 台語; iOS `SettingsKeyTests` absent-keys / reset land hanji-first. Suites that mean "roman-first" opt in explicitly. Dogfood **S52**.

## §49 — The Syllable Separator rewrites the romanization the user sees, never the keys the engine learns by

### `INVARIANT_SYLLABLE_SEPARATOR_DISPLAY_ONLY`

A three-way picker — Hyphen (default) / Space / None — rewrites the separators the engine or dictionary produced in the romanization the user sees; user-typed hyphens stay, TPS is unaffected, custom-dictionary rows follow it, and identity keys never change. Under Space the neutral-tone dot stays on its syllable (`hōo ·guá`).

- **One field, one helper, three readers.** `AppConfig.syllable_separator` (proto enum field 14) → `AppConfig::rendered_syllable_joiner` (`engine/protos/src/lib.rs`): `None` = leave the romanization as it is (Hyphen, `UNSPECIFIED`, an unknown value, the TPS layout), `Some("")` = None, `Some(" ")` = Space → `phonetics::api::syllable_joiner_display(roman, joiner)`. Each run of `-` becomes the joiner followed by one `tps::KHINSIANN_DOT` `·` U+00B7 per `--`; the joiner only stands between two syllables, so a run opening the text or a word (`--ah` → `·ah`) or closing one adds none; spaces (word boundaries) are kept. None: `tâi-uân` → `tâiuân`, `hōo--guá` → `hōo·guá`, `sia̍p---khì` → `sia̍p·khì`. Space: `tâi uân`, `hōo ·guá`, `tsáu--tshut-khì` → `tsáu ·tshut khì`. Read by (1) `composing::continuous::assemble_candidates` Step 5 on every `RawCandidate.roman` after the POJ render and before the rendered dedupe (which runs whenever the joiner is set) — dictionary, custom and walker synth alike, BEFORE the §34 literal prepend; (2) `nextword::filter::shape_prediction` on `EnginePrediction.text`, same POJ-then-separator order; (3) `composing::api::nailed_prefix`, whose synthesised compound joiner becomes the joiner and whose typed-run boundary (§51) renders through `phonetics::api::push_syllable_joiner_run` — under Space writing the joiner in front of a piece that renders as `·guá`. Nothing else changes: `derived_display` (preedit, §34 literal, Enter commit), the §21 leading `--` literal, the FST / shadow keys, `roman_reading_eq`, `canonical_tl_form` and `api::learned_reading` (§50) never see the setting.
- **Identity fields keep the dictionary form.** `CandidateMessage.display_text` / `canonical_tl` and `EnginePrediction.tl` are untouched, so frequency, `CommitContinuous.canonical_text` / `association_tl` and `WordSelected` learn `tâi-uân`. Android's non-continuous tap paths (`CandidateClickHandler`) hand NextWord the `CANONICAL_TL` sidechannel, falling back to `roman` only for rows with none (English) — mirroring iOS `ActionHandler+Suggestions.swift` `associationRoman`.
- **User-typed hyphens are literal (§30).** `tai-uan` shows the literal `tai-uan` at slot 0 beside the dictionary `tâiuân` / `tâi uân` 台灣; `--ah` still commits `--ah` / `--矣` (§21). The rewrite runs before the literal `retain` and §44 dedupe, so those compare the rendered form.
- **TPS is exempt, applied by the engine.** The platform re-splits `CandidateMessage.roman` on `-` for `TlDisplayToTps`, so `rendered_syllable_joiner` answers `None` for `input_mode = "tps"`. Every platform sends the setting as stored on every request's config (iOS `SharedSettings.syllableSeparator`, Android `PrefHelper.syllableSeparator`, desktop-core `EngineSettings::syllable_separator`).
- **One key spelling.** `syllableSeparator` with raw values `hyphen` / `space` / `none` on iOS, macOS (`SettingsStore.Keys`, General pane under Output Script, in `resetGeneralSettings`; desktop-core reads it through `key_path.rs`) and Windows / Linux (`keys::SYLLABLE_SEPARATOR`, `GENERAL_KEYS`, General page); Android `keyboard__syllable_separator`. Mobile: host-app Input section, no in-keyboard overlay row. Default Hyphen.
- **The retired switch carries over once.** A stored `hyphenlessRomanEnabled = true` (Android `keyboard__hyphenless_roman`) becomes `none` unless a separator is already stored; the old key is removed either way — `carryOverHyphenlessRoman` per store: iOS `SharedSettings` (at the app's and the keyboard's launch), Android (in `migrateFromSharedPreferences`'s transaction), macOS `RetiredSettingsCleanup.run`, desktop-core `carry_over_hyphenless_roman` in `SettingsDocument::from_json`.

**Scope**: engine + all five platforms; `make build` before any iOS / Android / macOS build or test. i18n: `settings.syllableSeparator`, `settings.syllableSeparatorSpace`, `settings.syllableSeparatorNone`, `settings.syllableSeparatorInfo` (mobile); the Hyphen value reuses `desktop.telexGuideHyphen`.

**Tests**: engine `protos` `syllable_joiner_follows_the_separator_off_the_tps_layout` / `tps_layout_never_rewrites_the_syllable_hyphen`, `phonetics::api::tests::syllable_joiner_*`, `composing::api::tests::syllable_joiner_joins_the_compound_run_and_keeps_the_typed_hyphen` / `typed_run_renders_through_the_syllable_joiner` / `space_joiner_separates_a_rendered_dot_piece_after_a_typed_run`, `nextword::filter::tests::syllable_separator_shapes_prediction_text_but_not_its_tl_key`, `composing/tests/syllable_separator_display.rs`, `continuous_typed_separator.rs`; golden unchanged (default Hyphen). iOS `SettingsKeyTests`, `RustEngineBridgeAppConfigTests`; Android `SyllableSeparatorTest`, `EngineAppConfigTest`; macOS `SettingsStoreTests`, `RetiredSettingsCleanupTests`, `DesktopCoreRuntimeTests`; desktop-core `document.rs` `syllable_separator_reads_the_stored_choice` / `a_stored_no_hyphens_switch_carries_over_as_none`, `bridge.rs` `syllable_separator_reaches_the_config`. Dogfood **S60** (None), **S106** (Space + the picker).

## §50 — A phrase composed segment by segment becomes a whole-buffer candidate

### `INVARIANT_LEARNED_PHRASE_COMPETES_NEVER_OVERRIDES`

Type `kikhilai`, pick 記 → 起 → 來 one segment at a time; the next `kikhilai` offers 記起來 as one candidate.

- **The engine learns and stores.** `commit_continuous`'s FINAL branch (pending consumed in full) hands back a learned `(hanji, canonical_tl)` pair (`composing::Applied.learned`), which the engine writes to its own `learned_phrases.db` when the composition has ≥ 2 nailed segments, every segment carries `NailedSegment.hanji` and a non-empty `association_tl`, and the joined TL has ≤ `MAX_LEARNED_PHRASE_SYLLABLES` (6) syllables (`phonetics::api::tl_syllables` — counted off the TL, not the segments' echoed `syllable_count`, which a custom pick reports as `1`). `hanji` = the segments' hanji concatenated; `canonical_tl` = the segments' canonical TL under the word boundaries the roman commit renders (`api::learned_reading` — the §51 nailed join over `association_tl`, whatever the display settings): a run of single-syllable picks forming a dictionary compound joins with `-`, other words with a space, so 做 → 進 → 出 → 口 over `tsotsintshutkhau` learns `tsò tsìn-tshut-kháu`. The separator the user typed between two picks wins: the `-` run typed before a segment folds into that segment's raw prefix (記 picked over `ki` in `ki--khilai` leaves `--khilai` pending); two or more `-` learn the khinsiann `--`, one learns `-`. The separator KIND is remembered, never the verbatim run (`ki---khilai` → `kì--khí-lâi`, `transition::canonical_separators`). A piece already opening with `--` keeps its dictionary form and binds to the word before it (`kì` + `--khí-lâi` → `kì--khí-lâi`). Only the boundary between two picks reads the typed run: a pick that is itself a multi-word walker synth carries its space-joined `canonical_tl` (`khí lâi`) as is. A mid-commit, a single segment, a hanji-less segment (§34 literal, OOV synth, roman-only custom row), an empty TL, an abort or a backspace-unnailed segment never learn (`transition::learned_phrase`).
- **Hanji is an explicit sidechannel.** `CommitContinuous.hanji` (proto3 `optional`, field 6) carries the chosen `CandidateMessage.hanji` whenever the pick has one, whatever script the document received (identity = the `(Hanji, canonical-TL)` pair, Core Principle #6). The engine never infers hanji from `canonical_text` vs `association_tl`. A caller that omits the field never learns.
- **Learned and custom rows compete.** Learned rows reach a fetch as `composing::UserRows.learned` (the engine's own store read), never as custom rows. In the walker, `learned_edge_key` (rendered to POJ first under POJ input, then `custom_toneless_key`, which folds a stored reading's word space into `-` for custom and learned rows alike) maps each row onto the mode's key family through the same `edge_key_map` the custom map uses (nasal-`oo` alias registered too); rows keyed to an edge join `lexicon::best_candidate_for_key_with_barriers`'s `CandidateSortKey` pick with `frequency = 0`, `bitmask = 0`, `is_custom = false`, so a dictionary homophone under the same key wins unless `user_frequency` prefers the learned pair, and the learned row IS the edge only when the dictionary has nothing under the key. The edge's `span_walker_cost` is capped at the loaded dictionary model's custom-word cost whenever a learned row matched, so the span the user once composed as a word keeps winning the segmentation. The span-local list carries the same row at `(0, raw_len)` (`lexicon::learned_entry_to_candidate`); the `(canonical_tl, hanji, span)` dedupe keeps a dictionary or custom duplicate over it. Custom rows under the edge's key (`custom_toneless_key`, tone-pin eligible) join the same pick ahead of the learned rows, at the edge's own span, after the same dedupe — so the edge's word follows the list's `preference_class` (USER 2026-10-16: a custom 李 picked 7× takes the `li2` edge from a never-picked custom 屢 filed first, and a picked 你 from both), and a custom row caps the span at the loaded dictionary model's custom-word cost like a learned one.
- **Result**: with the 台日大辭典 source off (default), learned 記起來 is the first hanji candidate for `kikhilai` after ONE composition; with it on, the dictionary row leads and the pair is listed once; a mistaken learned phrase never displaces a dictionary word the user did not pick; a never-selected custom row still leads a never-selected learned one.
- **A slip and its correction are two rows; the settled one surfaces.** The store key is `UNIQUE(hanzi, roman)`, so `guá-sī` and `guá--sī` for 我是 are two rows, each with its own `learn_count` / `updated_at`; the exact pair stays the identity everywhere (`user_frequency.db` keys them apart too). Recall folds them: `composing::requests::first_learned_per_reading` drops a row whose `hanji` equals an earlier row's and whose `canonical_tl` is the same reading under `continuous::roman_reading_eq`, so only the store's first row (`learn_count DESC, updated_at DESC`) becomes a candidate or walker edge. The fold is between learned rows only; the store's `LIMIT 5` runs before it.
- **Own store, invisible, learning-data lifecycle.** Learned phrases live in `learned_phrases.db` on every platform (`engine/userdata/src/learned_phrases.rs`: `learned_phrases` + `learned_search_key`, keys from the custom dictionary's derivation), never in `custom_dictionary` — so they never muddle the user's custom dictionary, which keeps its contract (no provenance column, every row counted / listed / exported). The `.taigi` backup never carries learned phrases (a dev-only v3 file's `origin = 1` rows are skipped). No list UI: the only user surface is the learning-data wipe (Delete Learning Records, `ResetUserData`), which clears frequency, association and learned phrases, each attempted independently; the mobile Settings-tab Reset to Defaults resets settings only. No cross-store rule (accepted: a custom duplicate holds one of the 2 000 slots and one of the 5 per-fetch rows). A dev custom-dictionary file carrying inert `origin` / `learn_count` columns loses them when the engine takes it over (`custom_dictionary.rs::drop_learned_rows_if_present`).
- **Known limit**: under TPS input `custom_toneless_key` accepts Bopomofo bodies only, so a TL-keyed learned row is a span-local row but not a TPS walker edge.

**Scope**: engine only — learning (`composing`), the store and its whole-buffer query (`userdata`), the write and the read (`dispatch::user_data::handle_composing` / `with_stores::buffer_rows`); a pick of a learned candidate touches it (`LearnedPhraseStore::touch_phrase`), recorded by the engine when it resolves the commit. **No toggle** — learning is always on. Manual rows keep Enable Custom Dictionary as their own gate. `make build` before any mobile test.

**Tests**: `engine/composing/tests/continuous_learned_phrase.rs` (learning: join, khinsiann, typed `--` / `-` / nothing / `---` joiner, dictionary khinsiann piece over a typed `-`, typed `--` through fetch → pick → fetch → pick, three singles, single segment, hanji-less, empty TL, 6 vs 7 syllables, TL-derived count, multi-word space; recall: no-dict lead, khinsiann key, separator-only duplicates collapse, tone-different rows both kept, POJ-typed buffer, dictionary homophone beats learned until picked, dictionary duplicate listed once, custom outranks learned, space-TL key, nasal-`oo` alias). Offline repro: `DUMP_LEARNED="記起來:kì--khí-lâi" DUMP_BITMASK=8159 DUMP_INPUTS=kikhilai cargo test -p composing --test prod candidate_dump -- --ignored --nocapture`.

## §51 — A separator typed between two picks reaches the document as typed

### `INVARIANT_NAILED_JOIN_HONOURS_TYPED_RUN`

Type `tng--lai`, pick 轉, pick 來: the document gets `tńg--lâi`. The `-` run typed before a pick folds into that pick's consumed prefix (`NailedSegment.raw_text` = `--lai`, `transition::commit_continuous`), and the nailed-prefix join (`api::nailed_prefix_with_oracle`) renders it.

- **The run in front of a segment is the boundary.** When the join is roman-ish (`continuous_word_space`), a boundary in front of a segment whose `raw_text` opens with a `-` run renders that run verbatim — in place of the word space and of the compound joiner alike (`tng--lai` → `tńg--lâi`, `tng-lai` → `tńg-lâi`, `tng---lai` → `tńg---lâi`). Nothing typed → the default policy (compound oracle `-`, else space). Under the §49 Syllable Separator the run goes through `phonetics::api::push_syllable_joiner_run` — the joiner, then one `·` per `--` (None: `--` → `·`, `-` → nothing, `----` → `··`; Space: `--` → ` ·`, `-` → ` `).
- **Grouping is untouched.** The compound longest-match still decides which segments form one word; a typed run only replaces what is written at its own boundary (台|灣|人 with `--` before 灣 → `tâi--uân-lâng`).
- **Never doubled.** With a run typed, a segment whose rendering already opens with its separator — a dictionary khinsiann piece (`--khí-lâi`), a §34 literal (`--lai`), their `·` forms — gets nothing in front of it, not even the compound joiner (`tńg` + `--lâi` → `tńg--lâi`), except the §49 Space joiner in front of a `·` form (`hōo` + `·guá` → `hōo ·guá`). A hyphen-continuation segment before it (`tai-`) ends the boundary by itself.
- **The pending tail carries its own run.** No boundary space between the nailed prefix and a tail opening with `-` (`tńg--lai`); Enter commits it as is; backspace-unnail restores the run with the segment's `raw_text`, and a repick commits it again.
- **Hanji-first / TPS unchanged** — no roman separator is rendered there; `both-scripts` follows the roman rule.

**Tests**: `engine/composing/src/api.rs` (`typed_run_*`, `pending_tail_opening_with_a_typed_run_gets_no_boundary_space`); `engine/composing/tests/continuous_typed_separator.rs` (`two_picks_*`, `enter_after_the_first_pick_commits_the_typed_tail_as_is`, `unnail_restores_the_run_and_a_repick_keeps_it`).

## §52 — A typed hyphen is a syllable boundary

### `INVARIANT_TYPED_HYPHEN_IS_A_SYLLABLE_BOUNDARY`

A separator the user types is an explicit syllable break. Typing `jim--khi--ah` for 撏去矣 `jîm--khì--ah`, after 撏 is picked the pending `--khi--ah` must offer 去 `khì`, not 隙 `khiah` (one syllable across the typed `--`). Two layers enforce it: the TL lattice and the span's lookup pin.

- **No syllable crosses a typed `-`.** `syllabifier::tl::valid_span_endings_lowered` refuses any single-syllable link with a barrier strictly inside it (mirror of the TPS §35 rule); chains still meet AT the barrier, so `khi`+`ah` spans `khi--ah` while `khiah` does not. TL and POJ; English keeps its barrier-blind reading. An empty barrier slice is byte-identical to no barriers (`tania`, `iah8` unchanged).
- **A reading must end a syllable on every typed `-`.** `TonePin::TypedTones` carries `boundaries` — byte offsets into the typed span where a `-` was — and `reading_passes_typed_tones` rejects a reading whose syllable ends skip one: 隙 `khiah` and 屐 `khia̍h` drop under `khi--ah`, 去啊 `khì--ah` stays; `khi3--ah` needs both the tone and the boundary. Built by `shadow::span_tone_pin` for any TL / POJ span with a digit or a boundary; the whole-buffer pin (`fused_shadow_with_barriers`, through the same `separator_layers`) carries it too, so custom rows, learned rows and partial-prefix extensions answer to it. The letters match is ASCII-case-blind (`Khì--ah` passes, `Khiah` does not; a capitalised custom row under a typed tone passes). A barrier at a span's own start belongs to the span before it; a trailing `-` (`tai-`) is the span end — `tâi-uân` passes, `tâin` does not.
- **The kind of the typed run must match the reading's own separator.** `TypedBoundary { at, khinsiann }` — two or more `-` is the khinsiann `--`, one `-` is plain — and where the reading continues past the boundary its separator there must be the same kind (`phonetics::tl_syllable_khinsiann_flags`; a dictionary space counts as plain): `khi--ah` keeps 去啊 `khì--ah` and drops the compound 忍氣 `jím-khì` under `jim--khi` (the walker then builds the singles); `khi-ah` / `jim-khi` do the reverse; `ia-si` keeps 也是 `iā sī`, `ia--si` builds 也 + 是 `iā--sī`; 予我 `hōo--guá` answers `hoo--gua` only (`hoo-gua` reads 予 + 我 `hōo-guá`; `ki-khi-lai` reads 機起來 `ki-khí-lâi`, `ki--khi-lai` 記起來). A trailing run (`tai--`) constrains the next word, not the reading ending there. The run typed right before a span is a boundary at 0, read only by a reading that itself opens with `--` (a custom 矣 stored `--ah` answers `khi--ah`, not `khi-ah`); the walker join never stacks a typed run onto such a part (`khì--ah`, never `khì---ah`).
- **OOV readings split there too.** `greedy_longest_syllabification` / `span_min_syllable_count` take the span-local barriers (TL / POJ only: `oov_reading_barriers`), and a hanji-less walker edge that would fuse letters across a typed `-` is dropped (`khi-lai-lai` → `khì-lai-lai`, never `khì-lailai`).
- **A typed `-` closes the syllable before it** for the §18 guard (d): `iah-` suppresses the closed dead end `ia` the way `iah8` does; `iah` alone keeps it.
- **A closed stretch that is one syllable IS that syllable** (the `-` lets the user define the split: `siam-tioh` must offer 閃 `siám` above 寫 `siá`). A stretch runs from the buffer start or a typed `-` to the next typed `-`; when the inventory reads the whole stretch as one syllable (`syllabifier::one_syllable_segments`), nothing inside it is a cut. The TL scanner admits no link that starts or ends strictly inside it, and the stretches ride on the lookup pin (`TonePin::TypedTones::one_syllable_stretches`), which drops a reading with a syllable end inside one (`ai-` keeps 愛 `ài`, drops 阿姨 `a-î`; `iau-` drops 以後 `í-āu`; custom rows too). The user splits by typing the `-` (`a-i` → 阿姨). A digit typed after the `-` is no tone of the syllable it closed (`tai-2` keeps 台). A stretch that is no single syllable (`taigi-bun`) and the open tail after the last `-` constrain nothing; nothing typed (`siamtioh`) keeps `sia|m|tioh`. TL / POJ only.
- **Unchanged**: `khiah` with nothing typed keeps every reading; TPS (§35 / §41 — the whole-buffer pin reads the space barriers only, so `ㄒㄧ-` pins nothing); the §34 literal; English; the rendering rules (§51, §55).

**Tests**: `engine/composing/src/shadow.rs` (`typed_hyphen_barrier_*`, `typed_hyphen_closing_one_syllable_admits_no_cut_inside_it`, `oov_readings_split_*`, `span_key_pins_typed_boundaries_*`, `whole_buffer_pin_carries_typed_tones_for_tl_poj`), `engine/phonetics/src/syllable.rs` (`khinsiann_flag_tests`), `engine/lexicon/src/continuous/` (`typed_boundary_*`, `typed_boundary_kind_*`, `trailing_boundary_*`, `boundary_check_reads_case_blind`, `a_reading_may_not_split_a_one_syllable_stretch`), `engine/composing/tests/continuous_typed_separator.rs` (`hoo--gua` vs `hoo-gua`), `engine/composing/tests/continuous_typed_boundary.rs`, `engine/composing/tests/continuous_keys_tl_lattice.rs` (`iah-`), `engine/composing/tests/continuous_one_syllable_segment.rs`, `engine/composing/tests/one_syllable_segment_prod.rs`. Offline: `DUMP_INPUTS="khi--ah" cargo test -p composing --test prod candidate_dump -- --ignored --nocapture`.

## §53 — "Nasal mark in POJ capitals" decides the case of the POJ nasal marker, everywhere the user reads it

### `INVARIANT_NASAL_MARKER_CASE_FOLLOWS_THE_SWITCH`

The POJ nasal marker has an uppercase (`ᴺ`) and a lowercase (`ⁿ`) spelling. Setting key `nasalMarkerUppercaseEnabled` (iOS, macOS, desktop-core for Windows / Linux; Android DataStore `taigi__nasal_marker_uppercase`), default ON on all five platforms; sent to the engine inverted as `AppConfig.force_lowercase_nasal_marker` (proto3 default `false` = the marker follows the case).

- **The setting is a picker over the switch**: "Nasal mark in POJ capitals" with "ᴺ (SIÂᴺ)" = ON (the default) and "ⁿ (SIÂⁿ)" = OFF, on all five platforms — a pop-up on the desktop General pane, a selection sub-page on mobile (beside Candidate Display).
- **One rule, one function.** `phonetics::case_transform::apply_nasal_marker_case(text, force_lowercase)`: ON → `adjust_nasal_marker_case` (the marker follows the preceding letter's case: `SIÂᴺ`, `Siâⁿ`); OFF → `lowercase_nasal_markers` (every `ᴺ` U+1D3A written as `ⁿ` U+207F, letters untouched: `SIÂⁿ`). A rewrite, never a skip. Triggered by the marker, not the mode: a TL row or preedit carrying one follows the same rule.
- **Where it runs**: the preedit / §34 literal (`normalize_tone`, last step); the candidate `roman` (`assemble_candidates` Step 5, after the POJ render, before the rendered dedupe — `display_text` / `canonical_tl` untouched; both glyphs fold to `nn` in every identity key); every case-transform op (`dispatch::case`; the bridges pass the switch on every call), so mobile key labels, typed input and the suggestion transform agree. Next-word predictions need no seam (their POJ render never writes the capital marker). The nailed prefix concatenates text already rendered. So under Caps Lock the strip and the preedit both read `SIAᴺ`, a pick and Enter commit the same text, and under a single-script display the literal and the dictionary row collapse into one cell (`adopt_collapsed_dict_identity`).
- **Key label follows the switch** (mobile): the POJ `nn` key and Android's `ⁿ` long-press cell show `ᴺ` under shift / Caps Lock only while the switch is ON — iOS `ButtonTextProvider.characterText`; Android `KeyContent.computeKeyLetter` maps the `nn` key to `ⁿ` and lets `KeyLabelCaseCache` → the case ops apply the rule. The key still inserts ASCII `nn` / `NN`.
- **Unchanged**: lowercase typing (`siann5` → `siâⁿ`); the double-tap folds; TL / TPS / English output; every identity key. The §34 literal keeps the marker case in its learning text.

**Tests**: `engine/phonetics/tests/normalize_tone_nasal_case.rs` (`normalize_tone_lowercase_nasal_marker_*`), `engine/composing/tests/nasal_marker_case.rs` (Caps Lock default `SIAᴺ` literal + candidate, forced `SIAⁿ`, compound `SIAⁿ-IM`, lowercase control, TL custom row, Romanization Only collapse), `engine/phonetics/src/case_transform.rs` (`apply_nasal_marker_case_*`), `engine/dispatch/src/case.rs` (`dispatch_lowercase_nasal_marker_*`), `desktop/crates/taigi-desktop-core/src/engine/bridge.rs` + `settings/document.rs` (`nasal_marker_uppercase_*`), macOS `SettingsStoreTests.swift`.

## §54 — Every composing request renders under the settings it carries

### `INVARIANT_EVERY_COMPOSING_OP_CARRIES_THE_RENDERING_CONFIG`

The engine re-renders the whole preedit from the request's config on every op (`api::combined_display(nailed, raw, config)` via `transition::step_response`), so an op sent with a flag-free base config renders the nailed prefix wrong (after nailing 珠 under Hanji-first the next keystroke would read `珠 仔 guat`, and a separator setting would revert until the next commit).

- **The engine holds no settings of its own; every request carries the snapshot it should be rendered under** (`engine/dispatch/src/lib.rs` `request.config_snapshot`). No op is exempt: a nail and the keystroke after it must agree on the prefix. Only `Reset`, which carries no config, stays outside.
- **One builder, no defaults, every platform.** Every composing bridge op takes the live `EngineSettings` and builds its config through the platform's one `AppConfig` builder — iOS `continuousAppConfig(settings)` (`RustEngineBridge+Composing.swift`), Android `continuousAppConfig(settings)` (`engine/EngineAppConfig.kt`), macOS, Windows and Linux `taigi-desktop-core` `engine/bridge.rs::app_config`. The swap goes out as the Candidate-Display-projected setting; the TPS layout goes out as `input_mode = "tps"` and the engine renders it Hanji-first itself (`AppConfig::renders_hanji_first`). No per-op default parameter exists for a call site to forget.
- **Android fetch keeps its snapshot**: `composingFetchAtPos(config:)` takes the `AppConfig` that `ComposingManager.fetchContinuousCandidates` builds once from one `settingsProvider.current` read (the same snapshot supplying the source toggles and the literal / custom-dictionary flags), because `EngineSettingsProvider.current` is a live view (§11). The fetch is one engine call (the engine reads the user rows and re-ranks in the same `FetchAtPos`).
- **Unchanged**: what commits (the engine resolves a candidate commit's text from the same config); candidate order; every identity key.

**Tests**: `ios/TaigiKeyboardTests/RustEngineBridgeContinuousTests.swift` (`testAppendAfterNail_HanjiFirst_KeepsPrefixUnspaced`: 台 nailed under `isHanjiFirst`, `Append` → `台bakk`, `DeleteBackward` → `台bak`; `testAppendAfterNail_TpsLayoutSwapStoredOff_KeepsPrefixUnspaced`); the wire config per field: iOS `RustEngineBridgeAppConfigTests`, Android `EngineAppConfigTest` (JVM builds the proto only; the engine round-trip is dogfood **S71**).

## §55 — The separator at every typed boundary is the one the user typed

### `INVARIANT_TYPED_SEPARATOR_RENDERS_AT_EVERY_BOUNDARY`

`pang-tang-lai` renders 放重利 as `pàng-tāng-lāi` (the MOE dictionary writes `pàng-tāng-lāi--ê`), though the record stores the phrase with a space (`dictionary/output/dictionary.csv` 放重利 `pàng tāng-lāi`): the user's typed separator takes precedence inside a dictionary word, not only between walker segments.

- **The run typed at a boundary is what is rendered there — inside a dictionary word as between walker segments.** `phonetics::api::render_typed_separators`, the Step 5 presentation pass of `composing::continuous::assemble_candidates` (the seam of §49 and §53; the typed runs arrive as `ContinuousKeys.hyphen_runs`, the `(offset, run length)` carrier the §52 kind pin reads): for every emitted row (dictionary, custom, learned, partial-prefix, walker slot-0), at every typed boundary that ends a syllable of the reading other than its last, the separator run in `roman` is replaced wholesale by the typed run, verbatim (`pang-tang-lai` → `pàng-tāng-lāi`, `pang--tang-lai` → 放 + 重利 `pàng--tāng-lāi`, `goa--si` → `gô-á--sī`). Nothing typed → the record's / walker's form (`pangtanglai` → `pàng tāng-lāi`). Wholesale, so a part opening with its own `--` reads `khì--ah`, never `khì---ah`.
- **Which words are offered is §52's, not this pass's.** The kind pin decides admission; this pass only writes the separator on what was admitted. Sub-span rows, scores, order and the §34 literal are untouched.
- **Presentation only.** `display_text` / `canonical_tl` keep the record's or the space-joined form, so the `(hanji, canonical_tl)` identity and the `user_frequency.db` key never fork on a typed separator. The walker's wire roman is its space join, rendered by this one pass.
- **Alignment is the §52 walk.** The reading's syllable ends in the typed body come from the same typed↔numeric-face walk the pin uses (`phonetics::typed_syllable_walk`: alias `onn`/`oonn`, typed digits, ASCII-case-blind), run on `canonical_tl`; tokens come from `roman` (which may be recased — `PANG-TANG-LAI` → `PÀNG-TĀNG-LĀI`). A reading that does not align (an abbreviation body, a TPS / English buffer) or whose token count differs is left as is.
- **Downstream passes see the typed form**: the POJ render keeps every `-` / space; the §49 Syllable Separator rewrites the typed `-` / `--` like any other hyphen (None: `pàngtānglāi`, `pàng·tānglāi`; Space: `pàng tāng lāi`, `pàng ·tāng lāi`); the rendered dedupe runs whenever this pass changed a row.
- **Unchanged**: the nailed-prefix join between picks (§51); learned phrases (§50, `api::learned_reading`); TPS; English.

**Tests**: `engine/phonetics/src/api.rs` (`render_typed_separators_tests`), `engine/composing/tests/continuous_typed_separator.rs` (`typed_hyphen_replaces_the_records_space`, `typed_separator_kind_selects_the_dictionary_word`). Offline: `DUMP_INPUTS="pang-tang-lai,pangtanglai,pang--tang-lai" cargo test -p composing --test prod candidate_dump -- --ignored --nocapture`.

## §56 — The previous word re-ranks the continuous candidates, never the segmentation

### `INVARIANT_CONTINUOUS_CONTEXT_RERANK`

A continuous fetch ranks by the word the pending tail follows: the **last nailed segment** of the composition (`composing::Engine::pending_snapshot(..).context`, canonical text + association roman — the identity the final commit's `preceding` carries), else the **last committed word inside its 10 s association window** (`nextword::EngineHandle::context_snapshot`, the same `decide::committed_context` a commit learns a pair by; `ResetAll` / `ContextTimeoutFired` clears it). `dispatch::context` turns that word into `ranking::ContextRanks` — its user-learned continuations (`user_association.db`, §24 recall, `CONTEXT_RANK_USER`) over its bundled ones (`association.bin`, the §24 word-key → character-key lookup, `CONTEXT_RANK_BUNDLED`), 30 rows each — and hands the map to the fetch (`Intent::FetchAtPos.context`, `ContinuousFetchCtx.context`). The lexicon stamps every candidate's `context_rank` by the `(display_text, canonical_tl)` pair (legacy `(display, "")` bucket as fallback, like `FrequencyMap`).

- **Where it ranks.** `CandidateSortKey` = `(tier, abbreviation, preference_class, coverage_kind, −user_weight, context_rank, walker_cost, −score, −freq, −coverage, source_rank, stable_idx)` (`continuous-input-ranking.md`). Below `tier` / `abbreviation` / `preference_class` / `coverage_kind`: a context hit never lifts a front-segment row over a whole-input one, an abbreviation over a reading, a never-selected word over a selected one or a never-selected dictionary word over a never-selected custom one, or a partial prefix over a full syllable of its tier and class. Below `user_weight`: a heavier-selected word still beats a lighter-selected one. Above `walker_cost` and `score`: a context hit outranks the corpus choice, in the list and for the edge's word. The span-local list, the partial-prefix block (its cap truncates after the sort) and the walker's per-edge word pick (`lexicon::pick_edge_word`) all use it.
- **Never the segmentation.** The walker prices an edge on `EdgeBest::span_walker_cost` (the key's minimum corpus cost, whatever the pick), the pick's `syllable_count` and its `user_weight` (`lattice::edge_cost`). `pick_edge_word` finds the context-free winner first and chooses the contextual word only among homophones with its `syllable_count`; the chosen word keeps the context-free winner's `user_weight`. The walker bigram (`bigram-lm-roadmap.md` D6) prices the context-free winner too (`EdgeBest::bigram_rowid`, `pick_edge_word`'s baseline). Every cost input is unchanged → the segmentation is the context-free one; only the word shown for an edge can differ. Custom edges and the §34 literal are untouched.
- **No context = the context-free order.** An empty map ranks every candidate `CONTEXT_RANK_NONE`, and the walker skips the two-pass pick; the golden fixtures run with an empty map and must not move.
- **Same tables as the strip.** The same `lookup_associations` (word key, character-key backoff — every source on, top 30, where the strip filters by the source toggles), the same user rows, user over bundled. The character-key backoff therefore also favours intra-word continuations (真 → 正 from 真正) — accepted for consistency with §24.

**Tests**: `ranking/src/sort_key.rs` — `context_hit_beats_higher_score`, `context_hit_never_crosses_tier_coverage_kind_or_user_weight`, `no_context_is_the_context_free_order`, `walker_cost_beats_score_below_user_weight_and_context`; `lexicon/src/continuous/mod.rs` `edge_word_pick::*`; `ranking/src/context.rs`; `nextword/src/handle.rs::context_snapshot_follows_the_association_window`; `composing/tests/continuous_context_rerank.rs` (after 真, `tse` leads with 濟 instead of the context-free 今, same word set; unrelated context = baseline; S5 `taiuan` keeps its slot 0; `pending_context_is_the_last_nailed_segment`); `dispatch/tests/composing_context.rs` (through `process_request`: committed 真 → 姊 (user) then 濟 (bundled); outside the window / after `ResetAll` = baseline; nailed 真 inside the composition).

## §57 — Every dictionary switched off offers no dictionary candidates

### `INVARIANT_DICTIONARIES_ALL_OFF_OFFERS_NO_DICTIONARY_CANDIDATES`

With every dictionary toggle off, the keyboard offers no dictionary candidates, on every platform. (A `0` mask must never reach the fetch as the proto3 "not wired" sentinel, which means all sources on.)

- **The engine resolves the toggles itself.** Every platform sends the user's `DictionarySourceToggles` in `FetchAtPos.toggles` (the message Tab3 browse and `PredictNext` carry) and `composing::requests::fetch_at_pos_intent` resolves them with the lexicon encoder (`lexicon::api::dictionary_filter_bitmask`, the one behind `DictionaryFilters`). Every dictionary off resolves to `0`, and `Filter::from_enabled_bitmask(0)` yields `enabled_mask = 0`, `all_enabled = false`, so `DictionaryReader::passes_filter` rejects every `dictionary.bin` record on the span-local, partial-prefix, abbreviation and walker-edge paths alike. No platform computes or forwards a bitmask for the keyboard.
- **Absent toggles keep every source on.** A `FetchAtPos` without `toggles` (fixtures, an un-wired sender) resolves to `u32::MAX`, the same un-wired default as the negative fields 6 and 8. Every sender in this repository sends them.
- **Only dictionary records are gated.** The §34 literal (gated only by `literal_roman_candidate_disabled`), custom-dictionary rows (`merge_custom_dedupe_sort` and the walker's edge pick, gated only by `FetchAtPos.custom_dictionary_disabled`), learned phrases (§50) and the OOV / roman-only synthesised rows never pass through the source filter, so with every dictionary off they still appear.
- **Search reads the same mask.** Dictionary search (`SearchWithSources` / `SearchByHanji`) builds its filter from the `DictionaryFilters` mask verbatim, so `0` means "nothing enabled" there too; a failed resolve sends `u32::MAX`.

**Tests**: engine `composing/tests/golden_fetch_at_pos.rs::fetch_at_pos_resolves_the_dictionary_toggles_it_carries` (no toggles → every source, all off → no hanji with the literal still leading, `lkk` alone → the `lkk`-tagged rows); iOS `RustEngineBridgeContinuousTests.testFetchAtPos_EveryDictionaryOff_OffersNoDictionaryCandidates`; desktop `taigi-desktop-core/tests/engine_roundtrip.rs::all_sources_off_fetches_no_dictionary_candidates`; Android `DictionaryTogglesProtoTest.kt` (the toggles `composingFetchAtPos` sends — JVM tests cannot load the engine).

## §58 — A desktop user-data list sends its filter as typed, reloads after every write, and selects only what is on screen

### `INVARIANT_USER_DATA_LIST_FILTER_RELOAD_SELECTION`

The paged Custom Dictionary and Learning Records lists behave the same on macOS (`UserDataPagedList`, `CustomDictionaryPageModel`, `LearningRecordsPageModel`) and on Windows / Linux (desktop-core `settings/listing.rs` `Listing`, `settings/custom_dictionary.rs`).

- **The filter goes as typed; the engine trims it.** The list request carries the box's text unchanged; `engine/userdata` trims it (`custom_dictionary.rs` `rows` / `count_matching`, `learning_records.rs` `list`), so `" tsit"` and `"tsit"` list the same rows; a box of only spaces lists everything. The empty-list message is read off the box as typed, so a box of spaces over an empty store reads "no results". desktop-core does not trim a second time.
- **Every write reloads, whatever it answered.** Save, delete, delete-all and CSV import reload the list after a failure as after a success: the engine commits a large import 500 rows at a time (`IMPORT_CHUNK_SIZE`), so one that fails partway has still added rows. The failure's alert shows first; a reload that fails too replaces it with the read failure (Windows, macOS — one alert at a time; Linux toasts both). Every editor closes before its write returns, and a reload touches only the list — rows, counts, page, selection — never an editor or the filter box.
- **The selection is a row on screen.** A load that lands without the selected row drops the selection, so `−` (and ✎ on Windows / Linux) never act on a row chosen before. A stale or failed load leaves the selection alone. A new kind or order on Learning Records drops it (`Listing::rewind`); a new filter does not by itself.

**Tests**: engine `userdata/src/requests.rs::the_custom_dictionary_page_adds_edits_filters_and_deletes` (`" 臺 "` = `"臺"`); desktop-core `settings/custom_dictionary.rs` (`a_load_sends_the_filter_as_typed_and_asks_for_the_page_on_screen`, `a_blank_filter_is_kept_as_typed_and_reads_as_a_filter`, `a_write_reports_only_its_failure_and_reloads_either_way`, `an_adopted_page_is_the_engines_and_drops_an_off_page_selection`, `a_failed_load_changes_nothing_on_screen`); macOS `UserDataPageChromeTests.swift` `CustomDictionaryListParityTests`, `LearningRecordsPageTests.swift` (`UserDataPagedListTests`, `testAChangeOfKindOrOrder_dropsTheSelection`). iOS / Android are not covered: their pages filter in memory, untrimmed, with no paging or held selection.

## §59 — Under TPS the desktop preedit shows the Hanji conversion, and only a pick teaches

### `INVARIANT_TPS_PREEDIT_HANJI_CONVERSION`

Under TPS on macOS, Windows and Linux the composition is converted to Hanji as it is typed, the way the Zhuyin input methods convert (design and as-built notes: `desktop-tps-hanji-conversion-roadmap.md` H1–H8). iOS, Android, TL and POJ send no conversion switch and keep their behaviour byte for byte.

- **The switch.** The desktop core's one config builder (`engine/bridge.rs::app_config`) sends `AppConfig.hanji_conversion`, with the dictionary switches, on every request under TPS and on no other. The engine takes none of the paths below without it.
- **What the preedit shows (H1, H2).** Nailed segments, then the pending tail in raw order: the walker's words over closed readings, and the glyphs of the text no word covers — what is still open, and any gap an edit inside the tail left. A reading closes on its tone mark, or on Space for tones 1 and 4; the walk runs inside the request that closes, removes or re-opens a reading, and may re-segment the words before it. Typing an open reading at the end of a tail whose words cover it from its start re-uses those words without a walk; with glyphs left between words the closed part is walked again (`conversion.rs` H-P1 / H-P2 rules). A key answers with one already converted `UpdatePreedit`. A word without Hanji, an unclosable tail and every source switched off show glyphs. The conversion is engine state (`Phase::Continuous.conversion`), shown only to the source filter it was walked for. One underline, one caret.
- **The caret (H3).** ← → and the caret chord step one word over the conversion and one glyph inside open text. ← from the start of the tail re-opens the last nailed segment (its glyphs back in front of the tail, walked afresh). The caret chord on ↑ / ↓ (⌥↑ / ⌥↓, Ctrl+↑ / Ctrl+↓) and bare Home / End (also under TL / POJ) jump to the start / end of the tail and never re-open (`MoveCaret` `START` / `END`). Backspace takes one glyph.
- **Choosing (H4).** ↓, the navigation and paging rows, or Space after a closed reading with the caret at the end (inside the composition that Space does nothing) open the list of the word before the caret (at the start of the tail, the first word): that word's homophones, longer words first. A pick nails everything before the list's start as shown, nails the pick, walks what follows again, closes the list, puts the caret at the end and never writes to the document — the last word included.
- **Commits (H5).** Every commit of a TPS composition that is not Commit as Typed — Enter, a key that commits first, with the window off a navigation key or Space after a closed reading at the end, a mode switch, a host's Finalize, focus loss on the shells that commit on it — writes exactly the preedit shown. Commit as Typed (Shift+Enter) writes the glyphs of the whole composition, separators removed. Both go out under the settings the preedit was last written with, as do a TPS fetch and pick.
- **Learning (H7).** Only a pick records usage, under (Hanji, canonical TL) (Core Principle #6); the walker ranks with those counts. A word accepted by a commit records nothing. A commit teaches the §50 phrase and the next-word sequence only when every segment was picked; otherwise it answers `NextWordClearForNewComposing` without `NextWordWordSelected` and the desktop forgets the committed context.
- **The macOS window (H8).** The list is anchored at the caret: the window sits under the character before it, at the end of the marked text when the caret is there — usually the last character of the listed word. Two accepted exceptions: at the start of the tail after a pick, the list is the word after the caret and the window sits under the nailed text's last character; with the caret after a run of glyphs no word covers, the list is the run's first reading and the window sits under the run's last glyph (`conversion::word_list_start`). TL and POJ lists anchor at the end. Windows and Linux keep their placement (Windows: the composition's end, `taigi-windows-tsf/src/ui/caret.rs`).

**Tests**: engine `composing/tests/tps_hanji_conversion.rs` (typing forward, the switch, the source filter), `tps_hanji_conversion_caret.rs` (H3, `the_caret_counts_a_non_bmp_word_in_utf16_units`, `start_and_end_jump_over_the_words_and_never_reopen`), `tps_hanji_conversion_pick.rs` (H4, the commits, learning, `random_sessions_keep_the_conversion_invariants`), `tps_hanji_conversion_prod.rs` (production lexicon); desktop-core `tests/composing_manager.rs` (`the_display_caret_follows_the_engine_and_resets_with_the_composition`, `a_tps_commit_writes_the_shown_hanji_after_the_dictionaries_change`, `a_tps_pick_after_the_dictionaries_change_keeps_the_words_before_it`, `a_tps_commit_forgets_the_context_unless_every_word_was_picked`); macOS seam `taigi-macos-ffi/src/session.rs::under_tps_the_list_anchors_at_the_caret`, Swift `TaigiInputControllerCandidateTests.testTpsList_anchorsUnderTheWordBeforeTheCaret`. Dogfood S94, S95.

## §60 — The desktop input indicator shows the script and what a commit writes

### `INVARIANT_DESKTOP_MODE_INDICATOR`

Each desktop shows the input state as script × output, each platform by its own convention (Mozc shows あ / _A); macOS keeps one static menu-bar icon.

- **The state.** desktop-core `mode_indicator.rs` `ModeIndicator::of(engine_settings, language_mode)`: English (Windows' Shift tap) overrides everything; TPS is one state (the swap and the display mode are inert there); otherwise the romanization × the EFFECTIVE swap (§42) — Hanji with Romanization shows the Hanji state, Romanization Only the romanization state, side by side follows the stored swap. A Hanji with Romanization swap (punctuation width only) leaves the indicator as it is. Showing the indicator changes no setting, composition or selection.
- **The symbols.** 台 / Ts (Tâi-lô, Hanji / romanization), 白 / Ch (Pe̍h-ōe-jī), 方 (TPS), 英 (English): a hanji when a commit writes Hanji, the romanization's own spelling of /ts/ when it writes romanization. One or two characters (GNOME Shell's limit). The icons are those glyphs on the app icon's tile, drawn by `tools/desktop/make-app-icon.swift` and committed.
- **Per platform.** Windows: the taskbar button's `GetIcon` loads the mode's DLL icon (`mode_icons.rs`, ids 2–7; mozc `tip_lang_bar_menu.cc:98-111`) and `GetText` its symbol; a switch made here pushes `OnUpdate`, and so does `OnSetThreadFocus`, for a change the settings app or another host made. Fcitx5: `subModeIconImpl` returns the hicolor icon name (`taigikeyboard-tl-hanji` …), as fcitx5-mozc does, and `subModeLabelImpl` the symbol (drawn only under Prefer Text Icon). IBus: the `InputMode` menu root carries the symbol and the icon. Every shortcut that changes the state redraws it — the script switches, the display cycle and the swap. macOS: one static menu-bar icon (`ComponentInputModeDict` not adopted); the swap flashes the output script (`outputScriptHanji` / `outputScriptRoman`) when it changes what a commit writes and no candidate list is on screen.

**Tests**: desktop-core `mode_indicator.rs` (`invariant_desktop_mode_indicator_every_setting_combination_maps_to_its_indicator`, `english_overrides_every_setting`, `icon_assets_exist_for_every_indicator`); Linux `chrome.rs::the_swap_redraws_the_indicator_and_is_inert_under_roman_only`, IBus `wire.rs::property_and_prop_list_have_the_ibus_signatures`; Windows `lang_bar.rs::every_indicator_has_its_own_icon_resource_after_the_app_icon`; macOS `TaigiInputControllerShortcutTests.testTheTranslateSwap_withNoList_announcesWhatACommitNowWrites`, `TaigiInputControllerCandidateTests.testTogglingTranslateSwapped_rerendersTheBarInPlace`. Dogfood S99.

## §61 — On macOS an activation makes a session eligible; a delivered key makes it the owner

### `INVARIANT_MACOS_INPUT_SESSION_OWNERSHIP`

IMK can activate a second controller (a system client) while it keeps delivering keys to the first, so `activateServer` is not exclusive focus. One process-wide engine has one owner (`ComposingSessionCoordinator`); the rules below decide who it is. macOS only: Windows TSF and Linux Fcitx5 / IBus take focus from their own exclusive focus events.

- **Activation registers eligibility.** `activateServer` registers the controller (`registerActiveSession`) and takes the engine only when no live owner exists or it already owns it. Otherwise it waits: the owner keeps its composition, candidate bar, symbol picker and TPS key panel. A waiting controller still receives keyboard-layout overrides, so macOS translates its first key correctly.
- **A delivered key selects the owner.** `handle(_:client:)` on an active, non-owning controller with a client takes ownership first (`takeInputOwnership`: engine `Activate`, shortcut target, panels handed over), then processes the key. The handover drops the previous owner's composition in the engine; the region is cleared on that controller's own teardown.
- **A retired controller cannot reclaim.** Teardown (`deactivateServer`, `inputControllerWillClose`) revokes eligibility before anything else; a late key to it passes through unhandled.
- **A sole waiting session resumes.** After the owner's teardown releases the engine, `resumeWaitingSession` hands it to the one remaining active controller, so panels and hotkeys work before the first key. With several waiting, none is chosen until a key arrives — the global hotkeys act on nothing until then. A deallocated controller (weakly held) never counts as a live owner.
- **Reentrant teardown yields.** A commit during teardown can reactivate the same controller; the teardown then stops (`tenure` changed) and neither releases the engine nor drops the new tenure's observers or marked-region bookkeeping. Activation still makes no synchronous document query (Chromium deadlock; cf. `references/azooKey-Desktop/azooKeyMac/InputController/azooKeyMacInputController.swift:173`).

**Tests**: macOS `TaigiInputControllerOwnershipTests` (extra activation keeps the typing session, key handover, no late reclaim, sole vs several waiting shortcut targets, auxiliary close), `ComposingSessionCoordinatorTests` (`testResume_handsTheEngineToASoleWaitingSessionOnly`, `testResume_leavesALiveOwnerAlone`, `testADeallocatedOwner_isNoLiveOwner`, `testReregisteringAToken_replacesItsEarlierTarget`), `TaigiInputControllerTests` (`testReactivationDuringDeactivationCommit_keepsAcceptingInput`, `testTypingDuringReactivation_preservesTheNewMarkedRegionUntilHandover`), `TaigiInputControllerKeyboardLayoutTests.testAWaitingSession_receivesLayoutChangesBeforeItsFirstKey`. Dogfood S128.

---

## Change protocol

- Adding a new invariant: add it after the last numbered section + to the index; land a test naming its `INVARIANT_*` label in the same PR (`tools/invariant_labels.py` gates it).
- Removing an invariant: requires a written rationale in the commit message and a Codex review pass.
- Modifying a constant (decay, weights, thresholds): the Rust engine crate is the single source; a platform-side mirror that still exists moves in the same commit as the engine + this doc; reject cross-platform drift at review time.

### Parity decisions left open by the 2026-09 refactor round

Known divergences left untouched because unifying them changes an observable property — each needs its own round, dogfood, and (where a public op changes) an invariant entry here:

| Divergence | Why it is not a refactor |
|---|---|
| `engine/phonetics/src/tps.rs` `is_tps_tone_mark` (8 fixed scalars incl. U+0307) vs `tps_adjust.rs` `TONE_MARK_CHARS` (table-derived, excludes U+0300–036F) | different sets; the `tps_adjust` version backs the public `Method::IsTpsToneMark` op — unification changes a public op |
| `KeyFamily::Poj.toneless_face` (`phonetics/src/key_family.rs`) → `poj_num_syllable_ends_from_tl` | replace only with a parity proof over every dictionary row (byte-equality test) |
| Byte-identical `*.pb.swift` macOS ↔ iOS | build-graph change in `.xcodeproj` / `Package.swift` — maintainer-only config (AGENTS.md Core Principle #1) |
| Settings-key table ×2 (macOS `SettingsStore.swift` ↔ desktop-core `keys.rs`) | needs a generator = new tooling |
| Engine `WalkerSlot0` ↔ `RawCandidate` mirror, `bounded` fetch trio, `prefix_index.rs` four lookups, `SyllableReach` / `KeyFace` | each source doc explains a deliberate divergence (sign bridge D3, cap ordering, `INVARIANT_LEX_LOOKUP_ROWIDS_ORDER`) |

---

## Cross-references

- Live Rust / native ownership inventory: `docs/engine/migration-inventory.csv`.
- Per-platform criteria + exclusions: `docs/contributing/ios-architecture.md` §5, `docs/contributing/android-guidelines.md` §1.
- Data-artifact portability (`dictionary.fst` / `.bin` / SQLite user data): `docs/architecture/data-artifacts-portability.md`.
