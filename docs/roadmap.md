# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `deferred items`
> **Status**: Active
> **Last updated**: 2026-05-25 (Phase 0 admin PR — D = TPS 三索引 plan drafted; entry under Active / In-flight items)

---

## Summary

- Single source of truth for **forward-looking** work items only.
- **Shipped release detail** lives in `docs/releases/<version>/plan.md` archives + `changelog/<version>.md` release notes.
- **Active items** (in-flight implementation plans) — currently: `v3.5.9 D = TPS 三索引` (plan drafted 2026-05-25, awaiting USER kick-off for first impl PR). Release scope/timing is user-gated per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].
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

### v3.5.9 D = TPS 三索引 (POJ + TL + TPS first-class in lattice / walker)

**Source**: USER-scoped 2026-05-25 (Phase 0 admin PR). Evaluation memo (2026-05-20) destination = C (三索引). Sequencing originally B-first → C; B 雙索引 prerequisite already shipped via PRs #307-#311.
**Status**: plan drafted, awaiting USER explicit kick-off for first impl PR (C-0). Scope / timing / release-tag user-gated per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].

**Goal**: TPS first-class user mode — `tps:` FST key family parallel to existing `tl:` / `poj:` families. Removes:

- `is_tps` short-circuit at `engine/composing/src/dispatch.rs:183` + `engine/composing/src/continuous.rs:890,923,982`
- `tps_or_mapped_to_er` runtime branch at `engine/lexicon/src/search.rs:91,133`
- `tps_to_tl` canonicalize chain at `engine/lexicon/src/classification.rs:20,77`

Mode-axis (Input + Key + FST) becomes three-layer symmetric across TL / POJ / TPS; new modes (e.g. additional script) become pure data axes (~+250 LOC) rather than architecture changes (~+1200 LOC).

**Prerequisite already shipped**: B 雙索引 (POJ first-class lattice) = SHIPPED via PRs #307-#311 (B-0c `5b836a80` / B-1 `9b365c9c` / B-2 `f2a4f4e1` / B-3+B-4 `ad085b3b` / B-7 `7b9604af`). The evaluation memo's original "C-2 POJ continuous first-class" line item = subsumed by B-2 PR #309. Remaining D scope = C-0 / C-1 / C-3a / C-3b / C-4 / C-5.

**Phase shape — 6 PR** (Codex sandwich per PR unless flagged otherwise):

| # | Phase | Scope | LOC est. | Risk |
|---|---|---|---|---|
| 1 | **C-0** | `dictionary/build/`: `dictionary_records.py` + `merge_csv.py` add 3 cols `tps_num` / `tps_notone` / `tps_abbrev` (with TPS-aware `derive_tps_*` helpers referencing `phonetics::is_tps_initial` + `tps.rs:43-62` tone-mark inventory); `create_fst.py:124-127` emits `tps:` family (3 forms). **dict.bin / FST size POC measurement go/no-go gate**. dict.bin schema 版號 + engine startup verify (rollback strategy). | ~+150 build, ~+30 engine | LOW |
| 2 | **C-1** | `engine/lexicon/src/key_normalizer.rs:34` `KeyMode::Tps => format!("tps:{normalized}")` replaces the `tl:` fall-through; `engine/lexicon/src/classification.rs:20,77` `if contains_tps(raw) { tps_to_tl(raw) }` retired (mode-based instead). | ~+30 engine | LOW |
| 3 | **C-3a** | TPS er↔or 方言變體 (Risk Register §8.6 L2) upmoved to build pipeline — dual-emit `tps:<er>` + `tps:<or>` same rowid (USER chose A always-on). `engine/protos/proto/lexicon.proto:118,129` `SearchRequest.tps_or_mapped_to_er` marked obsolete (wire-compat retained, runtime ignored); `engine/lexicon/src/search.rs:91,133-145` runtime branch + expansion logic removed. | ~+100 build / -50 engine | MED |
| 4 | **C-3b** | TPS continuous first-class — `engine/composing/src/shadow.rs::mode_key_prefix(Tps) => "tps"`; `build_shadow_lattice(.., Tps)` walks `SyllableInventory::contains_in(Tps, ..)` against `syllables.fst` `tps:` family (provided by C-0 emit); `engine/composing/src/syllabifier/tps.rs::valid_span_endings` aligned to TL syllabifier signature and plugged into `build_lattice`; `engine/composing/src/dispatch.rs:181-183` removes char-based `let is_tps = contains_tps(raw);` in favour of `mode == InputMode::Tps`; `engine/composing/src/continuous.rs:890,923,982` `is_tps` branches removed (TPS walks shared walker path, inventory family is the only switch); `assemble_candidates` signature drops `is_tps: bool` param. | ~+200-250 engine | **HIGH** (v3.5.8-hot composing path, non-Tier-A safety umbrella per eval memo §10) |
| 5 | **C-4** | Rename `engine/composing/src/continuous.rs:203 render_roman_for_mode` (function name misleading — only POJ ASCII → display-glyph rewrite `oo→o͘` / `nn→ⁿ`, NOT TL→POJ canonicalize retirement). Suggested rename: `apply_poj_display_glyphs` + doc clarification + caller / test rename. Behavior-neutral. | ~+10 engine | LOW (mechanical rename — sandwich skipped per `~/.claude/rules/round-workflow.md` carve-out) |
| 6 | **C-5** | `engine/composing/tests/golden_fetch_at_pos.rs` extends TPS continuous golden cases (per 硬約束 §9 #6, `UPDATE_GOLDEN` disabled — no shipped case overwritten); POJ continuous golden re-pinned if drift; `engine/lexicon/tests/` adds `tps_abbrev_parity` + `tps_notone_parity` (mirrors `poj_notone_parity.rs` against full dict.csv). On-device dogfood per `~/.claude/rules/code-review-rules.md §9` qualitative — per-keystroke composing (TPS abbrev / notone / full+tone) + continuous + L2 er↔or dual-emit verification. | tests | LOW (test / dogfood) |

**Best practices alignment** (direct prior art, verified via cavecrew-investigator deep-dive 2026-05-25):

- First-initial abbrev (`tps_abbrev`): `references/rime-moetaigi/tsuim.yaml:60` per-syllable algebra rule `abbrev/^(.)(.)[12357]$/$1/` + librime `src/rime/algo/algebra.cc` build-time prism expansion + Taigi's existing `tl_abbrev` / `poj_abbrev` build-time precedent (`dictionary/build/merge_csv.py:241-242 extract_abbrev`).
- Tone-free match (`tps_notone`): `references/rime-moetaigi/tsuim.yaml:56-58` `abbrev/^(.+)[12357]$/$1/` + librime `derive/^(.+)[0-9]$/$1/` + Taigi's `tl_notone` / `poj_notone` precedent (`merge_csv.py:239-240 remove_tone`).
- Build-time emission vs librime runtime regeneration: chosen for mobile-keyboard latency (mmap FST lookup is O(log n) per query; runtime regex expansion would add per-keystroke ms cost).
- Tagged-single-FST multi-family (`tl:` / `poj:` / `hanzi:` / `tps:` in one `dictionary.fst`): existing pattern from B-2 PR #309 (`engine/composing/src/shadow.rs:17` documents the design), TPS adds no new infrastructure.

**Deliberately not adopted**:

- librime runtime algebra expansion (per-query regenerate via `Script::Merge`) — Taigi keeps static build-time emit per existing `tl_*` axis precedent.
- librime abbrev / fuzzy `-ln 2` ranking penalty — Taigi equal-ranks abbrev with full input (USER intent is shortcut, not typo correction); scorer is mode-blind per B-2 grep verification.
- McBopomofo abbrev — N/A (Mandarin single-platform, full-syllable only; reference for DAG / unigram scoring patterns elsewhere but not for abbrev).
- L2 er↔or two-FST variant + runtime post-filter (Risk Register §8.6 L2 options B / C) — USER chose A always-on; one FST, dual-emit at build time.
- Tone-axis as runtime fuzzy matcher — Taigi treats tone-free as a build-time variant key (`tps_notone` col), symmetric with `tl_notone` / `poj_notone`.

**USER-answered forks** (2026-05-25):

1. L2 er↔or toggle policy = **A always-on** (build-pipeline dual-emit).
2. PR sharding = **拆細** (6 PR, 200-500 LOC each, Codex sandwich per PR, low regression risk).
3. C-4 `render_roman_for_mode` scope = **rename + doc clarification** (behavior-neutral; function is misnamed, not obsolete).

**Open forks awaiting USER** (not pre-decided in plan):

1. dict.bin / FST size POC pass threshold (C-0 measures actual bytes / mmap profile / iOS 32 MB cap; USER picks go / no-go gate; suggested default: FST < +25% size, dict.bin < +15%, iOS keyboard memory headroom intact).
2. Release tag / timing / version assignment (v3.5.9 / v3.5.10 / v3.6 / other) — user-gated per `feedback_no_unilateral_release_scope`.
3. Whether to run in parallel with reactive-mode dogfood — C-3b touches v3.5.8-hot composing path; suggested sequential to avoid multi-source conflict on the hot path.

**硬約束 § 9 satisfaction** (from `memory/project_v359_triple_index_eval.md`): mode-axis three-layer (Input + Key + FST) ✓ — Walker / Scorer remain mode-blind; user-history key keeps hanji-優先 fallback (no surface-form split) ✓; dict.bin POC go/no-go in C-0 ✓; `enum InputMode` already in place from B-0c ✓; MOE-audit #5 split-point = (a) same as TL per B-2 USER answer ✓; S0 golden expansion with `UPDATE_GOLDEN` disabled in C-5 ✓; dict.bin schema 版號 + startup verify in C-0 ✓.

**Detail**: full plan including per-PR sandwich receipts placeholder, Risk Register expansion, and cross-memory linkbacks ([[project_v359_triple_index_eval]] / [[project_v359_b_plan_v3]] / [[project_v359_backlog_handoff]]) lives in Claude auto-memory `project_v359_d_tps_triindex_plan.md` (local-only, not git-tracked per `~/.claude/CLAUDE.md` auto-memory convention).

---

**v3.5.9 refactor track status** (informational, not a release commitment): the [v3.5.9 refactor track](https://github.com/siansiansu/taigikeyboard/pulls?q=is%3Apr+is%3Aclosed+v3.5.9) has shipped Tier-A engine refactor + Tier-B platform refactor PRs (#301-#332) as behavior-neutral changes on `main`. Whether / when to cut a v3.5.9 release tag is a user decision; no release commitment is implied by this paragraph.

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
