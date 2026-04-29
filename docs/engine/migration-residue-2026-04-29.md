# Migration Residue Audit — 2026-04-29

> Branch: `phase4b/migration-residue-skill` (HEAD `b546fd2` — re-run with full-FQN resolution after Codex PR #193 review)
> Run by: `/migration-residue` skill (first run + revision)
> Last slice: `v3.5.3` (engine workspace cleanup PR #191 + path-G JVM-duplication delete PR #192)

## Verdict

- **1 P1** finding (Codex revised — original audit missed it)
- **0 P2** findings
- **5 P3** findings (cosmetic — defer if pressed)

Overall: **NEEDS-CLEANUP**. The original symbol-name-only Dim E grep produced a false-clean. After Codex flagged the gap (PR #193 review `r3159249022`) and the skill's E-dim logic was rewritten to walk full-FQN suffix prefixes against `src/main` declarations, the audit catches 2 build-broken JVM test files that have been silent since the `ToneConverterModels` deletion in v3.5.1 (D9.4).

## P1 — Critical

### E — Build-broken JVM tests (Codex revised)

Two Android `src/test/` files import the deleted `ToneConverterModels` symbol; an unrelated top-level `InputMode` enum at `ime/core/settings/InputMode.kt` masked the breakage in the original audit's symbol-name-only grep. The new FQN walk against `src/main` declarations catches both.

- `android/app/src/test/.../ime/dictionary/SuggestionCaseTransformerTest.kt:<top>` imports `com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode`. The outer `ToneConverterModels` type does not exist in `src/main` — only doc-comment references at `ime/core/settings/InputMode.kt:10` and `ime/text/composing/AutocompleteInputClassifier.kt:48` remain (both noting it was deleted).
- `android/app/src/test/.../ime/text/composing/ComposingStateTest.kt:<top>` imports `com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels` directly.

**Fix**: rewrite both test imports against current symbols (likely `com.siansiansu.taigikeyboard.ime.core.settings.InputMode`); or, if a test cannot be salvaged because its target was deleted in a slice migration, delete the test per `feedback_path_g_delete_mirrors.md`.

**Why the original audit missed this**: the v1 Dim E logic searched for the trailing symbol name (`InputMode`) anywhere under `src/main` and got a hit because an unrelated `InputMode` enum exists at a different package. The walk in commit `b546fd2` resolves the full qualified import target before declaring success, mirroring how `kotlinc` actually resolves imports.

## P2 — Recommended

None.

## P3 — Hygiene

### D — Stale doc-comment references (4 findings)

1. **D9.x project-history wording in source comments**:
   - `engine/phonetics/src/api.rs:103` — "intentionally not implemented in D9.1"
   - `engine/phonetics/src/api.rs:304` — "Re-using these from D9.1 lets..."
   - `engine/phonetics/src/tps.rs:296` — "out of scope for D9.1 (Lexicon, Phase IV-B)"
   Fix: drop the `D9.x` framing and describe the capability/scope directly. The `Phase IV-B` framing is also outdated since the cadence renumbered slices to `v3.5.x` per `project_rust_migration_cadence.md`.

2. **Wrong version label in source comment** at `engine/phonetics/src/normalization.rs:160`:
   ```
   // taigi_unicode_base_form — moved from engine/ranking/src/nfd.rs in
   // v3.5.4 (consolidates the helper that backs both ranking and the
   // new Method::NfdPreprocessForLookup op). Test cases preserved
   // verbatim from the original ranking-side module.
   ```
   The "v3.5.4" attribution is wrong — this move landed in PR #192 which is the v3.5.3 follow-up (`project_rust_migration_cadence.md` release map: v3.5.4 = Composing slice). Fix: update to "v3.5.3 follow-up (PR #192)".

3. **Obsolete strategy cited without "obsolete" marker**:
   - `docs/engine/rust-core-proto.md:165` — narrative still describes `feedback_jvm_test_jni_compat.md` as live policy ("the bridge can't load `.so` from `src/test/`. The Rust phonetics crate retains the canonical implementations..."); reversed by path G in v3.5.3 follow-up.
   - `docs/engine/ranking-slice-audit.md:122` — "Strategy (per `feedback_jvm_test_jni_compat.md`, PR #187 pattern):" — same issue.
   Fix: add `(obsolete after v3.5.3 follow-up — see feedback_path_g_delete_mirrors.md)` parenthetical OR rewrite the relevant section.

### G — Memory + doc hygiene (1 finding)

4. **`MEMORY.md` cites missing memory file**:
   - `MEMORY.md` index references `project_android_theming.md` but the file does not exist in `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/`.
   Fix: remove the index entry, OR if the memory was deleted unintentionally, recreate from the Android theming notes.

## Dimensions that found nothing

### A — Cross-language algorithm duplication: CLEAN

After PR #192 the Android dictionary directory contains 20 Kotlin files, none of which mirror Rust algorithms. iOS `Phonetics/` directory was deleted in v3.5.1 (D9.4). Anti-pattern grep finds zero per-codepoint phonetic scans on either platform.

### B — Cross-crate Rust algorithm duplication: CLEAN

13 `pub fn` items across the workspace, all justified:
- `dispatch::process_request` — top-level FFI entry
- `phonetics::api::{convert,to_tone_marks,to_tone_number,poj_display_to_tl_display,tl_display_to_poj_display}` — CLI / integration test surface
- `phonetics::dispatch::handle` — domain dispatch (called by `engine/dispatch`)
- `phonetics::normalization::taigi_unicode_base_form` — cross-crate (consumed by `ranking::score`)
- `phonetics::poj::to_poj`, `phonetics::tl::to_tl` — re-exported at lib.rs for fixtures
- `phonetics::syllable::{strip_tone_mark,normalize_to_tl}` — re-exported at lib.rs
- `ranking::process_candidates` — top-level entry called by `engine/dispatch`

No two functions in different crates implement the same algorithm.

### C — Over-public Rust surface: CLEAN

Total public surface: 26 items across the workspace. Spot-checked the v3.5.3 visibility-tightening sweep (commit `ee63285`); no `pub fn` that should be `pub(crate)` or plain `fn` survived.

### E — Build-broken JVM tests: see P1 above

(Original audit reported CLEAN; corrected after Codex revision. The 9 remaining JVM test files are clean of `RustEngineBridge` JNI imports — the breakage is at the project-symbol-resolution layer, not the JNI layer. 2 of 9 test files affected.)

### F — Bridge surface parity: CLEAN

All 18 Rust phonetics ops (`NormalizeTone`, `StripTone`, `PojToTl`, `TlToPoj`, `NormalizeToTl`, `NormalizeInput`, `RestoreTone`, `HasToneMarks`, `GetToneVariations`, `NfdPreprocessForLookup`, `DeriveNotone`, `DeriveAbbrev`, `ContainsTps`, `TpsToTl`, `TlNumericToTps`, `TlDisplayToTps`, `IsTpsToneMark`, `TpsInputAdjust`) + 1 lexicon op (`ProcessCandidates`) have matching wrappers on both `RustEngineBridge.swift` and `RustEngineBridge.kt`. Diagnostics + install + sendRawBytes + panicForTestRaw covered on both sides.

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | 0 | 0 | 0 | clean |
| B — Cross-crate duplication | 0 | 0 | 0 | clean |
| C — Over-public surface | 0 | 0 | 0 | clean |
| D — Stale comments | 0 | 0 | 4 | D9.x wording + 1 wrong-version label + 2 obsolete-strategy cites |
| E — Build-broken JVM tests | 1 | 0 | 0 | 2 tests import deleted ToneConverterModels (Codex revised) |
| F — Bridge surface parity | 0 | 0 | 0 | clean |
| G — Memory hygiene | 0 | 0 | 1 | MEMORY.md cites missing project_android_theming.md |

## Suggested next slice scope

**P1 (Dim E)** — fix or delete `SuggestionCaseTransformerTest` + `ComposingStateTest`. These have been silently build-broken since the `ToneConverterModels` deletion (likely in v3.5.1 D9.4). Bundle into the v3.5.4 Composing slice prep OR a small follow-up PR. Either rewrite the imports against current symbols (most likely `ime.core.settings.InputMode`) or delete the tests per `feedback_path_g_delete_mirrors.md` if they cover algorithms now owned by Rust.

**P3 (Dim D + G)** — five label-drift / hygiene items. Bundle with the P1 fix or batch into a "doc-only hygiene" PR. Not urgent.

The migration architecture itself (Rust workspace + bridge + production callsites) remains in the cleanest state since shared-core extraction began. The P1 finding is ancient debt the original audit failed to catch, not new debt from v3.5.3.
