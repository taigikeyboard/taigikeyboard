# G9 — Engine Test Coverage Matrix

**Status**: audit landed 2026-04-19 as the first deliverable of G9. This doc maps Phase 0 invariants (`behavioral-invariants.md`) onto the test suite for the 10 shared-core candidates named in `ios-exemplar-plan.md` §G9 and records the gap list that the remaining G9 work closes.

**Gate rule (Codex + Claude joint decision)**: G9 must prove two things **separately** — (1) every Phase 0 `INVARIANT_*` label exists as a grep-visible test func name, and (2) each of the 10 candidates reaches ≥70% line coverage under `xcodebuild test -enableCodeCoverage YES`. Naming and numeric coverage are not interchangeable.

**Policy (Codex Q3)**: thick test files keep their existing descriptive `testXxx()` names and grow thin `test_INVARIANT_*` wrappers that assert the same behavior on a canonical Phase 0 fixture. Thin test files gain direct `test_INVARIANT_*` methods where they already pass through to the invariant path.

**Phase 0 is frozen for G9 (Codex Q5)**. New invariants discovered during this audit are recorded under *Post-G9 follow-ups* at the bottom, not appended to `behavioral-invariants.md` until after G9 closes.

---

## 1. Candidate × test file × invariant mapping

| Candidate | Test file | LOC | Phase 0 section(s) | Invariant labels required | Pre-G9 gap |
|---|---|---|---|---|---|
| `PhoneticsConverter` | `TaigiPhoneticsTests` | 465 | #1 TL↔POJ roundtrip | 4 | wrappers only |
| `SyllableParser` | `TaigiPhoneticsTests` (shared) | — | #1 support | 0 direct (covered via §1) | — |
| `TPSToTL` | `TPSConverterTests` | 882 | #3 TPS roundtrip | 3 | wrappers + idempotency assertion |
| `TLToTPS` | `TPSConverterTests` (shared) | — | #3 TPS roundtrip | 0 direct (shares §3 labels) | — |
| `InputNormalizer` | `InputNormalizerTests` | 739 | #4 mode-agnostic | 3 | wrappers + `notone`/`roman_num` hyphen split |
| `CandidateProcessor` | `CandidateProcessorTests` | 80 | #5 dedup, #6 scoring | 3 + 5 = 8 | **8 missing** — biggest gap |
| `NextWordScorer` | `NextWordScorerTests` | 79 | #7 decay, #8 weighting | 4 + 4 = 8 | 5 present in substance, 3 naming-only + constants pin |
| `AutocompleteContextBooster` | `AutocompleteContextBoosterTests` | 70 | **none in Phase 0** | 0 | coverage-only; see follow-up #FU-1 |
| `CaseTransformer` | `CaseTransformerTests` | 349 | #9 case transform | 2 | wrappers only |
| `CustomDictionaryDerivation` | `CustomDictionaryDerivationTests` | 63 | #10 custom-dict derivation | 2 | 1 wrapper + `searchPrefix` / `generateRomanNum` direct |

Phase 0 sections **#2 NFD** (`TaigiUnicode.swift`), **#11 settings live-read** (`EngineSettings.swift`), **#12 logger neutrality** (`LoggerBackend.swift`) are invariants on supporting utilities, not on the 10 G9 candidates. G9's gate does not require them; they are tracked under follow-ups.

---

## 2. Invariant-by-invariant audit

Status codes: **OK** = grep-visible test with the exact label; **WRAPPER** = behavior already asserted under a different name, add a thin `test_INVARIANT_*` that calls through; **ADD** = no substantive assertion, write a new test body; **N/A** = not in scope for G9 candidates.

### §1 — TL ↔ POJ round-trip (`PhoneticsConverter` / `SyllableParser`)

| Label | Status | Existing test (if any) |
|---|---|---|
| `INVARIANT_tl_to_poj_roundtrip_is_lossless` | WRAPPER | `testTlDisplayToPOJDisplay_*` + `testConvertSyllable_*` |
| `INVARIANT_poj_to_tl_roundtrip_is_lossless` | WRAPPER | `testPojDisplayToTLDisplay_*` + `testPojDisplayToTLDisplay_idempotentOnTL` |
| `INVARIANT_oo_combining_form_roundtrips` | WRAPPER | `testTlDisplayToPOJDisplay_ooHandling`, `testPOJ_oDot_*` |
| `INVARIANT_nasal_marker_variants_collapse_on_parse` | WRAPPER | `testPojDisplayToTLDisplay_nasalConversion` |

Wrapper approach: one new test per label that drives the full TL→POJ→TL (and POJ→TL→POJ) path on a Phase 0 fixture batch (`Guá` / `Goá`, `oo` / `o͘`, `ⁿ` / `ᴺ` / `nn`, stop-tone + non-stop).

### §3 — TPS ↔ TL round-trip (`TPSToTL` / `TLToTPS`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_tps_to_tl_roundtrip` | WRAPPER | `testRoundTrip_tpsToTlToTps` |
| `INVARIANT_tl_to_tps_roundtrip` | ADD | individual `testToTPS_*` tests exist; no bidirectional roundtrip |
| `INVARIANT_tps_adjuster_is_idempotent` | ADD | `testAdjustTPSInitialKey_*` covers positional rules, not idempotency |

### §4 — Input normalization, mode-agnostic (`InputNormalizer`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_input_normalizer_is_mode_agnostic` | WRAPPER | `testNormalize_pojMode_*`, `testNormalize_tlMode_*`, `testNormalize_tps*` — covers each mode separately |
| `INVARIANT_normalizer_preserves_hyphens_in_roman_num` | ADD | hyphens are dropped in every test case shown (600 autogen fixtures strip `-` → `''`) — dedicated `-preserved` mode test not present |
| `INVARIANT_normalizer_strips_hyphens_in_notone` | WRAPPER | implied by 600 autogen cases; needs an explicit named test |

Note: Phase 0 §4 says "two keys for two indexes — `roman_num` preserves hyphens, `notone` strips them." Inspection of the autogen fixtures in `InputNormalizerTests.swift` shows hyphens dropped in the `tl_num` output. Need to verify whether `InputNormalizer` truly maintains the dual `roman_num` vs `notone` split, or whether Phase 0 §4 misrepresents current behavior. Flagged under follow-ups as **FU-2**.

### §5 — Candidate dedup (`CandidateProcessor`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_engine_dedup_keys_on_roman_plus_hanzi` | WRAPPER | `testRemoveDuplicates_*` (5 tests cover key structure) |
| `INVARIANT_display_dedup_runs_after_sort` | ADD | `removeDisplayDuplicates` is untested |
| `INVARIANT_display_dedup_keeps_words_without_hanzi` | ADD | untested |

### §6 — Candidate scoring (`CandidateProcessor`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_score_is_deterministic` | ADD | `calculateScore` untested |
| `INVARIANT_user_freq_dominates_ranking` | ADD | `sortByScore` untested |
| `INVARIANT_completion_penalty_separates_tiers` | ADD | untested |
| `INVARIANT_recency_window_is_exactly_1_hour` | ADD | untested (boundary `<` vs `<=`) |
| `INVARIANT_roman_to_base_strips_tones_hyphens_digits` | ADD | `romanToBase` is `private` — must test through `calculateScore` with `exactBonus` probe |

### §7 — Next-word decay (`NextWordScorer`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_decay_half_life_is_168_hours` | WRAPPER | `testCalculateDecay_atOneHalfLifeIsHalf` |
| `INVARIANT_high_usage_decay_floor_95` | WRAPPER | `testCalculateUserScore_highUsageHasHigherFloor` |
| `INVARIANT_low_usage_decay_floor_30` | WRAPPER | same |
| `INVARIANT_scorer_constants_match_android` | ADD | pin test asserting `userWeight==50 && dictWeight==1 && learningBonus==300 && decayHalfLifeHours==168 && highUsageDecayFloor==0.95 && lowUsageDecayFloor==0.3 && highUsageThreshold==3` |

### §8 — Next-word weighting, user > dict (`NextWordScorer`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_user_weight_is_50` | WRAPPER | covered by `_atEpoch_hitsLearningBonusFloor`; isolate into a named pin test |
| `INVARIANT_dict_weight_is_1` | WRAPPER | `testScoreDict_isCountTimesDictWeight` |
| `INVARIANT_learning_bonus_is_300` | WRAPPER | implied by `_hitsLearningBonusFloor`; name it |
| `INVARIANT_user_entry_outranks_dict_entry` | WRAPPER | `testCalculateUserScore_userEntryOutranksDictEntry` |

### §9 — Case transformation (`CaseTransformer`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_case_transformer_is_deterministic` | WRAPPER | `testCapitalizeCandidate_*` (11 tests) — same inputs repeatedly assert same outputs |
| `INVARIANT_case_transformer_honors_auto_cap_flag` | WRAPPER | `testTransformForInput_autoCapOff_*` + `testCapitalizeCandidate_autoCapOff` |

### §10 — Custom-dictionary derivation (`CustomDictionaryDerivation`)

| Label | Status | Existing test |
|---|---|---|
| `INVARIANT_custom_derivation_matches_input_normalizer` | ADD | `generateRomanNum` is defined as `InputNormalizer.normalize(_, mode: .tl)` but no test asserts the parity |
| `INVARIANT_abbrev_key_is_one_char_per_syllable` | WRAPPER | `testGenerateAbbrev_*` (5 tests) |

`searchPrefix(for:)` is untested (tone-aware vs toneless routing). Flagged as **FU-3**.

---

## 3. Work breakdown for remaining G9 phases

### S3a — Thin test expansion (≥ +120 LOC total, per file)

| File | New tests to add | Asserts what |
|---|---|---|
| `CandidateProcessorTests` | 8 `test_INVARIANT_*` (5 scoring + 2 display dedup + 1 score-determinism pin); plus `testRemoveDisplayDuplicates_*` behavior | covers §5 labels 2/3 and all §6 labels |
| `NextWordScorerTests` | 4 `test_INVARIANT_*` (3 named pins, 1 constants pin) | covers remaining §7 + §8 naming |
| `AutocompleteContextBoosterTests` | coverage-only; 2 new edge-case tests: `displayText` fallback when hanzi empty string (not nil), and stable-partition with 100-item input | pushes line coverage past 70% on a 33-LOC source file |
| `CustomDictionaryDerivationTests` | 3 tests: `test_INVARIANT_custom_derivation_matches_input_normalizer`, `testSearchPrefix_toneAware`, `testSearchPrefix_toneless` | covers §10 label #1 + FU-3 |

### S3b — Thick wrapper wiring (low LOC, high label coverage)

| File | New wrappers | Asserts what |
|---|---|---|
| `TaigiPhoneticsTests` | 4 `test_INVARIANT_*` for §1 labels, each running a Phase 0 corner-case batch through `convertSyllable` + display conversions | §1 labelization |
| `TPSConverterTests` | 3 `test_INVARIANT_*` — roundtrip (reuse existing), adjuster idempotency (new assertion), TL→TPS reverse roundtrip (new) | §3 labelization + 1 new assertion |
| `InputNormalizerTests` | 3 `test_INVARIANT_*` for §4 labels; the `roman_num` hyphen-preservation test may fail — if so, raises FU-2 for Phase 0 wording review | §4 labelization + behavior probe |
| `CaseTransformerTests` | 2 `test_INVARIANT_*` wrapping existing assertions on a compact fixture | §9 labelization |

### S4 — Coverage helper + documentation

Add `scripts/ios-coverage-report.sh`:

```bash
#!/usr/bin/env bash
# Parse an xcresult bundle the user generated and print per-candidate
# line-coverage percentages. Read-only; never invokes xcodebuild itself.
set -euo pipefail
XCRESULT="${1:-}"
if [[ -z "$XCRESULT" || ! -d "$XCRESULT" ]]; then
  echo "usage: $0 <path-to-xcresult-bundle>" >&2
  exit 64
fi
# Prints a name + line-coverage row per target file matching the G9 roster.
xcrun xccov view --report --json "$XCRESULT" \
  | python3 -c '…parse & filter to the 10 candidate source files…'
```

User workflow (invoked manually, per `feedback_manual_build_test`):

```bash
# 1. Run the test suite with coverage
xcodebuild test \
  -scheme TaigiKeyboard \
  -destination "platform=iOS Simulator,name=iPhone 15" \
  -enableCodeCoverage YES \
  -resultBundlePath /tmp/taigi-g9.xcresult

# 2. Extract per-candidate numbers
./scripts/ios-coverage-report.sh /tmp/taigi-g9.xcresult
```

The helper parses a user-generated artifact — it never touches the build system. This satisfies the "reproducible numbers without Claude running xcodebuild" ruling on Codex Q2.

---

## 4. Post-G9 follow-ups (not in this PR, not in Phase 0 yet)

Codex Q5 rules that Phase 0 stays frozen during G9. Findings that emerge from this audit go here; they graduate to Phase 0 in a future PR after G9 closes.

- **FU-1 — AutocompleteContextBooster invariant**. The booster is a G9 candidate but has no Phase 0 section. Candidate wording: "bigram-predicted first-char candidates partition to the front while preserving in-partition stable order." Owner: next session.
- **FU-2 — InputNormalizer §4 wording vs reality**. Phase 0 §4 claims two distinct keys (`roman_num` preserves hyphens, `notone` strips them). Inspection of 600 autogen fixtures suggests the `InputNormalizer.normalize` public API strips hyphens uniformly; the dual-key split may live in `CustomDictionaryDerivation.searchPrefix` instead. If the wrapper test fails, re-scope Phase 0 §4 before unfreezing.
- **FU-3 — `searchPrefix(for:)` routing**. Untested. Tone-aware input (`"hó2"`) vs toneless (`"ho"`) routing is a behavior worth pinning.
- **FU-4 — Phase 0 §2 NFD**, **§11 EngineSettings**, **§12 LoggerBackend**: none of the 10 G9 candidates owns these, so they fall outside G9's gate. New Phase 0 labels for these exist without tests; either extend G9 beyond the top-10 roster (scope creep — not recommended) or open a dedicated follow-up PR after G9.
- **FU-5 — `TPSInputAdjuster` idempotency**. Phase 0 §3 names this but the existing test suite asserts positional rules, not `adjust(adjust(x)) == adjust(x)` idempotency. S3b addresses via a new assertion; if brittle, escalate.

---

## 5. Cross-references

- Phase 0 invariants: `docs/architecture/behavioral-invariants.md`
- G9 task definition: `docs/architecture/ios-exemplar-plan.md` §G9
- Shared-core roster: `docs/engine/shared-core-readiness.md`
- Codex ruling transcript: session 019da4cc-473c-7123-a261-ad99ba1d42c0 (2026-04-19)
