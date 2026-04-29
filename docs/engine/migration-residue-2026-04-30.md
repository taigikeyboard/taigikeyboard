# Migration Residue Audit — 2026-04-30

> Branch: `main` (HEAD `955bf62`)
> Run by: `/migration-residue` skill
> Last slice: `v3.5.3` + `v3.5.3-followup` (v3.5.4 = Composing slice prep, not yet started)
> Prior reports: `migration-residue-2026-04-29.md` (baseline) → `-r2.md` → `-r3.md` (PASS on cleanup branch).

## Verdict

- **0 P1** findings
- **0 P2** findings
- **0 P3** findings

Overall: **PASS** — main is in the same clean state r3 reached on the cleanup branch. PRs #193 (skill), #194 (r2/r3 fixes), #195 (iOS visibility / Android `stringDispatch` param name) merged without reintroducing any of the fixed smells.

## Dimension results

### A — Cross-language algorithm duplication: CLEAN

No platform helper shares a name with a Rust `pub fn` outside of thin `RustEngineBridge.<op>` wrappers. iOS `Phonetics/` directory was deleted in v3.5.1; Android dictionary mirrors deleted in v3.5.3-followup (PR #192).

### B — Cross-crate Rust algorithm duplication: CLEAN

Rust workspace has 13 `pub fn` items distributed across `dispatch`, `ranking`, `phonetics` with no name collisions. The historical `nfd_preprocessed` collision was resolved in PR #191 (`taigi_unicode_base_form` vs `trie_key_unicode_form`).

### C — Over-public Rust surface: CLEAN

13 `pub fn` items, all reachable across crate boundaries (dispatch routes, CLI consumers, FFI seams, integration tests). No tightening recommended.

### D — Stale doc comments: CLEAN

All surviving `D9.x` / `Phase IV-B` references match r3's "deferred — intentionally retained" list:

- `engine/README.md:31` — MSRV bump audit history (1.75 → 1.85 in D9.1, 1.85 → 1.86 in D9.2). Factual record.
- `engine/protos/proto/envelope.proto:18,32,37,53` — `reserved` markers + history of field-shape evolution. Editing forces regen of generated Swift+Java.
- `engine/protos/proto/phonetics.proto:24,25,162,163` — same class: `reserved` markers + oneof migration history.
- Source comments describing what D9.x **did** (`RustEngineBridge.swift:9`, `InputNormalizer.swift:6`, `CharacterInputPipeline.kt:12`, `CandidateOverlayView.kt:308`, etc.) — historical attribution, not stale recommendations.
- `dispatch::process_request` mentions across `engine/dispatch/`, `engine/swift-ffi/`, `engine/android-jni/` — current canonical FFI entry, not the deleted `phonetics::api::process_request`.

No `v3.5.4` literal appears in any production source file (only in historical audit reports, which are by-design archival).

### E — Build-broken JVM tests: CLEAN

Full-FQN suffix walk against `android/app/src/main` declarations: zero `BROKEN:` imports across `android/app/src/test/**.kt`. Zero tests reference `RustEngineBridge` (no `UnsatisfiedLinkError` risk). The two test files retargeted in PR #194 (`SuggestionCaseTransformerTest`, `ComposingStateTest`) still resolve cleanly post-merge.

### F — Bridge surface parity: CLEAN

Dispatchable methods enumerated from `oneof method` blocks: 18 phonetics ops + 1 lexicon op = 19. Per-op camelCase grep against both bridges: every op present on iOS `RustEngineBridge.swift` AND Android `RustEngineBridge.kt`. PR #195's `stringDispatch` param rename did not break parity.

### G — Memory + doc hygiene: CLEAN

`MEMORY.md` index walked against on-disk files in the per-repo memory dir: zero `MISSING:` lines.

## Summary table

| Dim | P1 | P2 | P3 | Notes |
|---|---|---|---|---|
| A — Cross-lang duplication | 0 | 0 | 0 | clean |
| B — Cross-crate duplication | 0 | 0 | 0 | clean |
| C — Over-public surface | 0 | 0 | 0 | 13 `pub fn`, all justified |
| D — Stale comments | 0 | 0 | 0 | retained refs match r3 deferred list |
| E — Build-broken JVM tests | 0 | 0 | 0 | full-FQN walk clean |
| F — Bridge surface parity | 0 | 0 | 0 | 19 ops × 2 bridges, all matched |
| G — Memory hygiene | 0 | 0 | 0 | index↔disk clean |

## Suggested next slice scope

None — audit is purely diagnostic. Composing slice planning proceeds on its own track.
