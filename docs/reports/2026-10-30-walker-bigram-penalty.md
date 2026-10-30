# Walker bigram P6.1: the penalty-only conditional bigram in the simulator

> **Date**: 2026-10-30 · **Plan**: [`architecture/bigram-lm-roadmap.md`](../architecture/bigram-lm-roadmap.md) D6 / P6.1 · **Trigger**: [`roadmap.md`](../roadmap.md) v3.9.0 fix list row 2

## Summary

- The unigram walker lets two very frequent single-syllable words undercut one mid-frequency word. TL `ue5` puts `有的` (`ū`+`ê`) at slot 0 above `鞋`; `khikhi` puts `去去` above `起去`.
- A conditional word bigram fixes both cases, but only in a **penalty-only** form, `base = max(−ln p_uni, −ln p(w|prev))`. The full conditional regresses the gold set.
- Penalty-only on the gold set: no stratum drops, `dev/phrase` exact goes 90 → 92 %, and 2 items change, both toward gold. Whole-key exposure: 223 → 127 losers, with 0 word→split and 96 split→word flips.
- `tools.walker_gold bigram` reproduces every number below. `build.walker_lm.BigramCounts` is the reference formula the engine will mirror (P6.3).

## 1. Root cause (engine, production artifacts)

`candidate_dump` (`engine/composing/tests/candidate_dump.rs`, TL mode). Edge cost is `cost.rs::edge_cost`: `walker_cost / len^0.2 × syll^0.2`.

| Input | Slot 0 | Its cost | Single word | Its cost |
|---|---|---|---|---|
| `ue5` | `ū ê` 有的 | 4.496 + 4.173 = 8.669 | `uê` 鞋 | 10.024 / 2^0.2 = 8.726 |
| `khikhi` | `khì khì` 去去 | 4.384 × 2 = 8.768 | `khí--khì` 起去 | 9.273 |

POJ shows the same `khikhi` result. POJ `oe5` is correct because `ôe` is a single syllable, so no split edge exists. `guhi` → `牛墟` did not reproduce a split.

## 2. Model

Counts come from `word_bigrams.tsv`, minus the held-out `taigi_typing` column, the same hold-out as the unigram. A pair is summed per model word, so `tshut-lâi` and `tshut--lâi` share one entry. That leaves 283 234 seen pairs over 22 046 previous words.

```text
p(w₂|w₁) = max(c(w₁,w₂) − D, 0) / C(w₁) + D·N(w₁)/C(w₁) · p_uni(w₂)      D = 0.75
```

For the reported pairs:

- `有→的`: C = 48 740, N = 3 370, c = 411, so −ln p = 4.687 against the unigram's 4.173. The split costs 9.183 > 8.726, and `鞋` wins.
- `去→去`: C = 14 233, N = 1 432, c = 27, so −ln p = 6.135 against 5.461. The split costs 9.309 > 9.273, and `起去` wins by a thin margin.

## 3. Gold set (`tools.walker_gold bigram`, TL full tone, engine edge picks)

Exact % / segmented %.

| Stratum | n | Unigram (today) | Penalty-only | Full conditional |
|---|---|---|---|---|
| calib/all | 68 | 91 / 96 | 91 / 96 | 91 / 93 |
| calib/phrase | 58 | 90 / 95 | 90 / 95 | 90 / 91 |
| dev/all | 264 | 92 / 93 | 92 / 93 | 91 / 92 |
| dev/common | 15 | 100 / 100 | 100 / 100 | 100 / 93 |
| dev/neutral | 15 | 100 / 80 | 100 / 80 | 100 / 73 |
| dev/phrase | 120 | 90 / 89 | 92 / 89 | 90 / 90 |
| dev/rare | 35 | 100 / 100 | 100 / 100 | 100 / 94 |
| dev/unseen | 39 | 100 / 100 | 100 / 100 | 97 / 97 |

The other strata are identical across all three models. Penalty-only changes two items: `dev-0020` 無 依法 → 無 伊 法 (gold) and `dev-0073` 彼 的 → 彼个 (gold hanji). The full conditional changes 25 items, mostly splits of words the gold keeps whole, such as 攏是 → 攏 是, 想欲 → 想 欲 and 囡仔人 → 囡仔 人. The corpus pair `攏→是` costs less than `是`'s unigram, so the full model rewards the split.

## 4. Exposure (every 2–3 syllable fully toned key, 39 527)

| Model | Whole-key word loses slot 0 |
|---|---|
| Unigram | 223 |
| Penalty-only | 127 (0 word→split, 96 split→word) |
| Full conditional | 666 |

The 96 split→word flips include 對年, 護照, 職災, 理工, 球鞋, 歹講, 氣質, 有價, 人工, 收視, 就醫, 士氣, 雨衫, 一戶, 人人, 島嶼, 依據, 就職, 唬人, 台視, 熱人, 織女, 主因, 受理, 來路, 死因, 照護, 受教 and 雨鞋. A few flips are doubtful without context: 李梁 for `你 娘`, 鞋揙 for `的 邊`, 鞋苴 for `的 自`. Each is a fully toned two-syllable key typed alone, and in running text the next edge's context decides it.

## 5. Discount sweep

| D | Penalty-only gold items changed | Losers |
|---|---|---|
| 0.5 | 3 | 125 |
| 0.75 | 2 | 127 |
| 0.9 | 2 | 127 |

The result is insensitive to D, so 0.75, the usual single-discount KN value, stays.

## 6. Limits

- The simulator types fully toned syllables only. The reported toneless inputs (`ue5`, `khikhi`) are engine regressions for P6.3, not simulator cases.
- The first edge takes no penalty. The walker has no D5 previous-commit context.
- The `khikhi` margin is 0.036 nats. `dictionary_records.py::_syllable_count` counts `--` as two separators (`khí--khì` gets syllable count 3), which overprices every neutral-tone word by (3/2)^0.2. It is a separate fix.
