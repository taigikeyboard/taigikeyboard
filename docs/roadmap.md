# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `deferred items`
> **Status**: Active
> **Last updated**: 2026-05-24 (B2 docs-only round — split shipped-version detail into `docs/releases/<version>/plan.md` archive per OSS convention)

---

## Summary

- Single source of truth for **forward-looking** work items only.
- **Shipped release detail** lives in `docs/releases/<version>/plan.md` archives + `changelog/<version>.md` release notes.
- **Active items** (in-flight implementation plans) — currently empty; release scope/timing is user-gated per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].
- **Out of scope / deferred** items below are truly forward-looking (NOT items already shipped in a prior version).

---

## Released versions index

Authoritative list of shipped versions, newest first. Each row links to the release notes (`changelog/`) and the detailed implementation plan archive (`docs/releases/`) when one exists.

| Version | Ship date | Release notes | Detailed plan archive |
|---|---|---|---|
| v3.5.8 | 2026-05-20 (`61df3028`) | [`changelog/v3.5.8.md`](../changelog/v3.5.8.md) | [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) — Phase 0-9 + 整句 lattice + walker S1-S9 + continuous-compound-hyphen fix |
| v3.5.7 | 2026-05-08 | [`changelog/v3.5.7.md`](../changelog/v3.5.7.md) | — |
| v3.5.6 | 2026-04-27 | [`changelog/v3.5.6.md`](../changelog/v3.5.6.md) | — |
| v3.5.5 | 2026-04-12 | [`changelog/v3.5.5.md`](../changelog/v3.5.5.md) | — |
| v3.5.3 | 2026-03-22 | [`changelog/v3.5.3.md`](../changelog/v3.5.3.md) | — |
| v3.5.2 | 2026-03-08 | [`changelog/v3.5.2.md`](../changelog/v3.5.2.md) | — |
| v3.5.1 | 2026-02-25 | [`changelog/v3.5.1.md`](../changelog/v3.5.1.md) | — |
| v3.5.0 | 2026-02-12 | [`changelog/v3.5.0.md`](../changelog/v3.5.0.md) | — |
| v3.4.x | 2025-2026 | [`changelog/v3.4.*.md`](../changelog/) | — |
| v3.3.x | 2025 | [`changelog/v3.3.*.md`](../changelog/) | — |

**Authoritative ship-date list**: see Claude auto-memory `project_released_versions.md`. Detailed plan archives are added retroactively only when source material exists; older versions remain release-notes-only.

---

## Active / In-flight items

**Currently empty.** Per project rule `~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope, release scope/timing/tag are user-gated. New active items land here only when a user has explicitly scoped them.

**v3.5.9 refactor track status** (informational, not a release commitment): the [v3.5.9 refactor track](https://github.com/siansiansu/taigikeyboard/pulls?q=is%3Apr+is%3Aclosed+v3.5.9) has shipped Tier-A engine refactor + 13 Tier-B platform refactor PRs (#301-#326) as behavior-neutral changes on `main`. Remaining items (B5 `TextInputManager` decompose, B9 cross-platform VM-state pair) are user-explicit-scope-gated. Whether/when to cut a v3.5.9 release tag is a user decision; no release commitment is implied by this paragraph.

---

## Out of scope / deferred (truly forward-looking)

The items below are **forward-looking deferred candidates**, NOT items already shipped in a prior version. (v3.5.8-era items that read like deferred candidates but actually shipped — e.g. `整句 lattice + walker`, `continuous compound-hyphen`, `Phase 9 user-freq plumb` — live in [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md), not here.)

### Borrow librime spelling-algebra (full pipeline)

**Source**: 2026-05-05 librime architecture comparison. **Status**: deferred. v3.5.8 §Phase 1b confirmed N/A — fused toneless key is already provided by upstream `dictionary/common/notone.py::remove_tone()` in the CSV stage. The full librime pipeline (declarative `spelling_rules.toml` + Rust rule engine + FST trailer `derivation_type_u8` / `form_u8` two-byte fields + ranking credibility multiplier) remains a candidate for a future round. Detail in the v3.5.8 archive [§Phase 1b](releases/v3.5.8/plan.md#phase-1b--na-fst-多音節-fused-toneless-變體-key--已由上游-notone-stage-提供).

### Back-edit / undo / cursor 任意位置編輯

v3.5.8 ships forward-only commit; backspace is limited to "pop committed segment". A future round may add arbitrary-cursor editing if dogfood demonstrates need. Scope unscheduled per `feedback_no_unilateral_release_scope`.

### FST trailer 兩位元組 (`derivation_type_u8` / `form_u8`)

Original roadmap Item 2 + Item 3 step 1 design (pre-v3.5.8). v3.5.8 instead uses explicit `consumed_span` and does not need the trailer. If a future round adopts the full spelling-algebra pipeline above, the trailer ships in lockstep.

### Continuous-input nextword bigram integration (S4)

Originally proposed in v3.5.8 [§整句 lattice + walker S4](releases/v3.5.8/plan.md#整句-lattice--walker-v358-must-solve實作中). Status post v3.5.8: YAGNI-deferred. Bigram in continuous-input is feasible architecturally but the v3.5.8 dogfood did not surface a need (1-3 word phrases pass via the unigram + length-bias walker). A future round may revisit if dogfood signals otherwise.

### MOE NailCandidate dual-cursor UX / `consumed_start`

Original v3.5.8 S2 Codex pre-impl Q1c chose option (ii) — engine-only walker, no per-segment user-clickable mid-span. If a future round adopts forward-and-back commit semantics, `consumed_start` is the API extension point. v3.5.8 archive [§整句 lattice + walker S2](releases/v3.5.8/plan.md#整句-lattice--walker-v358-must-solve實作中) records the design rationale.

### G1b full mid-sentence custom-dict edges

v3.5.8 S6 ships G1a (whole-buffer custom-dict match into slot-0 walker). G1b (any mid-sentence custom substring matched via synthesized non-syllabifier lattice edges) is YAGNI-deferred. Detail in v3.5.8 archive [§整句 lattice + walker S6](releases/v3.5.8/plan.md#整句-lattice--walker-v358-must-solve實作中).

### UserVoc / LearnedVoc split

MOE-style separation of manually-managed vs auto-learned user-frequency entries. v3.5.8 ships a single `user_frequency.db`. A split would mirror MOE Taigi APK's `mainstream-ime-comparison.md` §70 pattern; deferred until a dogfood need surfaces.

---

## Stub redirects for legacy section anchors

The following section anchors previously lived in this file; v3.5.8 archive [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) now hosts them. Inbound code comments citing `§ Phase N` / `§ 整句 lattice + walker S<N>` resolve via these stubs.

### Phase 0 / Phase 1 / Phase 1b / Phase 2 / Phase 3 / Phase 4 / Phase 5 / Phase 6 / Phase 7 / Phase 8 / Phase 9

See [`docs/releases/v3.5.8/plan.md` § 實作 Phases](releases/v3.5.8/plan.md#實作-phases).

### Phase 9 R2 Q3.a / Phase 9 sort_key formula / Phase 9 跨平台常數表 / Phase 9 回歸守護矩陣 / Phase 9 R3 / Phase 9 R1

See [`docs/releases/v3.5.8/plan.md` § Phase 9 — Continuous-input ranking 修復 + 主流 IME 對齊 (FINALIZED 2026-05-11)](releases/v3.5.8/plan.md#phase-9--continuous-input-ranking-修復--主流-ime-對齊-finalized-2026-05-11).

### 整句 lattice + walker (S1-S9)

See [`docs/releases/v3.5.8/plan.md` § 整句 lattice + walker](releases/v3.5.8/plan.md#整句-lattice--walker-v358-must-solve實作中).

### 連續輸入 compound-hyphen / 連續輸入 Option A

See [`docs/releases/v3.5.8/plan.md` § 連續輸入 compound-hyphen](releases/v3.5.8/plan.md#連續輸入-compound-hyphenv358-§102-option-adogfood-修復).

### 刻意不採用 / 最佳實踐對齊

See [`docs/releases/v3.5.8/plan.md` § 最佳實踐對齊](releases/v3.5.8/plan.md#最佳實踐對齊-rules--ime-主流) — the v3.5.8 plan's explicit non-goals (librime full pipeline / RIME SchemaYAML / neural LM / MOE Nail UX / etc.) remain non-goals for future rounds unless a user explicitly re-scopes them.

### Active item v3.5.8 / Phase 5 / Phase 6 / Phase 9

See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) header + the corresponding `§ Phase N` sections.

### 回歸守護矩陣 / 既有 dogfood 矩陣 / Verification (release-level)

See [`docs/releases/v3.5.8/plan.md` § Verification](releases/v3.5.8/plan.md#verification-release-level) + [§ 回歸守護矩陣](releases/v3.5.8/plan.md#回歸守護矩陣每-pr-跑91-為-acceptance-主).

### Phase 排序 + 狀態追蹤

See [`docs/releases/v3.5.8/plan.md` § Phase 排序 + 狀態追蹤](releases/v3.5.8/plan.md#phase-排序--狀態追蹤-cross-pr-handoffauthoritative).

---

## Per-round gates (process invariants, project-wide)

These gates apply to every coding round, regardless of release. Authoritative source: `~/.claude/rules/round-workflow.md`. Key points:

- Each round = own branch + PR + Codex sandwich pre + post + `Skill(simplify)`.
- Touched-target test scope by default (e.g. `cargo test -p <crate>`, `./gradlew :module:test`).
- Cross-platform parity-correction rounds must merge both platforms in lockstep.
- Doc-only rounds qualify as admin tier — direct-to-main allowed.
- iOS `pbxproj` is user-only (per `rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance (per v3.5.9 Tier-A spec, archive forthcoming).

---

## Closed phases / shipped audits

Historical archive entries live in Claude auto-memory `project_phase_archive.md`:

- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (PRs #212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (PRs #227-#231).
- v3.5.8 連續輸入 — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for the full implementation plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in the Active / In-flight items section above. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry here. -->
