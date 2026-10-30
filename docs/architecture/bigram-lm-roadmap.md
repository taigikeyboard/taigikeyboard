# Bigram Language Model — Roadmap

> **Type**: Design record
> **Keywords**: `bigram`, `language model`, `next-word`, `association.bin`, `taigi-corpus`, `lattice`, `walker`, `prediction`
> **Status**: Reopened 2026-10-30 for P6. P3–P5 shipped; P6 in progress (a segmentation error was reported: `ue5` → `有的` over `鞋`; D6 rewritten as a penalty-only conditional term); P7 not adopted; D7 cut. Unscheduled leftovers are in § Techniques not in this roadmap.

---

## Summary

- **Goal**: a **cross-word bigram model** built from running Taiwanese text replaces the old dictionary-word-completion next-word table (intra-word character pairs, `台→灣`), and the same data re-ranks composing candidates by the previous word.
- **Shape**: offline pair-count build (checked-in TSV) → `association.bin` v2 (word-keyed entries, char-keyed entries kept for backspace) → engine next-word looks up the word key → composing re-rank by previous word → penalty-only conditional bigram term in the lattice walker (D6, P6).
- **Two tracks, different risk**: the next-word track stays inside `engine/nextword` + `engine/dispatch/src/predict.rs`; the composing track touches whole-sentence ranking and needs `/code-review` + golden re-checks.
- Mainstream IME comparison (who puts a bigram in the walker, smoothing, caps): [`docs/references/mainstream-ime-comparison.md`](../references/mainstream-ime-comparison.md).

---

## Design

### D1 — Model: word bigrams over `(漢字, canonical-TL)` tokens

- Token identity = `(hanji, canonical TL)` (Core Principle #6). Two count files: `c(w₁, w₂)` across a **word boundary** and `c(w₁)` (unigram from the same running text — distinct from the dictionary `frequency` column, which is a lexicon weight).
- Sentence punctuation `。！？.!?`, line end and clause punctuation `，、；：` break the chain; the next-word context rule breaks on the same marks (`behavioral-invariants.md` §40). There is no sentence-start key.
- A token that is **not** a dictionary `(hanji, tl)` row breaks the chain: a next-word suggestion must be a word the engine can commit with its reading (librime-predict likewise keeps only lexicon words).
- **Scoring — next-word track**: within one key, order by `count`, top-K per key, min count 2 (librime-predict `make_predict_data` pruning). No smoothing: the list is per-key. **Source priority**: bundled score is capped at `DICT_SCORE_CAP` = 250, below `LEARNING_BONUS` (300), so every user row outranks every bundled row (`behavioral-invariants.md` §8 "always", not "at equal count").
- **Scoring — composing track**: ordering only (D5); the walker's segmentation term is D6's penalty-only conditional cost, not a bonus.

### D2 — Build location: this repository, checked-in intermediate

- **Corpus intake stays in `taigi-corpus`** (ingest, normalize, manifest; its "no tokenization" principle keeps alignment out of it). Alignment line N of `text` ↔ line N of `parallel_poj`.
- **Alignment + counting live here**: `dictionary/build/corpus_bigrams.py` reads `corpus/taigi-corpus/data/normalized/*.jsonl` and `corpus/taigi-typing/articles.js` directly (only derived counts are committed, never text), with `dictionary/output/dictionary.csv` as the token whitelist. Romanization goes through `taigi-converter` (Core Principle #3), never a hand-rolled table.
- Output = `dictionary/shared/data/word_bigrams.tsv` (`prev_hanji, prev_tl, next_hanji, next_tl, count`, one raw-count column per source) + `word_unigrams.tsv`, **committed**, regenerated on demand when the opt-in submodule is checked out (same pattern as `char_freq_merged.txt`; provenance row in `docs/SOURCES.md`). `make dict` therefore works without the 670 MB submodule and `association.bin` stays reproducible.
- Alignment — "1 hanji = 1 syllable" is measured per source, not assumed:
  1. Split the paragraph pair into sentences on shared punctuation; keep a sentence only when both sides split into the same count.
  2. TL side: normalize through `taigi-converter`; split on spaces; strip punctuation; `--` (khin-siann) stays attached to the preceding word.
  3. Hanji side: assign characters to TL words by syllable count; a hanlo Latin run is an **anchor** that must equal its TL word verbatim, otherwise the sentence is dropped. Totals differ → dropped.
  4. Whitelist each `(hanji, tl)` against `dictionary.csv`; an OOV token breaks the chain.
  5. Count pairs and unigrams; write TSVs sorted and deterministic.

### D3 — Artifact: `association.bin` **v2**, not a new file

- Same TKWA container, `version = 2`, same offset table and entry encoding. Two key namespaces in one sorted key section:
  - **word keys** `hanji\u{1}tl` (canonical TL; `\u{1}` keeps raw-UTF-8 byte sort, `binary-format.md` §6 rule 1) — corpus bigrams;
  - **character keys** (v1 shape, one hanji) — kept for the backspace path (`INVARIANT_NEXTWORD_LOOKUP_KEY_LAST_GRAPHEME` §24) and as the backoff when a word key is absent.
- Entry `bitmask` keeps its meaning (source flags of the **next** word, so `AssocFilter` keeps working); `count` = corpus pair count for word keys. Format: `binary-format.md` §2.
- Why not a new `bigram.bin`: five artifact install sites (iOS app + extension, Android, macOS, desktop) plus `InstallRequest`, `LexiconPaths` and `deploy.sh` for no gain over a versioned container. Why not SQLite: the bundled read path is mmap + binary search everywhere; user data is the only SQLite. The file stays mmapped, so it does not count against the iOS extension's 64 MB.

### D4 — Engine next-word adopts the word key (techniques 1 + 8)

- `predict.rs::filter_request`: key = `(predict.word, canonical TL of predict.roman)` → word-key lookup; **backoff** to the last-grapheme character key only when the word key returns nothing. This is the n-gram model's own lower-order backoff, not the "A fails → fall back to B" anti-pattern. **Empty roman → character key** always (hanji-only commits arrive with an empty roman).
- The backoff lives in the lexicon (`search::assoc_lookup`, its `previous_tl` argument); dispatch never builds keys. The reader filters by source while reading (`AssociationReader::lookup` takes the `AssocFilter`), so a key whose head comes from disabled sources is not starved.
- Lost continuous pairs (technique 2): the final commit carries the nailed segments (`NextWordWordSelected.preceding` / `WordSelected.preceding`, `CommittedWord`), forwarded verbatim by every platform; NextWord learns the whole commit as one sequence; nails learn nothing (`behavioral-invariants.md` §40 `INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING`). Recording at nail time would learn reversed pairs (the unnail rollback re-sends `UpdateLastSelectedWord`) and pairs from abandoned compositions. The 10 s window gates only the link to the previous commit.
- `DICT_SCORE_CAP` caps each prediction's **summed** dictionary contribution, after the `(hanzi, tl)` merge and the reading-variant fold. §24 `INVARIANT_NEXTWORD_WORD_KEY_BACKOFF` pins the backoff order.

### D5 — Composing re-rank by previous word (technique 7)

- **Engine-side snapshot, no proto field**: `nextword::EngineHandle::context_snapshot(now_ms) -> Option<(hanji, tl)>` takes the `nextword` mutex, applies `decide::committed_context` (the 10 s association window), clones two fields and releases the lock before returning. Inside a composition the list follows the word of the composition before it — the converted word ending where it starts, else the last nailed segment; glyphs cut the context (`composing::Engine::pending_snapshot` → `PendingSnapshot.context: ListContext`, `engine/composing/src/api.rs:736`). `dispatch` calls it before taking the composing lock — never nested, so no lock-order cycle. The snapshot is advisory; the composing generation check still governs the response. Both entry points (`handle_composing` and the user-data path) see the same `prev`.
- `dispatch::context` turns the previous word into `ranking::ContextRanks` (user bigrams over bundled `assoc_lookup` continuations) and passes it into the fetch (`Intent::FetchAtPos.context` → `ContinuousFetchCtx.context`); the lexicon stamps `RawCandidate.context_rank`. Desktops without user-data stores get the bundled context through the same module.
- `CandidateSortKey.context_rank` (user hit 0 < bundled hit 1 < none 2) sits below `coverage_kind` / `tier` / `user_weight` and above `score`, so the bonus is bounded by construction. Slot 0 is the walker's word, so the walker's per-edge pick (`lexicon::pick_edge_word`) uses the same key, restricted to homophones with the context-free winner's `syllable_count`.
- Segmentation is untouched, **proven by cost identity**: `edge_cost` reads only `span_walker_cost` (pick-independent), the pick's `syllable_count` (held equal) and `user_weight` (equal, since `context_rank` sits below it) — `behavioral-invariants.md` §56 `INVARIANT_CONTINUOUS_CONTEXT_RERANK`.

### D6 — Penalty-only conditional bigram in the lattice walker (reopened 2026-10-30)

**Trigger** (v3.9.0 fix list row 2, `docs/roadmap.md`): the unigram walker lets two hyper-frequent single-syllable words undercut one mid-frequency word. TL `ue5` → slot 0 `有的` (`ū`+`ê`) over `鞋`; `khikhi` → `去去` over `起去` (TL and POJ). Costs (`cost.rs::edge_cost`, `word_unigrams.tsv`): `有` 4.496 + `的` 4.173 = 8.669 < `鞋` 10.024 / 2^0.2 = 8.726; `去`+`去` 8.768 < `起去` 9.273. Negative control: POJ `oe5` (`ôe` is one syllable, no split edge) and `taiuan` → `台灣`.

**Model.** Interpolated absolute discounting (the lower-order mix KenLM's modified Kneser-Ney generalises), counts from `word_bigrams.tsv` / `word_unigrams.tsv` minus the held-out `taigi_typing` column:

```text
C(w₁)     = Σ_w c(w₁, w)          N(w₁) = |{w : c(w₁, w) > 0}|
p(w₂|w₁)  = max(c(w₁,w₂) − D, 0) / C(w₁) + D·N(w₁)/C(w₁) · p_uni(w₂)        D = 0.75
base(e)   = max(−ln p_uni(e), −ln p(pick(e) | pick(prev edge)))           # penalty-only
```

`p_uni(e)` is today's edge price (`span_walker_cost`, the key's homophone minimum), and it is also the `p_uni(w₂)` in the backoff term — the pair counts are looked up on the edge's pick, the unigram stays the edge's, so an empty table is exactly today's walk (simulator: 43 912 keys, 0 differences). The length / syllable biases and the user discount apply to `base` exactly as today. A `w₁` with no recorded continuation (`C = 0`) and the first edge take no penalty — no D5 context in the walker (YAGNI until a case needs it).

**Why penalty-only, not the full conditional** (offline spike over the E1 walker gold set, `dictionary/tools/walker_gold.py` simulator, same edge picks as the engine):

| Model | calib exact / seg | dev phrase exact | whole-key word loses slot 0 (of 39 527 2–3 syllable keys) | `ue5` | `khikhi` |
|---|---|---|---|---|---|
| unigram (today) | 91 % / 96 % | 90 % | 223 | split | split |
| full conditional `−ln p(w₂\|w₁)` | 91 % / 93 % | 90 % | 666 | word | word |
| **penalty-only** `max(uni, cond)` | 91 % / 96 % | 92 % | 127 | word (9.18 > 8.73) | word (9.31 > 9.27) |

The full conditional makes frequent collocations (`攏→是`) cheaper than their unigrams, so it splits words the gold set keeps whole. Penalty-only can only raise a multi-edge path's cost: word→split flips = 0 by construction, split→word = 96 (台視, 唬人, 受教, 有價, 彼个, 來路); 2 gold items change, both toward gold.

**Identity is a proof obligation.** The walker state becomes `(offset, incoming edge)`; each edge keeps today's single `EdgeBest` pick (no top-N alternatives, so custom / learned / `user_weight_delta` pricing is untouched). The P3 PR proves (a) with an empty bigram table every walk returns the same edges, choices and cost as `HEAD~1` (named `cost.rs` / `walker.rs` cases + golden dumps); (b) with data, a single-edge path's cost never changes. `/code-review` mandatory.

**Data.** `association.bin` v2 keeps the top 30 continuations with count ≥ 2 per key (`associations.py` `WORD_TOP_K` / `WORD_MIN_COUNT`), so its counts cannot give `C(w₁)`, `N(w₁)` or an unseen-vs-pruned answer. The walker needs every seen pair (283 234 pairs over 22 046 previous words today) as raw counts, so `p_uni` stays the runtime edge price under the user's source toggles.

**Artifact (P6.2): `dictionary.bin` v6 bigram section** (`binary-format.md` §1.5), keyed by rowid → model-word id. Measured on the 2026-10-30 build: 28 889 model words in a pair, 3.28 MB section, `dictionary.bin` 5.5 → 8.6 MB (mmapped, outside the iOS extension's 64 MB).

| Option | Size | Why not / why |
|---|---|---|
| **`dictionary.bin` v6 section, rowid → word id** | 3.28 MB | chosen: a rowid-keyed table must ship in the file that defines the rowids; the walker model (N, V, α) already lives in this header, so `D` joins it |
| `association.bin` v3 namespace, string keys | 5.42 MB | mixes next-word semantics (top-K, Bible ×0.5, variant folds) with raw walker counts; the pick carries display TL, and 460 model words have two display spellings (`tshut--lâi` / `tshut-lâi`), so `(hanji, tl)` keys would split their counts |
| new `walker_bigram.bin` | ≈ 3.3 MB | D3: five install sites for no gain, and rowid keys could drift from `dictionary.bin` |

The engine pick needs a rowid to look up; `RawCandidate` gains it in P6.3 with the walker change.

### D7 — Two-word context (technique 4) — cut

Not planned (YAGNI). If one-word context proves limiting, it returns as its own plan: user layer only (`user_association` + `prev2_word / prev2_tl`, lookup `(prev2, prev1) → prev1 → character`, age factor per vChewing `LXPerceptor.swift:968-1017`). Bundled trigrams never.

---

## Phase labels (cited from code)

| Phase | What it was |
|---|---|
| P3 | `association.bin` v2 writer + reader + artifact (D3) |
| P4a | Next-word word-key lookup + character backoff + `DICT_SCORE_CAP` (D4) |
| P4b | Lost continuous pairs: final commit carries `preceding` (D4) |
| P5 | Composing re-rank by previous word (D5) |
| P6 | Walker penalty-only bigram (D6), four PRs: P6.0 this rewrite · P6.1 `walker_gold.py bigram` simulator + report · P6.2 pair-count artifact + lexicon reader · P6.3 walker state + penalty + identity proof + prod regressions (`ue5`, `khikhi`) |
| P7 | Hanji-only sources via dictionary max-match — not adopted; the paired sources are the corpus |

## USER decisions

1. Predictions after Space — removed; Space keeps hiding predictions. Not to re-propose.
3. Hanji-only sources (P7) — not adopted.
4. Clause punctuation `，、；：,;:` — breaks the bundled chain and the next-word context (`behavioral-invariants.md` §40).
5. Source priority — `DICT_SCORE_CAP` = 250, clamp (§8 "equal count" → "always").
6. `lexicon::classification::is_hanji` reads `CJK_RANGES`, the same table as `dictionary/common/cjk.py` (parity test `lexicon/tests/cjk_ranges_parity.rs`; `behavioral-invariants.md` §15).

## Techniques not in this roadmap

The smart-suggestion techniques are numbered 1–11: (1) associated phrases keyed by `(漢字, TL)`; (2) record lost continuous pairs; (3) zero-query continuation from learned phrases; (4) two-word context with backoff; (5) negative feedback — drop user entries rarely picked when shown; (6) rule table for zero-query (`「`→`」`, number → measure word); (7) previous-word re-rank of composing candidates; (8) static word-bigram next-word table from a sentence corpus; (9) word → emoji; (10) offensive-word filter on unsolicited suggestions; (11) privacy gates (no learning in password / no-personalized-learning fields). This roadmap shipped 1, 2, 7, 8 and cut 4; 3, 5, 6, 9, 10, 11 are unscheduled.

---

## Deliberately not adopted

| Pattern | Why not |
|---|---|
| Neural LM re-rank (azooKey Zenzai, GGUF + llama.cpp) | no Taigi model; collides with the iOS 64 MB extension cap |
| Mandarin corpora, LLM-generated Taiwanese text (`tw-hokkien-seed-text`) | different grammar / synthetic model bias |
| New `bigram.bin` artifact | five install sites for zero gain (D3) |
| SQLite bundled LM (khiin, ChiaKey) | bundled read path is mmap everywhere; SQLite is user-data only (user-data-engine roadmap U2) |
| Full modified Kneser-Ney (continuation counts, three discounts) | next-word is per-key top-K; the walker uses only the interpolated absolute-discount form (D6) — one constant, enough to price a split |
| ChiaKey's `max(topBigram, topUnigram + bow(prev))` node rule as the walker cost (`references/ChiaKey/ChiaKey-Source/Frameworks/Manjusri/Headers/Node.h:419-445`) | the full conditional regressed the gold set (D6 table: seg 96 → 93 %, whole-key losers 223 → 666); D6 keeps only its penalty side |
| Bonus-only walker term (the pre-2026-10-30 D6) | a seen pair (`有→的`, 411) would make the reported split cheaper still |
| mozc POS connection matrix | needs POS tags no Taigi source carries |
| Bundled trigrams | sparse corpus, table size (D7) |
| Tokenization inside `taigi-corpus` | that repo's principle; alignment stays in `dictionary/build` |
| Building `association.bin` from the submodule at `make dict` time | opt-in 670 MB submodule would make the artifact machine-dependent; the checked-in TSV keeps the SHA gate |
