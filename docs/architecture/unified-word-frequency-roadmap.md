# E1 — Unified Word Frequency for the Walker

> **Type**: Planning (multi-PR roadmap)
> **Keywords**: `walker`, `edge_cost`, `word_unigrams`, `segmentation`, `slot 0`, `gold set`
> **Status**: complete (P0–P5b merged). Retiring the old dictionary `frequency` is an open candidate in `docs/roadmap.md`.

Release scope and timing are the maintainer's call; nothing here is assigned to a release.

**Goal** (maintainer, 2026-10-08): slot 0 is the word the user types often and wants. This plan fixes the segmentation half — a real dictionary word such as 教授 must not lose to a character-by-character composition because of a frequency-scale mismatch. User-selected words already lead the pick (`CandidateSortKey`, `engine/ranking/src/sort_key.rs:64`) and are not changed.

---

## 1. Problem (grounded in code)

Fully toned TL `kau3siu7` puts the walker composition 到受 at slot 0; the dictionary word 教授 *kàu-siū* sits lower. Reproduced with `engine/composing/tests/candidate_dump.rs` against the production artifacts with no user data → every platform. Found in the 2026-10-08 desktop upgrade sim; root cause co-confirmed by Codex (twice).

**Mechanism.**

- Walker edge cost (`engine/composing/src/lattice/cost.rs:320-372`, khiin port): `ln(CORPUS_TOTAL_FREQ / (1 + freq)) / toneless_len^0.2 × syllable_count^0.2 − ln(1 + δ)`; the walker takes the min-sum path (`lattice/walker.rs:180`).
- `freq` = `DictionaryRecord.frequency`, attached by `dictionary/common/stages/frequency.py:20-27` through `dictionary/common/frequency.py:22-100`, which mixes three scales:
  1. single characters — `char_freq_merged.txt`, counting every occurrence of the character, **including inside other words** (到/*kàu* 17,333; 受/*siū* 8,975 in `output/dictionary.csv`);
  2. multi-syllable words — `khiin_frequency.csv` word counts (教授 *kàu-siū* 10);
  3. everything else — `get_default_frequency` = `50 // syllables` (25 for two syllables, 16 for three; 75,057 / 39,992 rows carry exactly 25 / 16, most but not all of them from this default).
- Edge `frequency` = `EdgeBest::span_frequency`, the max over the key's homophones that pass the source / tone-pin / barrier filters (`engine/lexicon/src/continuous/mod.rs:1085-1121`; `engine/composing/src/continuous.rs:843-866`).
- Numbers (old frequencies): 到 5.3177 + 受 5.8460 = 11.1636 < 教授 11.2279.
- Slot 0 is the walker path (`continuous.rs:1310-1446`); the §22 promotion fires only when the walker's hanji already equals a full-span dictionary word of the same reading (`continuous.rs:1418`), so it cannot rescue 教授.
- Control: `tai5gi2` → 台語 (freq 394, cost 8.66) — a word whose khiin count happens to be large enough.

**Same-corpus recompute** (`dictionary/shared/data/word_unigrams.tsv`, N = 3,629,550 tokens, the current `+1` smoothing): 教授 246, 到/*kàu* 18,902, 受/*siū* 5,574 → 教授 7.7025 vs 到+受 9.4211 — 教授 wins. (The Lidstone model of §3 is measured in P1 / P2, not assumed.)

**Exposure** (previous session, a risk screen, not an error count; to be re-run reproducibly by the P1 harness): in fully toned 2–3 syllable input, 1,368 dictionary words lose slot 0 to a different hanji composition; 812 of them occur in the corpus. Many losers are odd variants (ê話 → 的話), where the composition is the better answer.

## 3. Direction — A2 (walker probability from one segmented corpus)

1. **Build time**: one walker cost per `dictionary.bin` record `(hanzi, canonical TL)` (Core Principle #6) from `word_unigrams.tsv` with the `taigi_typing` column excluded (§5).
2. **Unseen records** take the same model's smoothed value (Lidstone add-α over V: `p = (c + α) / (N + α·V)`, V = distinct model words, §4 D1) — no fallback to the old frequencies (AGENTS.md § No redundant fallback). Lidstone is the baseline, not a claim: khiin floors zero-probability words with a length-dependent value (`references/khiin-rs/khiin/src/data/segmenter.rs:82-86`) and McBopomofo scales by length (`fscale = 2.7`, `frequency_builder.py:55`), so P1's error strata decide whether unseen pricing needs a length term.
3. **Denominator** = `N + α·V`, computed at build time; `CORPUS_TOTAL_FREQ`, `corpus_total_freq.txt` and both test skips retire.
4. **Untouched**: `DictionaryRecord.frequency` keeps feeding the candidate-list sort (`ranking::calculate_continuous_score`), the FST row order, `association.bin` v2 and the corpus-count generator's own subsegment tie-break. Record order and rowids do not change.
5. No bigram (bigram P6/P7 stay closed).

### Not adopted

| Option | Why not |
|---|---|
| A1 — fixed multiplier on multi-syllable frequency | Keeps two scales; the factor is arbitrary and breaks as the sources change. |
| B — no sentence when the whole buffer is a word (librime `references/librime/src/rime/gear/script_translator.cc:495-506`) | Promotes every odd full-span variant (ê話) over a better composition; drops the walker exactly where the exposure screen says it helps. |
| C — fixed per-word penalty (librime `references/librime/src/rime/gear/grammar.h:23-26`, `kPenalty = ln 1e-6` per entry) | A per-edge constant tunes segmentation granularity, not the scale mismatch. |
| Per-source weighting of the corpus | Not until dev shows a domain skew (bible_nt is 11 % of tokens); P4 only, with numbers. |
| Katz / Good-Turing back-off to a character model | More machinery than the decision needs unless P1 shows unseen pricing as the dominant error class. |
| Sidecar `walker_lm.bin` | A second rowid-aligned artifact to deploy and version; D1 keeps one file. |
| Raw count + model params in the binary | Only worth it if the runtime had to retune the model; it does not. |

## 4. Design

**D1 — storage: `dictionary.bin` v4.** Add `walker_cost: u16` after `kautian_subtag`: `round(−ln p × 1000)` (milli-nats, round half to even), the smoothed probability **before** the khiin length terms. The writer fails the build on a value above `u16::MAX`; the worst case at α = 0.1 is ≈ 17,500. +2 B × 167,907 = 335,814 B (≈ 328 KiB). Record order, rowids, `frequency` and the FST are byte-identical to v3 apart from the new field and the version; checked once in PR #461 (one-off script, 0 differing records). `build_id()` (`dictionary/build/common.py:26`, CRC-32 of `dictionary.csv`) also hashes `word_unigrams.tsv` and α, so a model change changes the build id. One reader version: a v3 file is rejected. Deployment: Android re-copies assets only on a new `VERSION_CODE` or a missing file (`android/app/.../TaigiKeyboardApplication.kt:80`) — same-version dev installs need a clean install; dogfood steps say so.

Model word (P2, Codex pre-impl): the counter's token `(hanzi, tl_num.lower())`, not the CSV row. `corpus_bigrams` cannot tell `tshut-lâi` from the neutral-tone `tshut--lâi` (460 such keys, 921 records), so those records share one count and one cost; walker edges are keyed by `tl_num` and D3 takes the min over homophones, so the walker sees no difference. V = distinct model words (167,446 — the count events, not the 167,907 records); N = Σ counts without `taigi_typing` (3,627,713). α is a provisional 10 in `walker_lm.py` until P4. `walker_lm.py` is a module the `dictionary.bin` writer calls, not a build step of its own.

**D2 — cost formula.** `edge_cost` reads `walker_cost / 1000` in place of `ln(N / (1 + freq))`; the khiin length terms (`÷ toneless_len^0.2 × syllable_count^0.2`) and the OOV per-char `BIG` stay as they are in P3. With the length terms kept, the objective is a heuristic cost, not a pure `Σ −ln p`; changing them is P4 with a dev win.

The subsequent v5 binary format stores `N`, `V`, and `α` in its header.
The runtime derives the user-entry price from that loaded model and the
effective count `2_000`; dictionary or corpus updates no longer require a
manual `CUSTOM_EDGE_COST` update. The deliberate effective-count policy and
the cap's selection fade remain engine-owned. Current layout and validation:
[`binary-format.md`](../engine/binary-format.md).

The custom entry, the learned-phrase floor and the user-selected floor become one **cost cap** applied before the length terms: `base = min(walker_cost, cap)`.

- Exact translation of today's floor (`cost.rs:347-353`, `F = CUSTOM_EFFECTIVE_FREQ`, `s = clamp(δ / BOOST_ALPHA, 0, 1)`): `cap(s) = CUSTOM_EDGE_COST + ln((1 + F) / (1 + F·s))`, with `CUSTOM_EDGE_COST = ln(N / (1 + F))`. At `s = 0` the cap is `ln N` — no lift, finite.
- Re-anchoring `CUSTOM_EDGE_COST` to the new distribution is a calibration decision, made in P3 by the rule used for 2,000 (a custom entry beats a top single-character split, never a top single character) and checked on dev before merge.
- Custom (`continuous.rs:807`), learned (`continuous.rs:845-848`) and selected (`cost.rs:350`) share this one cap. Single-syllable edges keep `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE = 0.0` (no selection lift in segmentation).
- Behaviour pinned by tests: zero δ, fresh selection, decay, saturation, single-syllable, learned-only, custom-only, and a learned row whose dictionary source is toggled off. Today's decay test (`cost.rs:766`) is rewritten from the new formula, its invariant kept.

**D3 — which homophone prices the edge: (c) min `walker_cost` over the filtered homophones.** Today's "is this span a word" / "which word" decoupling (`continuous/mod.rs:1085`, `continuous-input-ranking.md` §3.2) re-expressed in the new scale; the per-edge pick (`pick_edge_word`, `CandidateSortKey` order) is unchanged. The min runs over exactly the rows that pass the source, tone-pin and barrier filters (filtering happens before aggregation, `continuous/mod.rs:704`).

Why not (a) "price on the pick's own value": the pick follows the old order, not the corpus. Runtime example: `kap` — with no user history 甲 and 洽 tie on frequency and 甲 wins on source rank (`continuous-input-ranking.md:144`); 甲 has corpus count 11, 佮 22,637 — pricing on the pick would price the commonest conjunction as rare. CSV-level screen (first-row tie-break, default sources): 5,111 full-tone keys where the old-frequency winner ≠ the corpus winner, 2,403 with a corpus winner ≥ 10. P1 replaces this screen with the runtime `CandidateSortKey` winner under default and all-enabled sources.

What (c) does not promise: each edge's word is unchanged, but a different winning path can still change the slot-0 hanji (that is the fix).

(b) "pick the edge word by corpus probability" changes the orthography of thousands of keys (擱→閣, 卜→欲 with variants enabled) — a product decision, kept as a maintainer-gated P5 — opened 2026-10-08 (§6 P5a / P5b, §8 item 4).

**Toneless / partial-tone caveat**: (c) over a toneless key takes the cheapest admissible reading across tones, so a rare reading can borrow a common reading's segmentation strength. P1 measures it as its own stratum.

**D4 — context.** Unchanged invariant (§56, `continuous/mod.rs:1129`): a previous-word context may change an edge's WORD, never its COST; the contextual winner is chosen among homophones of the context-free winner's syllable count. Gold runs use an empty context; separate mobile context cases check the invariant.

## 5. Gold set and measurement (before any A2 code)

**Licensing first.** `corpus/README.md:21` permits short quotes of articles in dogfood items and verbatim quotes of 教典 sentences (CC BY-ND); it does not grant a committed test corpus. So the committed gold file holds **no corpus text**: pointers plus judgements only. The harness reads the excerpt from the `corpus/taigi-typing` submodule at run time (`#[ignore]`d, skipped with a message when the submodule is absent). Hand-picked word items use dictionary entries only (our own data) and may be stored in full.

**Gold file** `engine/composing/tests/data/walker_gold.tsv`:

```
id  split  category  source_ref  span  extra_accepted
```

- `source_ref` = `kautian:<sentence index>` or `typing:<article id>:<line>`, or `dict` for a hand-picked dictionary item (then `span` holds its TL and hanji).
- `span` = syllable range of the excerpt (≤ 8 syllables).
- The excerpt's own hanji + TL (word boundaries from the TL side: space between words, `-` / `--` inside one) is the first accepted answer at run time, **only if** every word is a dictionary word and the excerpt has no punctuation or mixed script — otherwise the item is skipped and listed.
- `extra_accepted` = alternative orthographies / boundaries by human judgement (閣/擱, 佮/甲), drafted by Claude, reviewed by the maintainer for `variant` and `phrase` rows.
- `category` ∈ `common` (corpus count ≥ 100) · `rare` (1–99) · `unseen` (dictionary word, no count) · `variant` (competing orthographies) · `phrase` (2–4 words) · `neutral` (`--` words). A `reading` category (one hanji, several readings, e.g. 教 *kà* / *kàu*) joins in P3.

**Splits** (by document, not by sentence; cross-source duplicates checked against every corpus source before an item is admitted):

| Split | Source | Use |
|---|---|---|
| `dev` | 教典 example sentences + hand-picked word items, incl. the fixed regression inputs | Diagnosis; numbers optimistic (the sentences overlap `moe_kautian` 例句 in the counts) |
| `calib` | `mapped` typing articles A (half, by article) | Tuning α, `CUSTOM_EDGE_COST`, the user-delta scale |
| `final` | `mapped` typing articles B (other half) | Read **once**, at the end of P4; aggregates only (`GOLD_SPLITS=final`) |

Typing items are "source contribution excluded" (the `taigi_typing` column is out of the counts), not "unseen": the same words occur in other sources.

**Inputs generated by the harness** from the gold words' dictionary keys: TL full tone (digits) · TL partial tone (digit on each word's first syllable) · TL toneless · TL with typed `-` between syllables · POJ full / toneless · TPS with and without tones. Fetched through the real `Start → FetchAtPos` path with the default install's sources. P1 measures these eight; key-by-key (incremental) input, typed spaces and POJ spelling aliases join the P3 acceptance.

**Metrics** (per split × category × input variant):

1. **Slot-0 acceptable rate** — slot 0's full-buffer hanji ∈ accepted.
2. **Boundary accuracy** — F1 of slot 0's **displayed** word boundaries (one word per space-separated run of its roman) vs. the gold words', plus `segmented`: same boundaries and each word a homophone of the gold word. Displayed, not the walker's internal edges: the TL path exposes no edge list to tests, and what matters is what the user sees — the §22 promotion shows one dictionary word where the walker took several same-hanji, same-reading edges. The typed-separator variant shows the user's separators and has no boundary metric.
3. **Top-k displacement** — 0-based position of the first acceptable whole-buffer candidate, capped at k = 5 (5 = not in the first five); mean and top-5 rate.

**Error strata** reported by P1 (they steer P2–P4): per category and input variant (neutral, toneless, variant included), the runtime D3 screen (engine pick vs corpus winner, default and all sources), the synthetic-split share of the counts, and a reading of the misses (segmentation vs one-syllable edge pick). Unseen-by-cause strata join P3.

**Fixed regression inputs** (asserted from P3 on, in the `prod` test target): `kau3siu7` → 教授 · `tai5gi2` → 台語 · `hoogua` (§22) · `taiuan` → 台灣 · `ginalangtsiahpngbesai` → 囡仔人食飯袂使.

## 6. Phases

All merged; labels are cited from code. Measurements: [`e1-walker-baseline-corrections`](../reports/2026-10-08-e1-walker-baseline-corrections.md) (P1 baseline, corrected), [`e1-p2b-subsegment-tiebreak`](../reports/2026-10-08-e1-p2b-subsegment-tiebreak.md), [`e1-p4-calibration`](../reports/2026-10-08-e1-p4-calibration.md), [`e1-p5b-list-sort`](../reports/2026-10-08-e1-p5b-list-sort.md).

| Phase | Content |
|---|---|
| P1 | Gold file + `walker_gold.rs` harness + offline simulator (`dictionary/tools/walker_gold.py`) |
| P2 | `walker_lm.py` model, `dictionary.bin` v4 (`walker_cost`), engine v4 reader |
| P2b | Subsegment tie-break in `corpus_bigrams` (two passes) |
| P3 | `edge_cost` on `walker_cost`; one cost cap for custom / learned / selected |
| P4 | Calibration on `calib`; α stays 10 |
| P5a | Each walker edge's word picked by `walker_cost` |
| P5b | Candidate-list and dictionary-search sort on `walker_cost` |

## 7. Best-practices alignment

| Mainstream practice | Source `file:line` | This plan |
|---|---|---|
| One corpus probability for every segmentable unit, single characters included: `p = corpus_count / total_count` over one table | `references/khiin-rs/khiin/src/db/init/csv.rs:57,81`; consumed by `khiin/src/data/segmenter.rs:70-93` | P2 / P3 — our port kept khiin's formula but fed it two scales |
| Character counts made standalone by subtracting in-phrase occurrences; one normaliser; unseen phrase at a fixed pseudo-count; length scaling | `references/McBopomofo/Source/Data/curation/builders/frequency_builder.py:50-66` | P2 — segmented counts are standalone by construction (minus the synthetic-split share P1 measures); length scaling considered in P4 |
| Walker = relaxation / Viterbi over additive costs | `references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:134` | unchanged |
| User phrases enter the same weight space as dictionary entries | `references/librime/src/rime/dict/user_dictionary.cc` (`formula_d`) | P3 — one cost cap for custom / learned / selected |

Rules: `~/.claude/rules/planning.md` (roadmap + memory, grounded, P0 admin) · `diagnosis-discipline.md` § Verify pipeline claims (the D3 premise check) · `code-review-rules.md` §8 (Codex pre-impl per phase) · `docs/contributing/known-pitfalls.md` § Trace before assert (P3 expected values traced from `walker_lm_stats.txt`).

## 8. Decisions

The maintainer, 2026-10-08, after the plain-language summary: "proceed with your recommendation". Recorded as:

1. **D3** — (c) for P3; (b) stays an unopened, maintainer-gated P5 (opened 2026-10-08, item 4).
2. **Gold judgement** — the excerpt's own hanji is the automatic first answer; Claude drafts alternatives, the maintainer reviews the `variant` and `phrase` rows.
3. **Corpus weighting** — raw sum of the 8 non-held-out sources; revisited only if calib shows a domain skew (P4).
4. **P5 opened** — the maintainer, 2026-10-08, after `kausiu` / `kau-siu` showed 狗岫 at slot 0 (教授 4th): "do A and B together" — A = the edge word pick by corpus (P5a), B = the candidate-list sort by corpus (P5b). P5a ships first; P5b opens after P5a merges.
