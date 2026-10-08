# E1 P5b — the candidate list and the dictionary search page sort by `walker_cost`

> **Type**: Report (dated snapshot — frozen)
> **Keywords**: `candidate list`, `walker_cost`, `CandidateSortKey`, `dictionary search`, `frequency` retirement
> **Date**: 2026-10-08 · branch `feat/e1-p5b-list-sort-by-corpus` off `main` `fd30c0b3`
> **Plan**: [`architecture/unified-word-frequency-roadmap.md`](../architecture/unified-word-frequency-roadmap.md) §6 P5b · **Previous**: [P5a report](2026-10-08-e1-p5a-edge-pick.md)

## Summary

- The candidate list now sorts on the same corpus scale as slot 0. `RawCandidate` carries its record's `walker_cost`, and `CandidateSortKey` reads it for every candidate: the span-local list, the partial-prefix and abbreviation blocks, and the walker's edge pick. `with_walker_cost` and the `Option` P5a added are gone.
- Slot 0 is unchanged. `walker_gold_metrics` exact / segmented are identical before and after, and so is every slot 0's displayed roman (`GOLD_SLOT0_OUT` `shown`). The edge-pick screen changes slot 0 on only 4 of 14,849 keys, all typed with a literal `ⁿ` with every source on, where no walker path exists and the list's head is slot 0.
- The list is easier to correct from. A new harness, `walker_gold_word_ranks`, types each gold word alone and ranks it in the list. Mean rank goes 0.18 → **0.14** on `dev` and 0.15 → **0.12** on `calib`; top-5 goes 99.3 → **100.0** % on `dev`.
- The dictionary search page uses the corpus order too. The engine owns the whole order: corpus cost, then frequency, cut at the limit, then MOE (Kautian) rows first. The three platform re-sorts that ordered by raw frequency are deleted (iOS, Android, desktop core).
- The old `frequency` still has live readers, so it stays (§5 is the retirement plan for the maintainer).

## 1. Mechanism (grounded in code)

| Piece | Change |
|---|---|
| `ranking/src/sort_key.rs` | `CandidateRankFacts.walker_cost`; `CandidateSortKey.walker_cost: u16`, read by `new()`; dimension order unchanged: coverage kind, tier, −user weight, context rank, **walker cost**, −score, −freq, −coverage, source rank, insertion |
| `lexicon/src/continuous/mod.rs` | `RawCandidate.walker_cost` (internal, not on `CandidateMessage`); `exact_candidates_for_key`'s sink takes the candidate alone; `span_walker_cost` is still the minimum over the dictionary rows, taken before learned rows join; `pick_edge_word` uses the plain key |
| `lexicon/src/continuous/candidate.rs` | `record_to_candidate` copies the record's cost; custom and learned rows are `WALKER_COST_UNPRICED` |
| `composing` | the walker's slot-0 synth and the literal roman row are `WALKER_COST_UNPRICED` |
| `lexicon/src/search.rs` | `collect_filtered_sorted`: (cost ↑, frequency ↓, rowid order), cut at `limit`, then a stable move of rows that are Kautian by the effective source bitmask |
| iOS / Android / desktop core | `sortByMoeThenFrequency` ×2 and `LexiconRow::sorted_for_search` deleted, plus the fields only they read (`DictionarySearchResult.frequency`, `LexiconRow.length_score`); custom rows still lead the system rows |

- **Equal cost.** Corpus-unseen words share one smoothed cost (13,181 milli-nats). For them the old `score` / `freq` dimensions still decide. It is the same one comparator as P5a, not a fallback.
- **Unpriced rows.** A custom row sorts after every priced row of its tier unless user weight or the context lifts it. Before P5b its score was already 0, so it sat last the same way. Its source rank 0 now sits below the cost. The walker's custom edge override (`CUSTOM_EDGE_COST`) is untouched.
- **Kautian after the cut.** The search page first takes the corpus's top `limit` rows, then shows the MOE ones first. That keeps the pre-P5b rule (take the top rows, then MOE first). A Kautian row whose subcollection is switched off counts by its other sources (Codex pre-impl).
- **Mainstream**: McBopomofo lists each reading node's unigrams in the language model's order, the same scores the walk uses (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:206-232`).

## 2. Engine metrics

Slot 0 (`walker_gold_metrics`, default sources): exact and segmented are unchanged on every split, category and variant. The P5a numbers stand (`dev` all 84.9 / 88.6, `calib` all 83.3 / 85.5). Top-5 / mean rank of the whole item move slightly: `dev` all 90.9 / 0.53 → 91.0 / 0.52.

The list (`walker_gold_word_ranks`: each gold word typed alone; rank of the first whole-buffer candidate with the gold hanji, capped at 20):

| Split | Variant | n | slot 0 % | Top-5 % before → after | Mean rank before → after |
|---|---|---|---|---|---|
| dev | all | 3,664 | 90.1 | 99.3 → 100.0 | 0.18 → 0.14 |
| dev | tl_full | 458 | 94.1 | 99.8 → 100.0 | 0.08 → 0.07 |
| dev | tl_toneless | 458 | 84.9 | 98.9 → 100.0 | 0.33 → 0.24 |
| dev | tps_full | 458 | 90.6 | 99.3 → 100.0 | 0.16 → 0.12 |
| dev | tps_toneless | 458 | 84.9 | 98.9 → 100.0 | 0.31 → 0.24 |
| dev | variant (category) | 280 | 69.3 | 100.0 → 100.0 | 0.43 → 0.37 |
| calib | all | 1,304 | 92.9 | 99.5 → 99.5 | 0.15 → 0.12 |
| calib | tl_toneless | 163 | 87.7 | 98.8 → 98.8 | 0.30 → 0.23 |
| calib | tps_full | 163 | 94.5 | 100.0 → 99.4 | 0.09 → 0.10 |

Word by word, 27 words move up (賣, 會, 臺灣, 估價, 誠 …) and 6 move down: 个, 哪, 雞, 規, 捷, 題庫. In every downward move the corpus prefers another homophone. For example, toneless `e` lists 个 one place lower, and `kui` lists 規 behind 歸 (687 vs 1,652, the P5a residue). The `calib` `tps_full` top-5 drop is one word, 雞 (rank 3 → 5). Unmarked TPS ㄍㆤ admits every tone, and the corpus writes five of those homophones more often: 加 1,215 > 過/kè 1,127 > 嫁 968 > 家 901 > 假/ké 739 > 雞 512.

## 3. Lists (`candidate_dump`, TL, default sources)

| Input | Before | After |
|---|---|---|
| `kausiu` whole-buffer rows after slot 0 (教授) | 狗岫, 狗巢, 交收, 敎授 | unchanged: 狗岫 (count 1) before the unseen 狗巢 / 交收 / 敎授 |
| `kausiu` one-syllable rows | 到, 甲, 教, 交, 狗, 敎, 夠 | 到, 狗, 教, 交, 九, 夠 (corpus 18,903 / 929 / 886 / 879 / 617 / 328) |
| `taiuanlang` partial spans | 台, 代, 大, 臺, 台員, 台灣 | 台灣, 臺灣, 大, 台, 代 (台灣 4,084 > 大 1,281 > 台 1,006) |
| `tt` abbreviations | 湊陣, 鬥陣, 閗陣, 得着, 定定 | 得著, 得著 (`tit--tio̍h`), 直直, 拄著, 獨獨 |
| `kap` / `koh` / `beh` after slot 0 | 及, 與, 蛤 / 擱, 更, 過 / 卜, 要, 麥 | same leading rows (slot 0 佮 / 閣 / 欲 from P5a) |

The `kausiu` gap the P5a report described ("the list still runs 狗岫 > 狗巢 > 交收 before 教授") is about the sorted pool before the §22 promotion moves the dictionary's 教授 to slot 0. That pool now starts with 教授 (count 246), and the displayed list is unchanged.

## 4. Tests

- `ranking`: `walker_cost_beats_score_below_user_weight_and_context` (now through `new()`), `unpriced_candidate_follows_priced_unless_selected_or_in_context`. The fixtures share one unseen cost, so the other dimensions' tests still isolate their own dimension.
- `lexicon/tests/span_local_fetch.rs`: `list_orders_by_corpus_cost_and_leads_with_the_edge_pick` (台灣 6,000 > 臺灣 = 台員 9,000, frequency decides > custom; the list head is the edge pick); `list_custom_twin_of_the_cheapest_word_follows_the_priced_homophone` (a custom twin of 台灣 replaces the dictionary row and follows the priced 臺灣, as its score 0 already did); `list_context_hit_leads_the_corpus_order` (§56 above the cost); `list_partial_spans_of_mixed_length_follow_the_corpus_cost` (in the partial tier, cost decides between 台 and 台灣 whatever the length).
- `lexicon/tests/parity.rs`: `search_orders_by_corpus_cost_then_kautian_first_after_the_limit` and `search_moves_only_effective_kautian_rows_first` (both fail on `main`, which sorts 甲, 蛤, 合, 佮), `search_keeps_rowid_order_on_an_equal_cost_and_frequency`.
- `composing/tests/continuous_slot0_dict_roman.rs`: `slot0_promotes_the_separator_form_the_corpus_writes`. The §22 promotion takes the first same-reading row of the sorted list, so two separator forms of 予我 whose cost and frequency disagree now promote the corpus's form. Traced: `main` promotes the higher-score `hōo--guá`. On production data the gold set's displayed slot 0 is unchanged (§ Summary).
- `composing/tests/walker_fixed_inputs_prod.rs`: `list_orders_the_words_after_slot_0_by_corpus_count` (`taiuanlang`: 台灣 < 大 < 台).
- iOS `testSystemRows_keepTheEngineOrder` and Android `system rows keep the engine order …` (the Android test replaces the old Kautian-first platform test).
- S0 golden: no diff. The hermetic fixtures derive their cost monotonically from their frequency, so the corpus order equals the old order there.
- Desktop characterisation (`linux-check`, `macos-rust-check`): unchanged.

## 5. Retiring the old `frequency` — maintainer decision

After P5b these still read `DictionaryRecord.frequency` / the CSV `frequency` column:

| Reader | Role | What retiring it needs |
|---|---|---|
| `CandidateSortKey` −score / −freq (`ranking/src/sort_key.rs`) | breaks an equal corpus cost. About 69 % of dictionary rows are corpus-unseen and share one cost, so it orders most unseen homophones | a replacement tie-break (source rank, then rowid order), which reorders unseen homophones |
| `lexicon/src/search.rs` | the same tie-break on the search page | same |
| `CandidateMessage.score` (proto field 5) = `calculate_continuous_score(frequency)` for dictionary rows | the platforms copy it and order on nothing; slot 0 synth carries −cost | drop or redefine the field (wire change) |
| `TaigiWord.length_score` (proto field 4) | still emitted; no reader since this phase (platform sorts and the bridges' `lengthScore` deleted) | reserve the field |
| `dictionary/build/associations.py:168` | dictionary-derived association counts → `association.bin` | a corpus-count source for those rows |
| `dictionary/build/merge_csv.py:133,147`, `common/kautian_accent_wordgen.py:280` | max-aggregation of duplicate identities; CSV / FST row order (rowid order is the last tie-break) | a new row-order key |
| `dictionary/build/dictionary_records.py:180`, `create_dictionary_bin.py` | the `dict.bin` u32 field | a format version bump + reader + fixtures |
| `dictionary/common/frequency.py`, `khiin_frequency.csv` | build the column | delete with the column |
| tools (`walker_gold.py`, `compare_baseline.py`, `gen_dogfood.py`), `test-support` fixtures (cost derived from frequency) | measurement / hermetic data | port to the cost |

So `frequency` cannot be deleted without first choosing a tie-break for corpus-unseen words and a source for the dictionary-derived association counts. Both choices change user-visible order. The maintainer decides whether and when.

## 6. Commands

```
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold resolve
GOLD_OUT=<tsv> GOLD_SLOT0_OUT=<tsv> GOLD_WORD_OUT=<tsv> GOLD_WORD_RANKS_OUT=<tsv> GOLD_FAILURES=1 \
  cargo test --manifest-path engine/Cargo.toml -p composing --test prod walker_ -- --include-ignored --nocapture
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold d3
DUMP_INPUTS="kausiu,kap,koh,beh,taiuanlang,tt" DUMP_MODE=tl cargo test --manifest-path engine/Cargo.toml -p composing --test prod candidate_dump -- --ignored --nocapture
```

Before numbers: the same commands with the harness change applied over `main` `fd30c0b3`'s engine sources. `d3` (first whole-key homophone ≠ corpus winner): 28 → 25 keys with default sources, 131 → 115 with every source.
