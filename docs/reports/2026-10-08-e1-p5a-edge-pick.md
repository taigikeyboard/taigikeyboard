# E1 P5a — the walker picks each edge's word by `walker_cost`

> **Type**: Report (dated snapshot — frozen)
> **Keywords**: `walker`, `edge pick`, `walker_cost`, `CandidateSortKey`, `slot 0`, `orthography`
> **Date**: 2026-10-08 · branch `feat/e1-p5a-edge-pick-by-corpus` off `main` `d965e7b7`
> **Plan**: [`architecture/unified-word-frequency-roadmap.md`](../architecture/unified-word-frequency-roadmap.md) §6 P5a · **Previous**: [P4 report](2026-10-08-e1-p4-calibration.md)

## Summary

- The word that fills each walker edge is now the cheapest corpus word among the key's visible homophones. The order is user weight, then the previous-word context, then `walker_cost`, then the old dictionary score. P3 priced the edge on that word's cost but showed the word with the highest old `frequency`.
- `kausiu` / `kau-siu` → 教授 at slot 0 (was 狗岫); `kap` → 佮 (was 甲); toneless `koh` → 閣 (was 擱), `beh` → 欲 (was 卜).
- Engine, default sources: `dev` exact / segmented 79.9 / 87.1 → **84.9 / 88.6** % over 2,112 inputs; `calib` 75.7 / 81.3 → **83.3 / 85.5** % over 544. Every input variant improves on both splits.
- Slot 0 changes its hanji on 2,960 of 14,849 fully toned keys typed alone (default sources; 3,925 with every source on). The D3 screen's "pick ≠ corpus winner" falls from 904 to 28 keys (default sources) and from 4,011 to 131 (every source).
- The candidate list is unchanged. Until P5b, slot 0 and the list can order the same homophones differently: for `kausiu`, slot 0 is 教授 while the list still runs 狗岫 > 狗巢 > 交收 > 教授.

## 1. Mechanism (grounded in code)

- `CandidateSortKey` (`engine/ranking/src/sort_key.rs`) gains a `walker_cost` dimension between `context_rank` and `score`. `new()` leaves it `None` for every candidate (the comparison does not use the corpus dimension), so the span-local list, the partial-prefix block and the abbreviation path sort exactly as before. `with_walker_cost()` sets it.
- `best_candidate_for_key_with_barriers` (`engine/lexicon/src/continuous/mod.rs`) keeps each row's `walker_cost` from the shared `exact_candidates_for_key` sink and hands `(candidate, cost)` pairs to `pick_edge_word`. Learned phrases join with `WALKER_COST_UNPRICED`, so they take the edge only through user weight or a context hit, or when the dictionary has nothing under the key.
- **Equal cost.** Corpus-unseen words share one smoothed cost (13,181 milli-nats at α 10), and nearby counts can round to the same milli-nat. Such a tie falls to `score`, `freq`, coverage, source rank and insertion order, which is the pre-P5a order. Every key goes through this one comparator; nothing branches when the corpus has no count.
- **Unchanged**: the custom-dictionary edge override, the learned-phrase cost cap, `span_walker_cost` (still the key's minimum, D3), and §56 (the context may change an edge's word, never its cost). One known effect: the picked word's `syllable_count` feeds the length term of `edge_cost`, so a cheaper word with another syllable count can move a segmentation. The gold set shows that as the segmented gains below.
- **Mainstream**: McBopomofo's walker node shows its top unigram under the same score that prices the walk, and keeps the top unigram's price when the user overrides the value (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:446-459`). That is D3 (c) plus this pick.

## 2. Engine metrics (`walker_gold.rs`, default sources)

Exact % / segmented % / mean rank.

| Split | Variant | n | Before (main) | P5a |
|---|---|---|---|---|
| dev | tl_full | 264 | 86.4 / 97.0 / 0.46 | 91.7 / 97.0 / 0.25 |
| dev | tl_partial | 264 | 85.6 / 96.2 / 0.47 | 90.9 / 96.2 / 0.27 |
| dev | tl_toneless | 264 | 70.5 / 76.1 / 1.08 | 75.4 / 79.5 / 0.92 |
| dev | poj_toneless | 264 | 69.7 / 75.4 / 1.11 | 75.0 / 79.2 / 0.94 |
| dev | tps_full | 264 | 82.2 / 90.5 / 0.66 | 86.4 / 90.9 / 0.51 |
| dev | tps_toneless | 264 | 71.6 / 77.3 / 1.01 | 76.5 / 80.7 / 0.86 |
| dev | all | 2,112 | 79.9 / 87.1 / 0.71 | 84.9 / 88.6 / 0.53 |
| calib | tl_full | 68 | 85.3 / 95.6 / 0.74 | 91.2 / 95.6 / 0.44 |
| calib | tl_toneless | 68 | 61.8 / 64.7 / 1.85 | 70.6 / 73.5 / 1.47 |
| calib | tps_full | 68 | 77.9 / 85.3 / 1.10 | 86.8 / 88.2 / 0.66 |
| calib | tps_toneless | 68 | 63.2 / 66.2 / 1.78 | 72.1 / 75.0 / 1.40 |
| calib | all | 544 | 75.7 / 81.3 / 1.19 | 83.3 / 85.5 / 0.84 |

`poj_full` equals `tl_full`, and `tl_hyphen` (exact only) equals `tl_full` after P5a. By category on `dev`: `variant` 58.9 → 69.3 % exact, `phrase` 71.0 → 78.4, `common` 97.5 → 100, `regression` 92.5 → 100, `rare` 97.1 → 96.1.

**Item by item** (item × variant): 205 misses fixed, 62 new misses. In every new miss the corpus prefers the other homophone. Only the first group is a choice between two acceptable spellings; in the other two, slot 0 shows a different word from the one meant:

| Kind | Examples (gold → slot 0; corpus counts) |
|---|---|
| Orthography of a hand-picked word | 新詩 → 身屍 (4 vs 213), 反映 → 反應 (24 vs 132), 罩蘭 → 卓蘭, 蟮螂 → 新人, 志宏 → 志鴻 |
| One-syllable unigram, no context | 濟錢 → 坐錢 (`tsē`: 濟 2,574 vs 坐 2,991), 共規鼎飯 → 共歸鼎飯 (`kui`: 規 687 vs 歸 1,652) |
| Toneless key pooling every tone | `kamsi` 敢是 → 監視, `tingpue` 頂輩 → 重倍, `teho` 地好 → 第好, `linsu` 人事 → 恁事 |

The `rare` drop is the two toneless items `kamsi` and `tingpue`. Choosing words like 濟錢 correctly takes the neighbouring word into account. A unigram edge pick cannot do that, and P5b, which sorts the candidate list, does not either.

## 3. Slot-0 orthography change

`walker_edge_picks` types every fully toned key that has more than one homophone, alone, and now also records slot 0's hanji (`edge_picks.tsv` column `slot0`). Before / after on the same harness:

| Sources | Keys | Slot 0 changed | 1 / 2 / 3 / 4 syllables |
|---|---|---|---|
| default | 14,849 | 2,960 | 637 / 1,809 / 445 / 69 |
| all | 14,849 | 3,925 | 761 / 2,628 / 465 / 71 |

Examples (default sources): `kap4` 甲 → 佮, `kak4` 覺 → 角, `kui1` 規 → 歸, `tse7` 濟 → 坐, `sin1si1` 新詩 → 身屍, `huan2ing3` 反映 → 反應. Unchanged: `e5` 的, `be7` 袂, `koh4` 閣, `beh4` 欲, `tsit4` 這, `kau3siu7` 教授. With every source on, `koh4` 擱 → 閣, `beh4` 卜 → 欲 and `tsit4` 今 → 這 also change.

`tools.walker_gold d3` compares the first whole-key homophone in the list with the corpus winner, so it is a diagnostic, not the slot-0 count: 904 → 28 keys (default sources), 4,011 → 131 (every source). Its residue comes from the list order, which P5b changes.

## 4. Tests

- `ranking` — `walker_cost_beats_score_below_user_weight_and_context`.
- `lexicon` unit (`edge_word_pick_tests`): corpus cost over frequency, equal cost falls to the score, a selected word beats the corpus choice, learned rows in four cases, and the context pick keeps the corpus winner's cost inputs.
- `lexicon/tests/span_local_fetch.rs`: the cheaper word is picked over the more frequent one (the D3 test, rewritten); a filtered row lends the edge neither its word nor its cost; TPS picks the cheapest admitted substituted reading, and a Final-only barrier removes it. The two integration tests fail on `main`.
- `composing/tests/walker_fixed_inputs_prod.rs`: `kausiu` → 教授 (on `main`: 狗岫).
- S0 golden (`UPDATE_GOLDEN=1 … golden_fetch_at_pos`): no diff. The hermetic fixtures derive `walker_cost` monotonically from their frequency, so the corpus order equals the old order there.

## 5. Commands

```
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold resolve
GOLD_OUT=<tsv> GOLD_FAILURES=1 cargo test --manifest-path engine/Cargo.toml -p composing --test prod walker_ -- --include-ignored --nocapture
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold d3
DUMP_INPUTS="kausiu,kau-siu,kap,koh,beh" DUMP_MODE=tl cargo test --manifest-path engine/Cargo.toml -p composing --test prod candidate_dump -- --ignored --nocapture
```

Before numbers: the same commands on `main` `d965e7b7` (the slot-0 column with the harness change applied over `main`'s engine sources).
