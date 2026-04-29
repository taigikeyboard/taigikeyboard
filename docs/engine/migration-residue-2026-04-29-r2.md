# Migration Residue Audit — 2026-04-29 (r2)

> Branch: `main` (HEAD `d8455b8` — post-PR-#193 verification re-run)
> Run by: `/migration-residue` skill
> Last slice: `v3.5.3` + `v3.5.3-followup` (engine workspace cleanup PR #191 + path-G JVM-duplication delete PR #192)
> Prior report: `migration-residue-2026-04-29.md` (baseline run from `phase4b/migration-residue-skill@b546fd2`)

## Verdict

- **1 P1** finding (carried over from baseline — not yet fixed)
- **0 P2** findings
- **5 P3** findings (cosmetic — defer if pressed)

Overall: **NEEDS-CLEANUP**. Same posture as baseline. PR #193 added the audit skill + baseline report but did not fix any of its findings (per skill anti-goal: "Do not auto-fix"). Re-running on `main` HEAD confirms all P1 + most P3 findings persist; one G/P3 entry is corrected (baseline false-positive on `project_android_theming.md`).

## P1 — Critical

### E — Build-broken JVM tests (carried over)

Two Android `src/test/` files import the deleted `ToneConverterModels` symbol. The full-FQN walk against `src/main` declarations confirms the breakage:

```
BROKEN: import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
BROKEN: import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
```

- `android/app/src/test/java/com/siansiansu/taigikeyboard/ime/dictionary/SuggestionCaseTransformerTest.kt` — imports `ToneConverterModels.InputMode`.
- `android/app/src/test/java/com/siansiansu/taigikeyboard/ime/text/composing/ComposingStateTest.kt` — imports `ToneConverterModels` directly.

`ToneConverterModels` does not exist in `src/main`; the only surviving references are doc comments at `ime/core/settings/InputMode.kt` and `ime/text/composing/AutocompleteInputClassifier.kt` (both noting it was deleted). The unrelated top-level `InputMode` enum at `ime/core/settings/InputMode.kt` masks the breakage in a name-only grep — only the FQN walk catches it.

**Fix**: rewrite both test imports against current symbols (likely `com.siansiansu.taigikeyboard.ime.core.settings.InputMode`); or, if the test target was deleted in a slice migration, delete the test per `feedback_path_g_delete_mirrors.md`.

## P2 — Recommended

None.

## P3 — Hygiene

### D — Stale doc-comment references (4 findings, all carried over)

1. **D9.x project-history wording in source comments**:
   - `engine/phonetics/src/api.rs:103` — "intentionally not implemented in D9.1 — they"
   - `engine/phonetics/src/api.rs:304` — "Re-using these from D9.1 lets the iOS+Android fixture suite exercise them."
   - `engine/phonetics/src/tps.rs:296` — "out of scope for D9.1 (Lexicon, Phase IV-B)."

   Fix: drop the `D9.x` framing; describe capability/scope directly. `Phase IV-B` framing is also outdated since the cadence renumbered slices to `v3.5.x` per `project_rust_migration_cadence.md`.

2. **Wrong version label in source comment** at `engine/phonetics/src/normalization.rs:161` (inside `#[cfg(test)] mod tests`):
   ```
   // taigi_unicode_base_form — moved from engine/ranking/src/nfd.rs in
   // v3.5.4 (consolidates the helper that backs both ranking and the
   // new Method::NfdPreprocessForLookup op). Test cases preserved
   // verbatim from the original ranking-side module.
   ```
   The "v3.5.4" attribution is wrong — the move landed in PR #192 (v3.5.3 follow-up). Per `project_rust_migration_cadence.md`, v3.5.4 = Composing slice. Fix: update to "v3.5.3 follow-up (PR #192)".

3. **Obsolete strategy cited as live policy** (carried over):
   - `docs/engine/rust-core-proto.md:165` — narrative still describes `feedback_jvm_test_jni_compat.md` as the active rationale ("the bridge can't load `.so` from `src/test/`. The Rust phonetics crate retains the canonical implementations…").
   - `docs/engine/ranking-slice-audit.md:122` — "Strategy (per `feedback_jvm_test_jni_compat.md`, PR #187 pattern):".

   Both reversed by path G in v3.5.3 follow-up (PR #192) per `feedback_path_g_delete_mirrors.md`. Fix: add `(obsolete after v3.5.3 follow-up — see feedback_path_g_delete_mirrors.md)` parenthetical OR rewrite the relevant section.

### G — Memory + doc hygiene (1 finding — corrected vs baseline)

4. **`MEMORY.md` cites missing memory file** at line 42:
   ```
   - [Changelog timing](feedback_changelog_timing.md) — only update changelog at release time
   ```
   `feedback_changelog_timing.md` is **not present** in `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/`. Fix: remove the index entry, OR recreate the memory file from the changelog-timing convention if it was deleted unintentionally.

   **Baseline correction**: the prior report flagged `project_android_theming.md` as missing — that was a false positive. The file exists (634 B, last modified 2026-04-14). The actually-missing entry is `feedback_changelog_timing.md`.

## Dimensions that found nothing

### A — Cross-language algorithm duplication: CLEAN

After PR #192 the Android dictionary directory contains zero Rust algorithm mirrors; iOS `Phonetics/` directory was deleted in v3.5.1 (D9.4). Anti-pattern grep finds zero per-codepoint phonetic scans on either platform.

### B — Cross-crate Rust algorithm duplication: CLEAN

13 `pub fn` items across the workspace, all justified (top-level FFI entry, CLI/integration surface, domain dispatch, cross-crate consumed helpers, lib.rs re-exports). No two functions in different crates implement the same algorithm.

### C — Over-public Rust surface: CLEAN

Total public surface (26 items) reviewed; v3.5.3 visibility-tightening sweep (commit `ee63285`) holds. No `pub fn` should be `pub(crate)` or plain `fn`.

### F — Bridge surface parity: CLEAN

All 18 phonetics ops + 1 lexicon op (`ProcessCandidates`) have matching wrappers on both `RustEngineBridge.swift` and `RustEngineBridge.kt`. Diagnostics + install + sendRawBytes + panicForTestRaw covered on both sides.

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | 0 | 0 | 0 | clean |
| B — Cross-crate duplication | 0 | 0 | 0 | clean |
| C — Over-public surface | 0 | 0 | 0 | clean |
| D — Stale comments | 0 | 0 | 4 | D9.x wording + 1 wrong-version label + 2 obsolete-strategy cites |
| E — Build-broken JVM tests | 1 | 0 | 0 | 2 tests import deleted `ToneConverterModels` (carried over) |
| F — Bridge surface parity | 0 | 0 | 0 | clean |
| G — Memory hygiene | 0 | 0 | 1 | MEMORY.md cites missing `feedback_changelog_timing.md` (baseline false-positive on `project_android_theming.md` corrected) |

## Suggested next slice scope

**P1 (Dim E)** — fix or delete `SuggestionCaseTransformerTest` + `ComposingStateTest`. Silently build-broken since the `ToneConverterModels` deletion (v3.5.1 D9.4). Either rewrite imports against current symbols (most likely `ime.core.settings.InputMode`) or delete tests per `feedback_path_g_delete_mirrors.md`.

**P3 (Dim D + G)** — five label-drift / hygiene items. Bundle with the P1 fix or batch into a "doc-only hygiene" PR. Not urgent.

The migration architecture itself remains in the cleanest state since shared-core extraction began. P1 is ancient debt the original audit failed to catch, not new debt from v3.5.3.
