# E1 — Unified Word Frequency for the Walker

> **Type**: Planning (multi-PR roadmap)
> **Keywords**: `walker`, `edge_cost`, `word_unigrams`, `segmentation`, `slot 0`, `gold set`
> **Status**: P0–P4 merged 2026-10-08; P5 opened by the maintainer 2026-10-08 as P5a (edge pick, in progress) + P5b (candidate-list sort) — direction approved by the maintainer 2026-10-08 ("proceed with your recommendation"); revised after the Codex pre-impl review (REVISE, 8 changes folded in)
> **Last updated**: 2026-10-08

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

## 2. Facts measured for this plan (2026-10-08, `main` `a45bb704`)

Three universes, kept apart: raw `output/dictionary.csv` rows (168,958), `dictionary.bin` records after the builder's filter + dedupe (167,907; `dictionary/build/dictionary_records.py:156`), and the model vocabulary V = the `dictionary.bin` records. Syllable counts use the builder's own rule (`dictionary_records.py:92`).

| Fact | Value | Rule |
|---|---|---|
| `word_unigrams.tsv` rows / tokens | 52,017 / 3,629,550 (3,627,713 without `taigi_typing`; 14 rows are counted only by `taigi_typing`) | `count` = sum of 9 source columns |
| Unigram rows that are dictionary words | 52,017 / 52,017 | `corpus_bigrams.py` whitelists dictionary words |
| CSV rows by syllables / with a corpus count | 1: 25,254 / 7,842 · 2: 86,686 / 34,760 · 3: 42,789 / 7,857 · 4: 13,178 / 1,558 | join on `(hanzi, tl)` |
| Per-source tokens | nmtl 997,565 · kipsupin 939,274 · khinhoan 762,511 · bible_nt 407,675 · icorpus 403,074 · moe_kautian 例句 105,640 · kok4hau7 6,652 · sinpak 5,322 · taigi_typing 1,837 | column sums |
| `dictionary.bin` readers | Rust only (`engine/lexicon/src/dictionary_reader.rs`, one version accepted, `:153`); iOS bundles the `assets/dictionaries` folder reference; Linux installs the four files by name (`linux/Makefile:164`) | grep |
| `CORPUS_TOTAL_FREQ` guard | red on `main` since 2026-08-29 and skipped in `tools/test_select.py:75` + `.github/workflows/engine.yml:80` | file comments |

**How the corpus counts were made** (affects §3): `corpus_bigrams.py` aligns hanji and romanization per word; an out-of-vocabulary corpus word is split into dictionary words by fewest pieces, then the highest summed **old** frequency (`corpus_bigrams.py:273-290`), and a neutral-tone OOV word is not split (`:455`). So some counts are synthetic splits steered by the old frequencies. P1 measures that share.

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

| Phase | Content | Size | Behaviour change | Status |
|---|---|---|---|---|
| **P0** | Admin: this roadmap, `docs/roadmap.md` Active entry, memory topic file | docs | none | Merged #458 |
| **P1** | Gold file + `walker_gold.rs` harness + offline simulator of the A2 cost over the same items (Python, `dictionary/tools/`) + baseline report `docs/reports/<date>-e1-walker-baseline.md` (metrics, error strata, runtime D3 screen, re-run exposure numbers with the command and artifact fingerprint) | ~400 + TSV | none | Merged #460 — [baseline](../reports/2026-10-08-e1-walker-baseline.md) |
| **P2** | `dictionary/build/walker_lm.py` (model generation only; reuses `dictionary_records` filtering + identity), `dictionary.bin` v4 writer + verifier, `output/walker_lm_stats.txt` (N, V, α, unseen cost, percentiles), `build_id` fingerprint; engine v4 reader + v3 rejection test, `engine/test-support/src/tkdb.rs` fixture writer; `docs/engine/binary-format.md` v4; the P1 cloud-review fixes to `walker_gold.py` + a dated corrections report. Reads the committed `word_unigrams.tsv`. Walker still prices on `frequency` | ~450 + artifacts | none — one-off production v3-parity check + S0 golden empty | Merged #461 |
| **P2b** | Subsegment tie-break (the committed counts were current — [report](../reports/2026-10-08-e1-p2b-subsegment-tiebreak.md)): `corpus_bigrams` counts twice — pass 1 counts in-vocabulary tokens only; pass 2 breaks fewest-piece ties by the lowest summed quantised cost under the frozen pass-1 model (`SUBSEGMENT_ALPHA` 10, fixed apart from the walker's α), first split found on an exact tie — so the old frequency no longer steers the counts; `word_unigrams.tsv` + `word_bigrams.tsv` regenerated together; `association.bin` diff reviewed + mobile next-word / context dogfood (S116); `simulate` / `exposure` re-run | ~150 + data | **yes** — mobile next-word + composing context (association.bin) | Merged #462 |
| **P3** | `edge_cost` on `walker_cost` (D2); `EdgeBest` carries min `walker_cost`, handed over by the dictionary visitor with each row (D3; no `RawCandidate` field — Codex pre-impl); one cost cap for custom / learned / selected; `CUSTOM_EDGE_COST` re-anchored and checked on dev; retire `CORPUS_TOTAL_FREQ`, `corpus_total_freq.txt`, the two skips and the BUILDING note; hermetic fixtures keep their pre-E1 cost; fixed regression tests; S0 golden diff reviewed line by line; `continuous-input-ranking.md`, `cost.rs` head | ~500 | **yes** — segmentation / slot 0 | Merged #463 `6f95344c` — [report](../reports/2026-10-08-e1-p3-walker-cost.md) |
| **P4** | Calibration on `calib`: α ∈ {0.5, 2, 10, 50} (corrected P1 numbers, [corrections](../reports/2026-10-08-e1-walker-baseline-corrections.md): at α 0.5 A2 splits 1,059 dictionary words that win their key today; at α 10, 68), `CUSTOM_EDGE_COST`, the user-delta scale (0.0 kept unless calib shows otherwise), length exponents (changed only with a calib win); one `final` read; report | ~150 + report | constants only | Merged #465 `4ba40f21` — every constant kept (α 10 best on the engine grid) — [report](../reports/2026-10-08-e1-p4-calibration.md) |
| **P5a** | D3 (b): the word that fills each walker edge (`EdgeBest.candidate`, `engine/lexicon/src/continuous/mod.rs`) is picked by `walker_cost`; user weight and context rank still lead, custom / learned overrides unchanged; tie-break among unseen words (one shared unseen cost) is a rule of the pick, not a fallback; fixed regression `kausiu` → 教授; S0 golden diff reviewed; slot-0 orthography change counted (`tools.walker_gold d3`); report + dogfood | ~250 | **yes** — slot-0 orthography (佮/甲, 閣/擱, 欲/卜) | PR open — [report](../reports/2026-10-08-e1-p5a-edge-pick.md) (dev all 79.9 → 84.9 % exact; slot 0 changes on 2,960 / 14,849 toned keys); dogfood S118 |
| **P5b** | Candidate-list sort (`CandidateSortKey` score / freq, `engine/ranking/src/sort_key.rs`) on `walker_cost`, so the list and slot 0 use one scale; coverage kind → tier → user weight → context rank order unchanged; a retirement plan for the old `frequency` (dict.bin field, `dictionary/common/frequency.py`, `khiin_frequency.csv`) goes to the maintainer if no reader is left; whether the dictionary search page is in scope is a maintainer question | ~250 | **yes** — candidate-list order | Pending P5a merge (separate session) |

Every phase from P2 on: `make dict` → `make build`, refreshed artifacts committed; post-PR gate per touched platform via `tools/test_select.py`. P3 / P4 dogfood: one mobile + one desktop platform, clean install, the fixed inputs plus TL / POJ / TPS, mobile context cases, and the iOS keyboard-extension memory check (no new full-dictionary runtime map — `walker_cost` is read from the mmapped record like `frequency`; long input and repeated mode switches show no growth).

## 7. Best-practices alignment

| Mainstream practice | Source `file:line` | This plan |
|---|---|---|
| One corpus probability for every segmentable unit, single characters included: `p = corpus_count / total_count` over one table | `references/khiin-rs/khiin/src/db/init/csv.rs:57,81`; consumed by `khiin/src/data/segmenter.rs:70-93` | P2 / P3 — our port kept khiin's formula but fed it two scales |
| Character counts made standalone by subtracting in-phrase occurrences; one normaliser; unseen phrase at a fixed pseudo-count; length scaling | `references/McBopomofo/Source/Data/curation/builders/frequency_builder.py:50-66` | P2 — segmented counts are standalone by construction (minus the synthetic-split share P1 measures); length scaling considered in P4 |
| Walker = relaxation / Viterbi over additive costs | `references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp:134` | unchanged |
| User phrases enter the same weight space as dictionary entries | `references/librime/src/rime/dict/user_dictionary.cc` (`formula_d`) | P3 — one cost cap for custom / learned / selected |

Rules: `~/.claude/rules/planning.md` (roadmap + memory, grounded, P0 admin) · `diagnosis-discipline.md` § Verify pipeline claims (§2; the D3 premise check) · `code-review-rules.md` §8 (Codex pre-impl per phase) · `docs/contributing/known-pitfalls.md` § Trace before assert (P3 expected values traced from `walker_lm_stats.txt`).

## 8. Decisions

The maintainer, 2026-10-08, after the plain-language summary: "proceed with your recommendation". Recorded as:

1. **D3** — (c) for P3; (b) stays an unopened, maintainer-gated P5 (opened 2026-10-08, item 4).
2. **Gold judgement** — the excerpt's own hanji is the automatic first answer; Claude drafts alternatives, the maintainer reviews the `variant` and `phrase` rows.
3. **Corpus weighting** — raw sum of the 8 non-held-out sources; revisited only if calib shows a domain skew (P4).
4. **P5 opened** — the maintainer, 2026-10-08, after `kausiu` / `kau-siu` showed 狗岫 at slot 0 (教授 4th): "do A and B together" — A = the edge word pick by corpus (P5a), B = the candidate-list sort by corpus (P5b). P5a ships first; P5b opens after P5a merges.

## 9. Review log

- 2026-10-08 P5a (Codex pre-impl, ANALYSIS-ONLY: REVISE, folded in): the corpus dimension lives in `CandidateSortKey` (`with_walker_cost`, 0 = unset, so the list is unchanged until P5b) rather than a local key in `pick_edge_word`; an equal quantised cost falls to the remaining dimensions — one comparator, not a fallback; a learned row can still take the edge by a context hit (unchanged policy, now tested); the picked word's `syllable_count` can move a segmentation; the slot-0 change count reads candidate 0 (`edge_picks.tsv` `slot0`), not the first listed homophone. Maintainer 2026-10-08: plan approved; P5b also moves the dictionary search page (`lexicon/src/search.rs`) to the corpus order.
- 2026-10-08 P5 opened: trigger on `main` `86bf1fa2` (`candidate_dump`, TL) — `kau3siu7` → 教授 at slot 0, but `kausiu` / `kau-siu` → 狗岫 (教授 4th); `kap` → 甲, `koh` → 擱, `beh` → 卜. Mechanism: P3 prices the edge on the cheapest homophone's `walker_cost` (D3 (c)), while the word that fills it is still `EdgeBest.candidate` by `CandidateSortKey` on the mixed-scale `frequency` (狗岫 25 = the two-syllable default, 教授 10 from khiin; corpus 1 vs 246). Split into P5a / P5b (§6).
- 2026-10-08 P4 ([report](../reports/2026-10-08-e1-p4-calibration.md)): the simulator (TL full tone only) favoured α 50 and the maintainer approved it; the engine grid over all eight input variants then showed α 50 losing every toneless variant, and the maintainer chose "keep α 10". Every constant stays as P3 shipped it; calibrations that touch every input variant run the engine harness on all eight before choosing.
- 2026-10-08 P3 (Codex pre-impl, ANALYSIS-ONLY: REVISE, folded in): `CUSTOM_EDGE_COST` anchored as the model price of a 2,000-count word (7,878 milli-nats; one-syllable bound holds for len ≤ 23); the dictionary visitor hands each row's `walker_cost` to the sink instead of a `RawCandidate` field; hermetic fixtures derive their cost from the pre-E1 formula (`test_support::walker_cost_from_fixture_frequency`) plus explicit-cost fixtures where cost and frequency disagree, filter-before-min and tone-pin tests; the simulator's old model is frozen as `pre-p3` and its engine-agreement check compares A2 at the shipped α. Numbers: [P3 report](../reports/2026-10-08-e1-p3-walker-cost.md).
- 2026-10-08 P2b (Codex pre-impl, ANALYSIS-ONLY: REVISE, folded in): regeneration on `main` was byte-identical, so the P1 "stale counts" reading was wrong (`tok:in-vocab` leaves out `romanized-mapped`; [report](../reports/2026-10-08-e1-p2b-subsegment-tiebreak.md) §Summary) and the maintainer approved the slimmed phase; tie-break α fixed apart from the walker's α; the held-out rule shared through `walker_lm.model_from_counts`; two passes, no cache; per-source token totals asserted unchanged; `association.bin` reviewed for set changes too, because composing context ranks read it.
- 2026-10-08 P2 (Codex pre-impl, ANALYSIS-ONLY, + the P1 cloud review read back after merge): `word_unigrams.tsv` regeneration and the two-pass subsegment tie-break go to a separate P2b (they change `association.bin`, so P2 stays behaviour-neutral); model word = `(hanzi, tl_num.lower())` with V = distinct words, not records; α 10 provisional; `RawCandidate.walker_cost` moved to P3; v3 parity checked once in the PR, not as permanent code; P1 tooling fixed (resolve drift exits, D3 winner among visible rows, unsupported keys skipped) and every P1 number re-run in a [corrections report](../reports/2026-10-08-e1-walker-baseline-corrections.md) — D3 default screen 930 / 193 (was 2,016 / 236), conclusion (c) unchanged.
- 2026-10-08 P1 baseline ([report](../reports/2026-10-08-e1-walker-baseline.md)): α range widened for P4; P2 adds the `word_unigrams.tsv` regeneration decision (the committed file predates later dictionary rebuilds; ≥ 13 % of counted tokens are old-frequency subsegment splits); runtime D3 screen confirms (c) (236 default-source keys whose pick never occurs in the corpus while the winner does ≥ 10 times).
- 2026-10-08 Codex post-impl P1: **REVISE** → folded in: boundaries defined as displayed (§5 metric 2), gold words restricted to default-visible rows with first-row identity, homophones by fully toned key, multi-character words, simulator priced on the engine's own pick and `syllable_count`, runtime D3 screen, rank capped at k, synthetic-token numbers corrected; incremental / typed-space / alias inputs and the `reading` / unseen-by-cause strata moved to P3.

- 2026-10-08 Codex pre-impl (ANALYSIS-ONLY): **REVISE** → folded in: corrected §1/§2 numbers and universes; D3 evidence via the runtime pick (`kap` → 甲, not 洽) and no "orthography unchanged" promise; exact cost-domain translation of the selection floor; v4 quantisation, fingerprint, rowid parity, deployment; synthetic-split disclosure + error strata; licensing via pointers, calib / final split, boundary definition, input variants; P2 scope (spec, fixtures, parity), guard consumers, `association.bin` v2, context invariant, memory check.
