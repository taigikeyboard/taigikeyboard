# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `deferred items`
> **Status**: Active
> **Last updated**: 2026-05-26 (D = TPS 三索引 SHIPPED 6/6 PRs — C-0~C-5; awaiting v3.5.9 release tag, user-gated)

---

## Summary

- Single source of truth for **forward-looking** work items only.
- **Shipped release detail** lives in `docs/releases/<version>/plan.md` archives + `changelog/<version>.md` release notes.
- **Active items** (in-flight implementation plans) — none currently in flight. `v3.5.9 D = TPS 三索引` SHIPPED 6/6 PRs 2026-05-26 (C-0/C-1/C-3a/C-3b/C-4/C-5) and is in user dogfood; entry retained below under Recently shipped (pre-tag) for traceability. Release scope/timing is user-gated per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].
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

## Recently shipped (pre-tag)

### v3.5.9 D = TPS 三索引 (POJ + TL + TPS first-class in lattice / walker) — SHIPPED 6/6 PRs 2026-05-26

**Source**: USER-scoped 2026-05-25 (Phase 0 admin PR). Evaluation memo (2026-05-20) destination = C (三索引). Sequencing originally B-first → C; B 雙索引 prerequisite already shipped via PRs #307-#311.
**Status**: SHIPPED 6/6 PRs on `main` 2026-05-25 → 2026-05-26. User dogfood in progress. v3.5.9 release tag user-gated per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].

**Goal achieved**: TPS first-class user mode — `tps:` FST key family parallel to existing `tl:` / `poj:` families. Retired:

- `is_tps` short-circuit at `engine/composing/src/dispatch.rs` + `engine/composing/src/continuous.rs` — retired in C-3b (PR #337).
- `tps_or_mapped_to_er` runtime branch at `engine/lexicon/src/search.rs` — retired in C-3a (PR #336); `SearchRequest.tps_or_mapped_to_er` proto field 5 kept on wire as OBSOLETE.
- `tps_to_tl` canonicalize chain at `engine/lexicon/src/classification.rs` — retired in C-1 (PR #335).

Mode-axis (Input + Key + FST) is now three-layer symmetric across TL / POJ / TPS; new modes (e.g. additional script) become pure data axes (~+250 LOC) rather than architecture changes (~+1200 LOC).

**Prerequisite already shipped**: B 雙索引 (POJ first-class lattice) = SHIPPED via PRs #307-#311 (B-0c `5b836a80` / B-1 `9b365c9c` / B-2 `f2a4f4e1` / B-3+B-4 `ad085b3b` / B-7 `7b9604af`). The evaluation memo's original "C-2 POJ continuous first-class" line item = subsumed by B-2 PR #309. Remaining D scope = C-0 / C-1 / C-3a / C-3b / C-4 / C-5.

**Phase shape — 6 PR shipped** (Codex sandwich per PR unless flagged otherwise):

| # | Phase | PR / commit | Shipped scope | Risk classification |
|---|---|---|---|---|
| 1 | **C-0** | PR #334 (`5f207d3f`) | Build pipeline emits `tps:` FST family (`tps_num` / `tps_notone` / `tps_abbrev` cols + `create_fst.py` writer). POC dict.bin / FST size deltas accepted (`+52.8% / +5.07 MB` on dictionary.fst). Approach A = literal Bopomofo UTF-8 keys. dict.bin schema 版號 + engine startup verify added. | LOW |
| 2 | **C-1** | PR #335 (`c32a3fe4`) | `key_normalizer::build` `KeyMode::Tps => "tps:"` flip + `normalize_tps_key_body` (strip `-`/space + tone-8 `U+02D9` → `U+0307`); `classify_input` `tps_to_tl` identity passthrough retired; `search.rs` er↔or branch annotated DEAD POST-C-1 (retired in C-3a). | LOW |
| 3 | **C-3a** | PR #336 (`846095aa`) | TPS er↔or 方言變體 upmoved to build pipeline — dual-emit `tps:<er>` + `tps:<or>` same rowid (USER chose A always-on) via Bopomofo-level `apply_or_dialect_variant` (ㄜ→ㄛ). CSV +3 cols `tps_num_var` / `tps_notone_var` / `tps_abbrev_var`. `SearchRequest.tps_or_mapped_to_er` proto field 5 kept on wire as OBSOLETE; runtime branch + expansion logic removed. | MED |
| 4 | **C-3b** | PR #337 (`7ef27025`) | TPS continuous first-class — `phonetics::InputMode::Tps` enum variant + `parse_input_mode("tps")`; `shadow::mode_key_prefix(Tps) => "tps"` walks shared shadow/lattice path; `SyllableInventory::contains_in(Tps, ..)` against `syllables.fst` `tps:` family; new TPS `valid_span_endings_lowered` BFS-with-inventory variant gates ending via inventory (prevents malformed Bopomofo OOV); `dispatch::handle_fetch_at_pos` drops char-based `is_tps`, upgrades mode via `if contains_tps(raw) { Tps } else { parse... }`; `assemble_candidates` drops `is_tps: bool` param. Legacy `build_keys_tps` + `phonetics::tps_to_tl` `tl:`-fold short-circuit retired. | HIGH (v3.5.8-hot composing path, shipped clean) |
| 5 | **C-4** | PR #339 (`c0f88243`) | `render_roman_for_mode` → **`recase_tl_as_poj_display`** (USER-driven 2nd-pass rename — sibling-API alignment with `phonetics::api::tl_display_to_poj_display`; final form differs from initial `apply_poj_display_glyphs` proposal). Function continues to apply POJ display-glyph rewrite (`oo→o͘` / `nn→ⁿ`) without TL→POJ canonicalize; behavior-neutral. | LOW (mechanical rename — sandwich skipped per `~/.claude/rules/round-workflow.md` carve-out) |
| 6 | **C-5** | PR #340 (`0b36fb8d`) | Test-only capstone — `engine/lexicon/tests/tps_abbrev_parity.rs` (full-CSV runtime↔build-pipeline parity, 133k rows + `tps_abbrev_var` coverage gate); `tps_notone_parity.rs` extended (`tps_notone_var` 1107 nonempty + coverage gate); `golden_fetch_at_pos.rs` adds TPS family fixtures + `tps_notone_taigi` / `tps_continuous_taigikhipuann`; renames `tps_walker_excluded` → `tps_no_inventory_match` (semantic alignment post-C-3b); `phonetics::tps::to_zhuyin` `pub(crate)` → `pub` + re-export `tl_numeric_token_to_tps`. Zero production-code behavior change. | LOW (test / dogfood) |

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

**USER-answered forks** (2026-05-25, all resolved in shipped PRs):

1. L2 er↔or toggle policy = **A always-on** (build-pipeline dual-emit) — landed C-3a.
2. PR sharding = **拆細** (6 PR, 200-500 LOC each, Codex sandwich per PR) — sharding held; all 6 PR shipped clean.
3. C-4 `render_roman_for_mode` scope = **rename + doc clarification** — landed PR #339; USER-driven 2nd-pass rename produced `recase_tl_as_poj_display` (not the initial `apply_poj_display_glyphs` proposal — see [[feedback_naming_root_in_domain]]).

**Resolved post-ship**:

1. dict.bin / FST size POC = accepted (dictionary.fst `+52.8% / +5.07 MB`); C-0 (PR #334) shipped Approach A literal Bopomofo UTF-8 with explicit USER sign-off on the POC measurement.
2. Release tag / timing / version assignment — still user-gated per `feedback_no_unilateral_release_scope`; not pre-committed.
3. Reactive-mode dogfood sequencing — observed; C-3b shipped clean before reactive rounds resumed.

**硬約束 § 9 satisfaction** (from `memory/project_v359_triple_index_eval.md`, all satisfied in shipped PRs): mode-axis three-layer (Input + Key + FST) ✓ — Walker / Scorer remain mode-blind; user-history key keeps hanji-優先 fallback (no surface-form split) ✓; dict.bin POC go/no-go decided in C-0 ✓; `enum InputMode::Tps` added in C-3b (was previously TL-only at B-0c) ✓; MOE-audit #5 split-point = (a) same as TL per B-2 USER answer ✓; S0 golden expansion with `UPDATE_GOLDEN` disabled in C-5 ✓; dict.bin schema 版號 + startup verify in C-0 ✓.

**Detail**: full plan including per-PR sandwich receipts placeholder, Risk Register expansion, and cross-memory linkbacks ([[project_v359_triple_index_eval]] / [[project_v359_b_plan_v3]] / [[project_v359_backlog_handoff]]) lives in Claude auto-memory `project_v359_d_tps_triindex_plan.md` (local-only, not git-tracked per `~/.claude/CLAUDE.md` auto-memory convention).

---

**v3.5.9 refactor track status** (informational, not a release commitment): the [v3.5.9 refactor track](https://github.com/siansiansu/taigikeyboard/pulls?q=is%3Apr+is%3Aclosed+v3.5.9) has shipped Tier-A engine refactor + Tier-B platform refactor PRs #301-#332 plus the D = TPS 三索引 PRs #333-#340 (Phase 0 admin + C-0/C-1/C-3a/C-3b/C-4/C-5) as behavior-neutral / additive changes on `main`. Whether / when to cut a v3.5.9 release tag is a user decision; no release commitment is implied by this paragraph.

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
- iOS `pbxproj` is user-only (per `.claude/rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance (per v3.5.9 Tier-A spec, archive forthcoming).

---

## Closed phases / shipped audits

Historical archive entries live in Claude auto-memory `project_phase_archive.md`:

- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (PRs #212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (PRs #227-#231).
- v3.5.8 連續輸入 — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for the full implementation plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in the Active / In-flight items section above. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry here. -->
