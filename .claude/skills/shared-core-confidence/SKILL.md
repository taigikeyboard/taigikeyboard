---
name: shared-core-confidence
description: Score the project's readiness for Rust shared-core extraction across 9 weighted dimensions. Produces a dated report at docs/engine/shared-core-confidence-<YYYY-MM-DD>.md with composite score, per-dimension breakdown, categorical blockers, and gap analysis. Used at the Phase II end hybrid decision gate (rules/cross-platform-alignment.md §4) — composite ≥ 95% AND D9 FFI POC pass AND zero critical blocker required to open Phase III.
disable-model-invocation: true
---

# Shared-Core Confidence

Score iOS + Android readiness for Rust shared-core extraction. Pure measurement — no auto-fix. Fixes go in separate rounds per `feedback_round_hygiene.md`.

## Arguments

- **No argument** = full cross-platform scoring (default; emits gate verdict).
- `--ios-only` = diagnostic mode, iOS dimensions only. **Does not emit gate PASS/FAIL.** D2/D3/D4/D9 are inherently cross-platform.
- `--android-only` = diagnostic mode, Android dimensions only. **Does not emit gate PASS/FAIL.** Same reason.
- `--xcresult <path>` = optional path to a user-generated `.xcresult` bundle for D8 iOS coverage. If absent, iOS half of D8 = "not measured" (scored 0).
- `--diff <path>` = optional path to a previous report for the "Delta vs previous" table.

## Output

`docs/engine/shared-core-confidence-<YYYY-MM-DD>.md` (UTC date). If a same-date file already exists, append `-rN` suffix. Report shape in §Report template below.

## Steps

### 1. Read inputs

Read in parallel:

- `rules/cross-platform-alignment.md` — gate definition (§4) + shared-core-candidate constraint (§1c)
- `rules/ios-architecture.md` — iOS engine purity rules
- `rules/android-guidelines.md` §1 — Android shared-core candidate forbidden imports
- `rules/rust-best-practices.md` — Rust portability constraints
- `docs/engine/shared-core-readiness.md` — iOS candidate roster + criteria
- `docs/architecture/android-exemplar.md` §4 + §5 — Android candidate roster + gate target
- `docs/architecture/behavioral-invariants.md` — INVARIANT_* labels (D3, D5)
- `docs/architecture/g9-coverage-matrix.md` + `docs/architecture/android-g9-coverage-matrix.md` — top-10 coverage targets (D8)
- `docs/architecture/android-state-audit.md` §9 — Phase II gating signals
- `docs/engine/ffi-safety.md` + `docs/engine/rust-core-proto.md` + `docs/architecture/data-artifacts-portability.md` — boundary doc set (D7)
- Existing `docs/engine/shared-core-confidence-*.md` — for delta table if `--diff` provided

### 2. Build the candidate roster

Enumerate `Shared-Core Candidate` markers per platform:

```bash
# iOS
grep -rln "// MARK: - Shared-Core Candidate" ios/Sources/

# Android
grep -rln "// region Shared-Core Candidate" android/app/src/main/
```

Record counts as `iOS_count` and `Android_count`.

### 3. Score the 9 dimensions

For each dimension, compute a numeric subscore and capture pass/fail items.

#### D1 — iOS purity (10%)

For every iOS candidate file, scan for forbidden patterns. POSIX `grep -E` does NOT support PCRE `(?!...)` negative lookahead, so use a two-pass pipeline: first keep only lines whose first non-space character is NOT `/` (comment-only lines drop out), then ERE-match for forbidden patterns:

```bash
grep -nE "^[[:space:]]*[^/[:space:]]" "$file" | \
  grep -E "\b(import (UIKit|SwiftUI|KeyboardKit|Combine|OSLog))\b|\b(SharedSettings|KeyboardSettings\.store)\b|\.shared\b|\b(NotificationCenter|DispatchQueue|OperationQueue|Bundle\.main|FileManager)\b|@MainActor[[:space:]]+(class|struct|actor|protocol)"
```

Note on `Timer`: omitted from the ERE because `TimerInterval` would false-positive without lookahead. If a Swift candidate uses `Timer.scheduledTimer` it will surface in `DispatchQueue` / `OperationQueue` review or in dogfooding; not worth a regex fight.

A file is **dirty** if any forbidden pattern matches. Score:

```
D1 = (clean_iOS_files / iOS_count) × 10
```

Record list of dirty files + matched patterns under "D1 violations".

#### D2 — Android parity (15%)

Use the **logical-surface manifest** below (NOT basename matching — Android intentionally folds/splits some iOS files). For each manifest row, check that the iOS surface AND its Android counterpart both carry the `Shared-Core Candidate` marker. The manifest is sourced from `docs/engine/shared-core-readiness.md` §Roster + `docs/architecture/android-exemplar.md` §4–5.

| Logical surface | iOS file(s) | Android file |
|---|---|---|
| Phonetics facade | `Phonetics/TaigiPhonetics.swift` | `ime/dictionary/TaigiPhonetics.kt` (absorbs SyllableParser/TLFormatter/POJFormatter/PhoneticsConverter/RomanizationConverter) |
| Phonetics tables | `Phonetics/Tables/PhoneticsTables.swift` | folded into `TaigiPhonetics.kt` |
| Tone converter | `Phonetics/ToneConverter.swift` | `ime/dictionary/ToneConverter.kt` |
| Tone converter models | (inline) | `ime/dictionary/ToneConverterModels.kt` |
| Tone restoration | `Phonetics/ToneRestoration.swift` | `ime/dictionary/ToneRestoration.kt` |
| Tone utilities | `Phonetics/ToneUtilities.swift` | `ime/dictionary/ToneUtilities.kt` |
| TPS converter | `Input/TPS/TPSConverter.swift` | `ime/dictionary/TPSConverter.kt` (absorbs TPSTables/TPSInputAdjuster/TPSToTL/TLToTPS) |
| Input normalizer | `Lexicon/Trie/InputNormalizer.swift` | `ime/dictionary/InputNormalizer.kt` |
| Character input pipeline | `Input/CharacterInputPipeline.swift` | (Android folds into `TextInputManager` — exempt) |
| Case transformer | `Input/CaseTransformer.swift` | `ime/dictionary/SuggestionCaseTransformer.kt` |
| Composing state | `Input/Composing/ComposingState.swift` | `ime/text/composing/ComposingState.kt` |
| Composing transition | `Input/Composing/ComposingTransition.swift` | `ime/text/composing/ComposingTransition.kt` |
| Autocomplete providers | `Autocomplete/Services/AutocompleteProviders.swift` | (Android no equivalent — KeyboardKit-shaped provider abstraction; deferred) |
| Autocomplete classifier | `Autocomplete/Services/AutocompleteInputClassifier.swift` | `ime/text/composing/AutocompleteInputClassifier.kt` |
| Autocomplete context booster | `NextWord/AutocompleteContextBooster.swift` | `ime/text/composing/AutocompleteContextBooster.kt` |
| Engine settings | `Settings/EngineSettings.swift` | `ime/core/settings/EngineSettings.kt` |
| Engine settings provider | `Settings/EngineSettingsProvider.swift` | `ime/core/settings/EngineSettingsProvider.kt` |
| Tone toggles | `Settings/ToneToggles.swift` | `ime/core/settings/ToneToggles.kt` |
| Input mode | `Settings/InputMode.swift` | folded into `EngineSettings.inputMode: String` (Android divergence — see §A2 audit) |
| Logger backend | `Common/LoggerBackend.swift` | `ime/core/logging/LoggerBackend.kt` |
| Outcome | (iOS uses throws) | `ime/core/Outcome.kt` |
| Taigi word | `Lexicon/Models/TaigiWord.swift` | `ime/dictionary/TaigiWord.kt` |
| Input type | `Lexicon/Models/InputType.swift` | `ime/dictionary/InputType.kt` |
| Dictionary source | `Lexicon/Models/DictionarySource.swift` | `ime/dictionary/DictionarySource.kt` |
| Dictionary constants | `Lexicon/Models/LexiconConstants.swift` | `ime/dictionary/DictionaryConstants.kt` |
| Dictionary error | `Lexicon/Models/LexiconError.swift` | `ime/dictionary/DictionaryError.kt` |
| Dictionary search result | (inline) | `ime/dictionary/DictionarySearchResult.kt` |
| Custom dictionary entry | `Lexicon/Models/CustomDictionaryEntry.swift` | (Android in-service, candidate marker pending — see §10 N/A) |
| Custom dictionary derivation | `Lexicon/Database/CustomDictionaryDerivation.swift` | `ime/dictionary/CustomDictionaryDerivation.kt` |
| Frequency data | `Lexicon/Models/FrequencyData.swift` | `ime/dictionary/FrequencyData.kt` |
| Enabled dictionaries | (inline) | `ime/dictionary/EnabledDictionaries.kt` |
| Taigi unicode | `Lexicon/Utils/TaigiUnicode.swift` | `ime/dictionary/TaigiUnicode.kt` |
| Candidate processor | `Lexicon/Utils/CandidateProcessor.swift` | `ime/dictionary/CandidateProcessor.kt` |
| External lookup URL builder | (iOS inline) | `ime/dictionary/ExternalLookupURLBuilder.kt` |
| NextWord engine prediction | `NextWord/EnginePrediction.swift` | `ime/core/nextword/EnginePrediction.kt` |
| NextWord raw prediction | `NextWord/RawNextWordPrediction.swift` | `ime/core/nextword/RawNextWordPrediction.kt` |
| NextWord engine | `NextWord/NextWordEngine.swift` | `ime/core/nextword/NextWordEngine.kt` |
| NextWord outcome | `NextWord/NextWordOutcome.swift` | `ime/core/nextword/NextWordOutcome.kt` |
| NextWord scorer | `NextWord/NextWordScorer.swift` | `ime/core/nextword/NextWordScorer.kt` |

For each manifest row:
- **paired** = both sides marked (or both legitimately folded/exempt per the row note)
- **unpaired** = one side marked, the other missing or unmarked
- **deferred** = legitimately absent on one side (note in column 3 — e.g. "Android in-service")

Score:

```
D2 = (paired_rows / scorable_rows) × 15
```
where `scorable_rows` excludes "deferred" rows.

Record unpaired rows under "D2 gaps".

#### D3 — Behavior alignment (20%)

Extract every `INVARIANT_*` label from `docs/architecture/behavioral-invariants.md`:

```bash
grep -oE "INVARIANT_[a-z_0-9]+" docs/architecture/behavioral-invariants.md | sort -u
```

For each label, check matching test exists on **both** platforms:

```bash
# iOS
grep -rln "INVARIANT_<label>" ios/TaigiKeyboardTests/

# Android
grep -rln "INVARIANT_<label>" android/app/src/test/
```

Status per label:
- **both** = test on iOS AND Android
- **N/A-satisfied** = doc explicitly marks one platform N/A AND the other platform has the test (e.g. `INVARIANT_composing_external_region_clear_discards_state` is iOS-N/A per `behavioral-invariants.md` line 360, satisfied because Android has it)
- **one** = test on only one platform (other platform missing, no N/A exemption)
- **neither** = no test anywhere

Score:

```
satisfied = both + N/A-satisfied
D3 = (satisfied / total_labels) × 20
```

`total_labels` is the full count from `behavioral-invariants.md` — N/A labels stay in the denominator (they are still real invariants; they just don't require a test on the exempt platform).

#### D4 — Data model parity (10%)

For each shared data type, compare iOS struct/enum fields to Android data class/enum fields using this **fixed type map**:

| iOS Swift | Android Kotlin |
|---|---|
| `String` | `String` |
| `String?` | `String?` |
| `Int` / `Int64` | `Long` |
| `Int32` | `Int` |
| `Bool` | `Boolean` |
| `Double` | `Double` |
| `[X]` | `List<X>` |
| `[X: Y]` | `Map<X, Y>` |
| `enum case foo` | `enum class FOO` or `data class Foo` (semantic match) |
| `let` (immutable) | `val` |
| `var` (mutable) | `var` |

Types to compare (from manifest §D2 plus inline value types):

- `TaigiWord` / `InputType` / `DictionarySource` / `FrequencyData` / `EngineSettings` / `ToneToggles` / `ComposingState` (apply input type) / `ComposingTransition` / `RawNextWordPrediction` / `EnginePrediction` / `NextWordOutcome` (sealed cases) / `LexiconError` ↔ `DictionaryError`

For each type, count **semantic field/case mismatches**:
- Field present on one side, absent on the other → +1
- Field type mismatch per the type map → +1
- Enum case present on one side, absent on the other → +1
- Mutability mismatch (`val` vs `var`) → +1

Computed properties + private fields are out of scope (they don't cross FFI). Nullable vs non-nullable IS a mismatch.

Score:

```
D4 = max(0, 10 - total_mismatches × 0.5)
```

Record mismatch list under "D4 type parity gaps".

#### D5 — Cross-platform invariant comments (10%)

This dimension validates `// CROSS-PLATFORM INVARIANT` comments — NOT the `INVARIANT_*` test labels (those are D3's domain).

Find all such comments:

```bash
grep -rn "CROSS-PLATFORM INVARIANT" ios/Sources/ android/app/src/main/
```

For each comment that cites a mirror file:line (e.g. `mirrors NextWordScorer.swift:42`):

1. **Path exists** — the cited file is reachable
2. **Line in range** — line number is ≤ file LOC
3. **Surface name match** — the line contains a token that matches the surrounding comment's claim (e.g. cited line for "DECAY_HALF_LIFE_HOURS" should contain `DECAY_HALF_LIFE_HOURS`)

Required surfaces (per `behavioral-invariants.md` §5.3 four surfaces): `CandidateProcessor` scoring constants, `NextWordScorer` scoring constants, `TaigiUnicode.nfdPreprocessed`, `AssociationBinaryReader` / `DictionaryBinaryReader` bitmask layout.

Score (per-platform, then averaged):

```
required_surfaces_present = count of required surfaces with valid CROSS-PLATFORM INVARIANT comment
mirror_citations_total = count of CROSS-PLATFORM INVARIANT comments citing a mirror file:line
mirror_citations_valid = count where path/line/surface check all passed

required_score = (required_surfaces_present / 4) × 5
citation_score = (mirror_citations_valid / mirror_citations_total) × 5  # if no citations, = 5
D5 = required_score + citation_score   # max 10
```

Record stale citations under "D5 stale mirrors".

#### D6 — Rust portability (20%)

Scan candidate files for Swift/Kotlin features that don't translate cleanly to Rust. Each occurrence = penalty.

Apply the same two-pass comment-exclusion pipeline as D1 to avoid `///` doc-comment false positives.

**Swift penalties** (2 points each unless noted):

```bash
grep -nE "^[[:space:]]*[^/[:space:]]" "$file" | \
  grep -E "@propertyWrapper|@dynamicMemberLookup|KeyPath<|protocol [A-Za-z]+ where|@Published|ObservableObject|^import Combine"
```

Patterns: `@propertyWrapper`, `@dynamicMemberLookup`, `KeyPath<`, `protocol X where ...` (associated-type constraints — 1 pt), `@Published`, `ObservableObject`, `import Combine`.

**Kotlin penalties** (2 points each unless noted):

```bash
grep -nE "^import (android|androidx|kotlinx\.coroutines|java\.util\.concurrent)\." "$file"
```

Plus heuristic scans (1 pt each, manual review encouraged):
- `fun [A-Z][A-Za-z]+\.` — extension function on platform-looking type
- `companion object \{[^}]*\b(fun|var)\b` — companion with mutable state or behavior (pure-data exempt)

For each candidate file, count violations. Cap per-file penalty at 4 (one bad file shouldn't tank the score).

Score:

```
total_penalties = sum across all candidate files (capped per-file at 4)
D6 = max(0, 20 - total_penalties)
```

Record violators under "D6 portability blockers".

#### D7 — External deps boundary docs (5%)

Check that all four boundary docs exist and cover their assigned scope. Required docs:

| Doc | Required content | Detection grep |
|---|---|---|
| `docs/engine/ffi-safety.md` | `catch_unwind`, `Mutex`, `Drop`, error sentinel, log bridge | `grep -E "catch_unwind\|Mutex\|Drop\|log bridge"` |
| `docs/engine/rust-core-proto.md` | Request/Response schema, request-id, generation counter, settings push | `grep -E "request_id\|generation\|settings"` |
| `docs/architecture/data-artifacts-portability.md` | Asset copy semantics, stamp files, SQLite open patterns, update-in-place | `grep -E "asset copy\|stamp\|user_version\|update-in-place"` (case-insensitive) |
| `rules/rust-best-practices.md` | Workspace layout, FFI safety §2, error handling §3, unsafe §4, crate choices §5, opaque handle §10, non-goals §11 | `grep -E "workspace\|FFI\|unsafe\|crate"` (case-insensitive) |

If all 4 docs exist AND each grep finds ≥ 1 match → D7 = 5. Otherwise:

```
D7 = (docs_present_and_covering / 4) × 5
```

If D7 < 5, list missing docs / missing content under "D7 boundary doc gaps".

#### D8 — Test coverage on top-10 candidates (5%)

**Top-10 list** (per Phase IV-B extraction order from `project_shared_core_roadmap.md` + `android-state-audit.md:476`):

1. `TaigiPhonetics`
2. `InputNormalizer`
3. `CandidateProcessor`
4. `ToneConverter`
5. `SuggestionCaseTransformer` (iOS: `CaseTransformer`)
6. `ToneRestoration`
7. `TPSConverter`
8. `TaigiUnicode`
9. `NextWordScorer`
10. `CustomDictionaryDerivation`

**Android coverage** (Jacoco, from A9 baseline):
- Run `cd android && ./gradlew :app:jacocoTestReport` IF a fresh Jacoco run is required AND user is present. Otherwise read most recent `android/app/build/reports/jacoco/jacocoTestReport/jacocoTestReport.xml`.
- For each top-10 file, extract `LINE` counter `covered / (covered + missed)`. Pass = ≥ 70%.

**iOS coverage**:
- IF `--xcresult <path>` provided: run `scripts/ios-coverage-report.sh <path>` and parse output.
- ELSE: iOS half = "pending user run" → 0 score for iOS half.

Per `feedback_manual_build_test.md`, do NOT invoke `xcodebuild` directly — user runs it manually with the command in `scripts/ios-coverage-report.sh` header.

Score:

```
android_pass = count of top-10 ≥ 70% coverage on Android
ios_pass     = count of top-10 ≥ 70% coverage on iOS (0 if pending)
D8 = ((android_pass + ios_pass) / 20) × 5
```

#### D9 — FFI POC gate (5%)

**Boolean check**:
- Does `engine/` Rust workspace exist at the repo root?
- Does it contain a `phonetics` (or equivalent) crate?
- Does it build (`cargo check` from `engine/`)?
- Is it loaded into both iOS extension AND Android (look for `swift-bridge` / `jni` or equivalent integration)?
- Do the `INVARIANT_*` Phonetics tests pass with the Rust implementation?

```
D9 = 5 if all above true, else 0
```

Until Phase III, expect `D9 = 0`. Report **always** notes "gate UNREACHABLE without D9 POC" when D9 = 0, since the gate threshold of ≥95% requires D9 = 5.

### 4. Compute composite score

```
composite = D1 + D2 + D3 + D4 + D5 + D6 + D7 + D8 + D9
# max 100
```

### 5. Categorical blockers

Independent of composite — any one of these = gate FAIL:

- **B1** D9 not pass (Phase III deliverable)
- **B2** Any forbidden platform dependency in a marked shared-core candidate (i.e. D1 or D6 has at least one violation in a candidate file)
- **B3** Any of 4 prerequisite docs from Phase II.5 missing (D7 < 5)
- **B4** D7 = 0
- **B5** D3 < 50% of weight (i.e. D3 < 10)
- **B6** Any other dimension < 50% of its weight (fallback)

For each blocker triggered, record the offending evidence.

### 6. Gate verdict

```
gate = PASS if (composite ≥ 95) AND (D9 == 5) AND (no B1-B6 triggered)
gate = FAIL otherwise
```

Per `rules/cross-platform-alignment.md` §4 the technical gate is: composite ≥ 95% AND D9 pass. The user-facing gate (bug severity / backlog acceptable) is **out of scope for this skill** — user evaluates separately.

If `--ios-only` or `--android-only` was used, **do not emit a gate verdict**. Mark the report header "DIAGNOSTIC — NO GATE VERDICT".

### 7. Write the report

Output to `docs/engine/shared-core-confidence-<UTC date>.md`. If a same-date report exists, append `-rN` (r2, r3, …).

### 8. Optional delta table

If `--diff <path>` was provided AND the previous report parses cleanly:
- Compare composite + each dimension subscore
- Show `prev → current (Δ)` per row
- Do NOT mutate the previous report
- Place under "Delta vs previous" between §1 header and §2 composite table

---

## Report template

```markdown
# Shared-Core Confidence — <UTC date>

**Run mode**: full | ios-only | android-only
**Branch / commit**: <git rev-parse --short HEAD>
**Gate**: PASS | FAIL | DIAGNOSTIC (no verdict)

## Composite

| Dimension | Weight | Score | Notes |
|---|---|---|---|
| D1 iOS purity | 10 | X.X | (clean / total) |
| D2 Android parity | 15 | X.X | (paired / scorable) |
| D3 Behavior alignment | 20 | X.X | (cross-tested / total invariants) |
| D4 Data model parity | 10 | X.X | (mismatches: N) |
| D5 Cross-platform invariant comments | 10 | X.X | (required + citations) |
| D6 Rust portability | 20 | X.X | (penalties: N) |
| D7 External deps boundary docs | 5 | X | (4/4) or (N/4) |
| D8 Top-10 test coverage | 5 | X.X | (Android: N/10, iOS: N/10) |
| D9 FFI POC | 5 | 0 | UNREACHABLE without POC |
| **Composite** | **100** | **X.X** | — |

## Categorical blockers

- [ ] B1 D9 not pass
- [ ] B2 Forbidden dep in candidate (count: N)
- [ ] B3 Phase II.5 doc missing (count: N)
- [ ] B4 D7 = 0
- [ ] B5 D3 < 50%
- [ ] B6 Other dim < 50%

## Per-dimension breakdown

### D1 violations
<list>

### D2 gaps
<list>

### D3 invariant coverage
<status table per label>

### D4 type parity gaps
<list>

### D5 stale mirrors
<list>

### D6 portability blockers
<list>

### D7 boundary doc status
<table>

### D8 coverage rollup
<table>

### D9 POC status
<status>

## Gap analysis

What's needed to reach 95% composite + clear all blockers:
<bullet list of concrete next-round actions>

## Delta vs previous (optional)

| Dimension | Prev | Current | Δ |
|---|---|---|---|
...
```

---

## When to invoke

- After a Phase II A-round closes (verify alignment hasn't regressed)
- Before opening the Phase II end hybrid decision gate
- Before starting Phase III FFI POC work (baseline)
- After Phase III FFI POC completes (re-score with D9 = 5)

## Cross-refs

- Gate rules: `rules/cross-platform-alignment.md` §4 + §4a
- Roadmap: `project_shared_core_roadmap.md` (auto-memory) + Phase column in `docs/architecture/codex-review-2026-04-19.md`
- Candidate roster sources: `docs/engine/shared-core-readiness.md`, `docs/architecture/android-exemplar.md` §4–5
- Coverage helper: `scripts/ios-coverage-report.sh`
- Phase II audit: `docs/architecture/android-state-audit.md` §9
