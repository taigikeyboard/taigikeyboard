# Implementation Plan — Lexicon Refactor (active)

> **Branch**: `refactor-core-engine`
> **Source**: `docs/reports/lexicon-refactor-plan.md`
> **Started**: 2026-04-17
> **Active stage**: Stage 0 (Baseline)

This document tracks the **currently active stage** of the refactor. The full multi-stage plan lives in `docs/reports/lexicon-refactor-plan.md` §8.

---

## Stage 0 — Baseline

**Goal**: lock current behaviour and fix asset/init prerequisites BEFORE any structural change. Per the per-stage workflow (`docs/reports/lexicon-refactor-plan.md` §8): Codex review → simplify scan → fix all findings → implement → re-simplify → manual test → commit.

**Success criteria**:
- All hot-zone characterization tests written and green (4 categories: golden snapshot, init idempotence, migration, clock-injected ranking).
- Engine docs accurate and consistent with code.
- Build pipeline (`dictionary/build/`) audited against `docs/engine/binary-format.md`.
- Asset freshness no longer gated solely on `versionCode`.
- `AssetBootstrap` extracted (or equivalent ownership clarification) before any service split.
- Cross-platform fixture corpus committed.

| ID | Item | Status | Notes |
|---|---|---|---|
| 0-A | Codex independent plan review | ✅ done | 32 findings, integrated into plan |
| 0-B | Simplify scan of 12 hot-zone files | ✅ done | ~50% false positives — verified individually |
| 0-C | Write `docs/engine/binary-format.md` (P0-B) | ✅ done | + startup sentinel assertion still TODO |
| 0-D | Fix engine doc drift (`trie.md`, `nextword.md`) | ✅ done | POJ→TL location + schema v4 |
| 0-E | Apply Codex findings to plan | ✅ done | Plan v0.2 |
| 0-F | Build-pipeline audit (P0-C) | ⏳ pending | Read `dictionary/build/` 01–11 against `binary-format.md` §1–§4 |
| 0-G | Asset freshness fix (P0-D) | ⏳ pending | Add `build_ts` / hash gate to Android `LexiconService.copyAssetsIfNeeded` + `TrieService.getTriePath` |
| 0-H | `AssetBootstrap` extraction (P0-E) | ⏳ pending | Single owner of asset copy; both `LexiconService` and `NextWordService` depend on it |
| 0-I | Hot-zone characterization tests — Android (P0-A) | ⏳ pending | All 4 categories; 6 hot-zone files |
| 0-J | Hot-zone characterization tests — iOS (P0-A) | ⏳ pending | All 4 categories; 6 hot-zone files |
| 0-K | Cross-platform fixture corpus (P0-F) | ⏳ pending | `tests/fixtures/cross-platform/` shared by both |
| 0-L | Startup sentinel assertion (debug only) | ⏳ pending | Round-trip a known key through trie + binary reader |
| 0-M | Resolve open questions §9 Q3, Q4, Q7, Q8 | ⏸ blocked on user | See below |

### Tests required (per category)

**(a) Golden snapshot** — input → expected output JSON. Snapshot the *current* behaviour even if mildly buggy; document deviations as "to fix in Stage N" rather than fixing during snapshot.

**(b) Init idempotence** — `service.init()` × 2 sequential, × 2 concurrent, × 2 after `service.deinit()`. Verify no double-creation, no leaked file handles, no schema duplication.

**(c) Migration paths** — for each schema version transition:
- user_association v0 → v3 → v4 (UNIQUE constraint widening + prev_tl column)
- custom_dictionary v1 → v5 (verify each step's data preservation)
- Synthetic old-version DB on disk → service init → assert post-migration shape.

**(d) Clock-injected ranking** — replace direct `Date()` / `System.currentTimeMillis()` calls in `PredictionScorer` and `CandidateProcessor.calculateScore` with injected `currentTimeMs: Long` parameter. Tests pass fixed timestamps to make decay/recency deterministic. **This category requires a small surface-preserving edit to make code testable** — that edit is part of Stage 0.

### Outstanding questions (block Stage 0 completion)

These were updated in `docs/reports/lexicon-refactor-plan.md` §9 after Codex review:

- **Q3** (`taigi-converter` submodule status) — needed before any `TaigiPhonetics` reduction; not blocking until Stage 2 (P2-C).
- **Q4** (scoring constants — locked spec or per-platform?) — needed before Stage 6 (NextWord split).
- **Q7** (build-pipeline edits in this branch?) — needed before 0-F / 0-G can land any fix.
- **Q8** (asset-bootstrap platform priority — iOS or Android first?) — needed before 0-H starts.

### Suggested next concrete action

After user answers Q7 + Q8: start **0-F** (read-only build-pipeline audit) — zero-risk, builds confidence about format invariants before touching asset code.

---

## Conventions for Stage 0 commits

Per `feedback_branching` (target develop) + `feedback_changelog_timing` (no changelog mid-stage):

- One commit per logical sub-item (0-F, 0-G, …) — easier to revert.
- Commit messages: `refactor(lexicon-stage0): <what>` for code; `docs(lexicon-stage0): <what>` for docs.
- No CHANGELOG entry until Stage 6 ships.
- PR opens against `develop` once 0-A through 0-L all complete and tests green on both platforms.

---

## When Stage 0 completes

Move to Stage 1 (Symmetry & cleanup). Update this file's "Active stage" header, archive Stage 0 to `docs/reports/lexicon-refactor-plan.md` as "completed history".
