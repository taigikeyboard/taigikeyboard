# A0 — Android Invariants Coverage Matrix

**Status**: authored 2026-04-19 as the Phase II Round 1 / A0 deliverable (mirror of the iOS G9 coverage matrix landed same day in `g9-coverage-matrix.md`). This audit-only artifact maps the 12 Phase 0 invariants in `behavioral-invariants.md` onto current Android code and tests, so A1 / A5-impl / A9 rounds inherit a concrete gap list rather than rediscovering one per round.

**Gate rule (joint Claude + Codex decision, 2026-04-19)**: A0 closes on documentation alone. Adding / renaming `INVARIANT_*` test functions in Kotlin is **deferred to A9**, because A1 converts stateful `object` services to constructor-DI classes and A5-impl extracts `NextWordEngine` out of `NextWordService` — writing `INVARIANT_*` tests now would rewrite them twice. A9 bundles the labeling with the ≥70% coverage drive.

**Policy**: this doc is invariant-first (12 sections as rows), not candidate-first like iOS G9. Android folds iOS's per-file split differently (`TaigiPhonetics.kt` absorbs what iOS spreads across `SyllableParser` / `TLFormatter` / `POJFormatter` / `PhoneticsConverter` / `RomanizationConverter`), so mapping 1-to-1 onto an iOS 10-candidate roster would distort current Android reality.

**Phase 0 is frozen for A0 / A9** (same rule iOS G9 carried). Findings that emerge are recorded under *Post-A0 follow-ups* at the bottom.

**Non-goals**:
- No `.kt` test file changes in this PR. Every "ADD" row below is an A9 work item.
- No new invariants. Everything outside `behavioral-invariants.md` §1–12 is out of scope.
- No iOS-side changes. This is the Android mirror only.

---

## 1. Android surface × invariant × test status

Status codes (match iOS G9 vocabulary):

- **OK** — grep-visible `test_INVARIANT_*` function already exists on Android.
- **WRAPPER** — behavior is substantively asserted under a descriptive name in an existing Android test file; A9 adds a thin `test_INVARIANT_*` wrapper that calls through.
- **ADD** — no substantive Android assertion exists; A9 writes a new test body.
- **N/A (pre-req)** — the Android surface the invariant names does not exist yet; another A-round (A1 / A2 / A5-impl) creates it before A9 can test it.

| # | Invariant | Android owner surface | Current Android test file (LOC) | Status | A9 action |
|---|---|---|---|---|---|
| 1 | TL ↔ POJ round-trip | `ime/dictionary/TaigiPhonetics.kt` (absorbs iOS parser/formatter/converter) | `TaigiPhoneticsTest.kt` (530) | WRAPPER ×4 | Add 4 thin wrappers (`test_INVARIANT_tl_to_poj_roundtrip_is_lossless`, `..._poj_to_tl_...`, `..._oo_combining_form_roundtrips`, `..._nasal_marker_variants_collapse_on_parse`) driving Phase 0 fixture batch |
| 2 | Unicode NFD preprocessing | `ime/dictionary/TaigiUnicode.kt` (has `CROSS-PLATFORM INVARIANT` header) | **missing** — no `TaigiUnicodeTest.kt` | ADD ×3 | New test file. Parity target includes `INVARIANT_nfd_preprocessed_platform_parity` (compare byte-for-byte against the iOS fixture set), `..._poj_nasal_to_nn_substitution`, `..._o_dot_combining_collapse`. Gate note: parity test needs shared fixtures exported from iOS run — track as FU-A2. |
| 3 | TPS ↔ TL round-trip | `ime/dictionary/TPSConverter.kt` (merges iOS TPSTables / TPSInputAdjuster / TPSToTL / TLToTPS) | `TPSConverterTest.kt` (656) | WRAPPER ×1 + ADD ×2 | Wrap existing roundtrip via `test_INVARIANT_tps_to_tl_roundtrip`; add bidirectional `..._tl_to_tps_roundtrip` and adjuster-idempotency `..._tps_adjuster_is_idempotent` (mirror of iOS FU-5 / G9 S3b) |
| 4 | Input normalization, mode-agnostic | `ime/dictionary/InputNormalizer.kt` | `InputNormalizerTest.kt` (746) | WRAPPER ×1 + ADD ×2 | Wrapper for `..._input_normalizer_is_mode_agnostic` across POJ / TL / TPS / English; `..._normalizer_preserves_hyphens_in_roman_num` and `..._normalizer_strips_hyphens_in_notone` pending iOS FU-2 resolution (`InputNormalizer.normalize` public API may not expose the dual-key split on either platform — A9 may reroute these labels to `CustomDictionaryDerivation.searchPrefix` once iOS side lands). |
| 5 | Candidate dedup — engine vs display | `ime/dictionary/CandidateProcessor.kt` | **missing** — no `CandidateProcessorTest.kt` | ADD ×3 | New test file. A9 high-priority gap. Tests: `..._engine_dedup_keys_on_roman_plus_hanzi`, `..._display_dedup_runs_after_sort`, `..._display_dedup_keeps_words_without_hanzi`. `removeDuplicates` and `removeDisplayDuplicates` are already public top-level functions and are testable today without waiting for A1 — A9 may opt to land these three ahead of the §6 set if sequencing helps. |
| 6 | Candidate scoring — determinism + ordering | `ime/dictionary/CandidateProcessor.kt` | **missing** (same file) | ADD ×5 | Same new test file. A9 high-priority gap. Tests: `..._score_is_deterministic`, `..._user_freq_dominates_ranking`, `..._completion_penalty_separates_tiers`, `..._recency_window_is_exactly_1_hour` (boundary `<` vs `<=`), `..._roman_to_base_strips_tones_hyphens_digits`. Split into two tiers: (a) `calculateScore` is pure and already takes `frequencyData: UserFrequencyService.FrequencyData` + `currentTime: Long` as parameters — testable today, all 5 determinism / ordering labels can be written against `calculateScore` directly. (b) `sortByScore` currently reaches `UserFrequencyService.frequencyDataBatch(...)` inline (line 112); audit §2.3 / §8 decision #7 moves that call out in A1 so `sortByScore` takes `frequencyData` from the caller — A9 lands the `sortByScore` coverage after that change. `FrequencyData` hoist (audit §8 decision #3) affects surface shape but not the testability of `calculateScore`. |
| 7 | Next-word decay — RIME half-life | Scoring math inlined inside `ime/dictionary/NextWordService.kt` (`object`). Lines 57–73 already carry a `CROSS-PLATFORM INVARIANT` comment pinning constants. | **missing** — no `NextWordServiceTest.kt` / `NextWordScorerTest.kt` | ADD ×4, N/A (pre-req) | Blocked until A5-impl extracts `NextWordScorer` (stateless) out of `NextWordService` per audit §A5-impl. Then A9 writes `..._decay_half_life_is_168_hours`, `..._high_usage_decay_floor_95`, `..._low_usage_decay_floor_30`, `..._scorer_constants_match_android` (pin test asserting `USER_WEIGHT==50 && DICT_WEIGHT==1 && LEARNING_BONUS==300 && DECAY_HALF_LIFE_HOURS==168.0 && HIGH_USAGE_DECAY_FLOOR==0.95 && LOW_USAGE_DECAY_FLOOR==0.3 && HIGH_USAGE_THRESHOLD==3`). |
| 8 | Next-word weighting — user > dict | Same `NextWordService.kt` scoring block | **missing** (same) | ADD ×4, N/A (pre-req) | Same A5-impl gating. Tests: `..._user_weight_is_50`, `..._dict_weight_is_1`, `..._learning_bonus_is_300`, `..._user_entry_outranks_dict_entry`. |
| 9 | Case transformation | `ime/dictionary/SuggestionCaseTransformer.kt` | `SuggestionCaseTransformerTest.kt` (177) | WRAPPER ×2 | Add `test_INVARIANT_case_transformer_is_deterministic` + `..._case_transformer_honors_auto_cap_flag`. `.tps` no-op corner case already exercised. |
| 10 | Custom-dictionary search-key derivation | Derivation logic currently embedded in `ime/dictionary/CustomDictionaryService.kt` (object) — no dedicated `CustomDictionaryDerivation.kt` | `CustomDictionaryServiceTest.kt` (60) | WRAPPER ×1 + ADD ×1, N/A (pre-req partial) | A1 converts `CustomDictionaryService` from `object` to constructor-DI class and should extract `CustomDictionaryDerivation` as a pure Kotlin object/class (mirror of iOS `Lexicon/Database/CustomDictionaryDerivation.swift`). A9 then wraps `..._abbrev_key_is_one_char_per_syllable` from existing abbrev tests and adds `..._custom_derivation_matches_input_normalizer` (the parity claim that `generateRomanNum` output equals `InputNormalizer.normalize(roman, InputMode.TL)`). Tracks iOS FU-3 on `searchPrefix` tone-aware routing separately as FU-A3. |
| 11 | Settings read semantics — live vs snapshot | **missing** — no `EngineSettings` interface; engine code currently takes `PrefHelper` directly | N/A | ADD ×1, N/A (pre-req) | Blocked on A2 (introduces `EngineSettings` + `EngineSettingsProvider`). A9 adds `..._engine_settings_are_live_read` asserting that `PrefHelper`-backed `EngineSettingsProvider.current` re-reads `cachedPrefs` on every field access (the `val x = prefs.getX()` anti-pattern from `ios-exemplar.md` §3 must not reappear in Android engine code). |
| 12 | Logger backend neutrality | **missing** — no `LoggerBackend` interface; `android.util.Log` called directly in `CandidateProcessor`, `InputNormalizer`, `NextWordService` | N/A | ADD ×2, N/A (pre-req) | Blocked on A1 (introduces `LoggerBackend` + `AndroidLogLoggerBackend` per audit §8 decision #4). A9 adds `..._candidates_only_depend_on_logger_backend_protocol` (ktlint / Detekt rule or reflective scan) + `..._null_logger_is_the_default_factory`. |

---

## 2. Coverage rollup

| Status | Count | Notes |
|---|---|---|
| OK (grep-visible label today) | 0 | — |
| WRAPPER (existing substance, label missing) | 9 tests across §1 (4) / §3 (1) / §4 (1) / §9 (2) / §10 (1) | Low risk for A9 — wraps already-passing assertions. |
| ADD (new test body required, surface exists) | 16 tests across §2 (3) / §3 (2) / §4 (2) / §5 (3) / §6 (5) / §10 (1) | High-value A9 work. §5 / §6 are the biggest gap and are the largest block of new body writing. |
| N/A (pre-req — another A-round creates the surface) | §7 (4), §8 (4), §11 (1), §12 (2) = 11 tests | Gated by A5-impl, A2, A1 respectively. A9 schedules these last. |
| **Total INVARIANT_\* labels to ship at A9 close** | **36** | Sum matches the label count in `behavioral-invariants.md` §1–12 (§1=4, §2=3, §3=3, §4=3, §5=3, §6=5, §7=4, §8=4, §9=2, §10=2, §11=1, §12=2). |

Compared to the iOS G9 matrix: iOS G9 delivered 25 labels restricted to a 10-candidate roster and recorded the §2 / §7 / §8 / §11 / §12 residual under FU-1 / FU-4. Android's A9 closes the gap the other way — one matrix, all 36 labels. The extra 11 Android labels (§7 / §8 / §11 / §12) map onto iOS FU-1 / FU-4 and close together with Android's A9, not on iOS's own G9.

---

## 3. Pre-A9 dependency order

```
A0 (this doc + android-keyboard-baseline-2026-04.md) ← closed with this PR

Testable today (no A-round dependency): §1, §2, §3, §4, §5, §6-partial, §9, §10-partial
  └─ A9 may land these labels early if sequencing helps —
     covers 9 WRAPPER + up to 14 ADD labels (subtract 2 if §4 hyphen wording waits on FU-A4).
     §6-partial = the 5 determinism / ordering labels against calculateScore; sortByScore awaits A1.
     §10-partial = abbrev wrapper testable today; the derivation-matches-normalizer ADD
     awaits A1's CustomDictionaryDerivation extract.

A1 (object → class DI + LoggerBackend + FrequencyData hoist + CustomDictionaryDerivation extract
    + CandidateProcessor.sortByScore frequencyData-parameterization)
  └─ unblocks §5 sortByScore-dependent assertions (if any — §5's three labels
     target dedup, not sort; A9 may land §5 ahead of A1 instead),
     §6 sortByScore coverage (remaining §6 labels testable today),
     §10 custom-derivation-matches-input-normalizer label, §12 LoggerBackend labels.

A2 (EngineSettings + EngineSettingsProvider)
  └─ unblocks §11 Android test.

A5-impl (NextWordEngine / NextWordScorer extract)
  └─ unblocks §7, §8 Android tests (the 8 labels sit on extracted pure functions).
```

A0 does not block A1 / A2 / A5-impl — they can run in any order the roadmap picks. A9 schedules its 36 labels so that the ones with no A-round dependency ship first; the rest queue behind A1 / A2 / A5-impl.

---

## 4. Post-A0 follow-ups (not in this PR, not in Phase 0)

Joint policy with iOS G9 FU list: findings that emerge from this audit queue here and graduate to Phase 0 or to an A-round after A0 closes. Do **not** append to `behavioral-invariants.md` until after A0 is merged.

- **FU-A1 — Android-only engine tests not covered by any Phase 0 invariant**. `DictionaryCoverageTest.kt` (77), `EngineIntegrationTest.kt` (70), `ToneConverterTest.kt` (134), `ToneConverterModelsTest.kt` (404), `ToneRestorationTest.kt` (105) assert behaviors without an `INVARIANT_*` home. Two options: (a) extract the contract into new Phase 0 invariants (e.g. tone-mark double-tap toggles, `ToneRestoration.restore` round-trip), then label; (b) keep them as Android implementation-detail tests outside the invariant grep. Decision deferred to A9.
- **FU-A2 — NFD parity fixture export**. `INVARIANT_nfd_preprocessed_platform_parity` needs a shared fixture batch. Candidate: export from iOS `TaigiUnicodeTests` to a JSON or text file under `docs/architecture/fixtures/nfd-parity.txt`, read by both platforms' tests. Out of scope for A0 — proposal only.
- **FU-A3 — `searchPrefix` tone routing on Android**. Mirror of iOS FU-3. Untested on both platforms. A9 adds when derivation is extracted in A1.
- **FU-A4 — Phase 0 §4 dual-key wording**. Inherits iOS FU-2. If `InputNormalizer.normalize` on either platform is found not to expose `roman_num` vs `notone` separately, Phase 0 §4 must be reworded before the two Android ADD labels can ship.
- **FU-A5 — `Trace.beginSection` pre-instrumented points**. The Android perf baseline (`android-keyboard-baseline-2026-04.md`) notes that Option A requires temporary local instrumentation. A future round may decide whether to land a small set of permanent `android.os.Trace` sections around the keystroke pipeline for continuous Perfetto capture; out of scope here.

---

## 5. Cross-references

- Phase 0 invariants: `docs/architecture/behavioral-invariants.md` (the contract this doc mirrors).
- iOS G9 matrix this doc is the Android sibling of: `docs/architecture/g9-coverage-matrix.md`.
- Phase II plan: `docs/architecture/android-state-audit.md` §A0 (this doc is A0's primary deliverable), §A9 (where the 41 labels land), §A1 / §A2 / §A5-impl (pre-reqs).
- Android perf baseline (sibling A0 artifact): `docs/perf/android-keyboard-baseline-2026-04.md`.
- iOS perf baseline (cross-reference): `docs/perf/keyboard-baseline-2026-04.md`.
- Shared-core roster: `docs/engine/shared-core-readiness.md` (Android candidates map in §5 of the state audit).
