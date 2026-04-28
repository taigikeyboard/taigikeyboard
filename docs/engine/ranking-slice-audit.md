# Lexicon Ranking Slice — Pre-flight Audit

**Branch**: `phase4b/ranking-slice`
**Slice goal**: migrate `CandidateProcessor` (Lexicon ranking) from Swift/Kotlin to Rust shared core, mirroring the v3.5.1 Phonetics slice pattern (single FFI op via `RustEngineBridge`, INVARIANT parity tests, direct swap, no fallback toggle).

This document captures the cross-platform audit performed before any code change, satisfying Codex pre-impl review P1 fix #4 (JVM-test compat) and #5 (op-contract definition). It is **commit 0** of the slice PR — no production code is modified by it.

## 1. Scope inventory

### 1.1 iOS `CandidateProcessor` — public surface

`ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift` (266 LOC).

| Function | Phase | Platform deps |
|---|---|---|
| `isHanzi(_:)` | classify | none |
| `capitalize(_:basedOn:inputMode:isAutoCap:)` | case | `CaseTransformer.capitalizeCandidate` |
| `startsWithRomanLetter(_:)` | case helper | none |
| `removeDuplicates(_:)` | dedup | none |
| `removeDisplayDuplicates(_:)` | dedup | none |
| `calculateScore(word:normalizedInput:frequencyData:currentTime:)` | score | uses private `romanToBase` → `TaigiUnicode.nfdPreprocessed` |
| `sortByScore(_:normalizedInput:frequencyDataMap:currentTime:)` | sort | `LoggerFactory` (debug log) |
| `romanToBase(_:)` (private) | helper | `TaigiUnicode.nfdPreprocessed` |
| `inputToBase(_:)` (private) | helper | none |

### 1.2 Android `CandidateProcessor` — public surface

`android/.../ime/dictionary/CandidateProcessor.kt` (207 LOC).

| Function | Phase | Platform deps | Match iOS? |
|---|---|---|---|
| — (no `isHanzi`) | — | — | ❌ MISSING |
| — (no `capitalize`) | — | — | ❌ MISSING — Android does case in `SuggestionCaseTransformer.kt` |
| — (no `startsWithRomanLetter`) | — | — | ❌ MISSING — Android handles inside `SuggestionCaseTransformer` |
| `removeDuplicates(words)` | dedup | none | ✅ |
| `removeDisplayDuplicates(words)` | dedup | none | ✅ |
| `calculateScore(word, normalizedInput, frequencyData, currentTime)` | score | uses internal `romanToBase` → `TaigiUnicode.nfdPreprocessed` | ✅ formula identical |
| `sortByScore(words, normalizedInput, frequencyData, currentTime, logger)` | sort | `LoggerBackend` (passed in) | ✅ |
| `romanToBase(roman)` (`internal`) | helper | `TaigiUnicode.nfdPreprocessed` | ✅ logic identical |
| `inputToBase(normalizedInput)` (`internal`) | helper | none | ✅ |

**Stale import**: `import com.siansiansu.taigikeyboard.engine.RustEngineBridge` at line 6 is unused. Will be removed when the file becomes a thin wrapper.

### 1.3 Constants — cross-platform parity check

| Constant | iOS value | Android value | Status |
|---|---|---|---|
| `USER_FREQ_CAP` | `100` (inline `min(... , 100)`) | `100` | ✅ |
| `USER_FREQ_WEIGHT` | `100` (inline `* 100`) | `100` | ✅ |
| `RECENCY_WINDOW_MS` | `60 * 60 * 1000` | `60L * 60L * 1000L` | ✅ |
| `RECENCY_BONUS` | `200` | `200` | ✅ |
| `EXACT_BONUS` | `100` | `100` | ✅ |
| `COMPLETION_PENALTY` | `-1000` | `-1000` | ✅ |
| `CLOSENESS_WEIGHT` | `500` | `500` | ✅ |
| `BASE_FREQ_DIVISOR` | `10` (inline `/ 10`) | `10` (named const) | ✅ same value, named only on Android |
| `SOURCE_TIERS[0]` | bit 0, num 15 (kautian) | bit 0, num 15 (kautian) | ✅ |
| `SOURCE_TIERS[1]` | bit 1, num 13 (taigitv) | bit 1, num 13 (taigitv) | ✅ |
| `SOURCE_TIERS[2]` | bit 7, num 12 (stti) | bit 7, num 12 (stti) | ✅ |
| `SOURCE_TIERS[3]` | bit 6, num 11 (kungge) | bit 6, num 11 (kungge) | ✅ |
| `DEFAULT_TIER_NUMERATOR` | `10` | `10` | ✅ |
| `TIER_DENOMINATOR` | `10` | `10` | ✅ |

**No drift detected.** Adopt iOS as canonical for the Rust port; first-match-wins ordering preserved.

## 2. Caller inventory (`CandidateProcessor.*` outside the file)

### 2.1 iOS

| Caller | Method | Notes |
|---|---|---|
| `LexiconService.search:103` | `removeDuplicates` | merge custom + system results |
| `LexiconService.search:109` | `removeDisplayDuplicates` | **TPS-gated** (`if inputMode == .tps`) |
| `LexiconService.search:145, 147, 204, 210` | `capitalize` | iOS-only entry point |
| `LexiconService.search:146, 203` | `startsWithRomanLetter` | iOS-only |
| `LexiconService.rankByFrequency:237` | `sortByScore` | sole `sortByScore` caller |
| `AutocompleteInputClassifier.swift:28` | `isHanzi` | text-type classification |
| `DictionarySearchService.swift:81` | `isHanzi` | dictionary tab CJK detection |

### 2.2 Android

| Caller | Method | Notes |
|---|---|---|
| `LexiconService.kt:208` | `removeDuplicates` | merge custom + system |
| `LexiconService.kt:230` | `removeDisplayDuplicates` | **TPS-gated** (`if (settings?.inputMode == "tps")`) — same gate as iOS |
| `LexiconService.kt:212` | `sortByScore` | sole `sortByScore` caller |

## 3. Behavior parity decision

| Concern | iOS | Android | Rust slice owns? | Rationale |
|---|---|---|---|---|
| `isHanzi` text classify | inside CandidateProcessor | inlined in `AutocompleteInputClassifier.kt:51` | **NO** — keep platform | Two callers iOS-side (Autocomplete + DictionarySearch); Android already has a private helper. Cross-platform helper, not a ranking concern. Pulling into Rust would require an extra FFI op for a 6-line Unicode-range check — net loss |
| `capitalize` / `startsWithRomanLetter` | inside CandidateProcessor | inside `SuggestionCaseTransformer.kt` | **NO** — keep platform | iOS callers stay in `LexiconService.search`; Rust does NOT capitalize → preserves Android's no-op-in-ranking behavior. Refactor-only rule (no behavior change cross-platform) |
| `removeDuplicates` | LexiconService merge step | LexiconService merge step | **YES** | Identical pure logic both sides |
| `removeDisplayDuplicates` | TPS-gated inside `LexiconService.search` | TPS-gated inside `LexiconService.composeRanked` | **YES — but caller decides invocation** | Rust exposes the function; the TPS-gate decision stays platform (cleaner: `process_candidates` returns ranked list; platform calls `OP_REMOVE_DISPLAY_DUPLICATES` after if `inputMode == TPS`) — OR fold gate into request as `bool tps_dedup_enabled = N`. **Decision: include `tps_dedup_enabled` flag in request to keep one round-trip** |
| `calculateScore` math | identical formulae | identical formulae | **YES** | Math is the slice |
| `sortByScore` orchestration | maps + sorts + logs | maps + sorts + logs | **YES** | Pure |
| `romanToBase` NFD preprocessing | platform `TaigiUnicode.nfdPreprocessed` | platform `TaigiUnicode.nfdPreprocessed` | **YES — Rust internalises NFD logic** | `TaigiUnicode.nfdPreprocessed` stays platform per PR #187 (JVM-test compat); Rust `ranking` crate must do its own NFD walk. The `engine/phonetics/src/derivation.rs` already has `nfd_preprocessed` (literal-substring trie-key variant) — reuse or duplicate locally per Codex pattern |
| `inputToBase` digit/lowercase strip | identical | identical | **YES** | Trivial pure helper |
| Score debug logging | iOS `LoggerFactory.make` | Android `LoggerBackend` injected | **NO — stays platform** | Rust returns `ScoreBreakdown` per candidate; platform formats + logs. Avoids string-formatting allocations on Rust hot path; logger backend already lives in the bridge. **Decision: extend `ProcessCandidatesResponse` with optional per-candidate `ScoreBreakdown` repeated field**, platform formats |

## 4. JVM unit-test JNI compatibility audit (P1 fix #4)

`android/app/src/test/.../CandidateProcessorTest.kt` runs on host JVM. `RustEngineBridge.<init>` triggers `System.loadLibrary("rust_taigi")` → `UnsatisfiedLinkError`.

**Test methods that touch ranking internals**:

| Test | Touches | Production wire-through risk |
|---|---|---|
| `test_INVARIANT_engine_dedup_keys_on_roman_plus_hanzi` | `removeDuplicates` | HIGH if removeDuplicates routes through Rust |
| `test_INVARIANT_display_dedup_runs_after_sort` | `removeDisplayDuplicates` | HIGH |
| `test_INVARIANT_display_dedup_keeps_words_without_hanzi` | `removeDisplayDuplicates` | HIGH |
| `test_INVARIANT_score_is_deterministic` | `calculateScore` | HIGH |
| `test_INVARIANT_user_freq_dominates_ranking` | `calculateScore` | HIGH |
| `test_INVARIANT_completion_penalty_separates_tiers` | `calculateScore` | HIGH |
| `test_INVARIANT_recency_window_is_exactly_1_hour` | `calculateScore` | HIGH |
| `test_INVARIANT_roman_to_base_strips_tones_hyphens_digits` | `romanToBase` (`internal`) | HIGH |
| `sortByScore_honors_caller_supplied_frequency_data` | `sortByScore` | HIGH |
| `test_INVARIANT_tier_bonus_preserves_frequency_ordering` | `calculateScore` | HIGH |
| `test_INVARIANT_tier_bonus_first_match_wins` | `calculateScore` | HIGH |
| `test_kautian_beats_itaigi_at_comparable_frequency` | `sortByScore` | HIGH |

**Strategy** (per `feedback_jvm_test_jni_compat.md`, PR #187 pattern):

1. **Production swap goes through Rust**. `LexiconService.kt:208, 212, 230` call `RustEngineBridge.processCandidates(...)`.
2. **Platform Kotlin helpers retained** as the "canonical" copy for `src/test/`. Apparent duplication is intentional — JVM tests pin the math, Rust crate has its own unit + integration tests over the same fixtures.
3. **CROSS-PLATFORM INVARIANT comment** added to both Kotlin and Rust sides pointing at each other (mirrors PR #187 pattern for `TaigiUnicode` / `ToneUtilities.adjustNasalMarkerCase`).
4. **Acceptance**: Rust's own tests + iOS XCTest (links xcframework, no JNI issue) + Android instrumented dogfood are the primary parity gates. Android `src/test/` tests serve as a redundant fast-feedback loop and a JVM-side check that platform helpers haven't drifted from the Rust impl.

## 5. Rust `engine/ranking/` crate layout (locked)

```
engine/ranking/
├── Cargo.toml              # name = "ranking"; deps = protos, log, thiserror
└── src/
    ├── lib.rs              # public: process_candidates(req) -> resp
    ├── classify.rs         # iOS-only is_hanji REJECTED (out of scope per §3)
    ├── case.rs             # REJECTED (capitalize stays platform per §3)
    ├── dedup.rs            # remove_duplicates + remove_display_duplicates
    ├── score.rs            # SOURCE_TIERS const + ScoreBreakdown + calculate_score + roman_to_base + input_to_base
    ├── sort.rs             # sort_by_score
    └── nfd.rs              # local NFD preprocessing (mirrors phonetics::derivation::nfd_preprocessed semantics; PR #187 reasoning)
```

5 modules total. `classify.rs` and `case.rs` rejected after audit § 3 — keeps the crate scope tight.

## 6. Proto contract (locked, per Codex P1 fix #1 + #3)

### 6.1 `engine/protos/proto/envelope.proto` additions

```proto
enum CommandType {
  CMD_UNSPECIFIED = 0;
  CMD_PHONETICS = 1;
  reserved 2;        // CMD_COMPOSING for D9.3
  CMD_LEXICON = 3;   // NEW — ranking slice
}

message Request {
  // ... existing ...
  oneof payload {
    PhoneticsRequest phonetics = 10;
    // 11 reserved for composing
    LexiconRequest lexicon = 12;        // NEW
  }
}

message Response {
  // ... existing ...
  oneof payload {
    PhoneticsResponse phonetics = 10;
    // 11 reserved for composing
    LexiconResponse lexicon = 12;       // NEW
  }
}
```

### 6.2 `engine/protos/proto/lexicon.proto` (new file)

```proto
syntax = "proto3";
package taigi.engine;

import "phonetics.proto";  // reuse InputMode
option java_package = "com.siansiansu.taigikeyboard.engine.proto";
option java_multiple_files = true;

message TaigiWord {
  int64 id = 1;
  string roman = 2;
  optional string hanji = 3;
  optional int32 length_score = 4;
  optional uint32 source_bitmask = 5;
}

message FrequencyEntry {
  string display_text_key = 1;
  uint32 count = 2;
  int64 last_used_ms = 3;
}

message ScoreBreakdown {
  int32 user_freq_score = 1;
  int32 recency_bonus = 2;
  int32 exact_bonus = 3;
  int32 completion_penalty = 4;
  int32 closeness_bonus = 5;
  int32 base_freq_score = 6;
}

message ProcessCandidatesRequest {
  repeated TaigiWord raw = 1;
  string normalized_input = 2;
  // Tag 3 was originally `InputMode mode`; dropped during impl per § 3
  // (engine never derives the tps-dedup gate from input_mode; platform
  // decides). Tag is `reserved 3;` in the shipped lexicon.proto so the
  // wire layout stays aligned with this audit and the slot is not
  // accidentally reused.
  reserved 3;
  bool tps_dedup_enabled = 4;     // platform-decided gate
  repeated FrequencyEntry freq = 5;
  int64 now_ms = 6;
  bool include_breakdown = 7;     // for #if DEBUG logging on iOS
}

message ProcessCandidatesResponse {
  repeated TaigiWord ranked = 1;
  repeated ScoreBreakdown breakdown = 2;  // populated only if include_breakdown
}

message LexiconRequest {
  oneof method {
    ProcessCandidatesRequest process_candidates = 10;
  }
}

message LexiconResponse {
  oneof result {
    ProcessCandidatesResponse process_candidates = 10;
  }
}
```

### 6.3 Top-level dispatcher

`engine::dispatch::process_request(req: Request) -> Response` routes by `CommandType`. Refactor `phonetics::process_request` to keep its current scope; add `ranking::process_request` entry point. `swift-ffi` and `android-jni` thin wrappers update to call the top-level dispatcher.

## 7. Commit sequence (revised, locked)

| # | Title | Touches |
|---|---|---|
| 0 | `docs(engine): ranking-slice pre-flight audit` | This file only — no production code |
| 1 | `proto: add LexiconRequest/Response + lexicon.proto + envelope CMD_LEXICON` | `*.proto` + Swift `.pb.swift` regen + Java protobuf regen + Rust `protos` crate regen |
| 2 | `engine(ranking): crate skeleton + dedup module + tests` | `engine/ranking/` skeleton; `dedup.rs` (`remove_duplicates`, `remove_display_duplicates`) + tests |
| 3 | `engine(ranking): score module + nfd helper + tests` | `score.rs` (`SOURCE_TIERS`, `ScoreBreakdown`, `calculate_score`, `roman_to_base`, `input_to_base`) + `nfd.rs` (local NFD preprocess) + parity tests including tier first-match-wins |
| 4 | `engine(ranking): sort module + process_candidates orchestrator + tests` | `sort.rs` + `lib.rs` `process_candidates` orchestrator + integration tests against fixture ports of iOS+Android `CandidateProcessorTest` |
| 5 | `engine: top-level envelope dispatcher + lexicon op wiring` | `engine/src/dispatch.rs` (or equivalent) routes phonetics vs lexicon; `swift-ffi` + `android-jni` updated |
| 6 | `ios: swap CandidateProcessor production callers to Rust` | `LexiconService.swift` calls `RustEngineBridge.processCandidates(...)` on the connected-freq-DB path; `CandidateProcessor.swift` keeps `capitalize`/`isHanzi`/`startsWithRomanLetter` (out of slice scope) AND retains `removeDuplicates`/`removeDisplayDuplicates` for the cold-start fallback (see § 8 row "iOS cold-start fallback"); loses `calculateScore`/`sortByScore`/`romanToBase`/`inputToBase`/tier-bonus/`ScoreBreakdown`/`logScoreDetails`; xcframework regen; `CandidateProcessorTests.swift` parity tests pass against new path |
| 7 | `android: swap CandidateProcessor production callers to Rust + retain JVM-test platform helpers` | `LexiconService.kt` calls `RustEngineBridge.processCandidates(...)`; `CandidateProcessor.kt` retains internal Kotlin impl with `CROSS-PLATFORM INVARIANT` note pointing at Rust mirror; jniLibs regen; `src/test/` tests still green; remove stale `RustEngineBridge` import on Android `CandidateProcessor.kt` (unused) |

8 commits total (commit 0 = this audit doc).

## 8. Risks + mitigations (revised)

| Risk | Mitigation |
|---|---|
| Rust local NFD diverges from platform `TaigiUnicode.nfdPreprocessed` | Lift the `engine/phonetics/src/derivation.rs::nfd_preprocessed` (or local copy) and pin via parity tests fed by a Kotlin/Swift dump of expected outputs over the existing fixture set |
| `ScoreBreakdown` proto serialization cost on hot path | `include_breakdown=false` in production; iOS `#if DEBUG` and Android `BuildConfig.DEBUG` flip it on. The flag is plumbed all the way into `engine/ranking/src/sort.rs::sort_by_score`, which selects between `sort_with_breakdown` (DEBUG path) and `sort_totals_only` (release path) — release builds skip both the per-candidate `ScoreBreakdown` allocation AND the parallel `Vec<ScoreBreakdown>` build-up |
| `tps_dedup_enabled` gate semantics drift between platforms | gate is computed from the SAME source on both sides (iOS `inputMode == .tps`, Android `settings?.inputMode == "tps"`); audit table §3 confirms identical semantics today |
| `removeDuplicates` is called on a list that has NOT been ranked yet (iOS LS:103, Android LS:208) | Rust `process_candidates` does its own merge-dedup → rank → display-dedup pipeline internally given the raw merged list; matches platform shape |
| Stale `RustEngineBridge` import on Android `CandidateProcessor.kt` (line 6) | Removed in commit 7 cleanup |
| **FFI error path graceful degradation** — when `RustEngineBridge.processCandidatesDetailed` fails to round-trip through Rust (lexicon dispatch returns `nil`, engine returns non-OK, decode error, or the response carries no `process_candidates_result` payload), naively returning `raw` would skip dedup + display-dedup and surface duplicate candidates. Pre-migration `LexiconService.search` always dedup'd. | Both bridges add a private `fallbackRanked(raw, tpsDedupEnabled)` helper that calls platform `CandidateProcessor.removeDuplicates` + (TPS-gated) `removeDisplayDuplicates`. Score-sort is skipped (bridge owns user-frequency lookups), but the input arrives pre-sorted by `lengthScore` from `LexiconService.searchWithTrie`, so the order remains "reasonable". This is defense-in-depth — the production hot path is the Rust pipeline; the fallback only fires on engine bugs (which `recordFailure` surfaces via diagnostics). Same retained platform helpers as the iOS cold-start fallback row above; no extra code added beyond the two `fallbackRanked` private functions |
| **iOS cold-start fallback** — when the user-frequency SQLite DB has not yet completed its async connection, iOS `LexiconService.search` short-circuits past `RustEngineBridge.processCandidates` and invokes platform `CandidateProcessor.removeDuplicates` + (TPS-gated) `removeDisplayDuplicates` directly. **Intentional exception** to the "production swap goes through Rust" rule: this preserves the legacy "merged-order on cold-start" behavior so custom-dictionary entries continue to surface ahead of system candidates until the freq DB warms up. Routing through Rust would force a score-sort over an empty freq map and reorder candidates in the cold-start window. Android has **no equivalent fallback** because its `UserFrequencyService.frequencyDataBatch` is synchronous-from-the-caller's-view (always returns even on first call, possibly with empty entries) — there is no `isConnected` gate to short-circuit on. iOS retains `CandidateProcessor.removeDuplicates`/`removeDisplayDuplicates` solely to support this branch | Documented at the call-site (`LexiconService.swift::processCandidates`) and at `RustEngineBridge.swift::processCandidates`. If a future slice introduces a "dedup-only" Rust mode the fallback can be retired; until then the platform copies stay |
| Single PR ~10 files modified plus xcframework + jniLibs may exceed review attention | 8 commits provide clear review slices; Codex sandwich (pre-impl already APPROVE WITH FIXES; post-impl scheduled) + `/simplify` |

## 9. Out of scope (no version commitment, per `feedback_no_future_planning.md`)

- `isHanzi` Rust port (kept platform; only 2 iOS callers, Android already inlines)
- `capitalize` / `startsWithRomanLetter` Rust port (would change Android behavior — refactor-only rule)
- Score-debug logging Rust-side (zero-allocation principle on hot path)
- NextWord predictions slice
- Composing state machine slice
- Lexicon DB / Trie / SQLite reads slice
- Moving Android `src/test/` to `androidTest/` (instrumented) so the JVM-load workaround can be retired (separate test-infra concern)
