# Case-Transform Slice — Audit

**Status:** Implemented. Codex sandwich CLEARED (pre-impl + mid-slice). Plan: `case-transform-slice-plan.md`.

**Auto mode:** Owner authorized joint Claude+Codex decisions on low-stakes scope items per `feedback_codex_review_sandwich.md` + `feedback_review_before_impl.md`.

## 1. Domain definition

The slice migrates the **case-transformation subsystem** to the Rust shared core:

- **Per-char helpers** (single grapheme cluster): `uppercase_tone_char` (first-char upper, sentence case), `full_uppercase_tone_string` (all chars upper, caps lock), `lowercase_tone_char`. Mode-aware via POJ + TL tone-letter mapping tables.
- **Per-string compound transforms**: `transform_input_case` (engine-input case pipeline), `capitalize_candidate` (auto-cap gating), `transform_suggestion` (per-word matchCase + remainder + nasal post-process).
- **Nasal marker case adjust** (`adjust_nasal_marker_case`): re-homed from `case_adjust.rs` into `case_transform.rs` for cohesion. Exposed as a public Rust API; called in-band by `Method::NormalizeTone` and as post-process inside `transform_suggestion`. NOT exposed as a standalone proto method (Codex mid-slice review — no platform call site needs it standalone).

Out of scope: the broader candidate-processing pipeline (`CandidateProcessor.capitalize`, `startsWithRomanLetter`) remains a 2-method platform shim (28 LOC iOS) over `RustEngineBridge.capitalizeCandidate`.

## 2. Inventory — iOS

Path roots: `ios/Sources/TaigiKeyboard/Input/`, `ios/Sources/TaigiKeyboard/Autocomplete/Services/`, `ios/Sources/TaigiKeyboard/Lexicon/Utils/`, `ios/Sources/TaigiKeyboard/Engine/`. LOC counts measured 2026-05-04.

### 2.1 Migrated to Rust + platform deleted

| File | Pre LOC | Post | v3.5.x disposition |
|---|---:|---|---|
| `Input/CaseTransformer.swift` | 111 | **DELETED** (commit 8) | `LetterCase` enum + `transformForInput` + `capitalizeCandidate` rewired to `RustEngineBridge.transformInputCase` / `.capitalizeCandidate` (commit 7). |
| `Input/ToneUtilities.swift` | 46 | **DELETED** (commit 8) | All 3 helpers (`uppercaseToneLetter` / `lowercaseToneLetter` / `adjustNasalMarkerCase`) replaced by bridge calls or in-band engine post-process. |
| `Autocomplete/Services/SuggestionCaseTransformer.swift` | 227 | **56** (commit 7) | Body shrunk to thin per-word bridge keeping the `additionalInfo` skip filter; algorithm body deleted. |
| `TaigiKeyboardTests/CaseTransformerTests.swift` | 392 | **DELETED** (commit 8) | Algorithm tests replaced by Rust crate tests (29 golden + 27 unit + 9 dispatch = 65 case tests). |

### 2.2 Modified call sites (kept on platform)

| File | Change |
|---|---|
| `Actions/ActionHandler+KeyActions.swift:23` | `CaseTransformer.transformForInput` → `RustEngineBridge.transformInputCase`. Drops `isAutoCapitalizationEnabled` flag (legacy iOS impl ignored it). |
| `Actions/KeyboardCaseAdapter.swift` | `Keyboard.KeyboardCase.asLetterCase` return type changed from engine `LetterCase` (deleted) to `RustEngineBridge.CaseTransformLetterCase`. |
| `Lexicon/Utils/CandidateProcessor.swift` | `capitalize(...)` body becomes a one-liner: `RustEngineBridge.capitalizeCandidate(...)`. File shrinks 45 → 28 LOC. |
| `KeyboardExtension/TaigiKeyboardView.swift:80` | No change — calls `SuggestionCaseTransformer.transform(...)` which is now a thin wrapper. |

### 2.3 Added

| File | LOC |
|---|---:|
| `Engine/RustEngineBridge+CaseTransform.swift` | 125 |
| `Engine/RustEngineBridge.swift` (`caseDispatch` helper) | +47 |

**Net iOS delta:** −549 LOC (deleted) + −188 LOC (shrunk bodies) + 200 LOC (added bridge + helper) = **−537 LOC**.

## 3. Inventory — Android

Path roots: `android/app/src/main/java/com/siansiansu/taigikeyboard/`. LOC counts measured 2026-05-04.

### 3.1 Migrated to Rust + platform deleted

| File | Pre LOC | Post | v3.5.x disposition |
|---|---:|---|---|
| `ime/dictionary/ToneUtilities.kt` | 260 | **DELETED** (commit 10) | POJ + TL mapping tables (102 entries each direction) + nasal adjust + per-char helpers. Algorithm-of-record now `engine/phonetics/src/case_tables.rs`. |
| `ime/dictionary/SuggestionCaseTransformer.kt` | 257 | **50** (commit 9) | Body shrunk to thin per-word bridge keeping the numeric-id skip rules. |
| `app/src/test/.../dictionary/SuggestionCaseTransformerTest.kt` | 260 | **DELETED** (commit 10) | Algorithm tests replaced by Rust crate tests (same 65 cases as iOS — single source-of-truth). |

### 3.2 Modified call sites (kept on platform)

| File | Change |
|---|---|
| `ime/text/key/KeyView.kt:295-304` | Three separate `ToneUtilities` calls collapsed into single `KeyLabelCaseCache.getOrCompute(baseLabel, mode, caps, capsLock)` call. R3 mitigation. |
| `ime/text/TextInputManager.kt:846-850` | Three separate `ToneUtilities` calls collapsed into single `CaseTransformBridge.transformInputCase(text, LetterCase.from(caps, capsLock), mode)` call. Per-keystroke (not per-frame) — no cache. |
| `ime/text/smartbar/SmartbarManager.kt:404` | No change — calls `SuggestionCaseTransformer.transform(...)` which is now a thin wrapper. |

### 3.3 Added

| File | LOC |
|---|---:|
| `engine/CaseTransformBridge.kt` | 190 |
| `ime/text/key/KeyLabelCaseCache.kt` | 70 |

**Net Android delta:** −520 LOC (deleted) + −207 LOC (shrunk bodies) + 260 LOC (added bridge + cache) = **−467 LOC**.

## 4. Combined platform deletion

iOS −537 LOC + Android −467 LOC = **~−1004 LOC** of duplicated platform case-transform code removed in this slice.

Single Rust source-of-truth lives at `engine/phonetics/src/case_transform.rs` (~370 LOC) + `case_tables.rs` (~155 LOC) + `engine/dispatch/src/case.rs` (~205 LOC including tests) = ~730 LOC. **Net codebase delta: ~−270 LOC for equivalent functionality**, with the added benefit of cross-platform behavioral parity by construction.

## 5. Cross-platform divergences resolved

See `case-transform-slice-plan.md` §3 for the full table. Summary:

| # | Divergence | Pre-slice state | Post-slice state |
|---|---|---|---|
| 1 | Combining-mark round-trip | iOS used stdlib `String.uppercased()` (Unicode quirks); Android used explicit tables | Both bridge to Rust; tables identical (Android-canonical) |
| 2 | `uppercaseToneLetter` semantics (all-chars vs first-only) | iOS overloaded one function; Android had two | Rust splits into `uppercase_tone_char` + `full_uppercase_tone_string`; bridges disambiguate |
| 3 | Suggestion skip rules (flag-based vs id-based) | iOS `additionalInfo` flags; Android `id` markers | Stay platform-side — each platform's bridge filters before calling `transform_suggestion` |
| 4 | Two `capitalizeFirstLetter` functions in iOS Swift | Easy to confuse; nearly caused a regression | Rust split into `capitalize_first_letter` + `capitalize_first_letter_in_text`; golden test pins the suggestion-path semantics |

## 6. Codex sandwich gates

### Pre-impl (sandwich gate 1) — 2026-05-03

**Verdict:** Conditional-approve. 4 blockers raised:

1. Use `case.proto` (not `lexicon.proto` extension) ✅
2. Move `adjust_nasal_marker_case` from `case_adjust.rs` into `case_transform.rs` ✅
3. KeyView FFI mitigation must be in place before Android rewiring ✅ (KeyLabelCaseCache LRU)
4. Insert mid-slice Codex gate between Rust impl and platform rewiring ✅

### Mid-slice (sandwich gate 1.5) — 2026-05-03

**Verdict:** AMEND before rewiring. 1 must-fix + 5 defer items:

| # | Item | Action |
|---|---|---|
| Must-fix | `Method::AdjustNasalMarkerCase` standalone proto method has zero platform callers | **Removed** in commit 6.5; tag 30 reserved |
| Defer | `ch.to_string()` cleanup (5 sites) | Done in 6.5 |
| Defer | 4 missing Android golden cases (TL `ôo`, digits-not-counted, empty roman, capsLock dominance) | Done in 6.5 |
| Defer | Round-trip property test for table entries | Done in 6.5 (POJ + TL) |
| Defer | File split for compound suggestion helpers (370 LOC) | Skipped — under 500 hard cap |
| Defer | Hotfix commit relabeling | Skipped — already explicitly labeled `fix(engine):` |

### Post-impl (sandwich gate 2) — runs after this PR opens

Codex review on full diff — must clear before merge per `feedback_codex_review_sandwich`.

## 7. Test coverage

| Suite | Count | Location |
|---|---:|---|
| `case_transform.rs::tests` + `case_tables.rs::tests` (inline unit) | 27 | `engine/phonetics/src/{case_transform,case_tables}.rs` |
| `case_transform_golden.rs` (integration) | 29 | `engine/phonetics/tests/` — ports iOS+Android golden tables verbatim |
| `dispatch::case::tests` (round-trip) | 10 | `engine/dispatch/src/case.rs` |
| **Total Rust case tests** | **66** | |
| iOS bridge round-trip tests | TBD | post-merge — bridge surface tests verify FFI plumbing only |
| Android bridge round-trip tests | TBD | post-merge — same |

## 8. Risk register (closed)

| # | Risk | Mitigation | Status |
|---|---|---|---|
| R1 | Combining-mark Rust impl correctness | All 102 Android table entries ported verbatim + POJ/TL round-trip property tests | Closed |
| R2 | Per-word FFI in autocomplete hot path | Single-hop per-word; SuggestionCaseTransformer runs ≤30× per keystroke (within ranking-slice budget) | Closed (verify in dogfood) |
| R3 | KeyView render-path FFI cost (per-key per-frame) | KeyLabelCaseCache LRU; cache key carries state — no stale results | Closed (verify in dogfood) |
| R4 | Naming clarity (`LetterCase` enum vs Android `caps`+`capsLock` boolean pair) | Bridge `LetterCase.from(caps, capsLock)` adapter | Closed |
| R5 | iOS Swift `capitalizeFirstLetter` name collision | Rust split into 2 named helpers; golden test `suggestion_caps_capitalize_next_letter_after_typed` pins | Closed |

## 9. Behavioral invariants

(To be added to `docs/architecture/behavioral-invariants.md` as part of post-merge follow-up.)

- `INVARIANT_CASE_TONE_TABLE_ROUND_TRIP` — for every (mode, lowercase_entry) pair, `lowercase(uppercase(entry, mode), mode) == entry`
- `INVARIANT_CASE_NASAL_MARKER_FOLLOWS_PRECEDING_LETTER` — `ⁿ` after lowercase letter, `ᴺ` after uppercase letter
- `INVARIANT_CASE_SUGGESTION_PRESERVES_TYPED_CASE` — typed-portion of candidate matches case of `composing_text` letter-by-letter
- `INVARIANT_CASE_SUGGESTION_SKIP_RULES_PLATFORM_OWNED` — Rust never sees skip filters; platform pre-filters before `transform_suggestion`

## 10. Dogfood matrix (R1-R3, no R4 7-day soak per `feedback_perf_gate`)

### R1 — Sanity
- iOS: build keyboard extension; type "g", confirm "góa" candidate appears with correct case
- Android: `./gradlew assembleDebug`; same
- Both: caps lock typing "GOA" → "GÓA" candidates upper-case correctly

### R2 — Case-transform risk surfaces
- Sentence case (Uppercased): type "g", expect "Goa" candidate
- Caps lock (CapsLocked): toggle CapsLock, type "tsh", expect "TSH"
- Mixed-case nasal marker: type input that surfaces a candidate with `ⁿ` after uppercase letter, confirm it becomes `ᴺ`
- Combining marks: candidates containing `a̍`, `o̍͘`, `n̂` round-trip case correctly
- Composing text passthrough: composing-text candidate (`isComposingText` / `id == 0`) NOT case-transformed
- NextWord passthrough: NextWord candidate (`isNextWord` / `id < 0 && id != -2`) NOT case-transformed
- Custom dict (`id == -2` Android): IS case-transformed

### R3 — Parity edges
- iOS + Android side-by-side: type identical sequence, confirm visible candidates match character-for-character
- KeyView cache (Android): rapid layout switches, caps-toggle spam — confirm no stale label visible
- KeyView cache (Android): memory check — cache doesn't grow unbounded after 100+ layout switches (capped at 256 entries via LinkedHashMap LRU)

## 11. Next-slice handoff

The case-transform slice closes the last "non-trivial pure-logic" shared-core candidate (per `shared-core-readiness.md`). After this slice:

- Remaining shared-core candidates are DTOs, infrastructure (`LoggerBackend`, `EngineSettings`), or already-bridged thin wrappers
- Per `feedback_no_future_planning`: NO version commitment for the next slice. Next session per `project_post_v357_polish_phase` is bug-hunt + code review + cleanup, not new slices.
