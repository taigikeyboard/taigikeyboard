# v3.5.9 Refactor — Implementation Design Spec (S0 / A2 / A1)

---

## ⭐ 2026-05-20 Post-v3.5.8 Review Amendments (AUTHORITATIVE — read before everything below)

> This block supersedes the conflicting parts of §0–§7. The original sections
> stay as the historical first-hand trace. Produced from a pre-impl design
> review (first-hand re-verify against current `main` `61df3028` + Codex
> ANALYSIS-ONLY sandwich). The spec was written against `87489fd1`; v3.5.8 then
> **shipped as its own tag `61df3028`** (NOT folded) and dogfood PRs
> #295/#288/#297/#298/#299/#300 landed after it. `dispatch.rs` 2176→2674 LOC.
> Review verdict: **SAFE-TO-IMPLEMENT-AFTER-THESE-AMENDMENTS**, not a structural
> redesign. The 5-stage thesis, D1/D2/D3 findings, and A3 reframe all survive
> the drift; the A2 seam contract, the D1 proof, and the S0 matrix do not and
> are corrected here. **This round's scope = S0 only** (maintainer 2026-05-20).

### B1 (BLOCK) — A2 seam contract is materially stale, not line-drift
Current `handle_fetch_at_pos` runs a **POJ-only presentation pass after the
slot-0 prepend** (added by #299): for each candidate `cand.roman =
render_roman_for_mode(&cand.roman, mode)` then `dedupe_rendered_continuous(&mut
candidates)`, *then* `with_continuous`. Anchors `61df3028`:
`render_roman_for_mode` `dispatch.rs:710`, `dedupe_rendered_continuous`
`dispatch.rs:743`, the pass itself ~`dispatch.rs:282`/`:298`.
**Amendment to §2.1 / §2.5**: the seam is a **6-step** contract, byte-preserved
in this order:
1. `keys.is_empty()` → `is_tps` empty / else `fetch_via_lexicon_partial(raw,
   &freq_map, now_ms, &custom, is_poj)`.
2. else `c = fetch_via_lexicon(&keys, raw_len, &freq_map, now_ms, &custom)`.
3. per-candidate recase loop (`recase_roman` over `consumed_span`; presentation
   `roman` only).
4. if `!is_tps` and `fetch_walker_slot0(..)` → `Some(slot0)`: span-aware
   `retain` on `(roman,hanji,consumed_span)` then `insert(0, slot0)`.
5. **POJ presentation pass** (new): if `mode == POJ`, `for cand in &mut
   candidates { cand.roman = render_roman_for_mode(&cand.roman, mode) }` then
   `dedupe_rendered_continuous(&mut candidates)`.
6. `with_continuous(snapshot, ContinuousResponse { candidates:
   candidates.into_iter().map(raw_to_proto_candidate).collect() })` (the
   variable is `candidates` after step 5, not `c`).
`render_roman_for_mode` + `dedupe_rendered_continuous` move into
`composing::continuous` as a named **"post-assembly POJ presentation pass"** —
NOT into `composing::shadow` (they operate on assembled `RawCandidate.roman`,
incl. partial-prefix + walker slot 0). `dedupe_rendered_continuous` is
*first-wins* and runs **after** the step-4 span-aware retain-dedupe — both must
be preserved; do not collapse them.

### B2 (BLOCK) — D1 fold proof not discharged under concurrency
`build_shadow_lattice(raw, inv, is_poj)` is pure and called at
`dispatch.rs:386` (`build_keys_tl_with_inventory`) and `dispatch.rs:776`
(`fetch_walker_slot0`) — the double-build finding holds. But the byte-identical
proof in §2.2 ("no state mutation between the two `with_state` scopes") is **not
discharged**: there are multiple independent `LexiconHandle::with_state` scopes
(`dispatch.rs:337/:758/:1604/:1675`); `EngineHandle::install` can atomically
replace the singleton between them (`handle.rs:69,92`), and `with_state` holds
the mutex for only one closure (`handle.rs:101`). S0 (single-threaded hermetic)
cannot catch this.
**Amendment to §2.2**: state the proof obligation explicitly — on a mobile IME
`EngineHandle::install` runs only at keyboard/app startup, never concurrently
with a composing `FetchAtPos`; the D1 fold is byte-identical **under that
lifecycle invariant**, and additionally *strengthens* snapshot consistency
(one snapshot instead of two potentially-different ones under a hypothetical
concurrent reinstall). The implementer MUST re-grep for any
`EngineHandle::install` reachable concurrently with `handle_fetch_at_pos` and
record the result in the S0/A2 PR body; it is a stated invariant, not a
hand-wave "pure fn = identical".

### B3 (BLOCK) — S0 golden matrix must expand before freeze
§1.5's matrix has POJ-diacritic but **not** the behaviors #298/#299/#300 added:
POJ-mode ASCII toneless canonicalize (`is_poj`, `dispatch.rs:177,1302`),
POJ-mode continuous render `oo/nn`→`o͘/ⁿ` (`dispatch.rs:710`), post-render
dedupe (`dispatch.rs:743`), ≥6-syllable OOV-vs-dict (`OOV_PER_CHAR_PENALTY`
`cost.rs:201`). Without these, A1/A2 extraction can silently regress
#298/#299/#300 with an empty golden diff — defeating S0's purpose.
**Amendment to §1.5**: add golden cases, each grounded in an existing in-tree
test (same convention as the rest of §1.5 — derive the exact raw/custom/dict
fixture rows from the cited test at implementation time, do not invent
dictionary content):
- **POJ-ASCII toneless** — `chiah` and `goa`; fixture/raw pattern from
  `dispatch.rs::tests::canonicalize_poj_shadow_poj_ascii_chiah_folds_ch_to_ts`
  (and the `chhia`/`oa`/`oe`/`eng`/`ek` sibling test) (`dispatch.rs:2149+`).
- **POJ render `oo/nn`→`o͘/ⁿ`** — raw + dict row from
  `dispatch.rs::tests::render_roman_for_mode_poj_rewrites_oo_and_nn`
  (`dispatch.rs:2554`); the golden pins the rendered `roman`, confirming the
  #299 post-assembly pass survives extraction.
- **POJ post-render dedupe collision** — the custom-vs-dict shape from
  `dispatch.rs::tests::dedupe_rendered_continuous_drops_post_render_collision_keeping_first`
  (`dispatch.rs:2616`): a custom-POJ and a dict-TL row that render equal after
  the POJ pass, exercising the first-wins `dedupe_rendered_continuous`.
- **≥6-syllable long-OOV-vs-dict** — `ginalangtsiahpngbesai` (RC0 repro;
  equivalent dict-coverable ≥6-syllable input acceptable) for the
  `OOV_PER_CHAR_PENALTY` path.

POJ-mode cases set the input mode in the per-case `AppConfig` (mirror how
`is_poj` is derived: `parse_input_mode(config.input_mode) == Poj`).

### S4 (SHOULD) — A1 shadow boundary: `is_poj` + `build_partial_prefix_key_tl`
The shadow pipeline now threads `is_poj`: `build_keys_tl` `dispatch.rs:336`,
`build_keys_tl_with_inventory` `:381`, `build_shadow_lattice` `:441`,
`custom_toneless_key` `:1205`, `fetch_via_lexicon_partial` `:1664`.
**Correction to the pre-impl review's own draft**: `fetch_via_lexicon`
(`dispatch.rs:1597`) did **not** gain `is_poj`. **Amendment to §3.1**: `is_poj`
is part of the `composing::shadow` API contract; also move
`build_partial_prefix_key_tl` (`dispatch.rs:1640`, same
canonicalize/hyphen/tone pipeline) into the shadow module.

### S5 (SHOULD) — D2 rule-list export scope
`phonetics::normalize_to_tl` is the 9-rule body at `syllable.rs:56`;
`apply_normalize_to_tl_with_offsets` is the split 10-pass mirror at
`dispatch.rs:1388` (order verified identical, byte-identical proof in §3.2
holds). The separate `.replace("nn","")` at `syllable.rs:71` is in
`is_stop_tone`, a different helper. **Amendment to §3.2**:
`NORMALIZE_TO_TL_RULES` export scopes to `normalize_to_tl` **only** — it must
not absorb the `is_stop_tone` replace.

### S6 (SHOULD) — A3 constant inventory stale; conclusion strengthened
`UNKNOWN_SYLLABLE_DECAY` was removed by #298; `OOV_PER_CHAR_PENALTY: f64 = 1e10`
is now a Cluster 2 constant (`cost.rs:201`, `const _: () = assert!(>= 1e5)`
`:211`). **Correction**: §3A.1's "each with an adjacent `const _: () =
assert!`" overstates — current asserts cover only `CORPUS_TOTAL_FREQ` (`:125`),
the length biases (`:147`), and `OOV_PER_CHAR_PENALTY` (`:211`);
`CUSTOM_EFFECTIVE_FREQ` and `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE` have **no**
adjacent assert. The "two clusters, do NOT merge" conclusion is **strengthened**
(Cluster 2 still co-located after dogfood churn). Cluster 1 stays in `ranking`,
`pub`/re-exported as today (`score.rs:58,122`, `lib.rs:31`) — do not relocate
or narrow.

### S7 (SHOULD) — A2 sequencing: pure-move first, defer D7/D8
The A2 surface is now larger (POJ render/dedupe + `is_poj`) and adjacent to the
abbrev-collision guard (`continuous.rs:521,834`). **Amendment to §2.4 / §5**:
A2's first PR is a **pure relocation only** (the 6-step seam + D1 fold + D3
honest type). D7 (`ContinuousFetchCtx`) and D8 (`fetch_candidates_for_endings`
demotion) become a **follow-up PR after the A2 golden diff is empty**, not
bundled into the first cut.

### N1 (NIT) — banner/title corrected
v3.5.8 shipped at tag `61df3028` as its own release; v3.5.9 is the next
version. The "v3.5.8-folded / no separate version" framing (old title, §0
"Scope folded") is overtaken; S0-first sequencing is unaffected. The §0
precondition ("v3.5.8 fully merged incl. OOV edge-cost fix") is **satisfied**.

### Confirmed still-true (no amendment)
D3: `score` is wire field 5 on `CandidateMessage` (`composing.proto`:
`CandidateMessage` starts at `:307`, `score = 5` at `:312`);
slot-0 still emits `score: -(path.cost as f32)`, `frequency:0`, `bitmask:0`
(`dispatch.rs:1091`) — honest type must preserve the exact negated-cost bridge.
S0 structural facts: `engine_install_lock` `parity.rs:35`, `EngineHandle::
install` `parity.rs:249`, `LexiconPaths::validated(..,"",1)`
`parity.rs:241/296/303`, `build_minimal_install_fixture` `parity.rs:418`,
`build_inventory_from_pairs` `syllables_fst.rs:186`. §7's anchor table is the
`87489fd1` snapshot — the `61df3028` anchors in B1–S6 above are the current
re-spot; still re-grep any symbol not listed here before coding.

---

> **Type**: Implementation design spec (first-hand, structural). Companion to
> `docs/reports/2026-05-18-v3.5.9-refactor-plan-draft.md` (the *what/why*); this is the
> *how*, for the three highest-leverage slices.
> **Created**: 2026-05-18 against `main` `87489fd1` (NOT the plan draft's `f828c89e`).
> **Status — DESIGN ONLY, DO NOT EXECUTE YET**: produced during the v3.5.8 dogfood
> spare-budget window as **read-only** preparation. No code change, no PR, no Codex
> round, no S0 golden freeze, no tag until v3.5.8 behavior is frozen on `main`
> (maintainer-gated; see `memory/project_v359_refactor_plan_draft.md` ⚠️ timing block).
> **Line-number caveat**: every `file:line` here is a 2026-05-18 anchor that **WILL
> drift** once v3.5.8's remaining dogfood-bug-fix rounds land. The *contracts and
> proofs* below are structural and survive the drift; the line table in §7 must be
> re-spotted (re-`grep` the function names) before any slice is implemented. Trust the
> function/contract, never the number.
> **Scope (corrected — see ⭐ amendment N1)**: v3.5.8 SHIPPED as its own tag
> `61df3028` (2026-05-20), WITHOUT the refactor. v3.5.9 is the next, separate
> version. The §0 precondition is satisfied. The old "folded into v3.5.8 / no
> separate version" framing below is overtaken; S0-first sequencing is
> unaffected. **This round's scope = S0 only** (maintainer 2026-05-20).

---

## 0. The 5-stage pipeline (first-hand re-trace, current main)

`dispatch::handle` → `handle_fetch_at_pos` runs, for a non-TPS `Phase::Continuous`:

1. **Shadow normalize** — `to_ascii_lowercase` → `canonicalize_poj_shadow` (POJ→TL
   spelling + offset map) → `build_hyphen_shadow` (drop `-`, second offset map) →
   composed `shadow_to_raw_end`. Pure. Tangled in `dispatch.rs`.
2. **Lattice build** — `lattice::build_lattice(&shadow, inv, MAX_SYLLABLES)`. The one
   clean module.
3. **Per-edge/key lexicon resolution** — span-local: `fetch_via_lexicon` →
   `record_to_candidate` → `calculate_continuous_score`. Walker: `fetch_walker_slot0`'s
   edge provider → custom-precedence OR `best_candidate_for_key` → same
   `record_to_candidate` → same `calculate_continuous_score` OR OOV-synth.
4. **Two objectives** — (4a) candidate list slots 1..N: 8-dim `SortKey`,
   higher-better. (4b) slot 0: `walk_best` min-`Σ edge_cost`, lower-better; then
   no-dict carve-out + per-edge recase.
5. **Splice + encode** — recase span-local → `synth_consumed_span` gate →
   span-aware `retain`-dedupe slot0 vs list on `(roman,hanji,consumed_span)` →
   `insert(0, slot0)` → **[⭐ B1: + POJ render + post-render dedupe pass when
   `mode==POJ`, `dispatch.rs:282-303`]** → `raw_to_proto_candidate`.

Confirmed first-hand: the walker is **layered on top of** the span-local primitives
(`fetch_walker_slot0`'s provider reuses `best_candidate_for_key` →
`record_to_candidate` → `calculate_continuous_score`), then applies a second
min-cost objective. They are two stacked decision levels over one shared resolution
primitive — **not** two unrelated scorers. The debt is that stages 1, 3-seam, 4b, 5
are all tangled in `dispatch.rs`. Refactor = modularize + name the two levels;
**do not touch the objectives** (unification stays Tier C / v3.6+).

---

## 1. S0 — golden `FetchAtPos` snapshot harness

**Goal**: pin the full proto candidate vector for a representative input matrix so
every later behavior-neutral slice's acceptance = empty golden diff.

### 1.1 Hard structural facts (grounded)

- `dispatch::handle(FetchAtPos)` resolves candidates through the **process-global
  `lexicon::EngineHandle` singleton** (`LexiconHandle::with_state`), NOT through
  injected fixtures. The `lexicon/tests/` direct-call `build_fixture` helpers
  (`span_local_fetch.rs`, `user_freq_plumb.rs`) bypass the singleton and call
  `fetch_candidates_for_endings` directly — they do **not** exercise the proto path.
- The only existing pattern that drives real candidates through the singleton is
  `EngineHandle::install(LexiconPaths)` (`engine/lexicon/tests/parity.rs`,
  `engine_install_lock()` + `build_minimal_install_fixture`).
- **`parity.rs` passes `""` for `syllables_fst`** → `syllable_inventory = None` →
  `build_keys_tl` returns empty → only the partial-prefix path runs. The S0 harness
  **must build a real `syllables.fst`** (canonical builder pattern:
  `engine/lexicon/tests/syllables_fst.rs` `build_inventory_from_pairs`, file-path
  variant) covering every syllable the input matrix needs.

### 1.2 Location & shape

- New file `engine/composing/tests/golden_fetch_at_pos.rs` (zero collision; cargo
  compiles each `tests/*.rs` as its own crate). It lives in **composing** because it
  must call both `composing::dispatch::handle` *and* `lexicon::EngineHandle::install`
  (composing depends on lexicon; the reverse is impossible — this is the documented
  seam constraint in `dispatch_continuous.rs`).
- The hermetic fixture builders (`build_tkdb_v2`-equivalent, synthetic FST,
  empty-`TKWA` association, syllables FST) are **re-implemented in this test file**
  (mirroring `parity.rs` + `syllables_fst.rs`) — `lexicon/tests/common` is a
  lexicon-test-private module not visible to composing tests. Accepted duplication
  (same pattern `user_freq_plumb.rs` already uses).
- `engine_install_lock()` (a `OnceLock<Mutex<()>>`) is **mandatory** — the singleton
  is process-global; parallel test threads otherwise swap state mid-test (a
  reproduced flake, documented in `parity.rs`).

### 1.3 What is pinned (wire fields only)

`raw_to_proto_candidate` is a 1:1 copy of `RawCandidate` → `CandidateMessage`. Pin
**only the 9 wire fields**, one stable line per candidate, vector order significant:

```
{consumed_span_start}|{end}|{syllable_count}|{form}|{mode}|{display_text}|{roman}|{hanji|⌀}|{score:.4}
```

- **Exclude** internal `frequency` / `bitmask` / `recency_rank` / `coverage_kind` —
  not on the wire; pinning them would over-constrain a behavior-neutral refactor and
  is impossible through the proto boundary anyway.
- **`score` quantized to `{:.4}`** — deliberate. The codebase's own score-equality
  contract is `abs() < 1e-4` (`span_local_fetch.rs`); bit-exact f32 would be
  *stricter than the engine's own behavioral definition* and would flag
  behavior-neutral FP reassociation as a false regression. Documented as an
  intentional behavioral-equivalence definition, not a precision compromise.
- `now_ms: 0` in every default case → `recency_rank == 1` uniformly (kills
  wall-clock nondeterminism). The user-frequency case uses fixed literal
  `now_ms` + fixed `last_used_ms` (mirror `user_freq_plumb.rs`).
- Ordering is deterministic: `sort_by_cached_key` on `SortKey` whose final tiebreak
  `stable_idx` is stamped from pre-sort `enumerate()` (pinned by an existing test,
  Codex PR #262 r3216153007); slot 0 is an explicit prepend. Given a fixed fixture,
  output order is reproducible.

### 1.4 Record/verify

One `#[test]`, full matrix, one golden file
`engine/composing/tests/golden/fetch_at_pos.golden` addressed via
`CARGO_MANIFEST_DIR`. `UPDATE_GOLDEN=1` env var = re-record (write); default =
assert. Single test holds the install lock once and iterates cases on fresh
`Engine::new()` per case (matches the production model: per-request engine state,
read-only shared singleton post-install). Section headers `## name :: raw` localize
which case drifted.

### 1.5 Representative matrix (each `raw` already exercised by an existing test)

> ⚠️ **Superseded by ⭐ amendment B3** — this matrix MUST be expanded with POJ
> ASCII (`chiah`/`goa`), POJ render (`o͘`/`ⁿ`), POJ post-render dedupe
> collision, and a ≥6-syllable OOV input before freeze.

TL toneless multi-syll (`tsua`), TL toneless long-reach (`taigikhipuann`), TL
numeric single (`tsua7`), TL numeric multi (`tai1bak4`), POJ-diacritic (`tâi-uân`),
TPS walker-excluded (`ㄉㄧㄠˊㄨㄢˊ` — must fully syllabify; TPS partial-prefix
returns empty), custom-dict (`taigi` + `custom_entries`), MIXED (`iausi`), TAILO
no-hanji (`li`), user-freq-boosted (`tai` + `frequency_entries` + literal `now_ms`),
all-OOV / partial-prefix (`g`), trailing-hyphen (`tai-`, suppresses slot-0 synth),
case-sensitive (`HitTui`), headline ranking (`taiuantaigi`). The single install
fixture seeds the **union** of all rows these existing tests use (all bitmask
`1u16<<11` = rank-neutral). Cross-reference fixture rows from
`span_local_fetch.rs` / `user_freq_plumb.rs` at implementation time.

### 1.6 Two-step (no contradiction with "land S0 first")

(1) Scaffold harness + builders + input matrix + assertion mechanism — pure new
file, zero collision, can be written now as *design* (this doc) and as *code* only
once v3.5.8 is unfrozen-safe to add a PR. (2) **Freeze the golden values only after
v3.5.8 behavior is frozen on `main`** — values captured earlier encode known-buggy
output and would (correctly) be invalidated by the remaining dogfood fixes,
destroying the ability to distinguish refactor-breakage from bug-fix.

---

## 2. A2 — `composing::continuous` + the `assemble_candidates` seam

**Highest "A breaks B" risk slice. Extract-only. Acceptance = empty S0 golden diff.**

### 2.1 The seam's exact behavior contract (must be byte-preserved)

> ⚠️ **Superseded by ⭐ amendment B1** — the seam is a **6-step** contract on
> current main (the 3-step version below omits the #299 POJ render +
> post-render dedupe pass). Use B1's step list, not this one.

`assemble_candidates` = the `let candidates = if keys.is_empty() { … } else { … }`
block of `handle_fetch_at_pos`. Inputs: `raw`, `keys`, `raw_len`, `freq_map`,
`now_ms`, `custom`, `mode` (`parse_input_mode(config.input_mode)`), `is_tps`.
Output: ordered `Vec<RawCandidate>`. Exact order of operations to preserve:

1. `keys.is_empty()`:
   - `is_tps` → empty `Vec`.
   - else → `fetch_via_lexicon_partial(raw, &freq_map, now_ms, &custom)`.
2. else:
   - `c = fetch_via_lexicon(&keys, raw_len, &freq_map, now_ms, &custom)`.
   - per-candidate recase loop: for each `cand`, `recase_roman(&cand.roman,
     raw[cs..ce], mode)` over `cand.consumed_span` (presentation `roman` only;
     `display_text`/`hanji` untouched).
   - if `!is_tps` and `fetch_walker_slot0(...)` → `Some(slot0)`:
     `c.retain(|x| !(x.roman==slot0.roman && x.hanji==slot0.hanji &&
     x.consumed_span==slot0.consumed_span))` then `c.insert(0, slot0)`.
   - `c`.
3. `with_continuous(snapshot, ContinuousResponse { candidates:
   c.map(raw_to_proto_candidate) })`.

The dedupe **key** `(roman, hanji, consumed_span)` and the **after-recase,
before-prepend** ordering are load-bearing — any reordering changes visible output.
The relocated module preserves this verbatim; the seam is named, not redesigned.

### 2.2 D1 fold — build the shadow+lattice once (behavior-neutral perf win)

**Current**: `build_keys_tl(raw)` → `build_keys_tl_with_inventory` →
`build_shadow_lattice(raw, inv)` (call site 1). Then `fetch_walker_slot0(raw,…)` →
`build_shadow_lattice(raw, inv)` **again** (call site 2). The function doc claims
"constructed exactly once per fetch and the two consumers cannot drift" — true
*within each function*, **false across the two call sites**: it runs twice per
non-TPS `FetchAtPos` (canonicalize + hyphen-shadow + offset compose + lattice BFS,
all on the hot per-keystroke path).

**Fold**: hoist one `LexiconHandle::with_state` scope into the seam that (a) pulls
`inv`/`prefix`/`dict` once, (b) calls `build_shadow_lattice(raw, inv)` **once**, (c)
derives the left-anchored `keys` projection from that lattice (the exact loop body
of `build_keys_tl_with_inventory`), (d) passes the prebuilt
`(shadow, shadow_to_raw_end, lattice)` + `prefix`/`dict` into the relocated
`fetch_via_lexicon` and walker instead of letting the walker rebuild.

**Byte-identical proof**: `build_shadow_lattice` is **pure** — its output depends
only on `raw` and `inv` (no interior mutability, no I/O). Both current call sites
pass the *same* `raw` and the *same* `inv` (both read from
`state.syllable_inventory` of the same read-only post-install singleton; no
mutation occurs between the two `with_state` calls within one `FetchAtPos`). A pure
function called once and its result shared is, by definition, equal to calling it
twice with identical arguments. The left-anchored `keys` projection is the
unchanged loop over `lattice.edges()` filtered to `start == 0`. Therefore the fold
is provably behavior-neutral; S0 golden mechanically confirms it. **Proof
obligation at impl**: assert no lexicon-state mutation can occur between the two
former `with_state` scopes (re-grep for any `EngineHandle::install` / state-write
reachable from a `FetchAtPos` — there is none today; re-verify post-freeze).
**[⭐ B2 — superseded: this is NOT discharged by "pure fn". Multiple
independent `with_state` scopes exist and `EngineHandle::install` can atomically
swap the singleton between them; the fold is byte-identical only under the
stated mobile-IME lifecycle invariant (install at startup only, never
concurrent with a composing `FetchAtPos`) and additionally strengthens snapshot
consistency. Record the re-grep result in the PR body. See ⭐ B2.]**

### 2.3 D3 — honest slot-0 type (REFINED — narrower than the plan draft)

The plan draft §1A D3 says `score`/`frequency`/`bitmask` on the slot-0
`RawCandidate` are "dead-by-shape … never read". **First-hand correction**: slot 0
is `raw_to_proto_candidate`-encoded into `CandidateMessage`, and **`score` IS wire
field 5** — the platform receives slot-0's `score = -(path.cost as f32)`. Only
`frequency` and `bitmask` are truly dead (not on the wire; slot 0 is prepended,
never sorted, so `recency_rank`/`coverage_kind` are also sort-irrelevant but
`coverage_kind` is internal-only too).

D3 disposition (corrected): the honest type for the walker result must still
**produce the exact `score = -(path.cost as f32)` on the wire** to stay
behavior-neutral (S0 pins it, quantized). The "honest type" therefore = a walker
result struct that *names* `cost` and converts to the proto `score` at the seam via
the same `-(cost as f32)` bridge, with `frequency`/`bitmask` documented as
walker-N/A (or defaulted exactly as today: `0`/`0`). This is a *documentation +
type-name* clarification, **not** a value change. Do not "fix" the sign bridge —
it is the wire contract.

### 2.4 D6 / D7 / D8 fold-ins (behavior-neutral, while the seam is extracted)

> ⚠️ **Amended by ⭐ S7** — only **D6** (one-line doc fix) folds into A2's
> first PR. **D7** (`ContinuousFetchCtx`) and **D8** (`fetch_candidates_for_endings`
> demotion) are deferred to a **follow-up PR after the A2 golden diff is
> empty** — NOT bundled "while the seam is extracted".

- **D6** — `SortKey` self-doc: one comment says "seven-dimension", another
  "eight-dimension"; the struct has 8 fields (Item-10 `coverage_kind` prepended).
  One-line doc fix to "eight-dimension" consistently. No code change.
- **D7** — `#[allow(clippy::too_many_arguments)]` ×3 on
  `fetch_candidates_for_endings` / `fetch_candidates_for_keys` /
  `fetch_partial_prefix_candidates` (8 args, self-flagged "revisit if a builder
  pattern lands"). Introduce `ContinuousFetchCtx { prefix, dict, freq_map, now_ms,
  bitmask }` (exact field set TBD at impl by reading the three signatures
  post-freeze) bundling the invariant args; pure mechanical call-site rewrite,
  golden-diff-empty.
- **D8** — `fetch_candidates_for_endings` is now a test-only public API (production
  goes `dispatch → fetch_via_lexicon → fetch_candidates_for_keys`; it always passes
  `&[]` custom and is exercised only by tests). Demote to `#[doc(hidden)]` test
  seam (mirror the `build_keys_tl_with_inventory` pattern) — public-surface
  reduction, behavior-neutral.

### 2.5 Extraction boundary

Move into new `engine/composing/src/continuous.rs`: `fetch_walker_slot0` (whole,
incl. its custom-precedence map, the dict-hit / OOV-synth edge provider, the no-dict
greedy-longest carve-out, per-edge recase, `synth_consumed_span` gate) +
`fetch_via_lexicon` + `fetch_via_lexicon_partial` + the `assemble_candidates`
orchestration (seam §2.1). `handle_fetch_at_pos` shrinks to: phase/hanzi/position
guards → mode/freq_map/custom hoist → `assemble_candidates(...)` → `with_continuous`.
`continuous.rs` (lexicon crate) `record_to_candidate` / `best_candidate_for_key` /
`SortKey` stay where they are — A2 relocates the *composing-side* assembly, not the
lexicon resolution primitive.

---

## 3. A1 — `composing::shadow` module + D2 rule-list export

**Behavior-neutral. Gate on S0. High sensitivity (offset maps feed `raw_span` +
casing).**

### 3.1 Module boundary

> ⚠️ **Amended by ⭐ S4** — add `is_poj` to the shadow API contract and move
> `build_partial_prefix_key_tl` into the module; `fetch_via_lexicon` did NOT
> gain `is_poj`.

Move the pure shadow pipeline into new `engine/composing/src/shadow.rs`:
`build_shadow_lattice`, `canonicalize_poj_shadow`, `build_hyphen_shadow`,
`strip_ascii_tone_digits`, `apply_normalize_to_tl_with_offsets`,
`offset_aware_replace`, `greedy_longest_syllabification`, `span_min_syllable_count`,
`custom_toneless_key`, and the left-anchored projection helper extracted from
`build_keys_tl_with_inventory`. All are pure (depend only on `raw`/`span` + `inv` +
the lattice builder). Sensitivity: `shadow_to_raw_end` drives `raw_span` + recase;
S0's trailing-hyphen + POJ-diacritic + case-sensitive cases cover the off-by-one
risk on relocation.

### 3.2 D2 — single-source the normalize-to-TL rule chain

> ⚠️ **Amended by ⭐ S5** — `NORMALIZE_TO_TL_RULES` scopes to `normalize_to_tl`
> only; exclude the `is_stop_tone` `.replace("nn","")` (`syllable.rs:71`). Proof
> below still holds on current main.

**Current hand-mirror (verified first-hand, exact)**:

`phonetics::syllable::normalize_to_tl` = 9 chained `.replace`:
`ch→ts`, `ou→oo`, `o\u{0358}→oo`, `[\u{207f}\u{1d3a}]→nn`, `oa→ua`, `oe→ue`,
`eng→ing`, `ek→ik`, `oonn→onn`.

`dispatch::apply_normalize_to_tl_with_offsets` = 10 `offset_aware_replace` — the
char-class pass split into two single-pattern passes (`\u{207f}→nn`,
`\u{1d3a}→nn`), otherwise identical order + patterns. Guarded **only** by the
comment "Same order + same patterns … keeping the two in lockstep is a hard
prerequisite". A `phonetics` rule edit silently desyncs the offset-aware replay.

**D2 contract**: `phonetics` exposes the chain as ordered data:

```rust
// engine/phonetics/src/syllable.rs
pub const NORMALIZE_TO_TL_RULES: &[(&str, &str)] = &[
    ("ch","ts"), ("ou","oo"), ("o\u{0358}","oo"),
    ("\u{207f}","nn"), ("\u{1d3a}","nn"),
    ("oa","ua"), ("oe","ue"), ("eng","ing"), ("ek","ik"), ("oonn","onn"),
];
pub fn normalize_to_tl(text: &str) -> String {
    NORMALIZE_TO_TL_RULES.iter()
        .fold(text.to_string(), |acc, (f, r)| acc.replace(f, r))
}
```

`apply_normalize_to_tl_with_offsets` iterates the **same**
`NORMALIZE_TO_TL_RULES` via `offset_aware_replace` (it already uses the split
form, so the offset side is unchanged — only `normalize_to_tl`'s body changes).

**Byte-identical proof (D2)**: the only non-trivial equivalence is the current
`.replace(['\u{207f}','\u{1d3a}'], "nn")` (one pass, either char) vs. two
sequential entries (`\u{207f}→nn` then `\u{1d3a}→nn`). Equivalent because: (a)
neither source char occurs in the replacement `"nn"`, and (b) the two patterns are
disjoint and non-overlapping, so all-`\u{207f}`-then-all-`\u{1d3a}` =
both-in-one-pass (no reintroduction, order-independent). Every other rule is an
identical single `.replace`. `fold` over the list = the chained `.replace` calls by
associativity of left-to-right replacement. The offset-aware side already matches
the split list 1:1, so it consumes the exported list with **zero** behavior change.
Phonetics unit tests + S0's POJ-diacritic case (`tâi-uân`, exercises
`ou/o\u{0358}/oa`) confirm.

---

## 3A. A3 — cost/ranking constants (⚠️ REFRAMED vs plan draft)

**Behavior-neutral only as a same-crate grouping. The plan's "consolidate into one
module" is mis-framed — first-hand correction below, parallel to D3.**

### 3A.1 First-hand correction to plan draft §4 A3 / §6

> ⚠️ **Amended by ⭐ S6** — Cluster 2 inventory: drop `UNKNOWN_SYLLABLE_DECAY`
> (removed by #298), add `OOV_PER_CHAR_PENALTY` (`cost.rs:201`). "Each with an
> adjacent `const _: () = assert!`" overstates (only corpus/biases/OOV have
> one). "Do not merge" conclusion strengthened.

The plan draft says constants "straddle `score.rs` + `cost.rs`, SoT in doc comment
only" and proposes consolidating them "into one module". First-hand reading shows
there are **two independent single-source clusters with different, incompatible
contracts** — they must **not** be merged:

- **Cluster 1 — ranking, cross-platform invariant, `f32`, `pub`**
  (`engine/ranking/src/score.rs`): `BOOST_ALPHA: f32 = 0.1`, `MAX_BOOST: f32 = 5.0`,
  `CONTINUOUS_SOURCE_BITS` (table), `CONTINUOUS_DEFAULT_SOURCE_RANK: u8 = 5`,
  `RECENCY_WINDOW_MS`, `USER_FREQ_CAP`. These are **already** the documented
  single-source-of-truth and are an explicit **cross-platform invariant**: the
  doc-comments state "Platforms MUST NOT redefine — single source of truth per
  `rules/cross-platform-alignment.md` §3a". They are mirrored platform-side. So
  "SoT in doc comment only" is **wrong** — they are `pub` API + a contract.
  **Moving them out of `ranking` or narrowing `pub`→`pub(crate)` is NOT
  behavior-neutral — it breaks the platform-mirror contract.** They stay put.
- **Cluster 2 — composing/lattice walker model, engine-only, `f64`,
  `pub(crate)`** (`engine/composing/src/lattice/cost.rs`):
  `CORPUS_TOTAL_FREQ: f64`, `LETTER_COUNT_BIAS: f64 = 0.2`,
  `SYLLABLE_COUNT_BIAS: f64 = 0.2`, `UNKNOWN_SYLLABLE_DECAY: f64 = 10.0`,
  `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE: f64 = 0.0`,
  `CUSTOM_EFFECTIVE_FREQ: u32 = 2_000`, each with an adjacent compile-time
  `const _: () = assert!(…)` invariant. These are **already a co-located cluster**
  at the head of `cost.rs` — not "straddling". They are NOT cross-platform
  invariants (the walker is engine-only; no platform mirror).

The f32↔f64 boundary the plan flags is not a "watch out" — it is a **hard reason
the two clusters cannot be one module**: they are different numeric domains,
different crates, different visibility, different cross-platform status.

### 3A.2 Safe A3 scope (small, or no-op)

A3 is therefore **not** "merge ranking + cost constants". Behavior-neutral options,
in order of preference:

1. **Doc-only**: add a one-paragraph header to each cluster stating its
   single-source contract explicitly (Cluster 1 = cross-platform invariant, do not
   mirror-redefine; Cluster 2 = engine-only walker model, dogfood-tunable named
   constants). Zero code change. This is the genuinely useful, fully safe A3.
2. **Optional same-crate grouping** of Cluster 2 into a named submodule
   `engine/composing/src/lattice/cost/params.rs` (or `cost::params`), moving the
   current Cluster 2 constants **with each constant's adjacent `const _: () =
   assert!` where one exists** (per ⭐ S6: only `CORPUS_TOTAL_FREQ` /
   length-biases / `OOV_PER_CHAR_PENALTY` have one;
   `CUSTOM_EFFECTIVE_FREQ` / `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE` do not —
   "6 constants, each with an assert" is stale),
   `pub(crate)` and values/types **byte-identical**, re-exported so `edge_cost`'s
   call sites are unchanged. Pure intra-crate relocation; golden-diff-empty;
   `cargo test --workspace` green. Only do this if it demonstrably improves
   cohesion — Cluster 2 is already co-located, so the win is marginal (YAGNI check).
3. **Do not touch Cluster 1's location/visibility at all.**

A3 runs **last** in Tier-A (it is adjacent to `cost.rs`, the v3.5.8-hottest file)
and may legitimately collapse to option 1 (doc-only) after the re-spot.

## 3B. A4 — build-time `CORPUS_TOTAL_FREQ` guard

**Behavior-neutral iff it only fails builds/tests — must NOT recompute or alter the
runtime constant.**

### 3B.1 Current state (first-hand)

`CORPUS_TOTAL_FREQ: f64 = 12_910_574.0` is a baked literal (`cost.rs`), with:

- A compile-time `const _: () = assert!(CORPUS_TOTAL_FREQ > 184_694.0)` — asserts
  only the **lower bound** (keeps every `ln(1/p) > 0`), **not** that it equals the
  real dictionary sum.
- A test-side regeneration guard `assert_eq!(CORPUS_TOTAL_FREQ, MEASURED_SUM, …)`
  where `MEASURED_SUM` is **itself a hand-typed literal** in the test — so the
  current "guard" is a constant-equals-constant **arithmetic self-check**, exactly
  as the plan states. The doc says provenance = `Σ frequency` over
  `dictionary/output/dictionary.csv` (159 034 entries) = 12 910 574, measured
  2026-05-17, "recompute and update whenever the dictionary is rebuilt", and the
  bake-constant-not-runtime-scan decision is Codex S5 Q4 = option a.

### 3B.2 A4 design

Strengthen the regeneration guard from "hand-typed `MEASURED_SUM`" to "computed
from the actual dictionary the build produces", so a dictionary rebuild that
changes `Σ frequency` **fails the dict pipeline / a workspace test**, not silently
drifts. Hard constraints:

- A4 **only** adds a verifier. `CORPUS_TOTAL_FREQ` stays a baked literal consumed
  by `edge_cost`. A4 must **never** make the runtime cost model read a
  freshly-summed value — that would change the cost model = behavior change, out
  of scope (and re-opens the Codex S5 Q4 = option a decision).
- **Authoritative locus = the dictionary build pipeline** (where
  `dictionary/output/dictionary.csv` is canonically produced —
  `dictionary/build/*`, per the doc `create_dictionary_bin.py` neighbourhood):
  compute `Σ frequency` there and emit it into a small generated artifact (e.g.
  `dictionary/output/corpus_total_freq.txt`).
- A Rust cross-check (a `#[test]` in the `composing`/`lattice` cost tests, or a
  `build.rs`) reads that generated artifact **when present** and
  `assert_eq!(CORPUS_TOTAL_FREQ as u64, parsed_sum)`; **degrades gracefully**
  (skip with a clear message) when the artifact/csv is absent (minimal build
  contexts without the dictionary submodule). This makes the SoT the *pipeline*,
  not a hand-typed literal — the McBopomofo "build-time corpus computation"
  alignment the plan cites.
- Replace the hand-typed `MEASURED_SUM` literal with the parsed artifact value so
  there is exactly one computed source.

### 3B.3 Behavior-neutrality

`edge_cost` output is unchanged (same `CORPUS_TOTAL_FREQ` literal). S0 golden diff
empty. The only new failure mode is a **build/test failure** when the dictionary
and the constant disagree — which is the intended safety, not a behavior change.

## 4. Behavior-neutrality verification protocol (every slice)

1. S0 golden frozen on post-v3.5.8-freeze `main` is the baseline.
2. Each slice is extract-only / data-export-only — no logic edits.
3. Acceptance = `cargo test --workspace` green **and** `golden_fetch_at_pos`
   diff empty (no `UPDATE_GOLDEN`).
4. Per-round Codex sandwich (pre + post) + `/codex-pr-review` — but only **after**
   v3.5.8 freeze (no review churn during dogfood verification, per the timing
   directive).
5. If a golden diff is non-empty, the slice is **not** behavior-neutral — stop,
   do not `UPDATE_GOLDEN` to paper over it.

---

## 5. Proof obligations to re-verify at implementation (post-freeze)

- **§7 line table re-spot** — re-`grep` every function name; the numbers below are
  2026-05-18/`87489fd1` and will have drifted.
- **D1**: confirm no lexicon-state mutation is reachable between the two former
  `with_state` scopes within one `FetchAtPos` **[⭐ B2 — not "true today" by
  purity; it holds only under the mobile-IME install-at-startup lifecycle
  invariant. Record the re-grep + invariant in the PR body, not a hand-wave]**.
- **D3**: confirm `score` is still wire field 5 on `CandidateMessage` and slot-0
  still flows through `raw_to_proto_candidate` (don't let the "honest type" drop
  the `-(cost as f32)` wire value).
- **D7**: read the three 8-arg signatures post-freeze to fix the exact
  `ContinuousFetchCtx` field set (v3.5.8 may have changed an arg).
- **S0**: confirm `LexiconPaths::validated` arity + the syllables-FST builder
  signature haven't changed; ensure the matrix's syllables are all in the fixture
  inventory (TPS case included, TL-mapped).
- **A3**: re-confirm Cluster 1 is still `pub` + still doc-stamped as the
  `rules/cross-platform-alignment.md` §3a cross-platform invariant (do not relocate
  it); re-confirm Cluster 2 is still co-located + move each constant **with its
  adjacent `const _: () = assert!` where one exists** (⭐ S6: not every Cluster 2
  constant has an assert — `CUSTOM_EFFECTIVE_FREQ` /
  `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE` do not).
  Decide option 1 (doc-only) vs option 2 (same-crate grouping) by a YAGNI/cohesion
  check at that point.
- **A4**: re-confirm the dictionary pipeline entry point that produces
  `dictionary/output/dictionary.csv` and whether a generated-artifact handshake is
  feasible; confirm the test/`build.rs` degrades gracefully when the dict submodule
  is absent. Never let A4 feed a runtime-summed value into `edge_cost`.
- Re-read `docs/references/mainstream-ime-comparison.md` before re-asserting any
  "mainstream keeps one pipeline" framing in slice PR bodies.

---

## 6. Out of scope (do NOT smuggle in)

Per the plan draft §3/§6 and `feedback_no_unilateral_release_scope`: no objective
unification (Tier C / v3.6+), no collapsing span-local into the single walk, no
shared numeric score between `SortKey` and the walker, no new features. A1/A2/S0
are **relocation + data-export + a golden harness** only.

---

## 7. Current-main anchor appendix — WILL DRIFT, re-spot before impl

`main` `87489fd1`, 2026-05-18. **Do not trust these numbers at implementation
time — re-`grep` the function names.** Recorded only to make the first-hand trace
reproducible today.

| Symbol | File | Line (stale) |
|---|---|---|
| `handle_fetch_at_pos` | `engine/composing/src/dispatch.rs` | 135 |
| seam dedupe `.retain` / `.insert(0,…)` | `dispatch.rs` | 252 / 257 |
| recase loop | `dispatch.rs` | 230–235 |
| `build_keys_tl` | `dispatch.rs` | 294 |
| `build_keys_tl_with_inventory` (left-anchored projection) | `dispatch.rs` | 337 |
| `build_shadow_lattice` (D1 shared pure fn) | `dispatch.rs` | 396 |
| `greedy_longest_syllabification` | `dispatch.rs` | 436 |
| `span_min_syllable_count` | `dispatch.rs` | 492 |
| `fetch_walker_slot0` (D1 2nd build @650; OOV carve-out @860–927; D3 fields @960–979) | `dispatch.rs` | 632 |
| `build_hyphen_shadow` | `dispatch.rs` | 1014 |
| `custom_toneless_key` | `dispatch.rs` | 1074 |
| `canonicalize_poj_shadow` | `dispatch.rs` | 1151 |
| `apply_normalize_to_tl_with_offsets` (D2 hand-mirror) | `dispatch.rs` | 1234 |
| `offset_aware_replace` | `dispatch.rs` | 1273 |
| `raw_to_proto_candidate` | `dispatch.rs` | ~1541 (per S0 research) |
| `normalize_to_tl` (D2 source-of-truth) | `engine/phonetics/src/syllable.rs` | 56 |
| `fetch_candidates_for_endings` (D8) / `_for_keys` / `_partial_prefix` (D7 ×3) | `engine/lexicon/src/continuous.rs` | 352 / 472 / 633 |
| `best_candidate_for_key` / `record_to_candidate` | `continuous.rs` | 736 / 805 |
| `SortKey` (D6 doc "seven" @~1000 vs "eight" @~600; 8 fields) | `continuous.rs` | 1010 |
| `calculate_continuous_score` / `BOOST_ALPHA` | `engine/ranking/src/score.rs` | 443 / 122 |
| `build_lattice` / `walk_best` / `edge_cost` | `lattice/{builder,walker,cost}.rs` | 42 / 135 / 280 |
| install fixture pattern / `engine_install_lock` | `engine/lexicon/tests/parity.rs` | ~241 / ~35 |
| syllables-FST builder | `engine/lexicon/tests/syllables_fst.rs` | ~186 |
| proto `CandidateMessage` | `engine/protos/proto/composing.proto` | ~304 |

— end of design spec —
