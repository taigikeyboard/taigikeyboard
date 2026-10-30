# Continuous-Input Ranking

> **Type**: Specification
> **Keywords**: `Continuous`, `Ranking`, `CandidateSortKey`, `user_weight`, `walker`, `FrequencyData`
> **Related**: [composing.md](composing.md), [continuous-commit-and-display.md](continuous-commit-and-display.md), [binary-format.md](binary-format.md)
> **Authority**: the module docs of [`engine/composing/src/continuous.rs`](../../engine/composing/src/continuous.rs) (`assemble_candidates`), [`engine/lexicon/src/continuous/`](../../engine/lexicon/src/continuous/) (span-local fetch, `ContinuousFetchCtx`), [`engine/ranking/src/sort_key.rs`](../../engine/ranking/src/sort_key.rs) (`CandidateSortKey`) and [`engine/composing/src/lattice/cost.rs`](../../engine/composing/src/lattice/cost.rs) (walker cost). When this document and those module docs disagree, the module docs win.

---

## Pipeline

One `FetchAtPos` is answered by `composing::continuous::assemble_candidates` ([`continuous.rs`](../../engine/composing/src/continuous.rs)):

| Step | What | Code |
|---|---|---|
| 1 | Syllabify the pending `raw` and build the left-anchored FST keys (all modes: TL / POJ / TPS / English) | `shadow::build_continuous_keys` |
| 2 | No key → whole-buffer partial-prefix scan; else span-local exact fetch per key | `fetch_via_lexicon_partial_inner` / `lexicon::fetch_candidates_for_keys_with_barriers` |
| 3 | Recase each candidate's `roman` to the user's typed segment | `recase_roman` |
| 4 | Prepend the whole-sentence walker's best path at slot 0 (never sort-compared) | `fetch_walker_slot0_inner` |
| 4b | Whole-input prefix-extension scan (`COVERAGE_KIND_PARTIAL_PREFIX`) | `fetch_via_lexicon_partial_inner` |
| 4c | Whole-buffer abbreviation lookup | `continuous.rs` Step 4c |
| 4d | Re-sort the merged blocks by `CandidateSortKey`, keeping slot 0 in place | `ranking::sort_by_candidate_key` |
| 5 | POJ mode only: render `roman` as POJ, then dedupe the rendered list | `recase_tl_as_poj_display`, `dedupe_rendered_continuous` |

The dispatch caller runs the fetch twice: a neutral pass discovers the candidates, then their `user_frequency.db` rows are read and the fetch re-runs ranked (`engine/dispatch/src/user_data/with_stores.rs` `handle_composing` / `frequency_map`). No platform sends frequency rows.

---

## 3. Ranking

### 3.2 User weight and sort key

**Sort key.** `CandidateSortKey` ([`sort_key.rs`](../../engine/ranking/src/sort_key.rs)) is a lexicographic 12-field key, shared by the span-local list, the partial-prefix and abbreviation blocks, the Step 4d merged re-sort and the walker's per-edge word pick (`lexicon::pick_edge_word`):

| # | Field | Order | Meaning |
|---|---|---|---|
| 1 | `tier` | 0 first | 0 = the candidate consumes the whole input, 1 = only its front (`taiuanl` → 台灣人 before 台灣) |
| 2 | `abbreviation` | 0 first | 1 = abbreviation hit (`COVERAGE_KIND_ABBREV`); every reading of the tier — full syllable or partial prefix, custom or not — leads every abbreviation (`thng` → 湯 / 糖 before a custom 退伍 `thui-ńg` reached through its initials; USER 2026-10-16, librime's abbreviation credibility penalty) |
| 3 | `preference_class` | 0 first | 0 = selected (`user_weight > 0`, any source), 1 = never-selected custom-dictionary word, 2 = the rest. The user's picks lead the words the user filed, which lead the rest ("the word I put in shows up", USER 2026-10-12; below the picks, USER 2026-10-16: a bulk-filed custom 屢 `lí` no longer buries 你 on `li2`). A selected partial prefix therefore leads a never-selected full syllable. §43 bounds which custom words reach class 1 |
| 4 | `coverage_kind` | ascending | full syllable 0 < partial prefix 1 < abbreviation 2 |
| 5 | `user_weight` | descending | decayed user-selection weight; orders the selected words (field 3 already put them before every never-selected one) — a picked custom word gets no lift over a heavier-picked dictionary word |
| 6 | `context_rank` | ascending | continuation of the previous word (behavioral-invariants §56 `INVARIANT_CONTINUOUS_CONTEXT_RERANK`) |
| 7 | `walker_cost` | ascending | `dictionary.bin` corpus cost (milli-nats); `WALKER_COST_UNPRICED` for custom / learned / synthesized rows, which therefore follow every priced row of their tier unless a field above lifts them |
| 8 | `score` | descending | `calculate_continuous_score` = `freq × (1 + 0.1 × (syllables − 1))`, purely dictionary-derived |
| 9 | `freq` | descending | raw dictionary frequency |
| 10 | `coverage` | descending | consumed bytes — a weak tiebreak only |
| 11 | `source_rank` | ascending | `source_tier_rank`: custom 0, kautian 1, taigitv 2, stti 3, kungge 4, other 5 |
| 12 | `stable_idx` | ascending | insertion order |

There is no recency field: recency enters only through the decay inside `user_weight`. Fields 1–2 are `candidate_lead`; the TPS word list leads with them and `coverage_kind`, then puts longer words first whatever the counts or the custom source (H4), each length keeping this order. `CandidateSortKey::without_context` neutralizes field 6 for the walker's context-free edge pick.

**User weight.** `FrequencyData::user_weight(now_ms)` → `decayed_user_weight_delta(count, now_ms, last_used_ms)` ([`engine/ranking/src/score.rs`](../../engine/ranking/src/score.rs)):

```text
boost = min(1.0 + count × BOOST_ALPHA, MAX_BOOST)     // BOOST_ALPHA 0.1, MAX_BOOST 5.0 (saturates at count 40)
delta = (boost − 1.0) × exp(−age_ms / USER_WEIGHT_DECAY_TAU_MS)   // τ = 30 days
```

The cap is applied **before** the decay, so a huge stale count is not pinned high (Gap B: the user signal is bounded so one hot entry cannot dominate). A missing wall clock, a never-used row, or clock skew all return `0.0`. The user signal lives only in `user_weight` — never also inside `score`.

**Storage and ownership.**

| Item | Location |
|---|---|
| `FrequencyData { count: i32, last_used_ms: i64 }`, `FrequencyMap`, `user_freq_boost`, `decayed_user_weight_delta` | `engine/ranking/src/score.rs` |
| `FrequencyMap` key | the `(display_text, canonical_tl)` pair (Core Principle #6); `get` falls back to the legacy `(display_text, "")` bucket, never sums |
| Store | `user_frequency.db`, `UNIQUE(word, tl)` — `engine/userdata` `UserFrequencyStore` (`rows_for_words`) |
| Write | `UserDataStores::record_usage` (`engine/userdata/src/stores.rs`): count + 1 and `last_used` = now for the pair; a Hanji pick also touches its learned phrase. A `CommitContinuous` is recorded by the engine itself; a platform `RecordUsage` covers other picks |
| Read | `with_stores.rs` `frequency_map` → `ContinuousFetchCtx.freq_map` + `now_ms` |
| Sort | `ranking::sort_by_candidate_key`, called from `engine/lexicon/src/continuous/` and Step 4d |

### Walker cost (slot 0)

The slot-0 path is `min Σ edge_cost` over the lattice (`lattice::walker::walk_best`), a port of khiin's segmenter ([`cost.rs`](../../engine/composing/src/lattice/cost.rs) `edge_cost`):

The v5 dictionary header carries the same `N`, `V`, and `α` used to price its
records. Once per walk, the engine derives the custom-word cost as
`round_half_even(−ln((2_000 + α)/(N + αV)) × 1000)`. The effective count
`2_000` is an engine policy parameter; the derived price is never compiled
into the engine. The edge provider and selected-word cap use that same price.

- **Dictionary edge**: `base / toneless_len^0.2 × syllable_count^0.2 − ln(1 + applied_δ)`, `base = walker_cost / 1000`. `walker_cost` is the key's lowest homophone cost (`EdgeBest::span_walker_cost`), not the picked word's.
- **Walker bigram** (penalty-only, [`bigram-lm-roadmap.md`](../architecture/bigram-lm-roadmap.md) D6): after a previous edge, `base = max(base, −ln p(w₂|w₁))` with interpolated absolute discounting over the `dictionary.bin` v6 pair counts (`cost::bigram_penalty`). `w₁` / `w₂` are the edges' context-free winners (`EdgeBest::bigram_rowid`), so the previous committed word's context still never moves the segmentation. No penalty on the first edge, after a word with no recorded continuation, or when either side has no dictionary row (custom, learned, OOV). The walker state is `(offset, incoming edge)`; with an empty table the walk is the unigram walk.
- **User preference**: `applied_δ` is the edge word's `user_weight`, scaled to 0 on single-syllable edges (`WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE`) so a hot single character cannot sweep the sentence. A selected multi-syllable word's base cost is capped by `user_entry_cost_cap`.
- **User rows**: an edge with a custom or learned row under its key is priced at most at the loaded dictionary model's custom-word cost (the key's dictionary cost, walker-bigram penalty included, when cheaper).
- **OOV edge** (no dictionary hit): `OOV_PER_CHAR_PENALTY (1e10) × toneless_len`, so a dictionary-coverable buffer never collapses to an OOV blob.
- The edge's word is picked by `CandidateSortKey` among the key's homophones; the walker and its carve-out live in `continuous::fetch_walker_slot0_inner` (`engine/composing/src/continuous.rs`).

---

## 7. Goals

**G3 — the engine is the ranking authority.** Platforms render the engine's ordered list as-is: they never re-rank, add an exact-match bonus or filter by length. Ranking constants (`BOOST_ALPHA`, `MAX_BOOST`, `source_tier_rank`) are defined once in `engine/ranking` (`docs/contributing/cross-platform-alignment.md` §3a).

Mainstream comparison (librime, khiin-rs, McBopomofo, MOE Tâi-gí): [`docs/references/mainstream-ime-comparison.md`](../references/mainstream-ime-comparison.md) § Candidate ranking.

---

## 9. Open Questions

4. **Expose the consumed span to platform UI for navigation?** E.g. tap a Hanji segment of the composition to revisit its candidate list (MOE Tâi-gí's position-keyed `GetCandidatesAtNailPos`). A UX feature, not a ranking fix.
6. **Per-candidate metadata on `CandidateMessage`?** Hanji-vs-romanization is carried by the optional `hanji` field; not carried: a word count distinct from `syllable_count`, and the first-match dictionary source label (the data exists in the record's source bitmask).
7. **Segment-level commit instead of byte-level?** `CommitContinuous` still sends `consumed_bytes` taken verbatim from the candidate; a "commit candidate N at the current nail" contract (MOE `NailCandidate`) would remove that round-trip.

---

## 10. Commit Behavior & Display Split (Model B)

Moved → [`continuous-commit-and-display.md`](continuous-commit-and-display.md) §10.

### 10.2

Moved → [continuous-commit-and-display.md §10.2](continuous-commit-and-display.md#102-display-contract).
