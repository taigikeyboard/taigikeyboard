# Case-Transform Slice — Plan

**Status:** Implemented. Codex sandwich CLEARED (pre-impl + mid-slice; post-impl gate runs after this PR opens). Audit: `case-transform-slice-audit.md`.

**Branch:** `phase4b/case-transform-slice` off main `b29da91`. Single PR per `feedback_round_hygiene.md`.

**PR title prefix:** `slice:` (full subsystem migration to Rust shared core; cross-platform behavior preserved per the canonical-form decision in §3).

## 1. Scope

The slice migrates the **case-transformation subsystem** to the Rust shared core. Three iOS files (`Input/CaseTransformer.swift`, `Input/ToneUtilities.swift`, `Autocomplete/Services/SuggestionCaseTransformer.swift`) and three Android files (`dictionary/ToneUtilities.kt`, `dictionary/SuggestionCaseTransformer.kt`, plus inline call sites in `text/key/KeyView.kt` and `text/TextInputManager.kt`) collapse into one Rust source-of-truth at `engine/phonetics/src/case_transform.rs` + `case_tables.rs`.

**Codex pre-impl decision (Option C, full subsystem):** Path G + swap-preserves-pipeline + design-goals-4 cross-validate that an Option A (CaseTransformer-only) or Option B (CaseTransformer + ToneUtilities helpers) leaves half-mirrors and reproduces the D9.4 regression pattern.

## 2. Rust crate layout

Module additions to existing `engine/phonetics/`. No new crate (per Codex pre-impl review — case logic is conceptually phonetics-aware string transformation).

```
engine/phonetics/
  src/
    lib.rs              # +2 lines: pub mod case_transform; mod case_tables;
                        # -1 line: drop `mod case_adjust;`
    api.rs              # +1 import: case_transform::adjust_nasal_marker_case
                        # -1 import: case_adjust::adjust_nasal_marker_case
    case_transform.rs   # NEW — 559 LOC incl. tests (impl ~370 LOC under 500 hard cap)
    case_tables.rs      # NEW — 230 LOC incl. tests (POJ + TL maps + 3 inline tests)
    case_adjust.rs      # DELETED — content consolidated into case_transform.rs
    ...                 # other files unchanged
  tests/
    case_transform_golden.rs  # NEW — 522 LOC, ports iOS+Android golden cases
```

## 3. Cross-platform behavioral resolution (Rust = canonical)

| # | Divergence | Resolution |
|---|---|---|
| 1 | **Combining-mark round-trip**: Android keeps explicit POJ+TL mapping tables for `a̍`, `o̍͘`, `n̂`, `m̀`, etc. because Kotlin stdlib's `String.uppercase()`/`lowercase()` doesn't round-trip them. iOS relies on `String.uppercased()` directly which has Unicode quirks. | **Rust adopts Android's explicit-table form** (more strict, no Unicode quirks). 102 entries each direction; reverse maps lazy-init from forward. |
| 2 | **`uppercaseToneLetter` semantics**: iOS `ToneUtilities.uppercaseToneLetter("tsh", mode)` returns "TSH" (all chars upper via `String.uppercased()`); Android equivalent returns "Tsh" (first char only via `replaceFirstChar`); Android also has `fullUppercaseToneLetter` for all-chars-upper. | **Rust splits into two named functions**: `uppercase_tone_char` (single grapheme upper) + `full_uppercase_tone_string` (all chars upper). Bridge call sites disambiguate. |
| 3 | **Suggestion skip rules**: iOS uses `additionalInfo["isComposingText"]` / `["isNextWord"]` flags; Android uses `id == 0` (composing) and `id < 0 && id != -2` (NextWord/English) numeric markers. | **Stay platform-side.** Rust never sees them — only transform-eligible items reach `transform_suggestion(...)`. |
| 4 | **Two `capitalizeFirstLetter` functions in iOS Swift with different semantics**: `CaseTransformer.capitalizeFirstLetter` uppercases the literal first char + lowercases rest (engine input pipeline); `SuggestionCaseTransformer.capitalizeFirstLetter` finds the first LETTER (skip leading non-letters), uppercases it, lowercases subsequent letters (per-suggestion remainder). | **Rust splits into two private helpers** in `case_transform.rs`: `capitalize_first_letter` (engine input path) + `capitalize_first_letter_in_text` (suggestion path). Caught by golden test `suggestion_caps_capitalize_next_letter_after_typed`. |

## 4. Proto API shape

New `engine/protos/proto/case.proto` — own file, not a `lexicon.proto` extension. SRP: case isn't a lexicon concern.

`CaseRequest` oneof has 6 method variants (tag 30 reserved for the dropped standalone `AdjustNasalMarkerCase` per Codex mid-slice — `transform_suggestion` already runs nasal adjust as post-process and `Method::NormalizeTone` calls it in-band, so no standalone bridge call is needed):

| Tag | Variant | Replaces |
|---:|---|---|
| 10 | `UppercaseToneChar` | `ToneUtilities.uppercaseToneLetter` |
| 11 | `FullUppercaseToneString` | Android `ToneUtilities.fullUppercaseToneLetter` |
| 12 | `LowercaseToneChar` | `ToneUtilities.lowercaseToneLetter` |
| 20 | `TransformInputCase` | `CaseTransformer.transformForInput` |
| 21 | `CapitalizeCandidate` | `CaseTransformer.capitalizeCandidate` |
| 22 | `TransformSuggestion` | `SuggestionCaseTransformer.transform` (per-word body) |

`CaseStringResult { string output = 1 }` is locally defined (not re-exported from `phonetics.proto::StringResult`) to avoid cross-module proto coupling.

`LetterCase` enum carries `LETTER_CASE_UNSPECIFIED = 0` (proto3 best practice — engine maps Unspecified to `Lowercased` as safe-fallback).

`envelope.proto`: `CMD_CASE = 5`, payload tag 14 on Request/Response. Field name `case_transform` (not `case` — Swift keyword would force backtick escaping).

## 5. Platform bridge shape

### iOS (`ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+CaseTransform.swift`, ~125 LOC)

5 public methods + `CaseTransformLetterCase` enum mirroring proto. All converge on a single private `caseStringDispatch(method, op, mode, fallback)` helper. Mode forwarded via envelope `AppConfig.input_mode`; case-transform is independent of POJ doubletap so `ToneToggles()` defaults are fine.

`KeyboardCaseAdapter.swift` updated: `Keyboard.KeyboardCase.asLetterCase` returns `RustEngineBridge.CaseTransformLetterCase` instead of the deleted engine `LetterCase`.

### Android (`android/.../engine/CaseTransformBridge.kt`, ~190 LOC)

Top-level object mirroring `LexiconBridge` pattern. 5 public methods + `LetterCase` enum with `from(caps, capsLock)` adapter for Android's existing boolean-pair convention. Single private `dispatch(caseRequest, mode)` helper using `RustEngineBridge.dispatchRaw` for the JNI roundtrip.

### R3 mitigation — KeyView render path (`android/.../ime/text/key/KeyLabelCaseCache.kt`, ~70 LOC)

`KeyView.getComputedLetter` runs once per visible key per redraw frame. Naive per-call FFI = O(visible_keys × frames) JNI hops. **Solution**: process-wide LRU `LinkedHashMap<Key, String>` keyed on `(baseLabel, mode, caps, capsLock)`, cap 256 entries, access-order eviction. UI-thread only, unsynchronized. Cache key carries current state — stale results impossible.

`TextInputManager.kt:846-850` (per-keystroke, not per-frame) does NOT need the cache — direct `CaseTransformBridge.transformInputCase(...)` call.

## 6. Commit plan (12 commits incl. mid-slice amend + sandwich gates)

| # | Commit | Scope |
|---|---|---|
| 1 | `fix(engine): mark ClassificationResult/classifyInput internal` | Hotfix bundle from v3.5.7 |
| 2 | `feat(proto): add case.proto for case-transform dispatch` | Proto + envelope wiring + build.rs + dispatch stub |
| 3 | `feat(phonetics): add case_transform + case_tables modules` | Rust impl, drop case_adjust.rs |
| 4 | `test(phonetics): port iOS/Android golden case tests + fix capitalize-first divergence` | Golden tests + bug-fix |
| 5 | `feat(dispatch): wire case_transform proto handlers` | Real handler replaces commit-2 stub |
| 6 | **🛑 MID-SLICE Codex gate** | Cleared — see audit §6 |
| 6.5 | `fix(phonetics+proto): address Codex mid-slice review findings` | 1 must-fix + 5 defer items |
| 7 | `feat(ios): RustEngineBridge+CaseTransform + rewire callers` | iOS bridge + ActionHandler + CandidateProcessor + SuggestionCaseTransformer shrink + proto field rename to `case_transform` |
| 8 | `chore(ios): delete CaseTransformer + ToneUtilities + algorithm tests` | Path G platform-mirror deletion (−549 LOC) |
| 9 | `feat(android): CaseTransformBridge + KeyLabelCaseCache + rewire callers` | Android bridge + R3 cache + KeyView/TextInputManager/SmartbarManager rewire |
| 10 | `chore(android): delete ToneUtilities + SuggestionCaseTransformer test` | Path G platform-mirror deletion (−520 LOC) |
| 11 | `docs(engine): land case-transform slice plan + audit + rust-core-proto + readiness mark` | This commit |
| 12 | **🛑 POST-IMPL Codex gate** | Sandwich gate 2 — runs after PR opens |

## 7. User actions required (manual — flagged in PR description)

- iOS: pbxproj edit to remove deleted files (`CaseTransformer.swift`, `ToneUtilities.swift`, `CaseTransformerTests.swift`) and add `RustEngineBridge+CaseTransform.swift` (CLAUDE.md rule #4 — pbxproj is user-only)
- iOS: rebuild xcframework so generated `Taigi_Engine_CaseRequest`/`Taigi_Engine_CaseResponse`/`Taigi_Engine_LetterCase` types appear in `ios/Sources/TaigiKeyboard/Engine/Generated/`
- Android: `./gradlew assembleDebug` — protobuf gradle plugin auto-regenerates from the updated `case.proto`
- Real-device dogfood per the slice audit R1-R3 matrix
- `Cargo.lock` regen via `cargo build` from `engine/`

## 8. Verification

See `case-transform-slice-audit.md` §10 for the full dogfood matrix.

Quick check — workspace Rust tests:
```sh
cd engine
cargo test --workspace      # 89 case-related + ~140 other tests pass
cargo clippy --workspace --all-targets
cargo fmt --check
```
