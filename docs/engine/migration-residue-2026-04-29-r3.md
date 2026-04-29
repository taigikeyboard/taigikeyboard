# Migration Residue Audit — 2026-04-29 (r3)

> Branch: `cleanup/migration-residue-r2-fixes` (post-fix verification re-run)
> Run by: `/migration-residue` skill
> Last slice: `v3.5.3` + `v3.5.3-followup`
> Prior reports: `migration-residue-2026-04-29.md` (baseline), `migration-residue-2026-04-29-r2.md` (post-merge re-verification)

## Verdict

- **0 P1** findings
- **0 P2** findings
- **0 P3** findings

Overall: **PASS** (Codex revised). Every finding from r2 is resolved; the local post-fix sweep caught two adjacent same-class drift labels that the baseline + r2 missed (`engine/phonetics/src/normalization.rs:119` and `engine/ranking/src/process.rs:289`). The Codex second-opinion pass on PR #194 then disputed the `pre-v3.5.2` label at `process.rs:289` (the iOS Swift `removeDuplicates` cold-start fallback was actually replaced in PR #192, not v3.5.2) and surfaced **5 additional same-class stale labels** the local audit missed — all 5 also originate from `add2252` (PR #192) but were labelled "v3.5.4" in source. All 6 (1 disputed + 5 missed) corrected in this branch before merge.

## Resolved findings

### P1 — Dim E (build-broken JVM tests) — RESOLVED

Both test files now resolve against `com.siansiansu.taigikeyboard.ime.core.settings.InputMode` (the surviving enum used by production `SuggestionCaseTransformer.kt:6`, `LexiconService.kt:11`, `DictionaryConstants.kt:6`, etc.). POJ / TL / ENGLISH variants match the test fixtures one-for-one — no behavioral change, just import target.

- `android/app/src/test/.../ime/dictionary/SuggestionCaseTransformerTest.kt:3` — import retargeted.
- `android/app/src/test/.../ime/text/composing/ComposingStateTest.kt:4` — import retargeted; `private val tl = ToneConverterModels.InputMode.TL` rewritten to `private val tl = InputMode.TL`.

Re-run of the full-FQN walk against `src/main` declarations: zero `BROKEN:` lines.

### P3 — Dim D (stale doc comments) — RESOLVED + 2 EXTRAS

Original four findings:

1. `engine/phonetics/src/api.rs:103` — D9.1 wording dropped; description rewritten without project-history framing.
2. `engine/phonetics/src/api.rs:304` — D9.1 wording dropped; intent rephrased ("Exposed so the iOS+Android fixture suite can exercise them").
3. `engine/phonetics/src/tps.rs:296` — D9.1 + Phase IV-B wording dropped; replaced with "belongs with the Lexicon slice".
4. `engine/phonetics/src/normalization.rs:161` — `v3.5.4` → `v3.5.3 follow-up (PR #192)`.

Two additional same-class drift labels caught during the post-fix sweep:

5. `engine/phonetics/src/normalization.rs:119` — CROSS-PLATFORM INVARIANT comment said platform copies were removed `until v3.5.4`; corrected to `until v3.5.3 follow-up (PR #192)`.
6. `engine/ranking/src/process.rs:289` — test doc said it mirrors `pre-v3.5.4 iOS Swift fallback`; the ranking slice landed in v3.5.2 (PR #189), so corrected to `pre-v3.5.2`.

Two narrative findings:

7. `docs/engine/rust-core-proto.md:165` — added explicit `(Obsolete after v3.5.3 follow-up — see feedback_path_g_delete_mirrors.md)` marker; rewrote the sentence to describe path G as the current state ("Rust phonetics crate is now the sole owner").
8. `docs/engine/ranking-slice-audit.md:122` — strikethrough applied to the obsolete "platform Kotlin helpers retained" + "CROSS-PLATFORM INVARIANT comment" steps; surrounding text now reads as historical.

**Codex-revised additions** (caught by `--codex` second-opinion pass on PR #194):

9. `engine/phonetics/src/api.rs:34` — error message string referenced `D9 — Lexicon slice in Phase IV-B`. Replaced with neutral "belongs with the Lexicon slice — not implemented here".
10. `engine/ranking/src/process.rs:289` — local audit relabelled `pre-v3.5.4` → `pre-v3.5.2`; Codex pointed out the actual replacement landed in PR #192 (v3.5.3 follow-up), not v3.5.2. Final form: `earlier iOS Swift fallback (replaced in v3.5.3 follow-up, PR #192)`.
11. `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge.swift:362` — `v3.5.4 simplification` → `Simplified in the v3.5.3 follow-up (PR #192)`.
12. `android/app/src/main/.../engine/RustEngineBridge.kt:401` — same fix (Kotlin doc comment).
13. `ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift:11` — `removed in v3.5.4` → `removed in the v3.5.3 follow-up (PR #192)`.
14. `ios/TaigiKeyboardTests/CandidateProcessorTests.swift:9` — same fix.

**Why the local audit missed these**: the seven-dimension scan only knows the explicit smell list under Dim D ("D9.x", `parser.rs`, `nfd_preprocessed`, `feedback_jvm_test_jni_compat.md`); the v3.5.3-follow-up-mislabelled-as-v3.5.4 class slipped through because no `v3.5.4` literal appears in the smell list. Codex's `git log -S` cross-reference against `add2252` was the disambiguating step. Adding `v3.5.4 (when used to attribute v3.5.3-followup work)` to the Dim D smell list would catch this class going forward.

**Scope-drift correction** (Codex flagged): both r2 and r3 contained "Next slice = v3.5.4 Composing" prose that violates `feedback_no_future_planning.md`. Removed from both reports in this branch.

### P3 — Dim G (memory hygiene) — RESOLVED

`MEMORY.md:42` cited `feedback_changelog_timing.md` which did not exist on disk. Recreated the file at `~/.claude/projects/-Users-alexsu-Workspace-taigikeyboard/memory/feedback_changelog_timing.md` with a frontmatter + body capturing the policy ("changelog updates land at release time only"). Re-running the index walk produces zero `MISSING:` lines.

## Deferred (out of scope for this PR)

A final tree-wide sweep surfaced three more `D9.1` / `Phase IV-B` references that are intentionally retained:

- `engine/README.md:31` — `1.75 → 1.85 in D9.1`, `1.85 → 1.86 in D9.2`. Factual record of the actual PRs that bumped MSRV. Removing the attribution would lose audit history.
- `engine/protos/proto/envelope.proto:32` (+ cascading regenerated `envelope.pb.swift:108`, `AppConfig.java:11, :263`) — "shape — minimal in D9.1, expands per slice". Editing the proto comment would force a regen of committed generated Swift + Java files, which is its own cleanup PR.
- `ios/.../RawNextWordPrediction.swift:20` + `android/.../RawNextWordPrediction.kt:14` + `docs/engine/rust-core-proto.md:261` — forward-looking `Phase IV-B` references that point at known future scope documented in `shared-core-readiness.md`. Renaming to v3.5.x cadence would cascade across that doc.

These don't violate Dim D's smell list (none cite deleted symbols or wrong versions), so they're not tracked as findings — just listed here for transparency.

## Dimensions that found nothing

A — Cross-language algorithm duplication: CLEAN.
B — Cross-crate Rust algorithm duplication: CLEAN.
C — Over-public Rust surface: CLEAN.
F — Bridge surface parity: CLEAN.

(All four were already CLEAN in baseline and r2; no changes required.)

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | 0 | 0 | 0 | clean |
| B — Cross-crate duplication | 0 | 0 | 0 | clean |
| C — Over-public surface | 0 | 0 | 0 | clean |
| D — Stale comments | 0 | 0 | 0 | 8 findings fixed (4 from r2 + 2 extras + 2 narrative) |
| E — Build-broken JVM tests | 0 | 0 | 0 | 2 tests retargeted to `ime.core.settings.InputMode` |
| F — Bridge surface parity | 0 | 0 | 0 | clean |
| G — Memory hygiene | 0 | 0 | 0 | `feedback_changelog_timing.md` recreated |

## Suggested next slice scope

None. The migration architecture is in the cleanest state since shared-core extraction began; this audit imposes no preconditions on the next slice.
