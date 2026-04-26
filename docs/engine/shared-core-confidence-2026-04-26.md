# Shared-Core Confidence — 2026-04-26

**Run mode**: full (cross-platform)
**Branch / commit**: `phase2.5/shared-core-confidence-skill` @ f738827 (main HEAD pre-branch)
**Skill version**: v1 (initial run)
**Gate**: **FAIL** — categorical blocker B1 (D9 FFI POC not built); composite 85.28 / 100 also below the 95 threshold

## Composite

| Dimension | Weight | Score | Notes |
|---|---|---|---|
| D1 iOS purity | 10 | **10.00** | 43/43 candidates clean (no forbidden imports/symbols in non-comment code) |
| D2 Android parity | 15 | **15.00** | 29/29 scorable manifest rows paired (39 total / 10 deferred) |
| D3 Behavior alignment | 20 | **16.28** | 35/43 satisfied (34 both + 1 iOS-N/A satisfied via Android-only) |
| D4 Data model parity | 10 | **6.50** | 12 types audited, 7 semantic mismatches → 10 − (7 × 0.5) |
| D5 Cross-platform invariant comments | 10 | **10.00** | 4/4 required surfaces present + 2/2 mirror citations valid |
| D6 Rust portability | 20 | **20.00** | 0 violations (no `@propertyWrapper`/`@Published`/`KeyPath`/`android.*`/`androidx.*`/`kotlinx.coroutines.*` in candidates) |
| D7 External deps boundary docs | 5 | **5.00** | 4/4 docs exist (`ffi-safety.md` 161L, `rust-core-proto.md` 248L, `data-artifacts-portability.md` 379L, `rust-best-practices.md` 199L) |
| D8 Top-10 test coverage | 5 | **2.50** | Android: 10/10 ≥ 70%; iOS: 0/10 (no `.xcresult` provided — pending user run) |
| D9 FFI POC | 5 | **0.00** | `engine/` Rust workspace absent; D9 unreachable until Phase III |
| **Composite** | **100** | **85.28** | — |

## Categorical blockers

- [x] **B1** D9 not pass — `engine/` does not exist; Phase III prerequisite
- [ ] B2 Forbidden dep in candidate (D1+D6 violations: 0)
- [ ] B3 Phase II.5 doc missing (4/4 present)
- [ ] B4 D7 = 0 (D7 = 5)
- [ ] B5 D3 < 50% of weight (D3 = 16.28, threshold = 10)
- [ ] B6 Other dim < 50% of its weight (D4 = 6.5 ≥ 5; D8 = 2.5 = 50% exact, NOT strictly <)

Only **B1** is triggered.

## Per-dimension breakdown

### D1 iOS purity — 43/43 clean

Initial regex was buggy (POSIX `grep -E` does not support PCRE negative-lookahead `(?!...)`; `^[[:space:]]*[^/]` allowed a leading space to satisfy `[^/]` so `///` doc comments slipped through and triggered false positives in `CandidateProcessor.swift` line 36 + `ComposingState.swift` line 12). Corrected to a two-pass pipeline:

```bash
grep -nE "^[[:space:]]*[^/[:space:]]" "$file" | grep -E "<forbidden patterns>"
```

First pass keeps only lines whose first non-space character is NOT `/`; second pass applies the ERE forbidden-pattern set. SKILL.md retains the corrected form.

After correction: zero candidate files contain forbidden imports/symbols on executable lines.

### D2 Android parity — 29/29 paired

Manifest from `docs/engine/shared-core-readiness.md` §Roster + `docs/architecture/android-exemplar.md` §4–5 enumerates 39 logical surfaces; 10 are deferred (intentional folds/splits per audit memos):

| Deferred surface | Reason |
|---|---|
| Phonetics tables | folded into `TaigiPhonetics.kt` |
| Tone converter models | Android-only (iOS inline) |
| Character input pipeline | Android folds into `TextInputManager` |
| Input mode | Android uses `String` field on `EngineSettings`; iOS enum (A2 audit divergence — see D4) |
| Outcome | iOS uses Swift `throws`; Android uses `sealed class Outcome<T,E>` |
| Dictionary search result | Android-only |
| Custom dictionary entry | Android in-service (marker pending) |
| Enabled dictionaries | iOS inline |
| External lookup URL builder | iOS inline |
| Autocomplete providers | iOS-only (`AutocompleteProviders.swift` — KeyboardKit-shaped provider abstraction; Android has no equivalent) |

The remaining 29 scorable rows all paired (both sides marked `Shared-Core Candidate`).

### D3 invariant coverage — 35/43 satisfied

`behavioral-invariants.md` declares 43 `INVARIANT_*` labels. One label
(`INVARIANT_composing_external_region_clear_discards_state`) is explicitly
iOS-N/A per the doc (line 360 — floating marked text has no in-document region
for the host to clear externally); Android has the test, so it counts as
satisfied.

| Status | Count | Notes |
|---|---|---|
| Both platforms tested | 34 | Most §1 / §3 / §4 / §5 / §6 / §7 / §8 / §9 invariants |
| N/A-satisfied (one side exempt by spec) | 1 | `INVARIANT_composing_external_region_clear_discards_state` |
| Android-only, iOS missing | 6 | §2 NFD trio (`o_dot_combining_collapse`, `poj_nasal_to_nn_substitution`, `nfd_preprocessed_platform_parity`), §11 settings live-read, §12 logger neutrality pair |
| iOS-only, Android missing | 2 | `INVARIANT_normalizer_preserves_hyphens_in_roman_num`, `INVARIANT_normalizer_strips_hyphens_in_notone` (FU-A4 — Android side parked pending iOS resolution) |
| Neither platform | 0 | — |

Score: 35 satisfied / 43 total × 20 = **16.28**.

**Gap labels** (need to add the missing platform):

| Label | Missing on |
|---|---|
| `INVARIANT_candidates_only_depend_on_logger_backend_protocol` | iOS |
| `INVARIANT_engine_settings_are_live_read` | iOS |
| `INVARIANT_nfd_preprocessed_platform_parity` | iOS |
| `INVARIANT_null_logger_is_the_default_factory` | iOS |
| `INVARIANT_o_dot_combining_collapse` | iOS |
| `INVARIANT_poj_nasal_to_nn_substitution` | iOS |
| `INVARIANT_normalizer_preserves_hyphens_in_roman_num` | Android (FU-A4) |
| `INVARIANT_normalizer_strips_hyphens_in_notone` | Android (FU-A4) |

These match the iOS G9 follow-ups FU-1 / FU-4 (see `docs/architecture/g9-coverage-matrix.md`) and Android A9 FU-A4.

### D4 type parity gaps — 7 mismatches across all 12 types

Full audit completed (was a partial 5-of-12 sample in the v1 draft; Codex post-draft P1 caught the omission):

| # | Type | Mismatch | Severity |
|---|---|---|---|
| 1 | `TaigiWord.sourceBitmask` | iOS `UInt16?` vs Android `Int?` (signedness + width) | low — `Int?` accommodates u16 range, but Rust mapping should pin `u16` |
| 2 | `EngineSettings.inputMode` | iOS `InputMode` enum vs Android `String` | medium — documented A2 divergence (`project_android_phase_ii_audit.md` § A2); future round unifies |
| 3 | `EnginePrediction` | Android carries extra `score: Double` field iOS lacks | low — Android wrapper preserves pre-A5 `TaigiWord.lengthScore = score.toInt()` mapping (`nextword-engine-boundary.md` §13.10); intentional, documented |
| 4 | `NextWordEngineSettings.inputMode` | iOS `InputMode` enum vs Android `String` | medium — same root cause as #2 (settings type alignment) |
| 5 | `NextWordPersistedState.currentGeneration` | iOS `UInt64` vs Android `Long` (signed 64-bit) | low — Kotlin lacks unsigned types in stable form; documented in `nextword-engine-boundary.md` §13.6 |
| 6 | `NextWordOutcome.Effect.QueryPredictions` | Android adds `nowMs: Long` field iOS lacks | medium — Android encodes the "ONE consistent nowMs per intent" contract directly (PR #153 r3109358772 fix); iOS reads clock at executor instead. Spec divergence — pick one for shared-core. |
| 7 | `NextWordOutcome.Effect.RescheduleContextTimeout` | iOS `after: TimeInterval` (Double seconds) vs Android `afterMs: Long` (milliseconds) | low — same 30-second invariant on both, just unit divergence per platform idiom (`Timer.scheduledTimer` vs `delay(Long)`) |

Other types (8 of 12) clean: `InputType`, `DictionarySource`, `FrequencyData`, `ToneToggles`, `RawNextWordPrediction`, `ComposingState` (Swift `mutating struct` ↔ Kotlin immutable `data class` is a documented idiom adaptation, not a semantic divergence), `ComposingTransition`, `LexiconError`/`DictionaryError` (6 cases match; iOS `LocalizedError` conformance is intentionally not mirrored on Android per `rules/android-guidelines.md` §10).

D4 score: max(0, 10 − 7 × 0.5) = **6.50**.

### D5 mirror citation reachability — all valid

Required surfaces (4 per `behavioral-invariants.md` §5.3):

| Surface | iOS comment | Android comment |
|---|---|---|
| `NextWordScorer` scoring constants | `NextWordScorer.swift:9` ✓ | `NextWordScorer.kt:14` ✓ |
| `CandidateProcessor` scoring constants | `CandidateProcessor.swift:88` ✓ | `CandidateProcessor.kt:17` + `:40` ✓ |
| `TaigiUnicode.nfdPreprocessed` | `TaigiUnicode.swift:5` ✓ | `TaigiUnicode.kt:11` ✓ |
| `Dictionary/Association BinaryReader` bitmask | `DictionaryBinaryReader.swift:28`,`:174` + `AssociationBinaryReader.swift:206` ✓ | `DictionaryBinaryReader.kt:124`,`:209` + `AssociationBinaryReader.kt:243` ✓ |

Mirror file:line citations checked: 2 (both on Android `NextWordEngine.kt` referencing `ios/Sources/TaigiKeyboard/NextWord/NextWordEngine.swift:27` and `:30`). Both target lines exist and contain the cited surfaces (`associationTimeoutMs` and `contextTimeoutSeconds` declarations). Other `CROSS-PLATFORM INVARIANT` comments cite by surface name without explicit file:line and are not subject to reachability check.

D5 = 5 (required) + 5 (citations 2/2 valid) = **10.00**.

### D6 portability blockers — 0 violations

Initial scan flagged `ComposingState.swift:12` for `@Published` — false positive on a `///` doc comment ("`@Published` fan-out, delegate side effects, ..."). Same two-pass `^[[:space:]]*[^/[:space:]]` correction as D1 dropped it. SKILL.md D6 now uses the same comment-exclusion pipeline as D1 (Codex post-draft P1).

After correction: no Swift candidate carries `@propertyWrapper`, `@dynamicMemberLookup`, `KeyPath<`, `protocol ... where`, `@Published`, `ObservableObject`, or `import Combine`. No Kotlin candidate imports `android.*`, `androidx.*`, `kotlinx.coroutines.*`, or `java.util.concurrent.*`.

D6 = **20.00**.

### D7 boundary doc status — 4/4

| Doc | LOC | Required content present? |
|---|---|---|
| `docs/engine/ffi-safety.md` | 161 | yes — `catch_unwind`, `Mutex`, error sentinel, log bridge all named |
| `docs/engine/rust-core-proto.md` | 248 | yes — request_id, generation counter, settings push all named |
| `docs/architecture/data-artifacts-portability.md` | 379 | yes — asset copy, stamp files, SQLite `user_version`, update-in-place all named |
| `rules/rust-best-practices.md` | 199 | yes — workspace, FFI, unsafe, crate choices all named |

D7 = **5.00**.

### D8 top-10 coverage rollup

**Android (Jacoco from PR #158, 2026-04-21)**:

| Class | Line cov | Status |
|---|---|---|
| TaigiPhonetics | 92.54% | PASS |
| InputNormalizer | 94.29% | PASS |
| CandidateProcessor | 100.00% | PASS |
| ToneConverter | 100.00% | PASS |
| SuggestionCaseTransformer | 100.00% | PASS |
| ToneRestoration | 100.00% | PASS |
| TPSConverter | 98.18% | PASS |
| TaigiUnicode | 100.00% | PASS |
| NextWordScorer | 100.00% | PASS |
| CustomDictionaryDerivation | 100.00% | PASS |

Android: 10/10 ≥ 70%.

**iOS**: no `.xcresult` provided this run. To score iOS, the user runs:

```bash
xcodebuild test \
  -scheme TaigiKeyboard \
  -destination "platform=iOS Simulator,name=iPhone 15" \
  -enableCodeCoverage YES \
  -resultBundlePath /tmp/taigi-g9.xcresult
```

Then re-invoke `/shared-core-confidence --xcresult /tmp/taigi-g9.xcresult`.

Per `feedback_manual_build_test.md` the skill never runs `xcodebuild`. iOS portion currently scored 0.

D8 = ((10 + 0) / 20) × 5 = **2.50**.

### D9 POC status — UNREACHABLE without Phase III work

`ls engine/` returns no such directory. No Rust workspace, no Phonetics crate, no `swift-bridge` / `jni` integration. Phase II.5 Round C delivered the contract docs (`ffi-safety.md` + `rust-core-proto.md`); Phase III builds against them.

D9 = **0**. Per the gate definition (composite ≥ 95% AND D9 = 5), the gate cannot pass until Phase III completes the POC.

## Gap analysis

To reach 95% composite + clear all blockers:

1. **D9 (5 points, B1 blocker)** — Phase III work. Build `engine/` Rust workspace with a Phonetics slice, integrate via `swift-bridge` (iOS) + `jni` (Android), prove `INVARIANT_*` Phonetics tests pass against the Rust implementation. Estimated effort: substantial (Phase III scope per roadmap).

2. **D8 iOS half (2.5 points)** — User runs `xcodebuild` per command above and re-invokes the skill with `--xcresult`. No code change needed; just an evidence collection step.

3. **D3 cross-platform invariant gaps (up to 3.72 points)** — Add the 6 iOS `test_INVARIANT_*` wrappers (mostly thin wrappers per G9 FU-1/FU-4):
   - `INVARIANT_candidates_only_depend_on_logger_backend_protocol`
   - `INVARIANT_engine_settings_are_live_read`
   - `INVARIANT_nfd_preprocessed_platform_parity`
   - `INVARIANT_null_logger_is_the_default_factory`
   - `INVARIANT_o_dot_combining_collapse`
   - `INVARIANT_poj_nasal_to_nn_substitution`

   Plus the 2 Android-side wrappers for FU-A4 (depend on resolution of whether `InputNormalizer.normalize` exposes the dual-key split on either platform's public API).

4. **D4 type parity (up to 3.5 points)** — Resolve the 7 mismatches:
   - High-impact: tighten `inputMode` (mismatches #2 + #4) — Phase II A2-followup, single PR converts both.
   - High-impact: pick one design for `NextWordOutcome.Effect.QueryPredictions` (#6) — Android-style `nowMs` baked-in vs iOS-style executor-side clock read. Spec decision.
   - Pin `TaigiWord.sourceBitmask` (#1) and `NextWordPersistedState.currentGeneration` (#5) to the chosen Rust types in Phase IV-A documentation.
   - `EnginePrediction.score` (#3) and `RescheduleContextTimeout` unit (#7) — documented intentional divergence; mark as deferred.

**Cheapest path to 95**:

- D9 + iOS coverage alone = 85.28 + 5 + 2.5 = 92.78 (insufficient)
- D9 + iOS coverage + 5 D3 wrappers (5/43 × 20 = +2.33) = 95.11 (just crosses)
- D9 + iOS coverage + all 8 D3 gaps closed = 85.28 + 5 + 2.5 + 3.72 = **96.50**
- D9 + iOS coverage + all 8 D3 + 4 high-impact D4 fixes = 96.50 + 2.0 = **98.50**

Once Phase III lands D9 = 5 the gate is mechanically reachable; D3 + D4 cleanup is recommended for soundness rather than gate-pass arithmetic.

## Recommended next-round actions

In priority order:

1. **(blocker)** Schedule Phase III FFI POC. Without D9 = 5 the gate is unreachable.
2. User runs `xcodebuild test -enableCodeCoverage YES` to provide an iOS `.xcresult`; re-invoke skill. Closes D8 iOS half (5-minute task).
3. Add the 6 iOS thin invariant-wrapper tests (G9 FU-1/FU-4). One PR, low risk — iOS-side test-only change. Closes ~2.79 D3 points.
4. Decide `NextWordOutcome.Effect.QueryPredictions` shape (Android `nowMs` baked-in vs iOS executor-side clock read) — pick one for shared-core spec; align the other platform.
5. Spec PR for `EngineSettings.inputMode` enum unification (Android `String` → typed enum); single PR closes mismatches #2 + #4.

## Methodology notes for v1

- D3 formula: `satisfied / total × 20`. N/A labels stay in the denominator and count as satisfied when the non-exempt platform has the test.
- D4 audited all 12 types listed in SKILL.md §D4; mismatch rule = +1 per semantic field/case mismatch, +0.5 weight.
- D5 mirror reachability is path + line-in-range + surface-name token presence. Intentionally lax — does not enforce semantic equivalence.
- D8 iOS scoring requires user-supplied `.xcresult`. The skill never runs `xcodebuild`.
- D1 / D6 grep recipes use a two-pass pipeline (filter comment-only lines via `^[[:space:]]*[^/[:space:]]`, then ERE pattern match) — POSIX `grep -E` does not support PCRE negative-lookahead.
- All scoring recipes embedded in `.claude/skills/shared-core-confidence/SKILL.md`.

## Codex sandwich record

- **Pre-impl review** (`/tmp/scc_skill_codex_pre.txt`): 8 corrections applied to D2 (manifest not basename), D4 (type map), D5 (required surfaces + reachability), D6 (regex enumeration), D7 (4 docs not 1), D8 (`scripts/ios-coverage-report.sh` + Jacoco), categorical blockers (B1–B6), `--ios-only` / `--android-only` diagnostic-only.
- **Post-draft review** (`/tmp/scc_skill_codex_post.txt`): 5 P1 blockers fixed in this revision —
  - D4 audit completed for all 12 types (was 5-sample); 7 mismatches surfaced (was 2).
  - D2 manifest gained `AutocompleteProviders.swift` row (deferred — iOS-only).
  - D1 / D6 grep recipes rewritten as two-pass pipelines (POSIX-compatible).
  - D3 cheapest-path arithmetic corrected (4 labels = 1.86 pts, not "~2.2").
  - D3 formula explicitly handles N/A labels in numerator + denominator.
