# Migration Residue Audit — 2026-04-30 (r2)

> Branch: `phase4b/v3.5.4-composing-slice` (HEAD `b9d2cb2`)
> Run by: `/migration-residue` (manual walk; skill tool model-invocation disabled)
> Slice under audit: **v3.5.4 Composing** (PR #197, 18 commits)
> Prior reports: `migration-residue-2026-04-30.md` (pre-slice baseline, 0 P1/P2/P3).

## Verdict

- **0 P1** findings
- **0 P2** findings
- **2 P3** findings (historical doc artifacts; intentionally retained per skill rules)

Overall: **PASS**. Migration architecture is clean. The two P3 findings are by-design historical wording inside planning/audit docs — modifying them would falsify the historical record.

## Dimension results

### A — Cross-language algorithm duplication: CLEAN

18 Rust `pub fn` items distributed across `phonetics` (14), `composing` (1: `dispatch::handle`), `ranking` (1), `dispatch` (1), with no platform-side mirror. Anti-pattern grep (per-codepoint scans `for char in unicodeScalars` / `for (c in table)`) returns zero hits on both platforms.

iOS `Input/Composing/` directory now holds 2 files (manager + delegate, 264 LOC total; was 4 files / 507 LOC). Android `composing/` still bundles autocomplete-orchestration adjacencies (`TaigiAutocompleteService`, `EnglishAutocompleteService`, `UserFrequencyService`) which are v3.5.5 NextWord scope per cadence — NOT residue.

### B — Cross-crate Rust algorithm duplication: CLEAN

Three normalization-class helpers (`taigi_unicode_base_form`, `preprocess_for_normalize_tone`, `convert_nasal_double_n`) all live in `engine/phonetics/`; no parallel implementation in `engine/composing/` or `engine/ranking/`. Composing's `derived::derived_display` calls `phonetics::api::normalize_tone` directly per plan §3.2a (one-way dep).

### C — Over-public Rust surface: CLEAN

18 `pub fn` items, all justified:

- `dispatch::process_request`, `process_candidates` — FFI entry points.
- `phonetics::dispatch::handle`, `composing::dispatch::handle` — domain-crate dispatch entries called by `engine/dispatch`.
- `phonetics::api::*` (14 items) — workspace-public Rust API. New v3.5.4 additions (`parse_input_mode`, `preprocess_for_normalize_tone`, `normalize_tone`, `contains_tps`) are called by `composing::derived` in another crate — `pub` is correct.
- `phonetics::syllable::strip_tone_mark`, `normalize_to_tl` — re-exported at crate root, called by integration tests.
- `phonetics::tl::to_tl`, `phonetics::poj::to_poj`, `phonetics::normalization::taigi_unicode_base_form` — used by `phonetics::dispatch` + workspace tests.

### D — Stale doc comments: 2 P3 findings (historical preservation)

#### D1 (P3) — `docs/engine/composing-slice-plan.md:425` + `composing-slice-audit.md:95,109,...`

These are the **planning artifacts** for THIS slice. They contain forward-looking statements like "Doc updates in `rust-core-proto.md` + `v3.5.3-cleanup-audit.md` — D9.3 / v3.6.0 wording → v3.5.4" — describing work that has now been completed (commit 12). Modifying these would falsify the planning record (per `feedback_no_future_planning.md` rationale: planning docs are time-stamped artifacts).

**Action**: NONE. Retain as historical record.

#### D2 (P3) — `docs/engine/v3.5.3-cleanup-audit.md:42` "Composing (D9.3) stays deferred to v3.5.4"

Accurate at the time the v3.5.3 cleanup audit was written (Composing was deferred to v3.5.4 from that vantage). Now v3.5.4 is in flight on this branch; the doc still says "deferred". Fix would be "shipped in v3.5.4 (PR #197)". But:

- The doc is a v3.5.3-era audit snapshot.
- Per r3 deferred-by-design list, similar audit-doc snapshots are intentionally preserved.
- After PR #197 merges, this line could optionally be updated as a v3.5.5-cycle housekeeping pass.

**Action**: NONE in this slice (would falsify v3.5.3-era record). Defer to a future hygiene round if desired.

#### Verified clean (no findings):

- `engine/README.md:31` MSRV history (`1.75 → 1.85 in D9.1, 1.85 → 1.86 in D9.2`) — factual audit record.
- `engine/protos/proto/envelope.proto` `// CMD_COMPOSING reserved for D9.3` was removed in commit 1 (now `CMD_COMPOSING = 2;`).
- `engine/protos/proto/composing.proto` D9.x references intentionally absent (clean slate).
- `docs/engine/rust-core-proto.md` §8 promoted to AS-IMPLEMENTED (commit 12).
- `rules/cross-platform-alignment.md:97` mentions "Phonetics + Composing" — accurate.

### E — Build-broken JVM tests: CLEAN

148 main FQNs walked; zero `BROKEN:` imports across `android/app/src/test/`. Zero `RustEngineBridge` references in test sources (no `UnsatisfiedLinkError` risk). The two test files retargeted in PR #194 (`SuggestionCaseTransformerTest`, `ComposingStateTest` — but the latter was deleted entirely in commit 10) all resolve.

### F — Bridge surface parity: CLEAN

Dispatchable method count: 18 phonetics + 12 composing + 1 lexicon = **31 ops total**.

- iOS `RustEngineBridge.swift`: 36 `static func` (>= 31 dispatchable + diagnostics + lifecycle helpers).
- Android `RustEngineBridge.kt`: 21 `@JvmStatic` annotations (some methods may not need the annotation depending on call site shape; per-op camelCase grep is the authoritative check).

Per-op parity grep against all 12 composing ops: zero gaps. Each `composingStart / composingAppend / composingAppendHyphen / composingReplaceLast / composingDeleteBackward / composingCommitDerived / composingCommitRaw / composingSelectSuggestion / composingCommitPreeditThenInsertExternal / composingReset / composingSetSelectedCandidateIndex / composingQueryState` exists on both bridges.

### G — Memory + doc hygiene: CLEAN

`MEMORY.md` index walked against on-disk files in the per-repo memory dir: zero `MISSING:` lines.

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | 0 | 0 | 0 | clean |
| B — Cross-crate duplication | 0 | 0 | 0 | clean (one-way `composing → phonetics` edge) |
| C — Over-public surface | 0 | 0 | 0 | 18 `pub fn`, all justified |
| D — Stale comments | 0 | 0 | 2 | historical planning/audit artifacts (retained) |
| E — Build-broken JVM tests | 0 | 0 | 0 | full-FQN walk clean |
| F — Bridge surface parity | 0 | 0 | 0 | 31 ops × 2 bridges, all matched |
| G — Memory hygiene | 0 | 0 | 0 | index↔disk clean |

## Cohesion check (user brief #1: "rust 模組內聚力高,與其他模組依賴少")

`engine/composing/` deps: `protos`, `phonetics`, `log`, `thiserror`, `once_cell` (workspace shared). One domain-crate dep (`phonetics`), unidirectional. No `ranking`, `dispatch`, `cli`, FFI seam coupling — clean separation.

`engine/composing/src/` layout:
- `lib.rs` (façade + Send assertion)
- `api.rs` (`Engine` + `Intent` + `Phase` types)
- `dispatch.rs` (proto decode entry)
- `handle.rs` (`EngineHandle` singleton)
- `transition.rs` (pure state machine)
- `derived.rs` (display computation)

Each file owns a single concern. Cohesion: **HIGH**. Goal met.

## Suggested next slice scope

None — audit imposes no preconditions on v3.5.5 (NextWord) planning.
