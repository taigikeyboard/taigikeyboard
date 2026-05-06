# Migration Residue Audit — 2026-05-04

> Branch: `main` (HEAD `b50004f`)
> Run by: `/migration-residue` skill
> Last slice: `v3.5.7` (classification + Tab3 hanzi-range fix). Branch HEAD also includes case-transform (PR #205, `4d2a69a`) and dead-FFI cleanup (PR #207, `affa307`); on top sit the keystroke trace IDs (PR #208, `b50004f`).

## Verdict

- **0 P1 findings**
- **6 P2 findings** (3 cross-language duplication, 2 over-public Rust surface, 1 stale doc cite)
- **3 P3 findings** (CSV inventory drift + 2 cosmetic D9.x history references)

Overall: **PASS** (no migration blockers; recommended cleanup before next slice)

---

## Migration Backlog Inventory

The user's lead question — "what modules/functions still need to migrate to Rust, how many are left" — answered against `docs/engine/migration-inventory.csv` cross-checked with Dimension A static analysis.

### 1. CSV `native_pending` rows (11 verbatim)

| # | status | area | platform | entity_name | A-flagged? |
|---|---|---|---|---|---|
| 1 | native_pending | lexicon | cross_platform | CustomDictionaryDerivation.swift/.kt | **NO — already Rust-shipped** (calls `RustEngineBridge.deriveNotone` / `deriveAbbrev` / `normalizeInput`; only platform residue is `searchPrefix` 5-line dispatcher on iOS, fully thin on Android). **CSV STALE.** |
| 2 | native_pending | autocomplete | ios | AutocompleteInputClassifier.swift | **NO** — 1-line wrapper (`RustEngineBridge.classifyInput`). Already-thin shell. |
| 3 | native_pending | autocomplete | android | AutocompleteInputClassifier.kt | **NO** — 1-line wrapper (`LexiconBridge.classifyInput`). Already-thin shell. |
| 4 | native_pending | autocomplete | ios | AutocompleteProviders.swift | **NO** — pure protocol declarations (Foundation glue). |
| 5 | native_pending | settings | ios | EngineSettings.swift | **NO** — protocol-only, 33 LOC, no algorithm. |
| 6 | native_pending | settings | android | EngineSettings.kt | **NO** — interface-only. |
| 7 | native_pending | settings | ios | EngineSettingsProvider.swift | **NO** — accessor protocol. |
| 8 | native_pending | settings | android | EngineSettingsProvider.kt | **NO** — accessor interface. |
| 9 | native_pending | settings | cross_platform | InputMode.swift/.kt | **NO** — DTO enum; bridges to `protos::InputMode`. |
| 10 | native_pending | settings | cross_platform | ToneToggles.swift/.kt | **NO** — value-type DTO (21 LOC). |
| 11 | native_pending | common | cross_platform | LoggerBackend.swift/.kt | **NO** — platform-only by design (logger sink). Status should arguably be `native_keep`. |

### 2. CSV `rust_partial` rows (2)

| # | status | area | platform | entity_name | A-flagged? |
|---|---|---|---|---|---|
| 12 | rust_partial | phonetics | ios | CharacterInputPipeline.swift | **NO** — 1-line wrapper (`RustEngineBridge.tpsInputAdjust`); 51 LOC including doc + struct. Already a thin shell. |
| 13 | rust_partial | lexicon | ios | CandidateProcessor.swift | **NO** (anti-goal exemption: `startsWithRomanLetter` is platform-only per skill spec; `capitalize` is 1-line bridge). |

### 3. New migration candidates surfaced by Dimension A (CSV gaps)

| # | area | platform | entity_name | path | LOC | finding |
|---|---|---|---|---|---|---|
| **N1** | lexicon | cross_platform | EnabledDictionaries.swift / .kt | `ios/Sources/TaigiKeyboard/Lexicon/Models/EnabledDictionaries.swift`, `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/EnabledDictionaries.kt` | 109 / 98 | **TRUE algorithm duplication** — both files compute identical `sourceBitmask()`/`associationBitmask()`/`dictionaryFilterBitmask()` with the same bit layout (0=kautian, 1=taigitv, …, 12=variant). Pure logic, no platform deps; CSV currently labels it `wont_migrate`/`native_keep` for the related `EnabledDictionaries` entry but the bitmask helpers are clear shared-core candidates. |
| **N2** | lexicon | cross_platform | ExternalLookupURLBuilder.swift / .kt | `ios/Sources/TaigiKeyboard/Lexicon/Utils/ExternalLookupURLBuilder.swift`, `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/ExternalLookupURLBuilder.kt` | 82 / 68 | Anti-goal exemption applies (skill spec: "URL builders — URL semantics are platform-owned"). However the inner `normalizeSyllableToDigit` body is mathematically identical between platforms; `toTLDigit` could be Rust if the URL-formatting wrapper stays platform. **Tracked as P3 candidate** rather than P2. |

CSV currently lists N1 row 109 (`EnabledDictionaries.swift/.kt`) as `wont_migrate` — recommend reclassifying to `native_pending` because the bitmask math IS algorithm-equivalent and would consolidate cleanly to `engine/lexicon/src/source_bitmask.rs`. The wire-format alignment risk (binary-format.md) is the same on either side; Rust authority would be safer.

### Bottom line

- **Per CSV:** 13 items remain to migrate (11 native_pending + 2 rust_partial).
- **Audit cross-validation:** 0 of the 13 are still genuine algorithm duplication. **All 13 are already either thin bridges (rows 1–4, 12, 13), platform-only DTOs/protocols (rows 5–10), or platform-glue infrastructure (row 11).** CSV row 1 (`CustomDictionaryDerivation`) is **stale** — should flip to `rust_shipped` (only ~5 LOC iOS dispatcher remains).
- **Audit-surfaced new candidates:** 1 P2 (EnabledDictionaries bitmask, ~80 LOC pure-logic duplication) + 1 P3 (ExternalLookupURLBuilder.toTLDigit inner math).
- **Headline:** **0 native_pending rows confirmed as still-needing migration; 1 net new P2 migration candidate; CSV needs ~3 row updates.**

---

## P1 — Critical

*(none — all dimensions clean)*

---

## P2 — Recommended

### A — Cross-language algorithm duplication

#### A.1 — `EnabledDictionaries.sourceBitmask()` mirrored verbatim across iOS + Android (P2)

- iOS: `ios/Sources/TaigiKeyboard/Lexicon/Models/EnabledDictionaries.swift:50-83`
- Android: `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/EnabledDictionaries.kt:34-67`

Both implement identical bit-mapping: `sourceBitmask()` → bits 0=kautian, 1=taigitv, 2=itaigi, 3=sitbut, 4=taihoa, 5=taijit, 6=kungge, 7=stti, 8=khpoo, 11=lkk; `dictionaryFilterBitmask()` adds bit 9=khiin, bit 10=dev, bit 12=variant. Comments on both sides reference `engine/lexicon/src/search.rs::build_filter` as the canonical layout. Pure stdlib code (Boolean → Int OR).

Recommended fix: extract to `engine/lexicon/src/source_bitmask.rs` (~30 LOC); platforms keep the `EnabledDictionaries` value-struct that captures user toggles, but call `LexiconBridge.computeFilterBitmask(...)` for the actual encoding. Eliminates a future drift risk where one platform updates the bit layout (e.g. adds a 13th source) and the other lags.

#### A.2 — Memo: thin-shell helpers correctly delegated (P3 informational)

The following platform helpers are 1-line `RustEngineBridge` calls — no duplication, but candidates for future inline-and-delete:

- `AutocompleteInputClassifier.classify` (iOS:15-18) / `.determineInputType` (Android:13-14)
- `CustomDictionaryDerivation.generateNotone` / `.generateAbbrev` / `.generateRomanNum` (both platforms)
- `CharacterInputPipeline.adjust` (iOS:37-43; Android:19-22) — Rust owns `tpsInputAdjust` orchestration
- `KeyLabelCaseCache.getOrCompute` (Android:51-67) — caches Rust call results, platform-glue (cache lifetime, key composition); keep
- `SuggestionCaseTransformer.transform` (both platforms) — thin per-word loop with platform-specific skip rules (id markers / additionalInfo flags); keep

No P-flag — these are intentional bridge layers per CSV documentation.

### C — Over-public Rust surface

#### C.1 — `phonetics::api::preprocess_for_normalize_tone` over-public (P2)

- `engine/phonetics/src/api.rs:59`
- Only caller across the workspace: `engine/phonetics/src/api.rs:103` (inside `normalize_tone`).

Recommend `pub(crate) fn` (or even private `fn`) — no cross-crate or test caller. Wire surface (`Method::NormalizeTone` dispatcher) does not call this directly; it goes through `api::normalize_tone`.

#### C.2 — `phonetics::case_transform::adjust_nasal_marker_case` over-public (P2 — DROPPED post-Codex)

- `engine/phonetics/src/case_transform.rs:229`
- Production callers across workspace: `case_transform.rs:177` (transform_suggestion) and `api.rs:105` (normalize_tone) — both inside the `phonetics` crate.
- **External integration-test caller** (caught by Codex pre-impl review, missed by initial audit): `engine/phonetics/tests/case_transform_golden.rs:13,409,414` imports the symbol from outside the crate. Tightening to `pub(crate)` would break that test.

**Outcome: KEPT `pub`.** The integration test deliberately exercises the helper directly (golden table for nasal-marker case behavior); changing it to go through `transform_suggestion` would lose that unit-level coverage. No fix shipped in cleanup PR #209.

#### C.3 — `engine/build-helpers/fst-builder` `pub fn run_query` / `run_build` over-public (P3 not P2)

- `engine/build-helpers/fst-builder/src/builder.rs:19` `pub fn run_build`
- `engine/build-helpers/fst-builder/src/query.rs:26` `pub fn run_query`

Both are only called from `fst-builder/src/main.rs` in the same binary crate. Recommend plain `fn` (or `pub(crate)`). Marked P3 because this is a build-helpers bin crate, not part of the runtime FFI surface — visibility hygiene matters less.

### F — Bridge surface parity

**Verdict: clean.** Cross-checked all four `oneof method { … }` blocks against iOS `RustEngineBridge*.swift` and Android `RustEngineBridge.kt` / `LexiconBridge.kt` / `CaseTransformBridge.kt`:

| proto module | dispatchable ops | iOS wrappers | Android wrappers | parity |
|---|---|---|---|---|
| Phonetics (`oneof method`) | 16 | 16 | 16 | ✓ |
| Lexicon | 8 | 8 | 8 | ✓ |
| Composing | 12 | 12 | 12 | ✓ |
| NextWord | 10 | 10 | 10 | ✓ |
| Case | 6 | 6 | 6 | ✓ |

No missing platform wrappers; no in-flight ops detected on the current branch beyond what `b50004f` ships.

---

## P3 — Hygiene

### B — Cross-crate Rust duplication

**Verdict: clean.** No cross-crate `pub fn` name collisions other than:
- `handle()` — appears in `phonetics/dispatch.rs`, `lexicon/dispatch.rs`, `composing/dispatch.rs` (per-module dispatchers; correct).
- `search_with_sources()` — appears in `lexicon/api.rs` + `lexicon/search.rs` (api → search delegation; correct).

No constant-table or NFD-helper duplication detected.

### D — Stale doc-comment references

#### D.1 — `phonetics::api::preprocess_for_normalize_tone` doc cites wrong module path (P2)

- `engine/phonetics/src/api.rs:58` — `/// Mirrors phonetics::dispatch::preprocess_for_normalize_tone.`

The function is at `phonetics::api::preprocess_for_normalize_tone`, not in `dispatch.rs`. Misleading reader who follows the citation. Recommend fixing the doc to `Mirrors iOS ToneConverter doubletap rules` or removing the cross-reference (the api/dispatch boundary is the wrong axis).

#### D.2 — `D9.1` / `D9.2` / `D9.4` historical labels in current docs (P3)

Surfaces in `engine/protos/proto/phonetics.proto:24-25`, `engine/protos/proto/envelope.proto:46,51`, `engine/README.md:31`, multiple bridges. Cosmetic project-history baggage per skill spec smell list. No misdirection risk; defer if pressed.

#### D.3 — Project memories cite `feedback_jvm_test_jni_compat.md` (deleted file) (P3)

- `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/project_v3_5_1_phonetics_round_progress.md:22`
- `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/project_v3_5_2_ranking_slice_progress.md:138`
- `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/project_phase3_d9_progress.md:28`

The cited memory file was deleted (superseded by `feedback_path_g_delete_mirrors.md`). Each citation appears in **historical context** of a closed slice progress doc — readers can follow the path-G memo for the current policy. Keeping these as historical breadcrumbs is acceptable; flagged P3 only for completeness.

### E — Build-broken JVM unit tests

**Verdict: clean.** Four JVM unit-test files exist on `main`:
- `OutcomeTest.kt`
- `LexiconServiceHanziGuardTest.kt`
- `EngineSettingsLiveReadTest.kt`
- `LoggerBackendNeutralityTest.kt`

None contain project-package imports (`import com.siansiansu.taigikeyboard.…`) and none reference `RustEngineBridge` / `System.loadLibrary("rust_taigi")`. They exercise framework-stdlib + project value-types only — host JVM-loadable.

### G — Memory + doc hygiene

**Verdict: clean** (modulo D.3 above).

Resolved memory dir: `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/` (68 files).

`MEMORY.md` cites 66 distinct `.md` paths; **all 66 exist** in the memory dir.

Release map in `project_rust_migration_cadence.md` versus `git log -20` consistent (v3.5.0–v3.5.7 + post-v3.5.7 polish all match).

#### CSV inventory drift (P3)

- `docs/engine/migration-inventory.csv` row 91 (`CustomDictionaryDerivation.swift/.kt`) carries status `native_pending`, but both files are now thin `RustEngineBridge.derive*` wrappers since D9.4. Update to `rust_shipped` with slice label `v3.5.1-D9.4-Phonetics`.
- Row 109 (`EnabledDictionaries.swift/.kt`) carries status `wont_migrate` — see Dimension A.1 above. Recommend reclassifying to `native_pending` candidate for a future bitmask consolidation slice.
- Row 101 (`LoggerBackend.swift/.kt`) — status `native_pending` is misleading; this is a platform-only logger sink BY DESIGN (`feedback_user_data_sqlite_stays_native.md` precedent applies). Consider relabeling `native_keep`.

---

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | 0 | 1 | 0 | EnabledDictionaries bitmask math (~80 LOC, true duplication) |
| B — Cross-crate duplication | 0 | 0 | 0 | clean |
| C — Over-public surface | 0 | 2 | 1 | preprocess_for_normalize_tone, adjust_nasal_marker_case → pub(crate); fst-builder helpers → pub(crate) |
| D — Stale comments | 0 | 1 | 2 | api.rs:58 wrong module path; D9.x labels; memory historical citations |
| E — Build-broken JVM tests | 0 | 0 | 0 | 4 tests, all clean |
| F — Bridge surface parity | 0 | 0 | 0 | 52/52 ops covered on both platforms |
| G — Memory hygiene | 0 | 0 | 3 | CSV row 91 stale, row 101 mislabel (LoggerBackend), row 109 mislabel (EnabledDictionaries) |

Totals: **0 / 4 / 6** (P1 / P2 / P3).

### Post-impl shipped state (PR #209)

After Codex pre-impl review and PR #209 (`cleanup/migration-residue-2026-05-04`):

- C.1 ✅ shipped — `preprocess_for_normalize_tone` → `pub(crate)`.
- C.2 ❌ **DROPPED** — Codex caught external integration-test caller at `tests/case_transform_golden.rs:13`; tightening would break the test. `adjust_nasal_marker_case` retains `pub`.
- C.3 ✅ shipped — `fst-builder::{builder::run_build, query::run_query}` → `pub(crate)`.
- C.3 bonus ✅ shipped — unused `const SEPARATOR` removed from `query.rs:9` (Codex-surfaced).
- D.1 ✅ shipped — wrong cite at `api.rs:58` removed.
- G/CSV-drift ✅ shipped — rows 91, 101 (NOT 100), 109 corrected.
- A.1 (EnabledDictionaries bitmask migration) — deferred to a future slice PR; CSV updated to `native_pending`.

---

## Suggested next slice scope

The highest-leverage cleanup that clusters around a single PR:

1. **`EnabledDictionaries` bitmask consolidation slice** (Dimension A.1 P2 migration — DEFERRED from PR #209).
   - Rust new file: `engine/lexicon/src/source_bitmask.rs` (~30 LOC) + dispatch arm (`Method::EncodeFilterBitmask`).
   - Delete `sourceBitmask()` / `dictionaryFilterBitmask()` / `associationBitmask()` from `EnabledDictionaries.swift` + `EnabledDictionaries.kt`; replace call sites with bridge calls.
   - The CSV row update was already shipped in PR #209 (row 109 → `native_pending`); the actual code migration is the remaining slice work.
   - Note: this is a true new slice (proto + dispatch + bridge + xcframework rebuild + dogfood) — incompatible with the current polish-phase mode per `project_post_v357_polish_phase.md`. Schedule when polish phase ends.

2. ~~**Visibility tightening micro-PR** (C.1 + C.2)~~ — **shipped in PR #209** for C.1; C.2 dropped (integration-test break).

After PR #209 merges, the residue picture is essentially clean: the only remaining P2 is the `EnabledDictionaries` bitmask migration above, gated on policy rather than discovery.

The Phase IV-B "non-trivial pure-logic" candidate list is now empty per `project_case_transform_slice.md` (case-transform was the last). Anything beyond the bitmask consolidation requires a fresh user decision on whether to migrate the lexicon DTOs / settings protocols (rows 5–11) — they have no algorithmic content and the audit does NOT recommend it.
