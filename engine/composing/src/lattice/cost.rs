//! v3.5.8 S5 — whole-sentence walker edge **cost** (min-cost model).
//!
//! **Lower = better.** The walker's path objective is `min Σ edge_cost`
//! over the chosen forward-only edge chain. This is a faithful port of
//! the khiin word-level DP segmenter
//! (`references/khiin-rs/khiin/src/data/segmenter.rs:82-93` +
//! `segment_min_cost` line 192), which is the same optimum as the
//! McBopomofo Gramambular `max Σ log P` relaxation
//! (`references/McBopomofo/algorithm.md`, `reading_grid.cpp:134`) and
//! librime's Viterbi-over-DAG: minimizing `Σ −ln(p)` ≡ maximizing the
//! path log-probability.
//!
//! ## Why S5 (the S2/S3 defect this slice corrects)
//!
//! S2/S3 (`edge_score`, removed here) cited khiin's formula but dropped
//! its two load-bearing parts: the `ln(1/p)` **probability
//! normalization** and the **minimization**. The result —
//! `(1 + ln(1 + freq)) × syll_bias × user_weight`, *maximized* and
//! summed — gave every edge a positive `1 +` floor and no per-segment
//! normalization, so a finer segmentation accumulated *more* reward.
//! With real dictionary frequencies (common single-character morphemes
//! like 伊 ≈ 63 255, 有 ≈ 53 685 dwarf any specific phrase such as
//! 台灣 ≈ 1 379) the all-single-char path won by ~4.7×: typing
//! `taiuan` surfaced `乾伊有俺` instead of `台灣` (dogfood-confirmed
//! 2026-05-17; `docs/roadmap.md` §整句 lattice + walker S5).
//!
//! ## The model (khiin `segmenter.rs:82-92`)
//!
//! ```text
//! p          = (1 + freq) / CORPUS_TOTAL_FREQ          // Laplace-smoothed corpus probability
//! cost       = ln(1 / p)                               // = ln(CORPUS) − ln(1 + freq) ≥ 0
//! cost       = cost / toneless_len^LETTER_COUNT_BIAS   // longer spelling → cheaper
//!                   × syllable_count^SYLLABLE_COUNT_BIAS
//! applied_δ  = syll <= 1 ? user_weight_delta × SINGLE_SYLLABLE_SCALE
//!                        : user_weight_delta
//! cost      −= ln(1 + applied_δ)                       // user preference = log-space discount
//! ```
//!
//! `CORPUS_TOTAL_FREQ > 1 + max(freq)`, so `p ∈ (0, 1)` and
//! `ln(1/p) > 0` for every **dict** edge (a freq-0 dict record is
//! still dict-priced — the branch is gated on `dict_hit`, never
//! `frequency == 0`). The decisive structural property the old
//! max-Σ model lacked: **every dict edge pays the
//! ~`ln(CORPUS_TOTAL_FREQ)` corpus-normalization toll**, so a finer
//! segmentation accumulates strictly more cost and a real phrase
//! out-competes its single-character decomposition. An OOV
//! (no-dict-hit) edge does NOT take this probability path at all —
//! RC0 prices it as khiin's per-char `BIG` (`OOV_PER_CHAR_PENALTY *
//! toneless_len`); see the OOV section below.
//!
//! ## OOV (no-dict-hit) edges — the v3.5.8 RC0 fix
//!
//! khiin has TWO distinct mechanisms; S5/S7 conflated them. (1) A
//! **known dictionary word** whose corpus probability is `<= 0` is
//! floored at `p = 1e-5 / 10^word_len` and priced on the `ln(1/p)`
//! curve (`segmenter.rs:75-92`) — a corpus-gap floor for an *indexed*
//! word. (2) A span with **no dictionary word** is not priced by any
//! probability at all: `segment_min_cost:198-206` advances one char
//! paying `BIG = 1e10`, so a contiguous uncovered region accumulates
//! `BIG` **per char**. Real words are `ln`-scale (~5-30), so in khiin
//! a single OOV char dominates any dict-coverable segmentation at any
//! length.
//!
//! S5 priced a genuine OOV span with mechanism (1)'s smooth
//! corpus-normalized probability (`ln`-scale). A whole-buffer OOV blob
//! is ONE edge, so it paid khiin's `÷ word_len^0.2` discount once over
//! the full buffer and undercut a dict-covering path that accumulates a
//! per-edge `ln(CORPUS/freq)` toll **linearly in edge count**. Past ~6
//! dict edges the blob won → `any_dict == false` → the carve-out
//! rendered bare roman (`ginalangtsiahpngbesai → "gin a lang tsiah png
//! be sai"` instead of 囡仔人食飯袂使; threshold exactly 6 syllables,
//! reproduced against the real dictionary 2026-05-18). S7's
//! `UNKNOWN_SYLLABLE_DECAY` only softened the slope; it did not
//! de-conflate the two mechanisms (`taiuanta`/`taiuantai` only happened
//! to fall below the 6-edge threshold).
//!
//! RC0 de-conflates them: an OOV edge is khiin's mechanism (2) —
//! `OOV_PER_CHAR_PENALTY * toneless_len` (per-char `BIG`), unbiased and
//! with no user discount. So "OOV loses to any dict-coverable path"
//! holds at **every length** — a *cost property* (khiin's own `BIG`
//! constant), NOT a lexicographic `dict_hit` short-circuit (Codex
//! pre-impl S7 Q1 / RC0 Q2). For a buffer with **no dictionary hit
//! anywhere** the min-cost path is still all-OOV; the user-facing
//! per-syllable romanization is produced by an explicit carve-out in
//! `continuous::fetch_walker_slot0_inner`, **outside** this cost function
//! (Codex pre-impl S5 Q2) — the OOV pricing here only governs *path
//! selection* (dict path vs OOV blob), never the rendered string.

// 中文: 全句 walker 單條 edge 成本(越小越好;min Σ cost = khiin segment_min_cost = McBopomofo max Σ log P)。
// 中文: 字典分支忠實移植 khiin segmenter.rs:82-92:p=(1+freq)/CORPUS、cost=ln(1/p)/len^0.2·syll^0.2、user log-space 折減。
// 中文: S2/S3 把 khiin 的 ln(1/p) 正規化 + minimize 拿掉 → maximize 結構性獎勵過度切分(taiuan→乾伊有俺,2026-05-17)。
// 中文: RC0 — khiin 兩機制被 S5/S7 混淆:(1) 已索引詞 p<=0 下限 1e-5/10^word_len(ln 尺度);
// 中文:   (2) segment_min_cost:198-206 真・未覆蓋 span 每字元付 BIG=1e10。S5 用 (1) 平滑機率定價真・OOV
// 中文:   → 整段 blob 一次吃 ÷word_len^0.2 折扣贏過逐邊累 toll 的字典路徑(>6 邊翻車 → carve-out 吐純羅馬字;
// 中文:   ginalangtsiahpngbesai 門檻恰 6 音節,2026-05-18 真實字典重現);S7 只調斜率沒解混淆。
// 中文: RC0 解混淆:OOV edge = OOV_PER_CHAR_PENALTY × toneless_len(khiin 每字元 BIG),無 bias/折減
// 中文:   →「OOV 輸給任何字典可覆蓋路徑」任何長度恆成立 = 成本性質非 dict_hit lexicographic 短路。
// 中文: 無字典命中 buffer 的逐音節羅馬字由 continuous::fetch_walker_slot0_inner 顯式 carve-out 產生(不進本函式,Codex S5 Q2)。

/// Total corpus frequency mass — the denominator that turns a raw
/// `DictionaryRecord.frequency` into a corpus probability
/// (khiin `segmenter.rs:82-86`, where `p = occurrences / total`).
///
/// **Provenance**: `Σ frequency` over every row of
/// `dictionary/output/dictionary.csv` (159 034 entries) =
/// `12_910_574`, measured 2026-05-17. `dict.bin` v2 carries only
/// per-record frequency, no corpus-total metadata
/// (`dictionary/build/create_dictionary_bin.py`), so this is baked as a
/// constant rather than summed at lexicon init (Codex pre-impl S5 Q4 =
/// option a, 2026-05-17): the exact value is not highly sensitive
/// (khiin itself notes "experiment with different models") and a
/// checked-in artifact + checked const is simpler and audit-friendlier
/// than new lexicon plumbing + a startup scan.
///
/// **Regeneration guard (v3.5.9 A4)**: the dictionary build pipeline
/// emits `dictionary/output/corpus_total_freq.txt` (a small key=value
/// artifact, see
/// `dictionary/build/create_dictionary_bin.py::write_corpus_stats`)
/// and the `corpus_total_freq_matches_dictionary_csv` test below reads
/// it and asserts `CORPUS_TOTAL_FREQ == Σ frequency`. A dictionary
/// rebuild that changes the sum and forgets to update this constant
/// fails `cargo test --workspace`, not silent drift. **Recompute and
/// update this constant whenever the dictionary is rebuilt** — the
/// artifact will print the expected value in the failure message.
// 中文: 語料總頻 = dictionary.csv 全列 frequency 加總(159034 列,12_910_574,2026-05-17 量測)。
// 中文: dict.bin v2 無語料總計 metadata → bake 成常數 + pipeline-emitted artifact 守門(Codex S5 Q4=a / v3.5.9 A4);字典重建須同步更新此值,失敗訊息會列實測值。
pub(crate) const CORPUS_TOTAL_FREQ: f64 = 12_910_574.0;

/// Compile-time invariant: `CORPUS_TOTAL_FREQ` must exceed
/// `1 + max(freq)` (的 = 184_693) so every `ln(1/p)` is strictly
/// positive — no zero/negative edge cost, no `ln` of a non-positive
/// number. A dictionary rebuild that pushes a single entry past this
/// (or a mistyped constant) fails the **build**, not just a test.
const _: () = assert!(CORPUS_TOTAL_FREQ > 184_694.0);

/// khiin `LETTER_COUNT_BIAS` (`segmenter.rs:20`, default `0.2`):
/// `cost / toneless_len^0.2` — a longer spelling is a (mildly) cheaper
/// edge, biasing the DP toward fewer, longer words. Kept as a named
/// cited constant, not a runtime tunable (Codex pre-impl S5 Q6, 2026-05-17:
/// changing the model and tuning it in one patch makes regressions
/// harder to reason about; dogfood can tune later).
// 中文: khiin LETTER_COUNT_BIAS(segmenter.rs:20,0.2);長拼寫 → 較便宜,偏好少而長的詞。固定 cited 常數非 runtime tunable。
pub(crate) const LETTER_COUNT_BIAS: f64 = 0.2;

/// khiin `SYLLABLE_COUNT_BIAS` (`segmenter.rs:28`, default `0.2`):
/// `cost × syllable_count^0.2` — a word spanning more syllables pays
/// slightly more, the khiin counter-bias that keeps `letter` length
/// preference from over-favouring very long single entries.
// 中文: khiin SYLLABLE_COUNT_BIAS(segmenter.rs:28,0.2);跨較多音節的詞略貴,平衡 letter 長度偏好。
pub(crate) const SYLLABLE_COUNT_BIAS: f64 = 0.2;

/// Compile-time invariant: both khiin length-normalization exponents
/// must stay positive — a `0.0` exponent collapses `x^bias` to `1.0`
/// and silently removes the length/syllable normalization that makes
/// the model penalize over-segmentation.
const _: () = assert!(LETTER_COUNT_BIAS > 0.0 && SYLLABLE_COUNT_BIAS > 0.0);

/// v3.5.8 RC0 fix — the per-char penalty an **OOV (no-dict-hit)** span
/// pays. khiin's literal `BIG` (`segmenter.rs:29` `const BIG: f64 =
/// 1e10`).
///
/// khiin has TWO distinct mechanisms that S5/S7 conflated:
///
/// 1. `segmenter.rs:75-92` (`Segmenter::new`) — a **known dictionary
///    word** whose corpus probability is `<= 0.0` is floored at
///    `p = 1e-5 / 10^word_len` and priced on the `ln(1/p)` curve. This
///    is a corpus-gap floor for an *indexed* word, NOT the price of a
///    genuinely uncovered span.
/// 2. `segment_min_cost:198-206` — a span with **no dictionary word**
///    is not priced by any probability at all: the DP advances one
///    char paying `costs[i-1] + BIG` (`BIG = 1e10`), so a contiguous
///    uncovered region accumulates `BIG` **once per char**. Real words
///    are `ln`-scale (~5-30); a single OOV char (`1e10`) therefore
///    dominates any dict-coverable segmentation at any length — khiin
///    essentially never emits an OOV blob when a dict path exists.
///
/// S5 priced a genuine OOV span with mechanism (1)'s smooth
/// corpus-normalized probability (`ln`-scale), so a whole-buffer OOV
/// blob — one edge paying khiin's `÷ word_len^0.2` discount once over
/// the full buffer — undercut a dict-covering path that accumulates a
/// per-edge `ln(CORPUS/freq)` toll linearly in edge count. Past ~6
/// dict edges the blob won → `any_dict == false` → the
/// `continuous::fetch_walker_slot0_inner` carve-out rendered bare roman
/// (`ginalangtsiahpngbesai → "gin a lang tsiah png be sai"` instead of
/// 囡仔人食飯袂使; threshold exactly 6 syllables, reproduced against the
/// real dictionary 2026-05-18). S7's `UNKNOWN_SYLLABLE_DECAY` only
/// softened the slope; it did not de-conflate the two mechanisms.
///
/// RC0 de-conflates them: an OOV edge costs `OOV_PER_CHAR_PENALTY *
/// toneless_len` (khiin's per-char `BIG`; `toneless_len` = the edge's
/// char count = khiin `word_len`), unbiased and with no user discount
/// (khiin's `BIG` is raw; an OOV edge always carries
/// `user_weight_delta = 0`). The dictionary branch keeps the full
/// khiin cost-map formula. This makes "OOV loses to any dict-coverable
/// path" hold at every length — as a **cost property** (khiin's own
/// `BIG` constant), NOT a lexicographic `dict_hit` short-circuit
/// (Codex pre-impl S7 Q1 / RC0 Q2). A cited constant, not a runtime
/// tunable (same rationale as the khiin bias exponents).
// 中文: RC0 修 — OOV(無字典命中)span 的「每字元」懲罰 = khiin 字面 BIG(segmenter.rs:29 `1e10`)。
// 中文: khiin 兩個被 S5/S7 混淆的機制:(1) segmenter.rs:75-92 已索引詞 p<=0 的語料缺口下限
// 中文:   p=1e-5/10^word_len(ln 尺度);(2) segment_min_cost:198-206 真・未覆蓋 span 不用任何機率,
// 中文:   DP 每前進一字元付 costs[i-1]+BIG(1e10),整段累加 BIG/字元 → 單一 OOV 字元(1e10)
// 中文:   壓過任何 ln 尺度(~5-30)字典切分,任何長度皆然(khiin 有字典路徑就幾乎不吐 OOV blob)。
// 中文: S5 用機制(1)的平滑機率定價真・OOV span → 整段 blob 一次吃 ÷word_len^0.2 折扣、贏過逐邊累 toll
// 中文:   的字典路徑(>6 邊翻車 → any_dict=false → carve-out 吐純羅馬字;ginalangtsiahpngbesai→純羅馬字,
// 中文:   門檻恰 6 音節,2026-05-18 真實字典重現)。S7 只調斜率沒解混淆。
// 中文: RC0 解混淆:OOV edge = OOV_PER_CHAR_PENALTY × toneless_len(khiin 每字元 BIG;toneless_len=khiin word_len),
// 中文:   無 bias、無 user 折減(khiin BIG 是 raw;OOV edge user_weight_delta 恆 0);字典分支維持完整 khiin 公式。
// 中文:   「OOV 輸給任何字典可覆蓋路徑」恆成立 = 成本性質(khiin 自己的 BIG 常數)非 dict_hit lexicographic 短路。
pub(crate) const OOV_PER_CHAR_PENALTY: f64 = 1e10;

/// Compile-time invariant: the OOV per-char penalty must dominate any
/// realistic dict-coverable path sum. A single dict edge's `ln(1/p)`
/// is at most `ln(CORPUS_TOTAL_FREQ) ≈ 16.4`, scaled by the khiin
/// biases to `O(50)`; even a thousand-edge sentence stays `O(1e5)`.
/// `1e10` per OOV char is `>= 1e5` orders clear of that, so a
/// dict-coverable buffer can never collapse to an OOV blob (RC0). The
/// guard also keeps the penalty strictly positive (finite f64 at the
/// `u8::MAX` toneless-length extreme: `1e10 * 255 = 2.55e12`).
const _: () = assert!(OOV_PER_CHAR_PENALTY >= 1e5);

/// v3.5.8 S3/S5 — fraction of the decayed user-frequency delta a
/// **single-syllable** (`syllable_count <= 1`) walker edge receives.
/// `0.0` = single-syllable edges get NO walker user discount, so a hot
/// single character (e.g. 「的」, dict freq ~184k) cannot ride its own
/// stale user history to sweep the whole sentence. That single-char
/// user preference is already honored where it belongs:
/// `lexicon::best_candidate_for_key` record selection (homophone
/// disambiguation *within* the edge). Letting it also discount the
/// *path objective* is the exact stale single-character-domination
/// failure the user-weight term must not reopen (Codex pre-impl S3 Q4c,
/// 2026-05-16, BLOCK condition — preserved verbatim through the S5
/// log-space rewrite: at scale `0.0` the discount is `ln(1) = 0`).
/// Multi-syllable edges get the full delta (scale `1.0`, applied in
/// [`edge_cost`]). **Dogfood-tunable 0.0..=0.25** if dogfood shows
/// single-syllable user preference is under-weighted in segmentation.
// 中文: S3/S5 — 單音節 edge 拿到的 decayed user delta 比例;0.0 = 不拿 walker user 折減。
// 中文:   熱門單字(如「的」)不可靠自己 stale user 史壓垮整句(Codex S3 Q4c BLOCK,S5 log-space 改寫沿用:scale 0 → ln(1)=0)。
// 中文:   其 user 偏好已由 best_candidate_for_key 選 record 時體現。多音節 edge 拿滿。dogfood 可調 0.0..=0.25。
pub(crate) const WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE: f64 = 0.0;

/// v3.5.8 S6 (Codex pre-impl S6 Q1, 2026-05-17, BLOCK condition) —
/// the **effective corpus frequency** a `custom_dictionary.db` edge is
/// scored with in the S5 min-cost model.
///
/// A custom entry carries no corpus frequency. Rather than an explicit
/// cost floor / discount — which would bypass the "every edge pays the
/// `ln(CORPUS_TOTAL_FREQ)` normalization toll" invariant that S5 added
/// to kill over-segmentation (Codex S6 Q1 **BLOCK**ed a floor) — a
/// custom edge enters [`edge_cost`] with this *effective* frequency, so
/// it competes inside the same probability-shaped objective as a
/// dictionary phrase (librime user-dict-as-frequency semantics,
/// `references/librime/src/rime/dict/user_dictionary.cc`).
///
/// **Value provenance** (Codex pre-impl S6 Q1 gate, measured on the
/// 2026-05-17 `dictionary/output/dictionary.csv`): multi-syllable max
/// frequency ≈ 1_562, single-syllable p95 ≈ 1_461, single-syllable
/// p99 ≈ 7_721. `2_000` makes a custom entry a strong **multi-syllable
/// phrase** competitor (beats a top single-character split) without
/// elevating it to a top-frequency single character (a single-syllable
/// custom edge still loses badly to a real phrase path — pinned by
/// [`tests`]). **Dogfood-tunable**; a named cited constant, not a
/// runtime tunable nor a magic literal.
// 中文: S6 — custom_dictionary.db edge 在 S5 min-cost 模型用的「等效語料頻率」(Codex Q1 BLOCK:用 proxy 不用 cost floor,
// 中文:   floor 會繞過「每 edge 付 ln(CORPUS) 正規化稅」不變式)。2_000 = dictionary.csv 多音節 max≈1562 / 單音節 p95≈1461 / p99≈7721
// 中文:   之間 → custom 是強多音節片語競爭者(勝單字拆分)但非頂頻單字(單音節 custom 仍輸真實片語,測試 pin)。dogfood 可調。
pub(crate) const CUSTOM_EFFECTIVE_FREQ: u32 = 2_000;

/// Min-cost for one lattice edge (**lower = better**).
///
/// - `frequency` — the chosen dictionary candidate's raw
///   `DictionaryRecord.frequency`; **unused when `dict_hit == false`**.
///   Raw frequency, never a ranked `score` (Codex pre-impl S5 Q3:
///   `c.score` must not feed the walker cost — that double-counts the
///   record-selection signal).
/// - `syllable_count` — the dict candidate's syllable count (`>= 1`;
///   clamped, defends against a leaked `0`); drives the khiin
///   `n_syls^0.2` bias. For an OOV edge it is **metadata only** (the
///   synthesized candidate's syllable sum) and does NOT enter the cost
///   (RC0 — the OOV penalty is char-length-keyed, not syllable-keyed;
///   Codex pre-impl RC0 Q3).
/// - `toneless_len` — the edge's toneless-key char count (khiin's
///   `word_len`). Both the dict `÷ word_len^0.2` bias and the OOV
///   per-char penalty multiplier (Codex pre-impl S5 Q1 BLOCK; RC0).
/// - `user_weight_delta` — the time-decayed user-frequency boost delta
///   for this edge's chosen candidate
///   (`ranking::decayed_user_weight_delta`, in `0.0..=4.0`; `0.0` =
///   no user history / never selected / bad clock = neutral). Computed
///   caller-side (`continuous::fetch_walker_slot0_inner`). OOV edges always
///   carry `0.0` and take no discount.
/// - `dict_hit` — `true` iff this edge is a lexicon-backed hit
///   (`dict.bin` record OR a `custom_dictionary.db` entry). The OOV
///   branch is selected solely from this flag, **never** inferred from
///   `frequency == 0`: a real dictionary record may legitimately have
///   frequency 0, and a custom edge is scored at the
///   `CUSTOM_EFFECTIVE_FREQ` proxy — both must use the dictionary
///   formula, not the OOV penalty (Codex pre-impl Q2; "never infer
///   no-dict from freq" invariant, `cost.rs` head + `dispatch.rs`
///   no-dict branch).
///
/// Two khiin mechanisms, de-conflated by RC0 (see
/// [`OOV_PER_CHAR_PENALTY`]):
/// - **dict** (`dict_hit`): khiin `segmenter.rs:82-92` —
///   `cost = ln(1 / p) / toneless_len^0.2 × syllable_count^0.2
///   − ln(1 + applied_delta)`, `p = (1 + frequency) /
///   CORPUS_TOTAL_FREQ`. `> 0` before the discount
///   (`CORPUS_TOTAL_FREQ > 1 + max(freq)`); the `ln(1 + δ) >= 0`
///   subtraction (δ clamped `>= 0`) only ever *lowers* cost (Codex
///   pre-impl S5 Q3 — log-space sign).
/// - **OOV** (`!dict_hit`): khiin `segment_min_cost:198-206` —
///   `cost = OOV_PER_CHAR_PENALTY × toneless_len` (khiin's `BIG` per
///   advanced char), unbiased, no user discount. `>= 1e10` per char
///   dominates every `ln`-scale dict-coverable path, so a
///   dict-coverable buffer never collapses to an OOV blob at any
///   length (RC0) — a **cost property**, not a lexicographic
///   `dict_hit` rule (Codex pre-impl S7 Q1 / RC0 Q2).
///
/// Always finite and `> 0`.
// 中文: 單 edge 最小化成本(越小越好),RC0 de-conflate khiin 兩機制:
// 中文: dict_hit → khiin segmenter.rs:82-92:ln(1/p)/len^0.2·syll^0.2 − ln(1+δ),p=(1+freq)/CORPUS;
// 中文:   分支只由 dict_hit 選,絕不從 freq==0 推(真實字典詞可 freq 0、custom 用 proxy freq)。
// 中文: !dict_hit → khiin segment_min_cost:198-206:OOV_PER_CHAR_PENALTY × toneless_len(khiin 每字元 BIG),
// 中文:   無 bias、無 user 折減;每字元 ≥1e10 壓過任何 ln 尺度字典可覆蓋路徑 → 字典可覆蓋 buffer 任何長度
// 中文:   都不會塌成 OOV blob(RC0)= 成本性質非 dict_hit lexicographic 規則。OOV 的 syllable_count/frequency 僅 metadata。
pub(crate) fn edge_cost(
    frequency: u32,
    syllable_count: u8,
    toneless_len: usize,
    user_weight_delta: f64,
    dict_hit: bool,
) -> f64 {
    if !dict_hit {
        // khiin `segment_min_cost:198-206`: a span with no dictionary
        // word is not priced by any probability — the DP pays `BIG`
        // per advanced char. `toneless_len` = the span's char count =
        // khiin `word_len`. Unbiased, no user discount (khiin's `BIG`
        // is raw; an OOV edge always carries `user_weight_delta = 0`).
        // `frequency` / `syllable_count` are intentionally unused here:
        // they are OOV metadata for the synthesized candidate, not cost
        // inputs (Codex pre-impl RC0 Q3).
        return OOV_PER_CHAR_PENALTY * toneless_len.max(1) as f64;
    }
    // khiin segmenter.rs:82-89 — Laplace-smoothed corpus probability.
    // A dict record with frequency 0 is still dict-priced (Codex
    // pre-impl Q2): the OOV branch is gated on `dict_hit`, never
    // `frequency == 0`.
    let p = (1.0 + f64::from(frequency)) / CORPUS_TOTAL_FREQ;
    let mut cost = (1.0 / p).ln();
    // khiin segmenter.rs:90-92 — `cost / word_len^0.2 * n_syls^0.2`.
    let len_bias = (toneless_len.max(1) as f64).powf(LETTER_COUNT_BIAS);
    let syll_bias = f64::from(syllable_count.max(1)).powf(SYLLABLE_COUNT_BIAS);
    cost = cost / len_bias * syll_bias;
    // S5 (Codex pre-impl Q3): the S3 user preference becomes a
    // log-space cost discount. Syllable-aware — a 1-syllable edge gets
    // only SCALE (=0.0) of the decayed delta so a hot single char
    // cannot ride the discount to sweep the sentence (the Q4c BLOCK
    // guard from #286, preserved: at scale 0.0 the discount is
    // `ln(1) = 0`). δ clamped `>= 0` so the discount cannot become a
    // penalty on a malformed input.
    let applied_delta = if syllable_count <= 1 {
        user_weight_delta * WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE
    } else {
        user_weight_delta
    };
    cost - applied_delta.max(0.0).ln_1p()
}

#[cfg(test)]
mod tests {
    use super::{
        edge_cost, CORPUS_TOTAL_FREQ, CUSTOM_EFFECTIVE_FREQ, LETTER_COUNT_BIAS,
        OOV_PER_CHAR_PENALTY, SYLLABLE_COUNT_BIAS, WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE,
    };

    /// Neutral-user-weight **dict-hit** edge cost — no user history
    /// (`user_weight_delta = 0.0`, `dict_hit = true`). The dictionary
    /// model invariants are expressed through this; a real dictionary
    /// record always uses dict pricing even at frequency 0 (Codex
    /// pre-impl Q2).
    fn ec(freq: u32, syll: u8, len: usize) -> f64 {
        edge_cost(freq, syll, len, 0.0, true)
    }

    /// Neutral-user-weight **OOV** (no-dict-hit) edge cost. RC0:
    /// `frequency` AND `syll` are both irrelevant in this branch — an
    /// OOV edge is priced solely `OOV_PER_CHAR_PENALTY * len` (khiin's
    /// per-char `BIG`), so `freq` is fixed at 0 and `syll` is metadata.
    fn oov(syll: u8, len: usize) -> f64 {
        edge_cost(0, syll, len, 0.0, false)
    }

    /// Resolve `dictionary/output/corpus_total_freq.txt` relative to
    /// this crate's manifest, regardless of test CWD. The path is
    /// `<repo>/engine/composing/../../dictionary/output/...`.
    fn corpus_total_freq_artifact_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("dictionary")
            .join("output")
            .join("corpus_total_freq.txt")
    }

    /// Parse the v3.5.9 A4 regeneration-guard artifact. Format:
    /// ```text
    /// total_frequency=<u64>
    /// entries=<u32>
    /// ```
    /// Order-insensitive; blank lines tolerated; both keys required.
    fn parse_corpus_stats(contents: &str) -> (u64, u32) {
        let mut total: Option<u64> = None;
        let mut entries: Option<u32> = None;
        for (lineno, raw) in contents.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once('=').unwrap_or_else(|| {
                panic!(
                    "corpus_total_freq.txt line {}: missing '=': {raw:?}",
                    lineno + 1
                )
            });
            match key.trim() {
                "total_frequency" => {
                    if total.is_some() {
                        panic!(
                            "corpus_total_freq.txt line {}: duplicate `total_frequency=`",
                            lineno + 1
                        );
                    }
                    total = Some(value.trim().parse().unwrap_or_else(|e| {
                        panic!(
                            "corpus_total_freq.txt line {}: bad u64 {value:?}: {e}",
                            lineno + 1
                        )
                    }));
                }
                "entries" => {
                    if entries.is_some() {
                        panic!(
                            "corpus_total_freq.txt line {}: duplicate `entries=`",
                            lineno + 1
                        );
                    }
                    entries = Some(value.trim().parse().unwrap_or_else(|e| {
                        panic!(
                            "corpus_total_freq.txt line {}: bad u32 {value:?}: {e}",
                            lineno + 1
                        )
                    }));
                }
                other => {
                    panic!(
                        "corpus_total_freq.txt line {}: unknown key {other:?}",
                        lineno + 1
                    )
                }
            }
        }
        let total = total.expect("corpus_total_freq.txt missing `total_frequency=`");
        let entries = entries.expect("corpus_total_freq.txt missing `entries=`");
        (total, entries)
    }

    #[test]
    fn corpus_total_freq_matches_dictionary_csv() {
        // v3.5.9 A4 regeneration guard. `CORPUS_TOTAL_FREQ` is the
        // baked literal consumed by `edge_cost`; this test cross-checks
        // it against `Σ frequency` over `dictionary/output/dictionary.csv`
        // as emitted by `create_dictionary_bin.py::write_corpus_stats`.
        // A4 only adds the verifier — `edge_cost` still reads the
        // literal at runtime (Codex pre-impl S5 Q4 = option a).
        //
        // Skip semantics (Codex pre-impl A4 B2): artifact-NotFound +
        // dictionary.csv-NotFound = minimal checkout (no dictionary
        // submodule), skip with clear message. Artifact-NotFound while
        // dictionary.csv is present = pipeline failed to emit; FAIL
        // loudly, never silent. Any other IO/parse error also FAILs.
        //
        // Entries assertion is secondary (Codex Q4 SHOULD): catches
        // "same Σ, different record universe" drift. Hand-typed
        // `EXPECTED_DICT_ENTRIES` matches the constant doc-comment
        // (159 034 @ 2026-05-17); update alongside the constant on
        // dictionary rebuild.
        const EXPECTED_DICT_ENTRIES: u32 = 159_034;

        let artifact = corpus_total_freq_artifact_path();
        let csv = artifact.parent().unwrap().join("dictionary.csv");

        let contents = match std::fs::read_to_string(&artifact) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Use `metadata()` not `exists()` so a non-NotFound CSV
                // error (permissions, …) is NOT collapsed to "absent"
                // and silently routed into the skip branch (Codex post-
                // impl A4 nit).
                match std::fs::metadata(&csv) {
                    Ok(_) => panic!(
                        "{} is missing but {} exists — the dictionary pipeline \
                         did not emit the regeneration guard. Re-run \
                         `bash dictionary/build.sh` (or `make dict`).",
                        artifact.display(),
                        csv.display(),
                    ),
                    Err(csv_err) if csv_err.kind() == std::io::ErrorKind::NotFound => {
                        eprintln!(
                            "skip corpus_total_freq_matches_dictionary_csv: {} and {} \
                             both absent (minimal build context without the dictionary \
                             submodule). Run `make dict` to enable the \
                             CORPUS_TOTAL_FREQ regeneration guard.",
                            artifact.display(),
                            csv.display(),
                        );
                        return;
                    }
                    Err(csv_err) => panic!(
                        "{} is missing and probing {} failed: {csv_err}",
                        artifact.display(),
                        csv.display(),
                    ),
                }
            }
            Err(e) => panic!("failed to read {}: {e}", artifact.display()),
        };

        let (measured_sum, measured_entries) = parse_corpus_stats(&contents);

        // Compare in f64 — `measured_sum as f64` is exact for sums far
        // below 2^53 (current corpus ≈ 1.29e7). NEVER cast the const
        // down to u64; a mistyped `12_910_574.5` would truncate and
        // pass silently (Codex pre-impl A4 B1).
        assert_eq!(
            CORPUS_TOTAL_FREQ, measured_sum as f64,
            "CORPUS_TOTAL_FREQ ({CORPUS_TOTAL_FREQ}) drifted from \
             dictionary/output/dictionary.csv Σfrequency={measured_sum} \
             over {measured_entries} entries — update the constant in \
             engine/composing/src/lattice/cost.rs."
        );

        assert_eq!(
            measured_entries, EXPECTED_DICT_ENTRIES,
            "dictionary.csv entry count drifted: artifact reports \
             {measured_entries}, EXPECTED_DICT_ENTRIES (test-local) is \
             {EXPECTED_DICT_ENTRIES}. Update EXPECTED_DICT_ENTRIES and the \
             CORPUS_TOTAL_FREQ doc comment (provenance line) alongside \
             the constant after a dictionary rebuild."
        );

        // The `> 1 + max(freq)` strict-positivity invariant is a
        // compile-time guard (`const _: () = assert!(…)` next to the
        // constant), not a runtime assertion here.
    }

    #[test]
    fn oov_edge_is_finite_and_strictly_positive() {
        // RC0: an OOV edge must stay finite & positive so the DP never
        // sees a `-inf`/`NaN`. Priced as khiin's per-char `BIG`:
        // `OOV_PER_CHAR_PENALTY * toneless_len`, unbiased.
        let c = oov(1, 3);
        assert!(c.is_finite() && c > 0.0, "{c}");
        let expected = OOV_PER_CHAR_PENALTY * 3.0;
        assert!((c - expected).abs() < 1e-9, "{c} vs {expected}");
    }

    #[test]
    fn freq_zero_dict_record_uses_dict_pricing_not_oov() {
        // Codex pre-impl Q2 BLOCK guard: a real `dict.bin` record may
        // legitimately have frequency 0. It must be priced on the
        // dictionary branch (`ln(CORPUS/1)`), NEVER the OOV per-char
        // penalty — the OOV branch is gated on `dict_hit`, not
        // `frequency == 0`.
        let dict_freq0 = ec(0, 2, 6); // dict_hit = true
        let expected =
            CORPUS_TOTAL_FREQ.ln() / 6f64.powf(LETTER_COUNT_BIAS) * 2f64.powf(SYLLABLE_COUNT_BIAS);
        assert!(
            (dict_freq0 - expected).abs() < 1e-9,
            "freq-0 dict edge must use dict pricing: {dict_freq0} vs {expected}"
        );
        // …and is strictly cheaper than the same-shape OOV edge.
        assert!(
            dict_freq0 < oov(2, 6),
            "freq-0 dict {dict_freq0} must be < OOV {}",
            oov(2, 6)
        );
    }

    #[test]
    fn higher_frequency_costs_less() {
        // Min-cost: a more frequent edge is cheaper (better).
        assert!(ec(1000, 1, 3) < ec(10, 1, 3));
        assert!(ec(10, 1, 3) < ec(0, 1, 3));
    }

    #[test]
    fn real_phrase_beats_its_single_char_decomposition() {
        // The motivating bug, with real dictionary frequencies.
        // `taiuan` → 台灣 (freq 1379, 2 syll, toneless "taiuan" len 6)
        // as ONE edge must cost strictly less than the 4 single-char
        // path 乾(2145,len2) 伊(63255,len1) 有(53685,len1) 俺(10635,len2)
        // — the path the broken S2/S3 max-Σ objective produced.
        let phrase = ec(1379, 2, 6);
        let four_singles = ec(2145, 1, 2) + ec(63255, 1, 1) + ec(53685, 1, 1) + ec(10635, 1, 2);
        assert!(
            phrase < four_singles,
            "phrase={phrase} four_singles={four_singles}"
        );
    }

    #[test]
    fn custom_two_syllable_beats_top_single_char_split() {
        // S6 Codex pre-impl Q1 regression guard #1: a 2-syllable custom
        // entry scored at CUSTOM_EFFECTIVE_FREQ with a short toneless
        // roman must out-compete the path through the two highest-
        // frequency single characters that cover the same span. Real
        // dictionary singles for the motivating `taigi` case:
        // 台(31281,len3) + 語(21976,len2). The custom 2-syll edge
        // (effective freq 2000, toneless "taigi" len 5) must cost less.
        // Custom edges are `dict_hit: true` (lexicon-backed, S6).
        let custom = edge_cost(CUSTOM_EFFECTIVE_FREQ, 2, 5, 0.0, true);
        let top_single_split =
            edge_cost(31_281, 1, 3, 0.0, true) + edge_cost(21_976, 1, 2, 0.0, true);
        assert!(
            custom < top_single_split,
            "custom={custom} top_single_split={top_single_split}"
        );
    }

    #[test]
    fn two_single_syllable_custom_lose_to_a_real_phrase_path() {
        // S6 Codex pre-impl Q1 regression guard #2: CUSTOM_EFFECTIVE_FREQ
        // must NOT elevate custom to a top-frequency single character —
        // a path of two single-syllable custom edges still loses badly
        // to one real multi-syllable dictionary phrase covering the same
        // buffer. 台灣 (freq 1379, 2 syll, toneless "taiuan" len 6) as
        // ONE edge must cost strictly less than two atomic custom edges.
        let two_single_custom = edge_cost(CUSTOM_EFFECTIVE_FREQ, 1, 3, 0.0, true)
            + edge_cost(CUSTOM_EFFECTIVE_FREQ, 1, 3, 0.0, true);
        let real_phrase = edge_cost(1_379, 2, 6, 0.0, true);
        assert!(
            real_phrase < two_single_custom,
            "real_phrase={real_phrase} two_single_custom={two_single_custom}"
        );
    }

    #[test]
    fn syllable_zero_clamps_to_one() {
        // syllable_count is a u8 from a record; a leaked 0 must clamp
        // to 1 on the dict branch (no powf(0)=… surprise, takes the
        // single-syllable user-scale branch identical to syll = 1).
        // The OOV branch ignores syllable_count entirely (RC0:
        // char-keyed penalty), so 0 vs 1 is trivially equal there.
        assert_eq!(edge_cost(0, 0, 3, 3.5, true), edge_cost(0, 1, 3, 3.5, true));
        assert_eq!(
            edge_cost(0, 0, 3, 0.0, false),
            edge_cost(0, 1, 3, 0.0, false)
        );
    }

    #[test]
    fn oov_blob_loses_to_intended_best_dict_path() {
        // Earlier dogfood bug (`taiuanta`), real frequencies.
        // `taiuanta` (len 8) with no whole-buffer dict word: the single
        // OOV blob edge (0,8) must cost strictly MORE than the
        // dict-covering path 台灣(0,6, freq 1379, "taiuan" len 6) +
        // 焦(6,8, freq 2145, "ta" len 2). RC0: the blob is now khiin's
        // `BIG`-per-char (`8 * OOV_PER_CHAR_PENALTY ≈ 8e10`) so it
        // dominates the `ln`-scale dict path by ~9 orders — it is a
        // **cost property** (khiin's `BIG` constant), not a
        // lexicographic `dict_hit` rule (Codex pre-impl S7 Q1 / RC0
        // Q2).
        let oov_blob = oov(3, 8);
        let dict_path = ec(1379, 2, 6) + ec(2145, 1, 2);
        assert!(
            oov_blob > dict_path,
            "oov_blob={oov_blob} must exceed dict_path={dict_path}"
        );
    }

    #[test]
    fn oov_costs_more_than_freq_zero_dict_for_multi_syllable() {
        // A multi-char OOV span must cost strictly more than a freq-0
        // *dict* edge of the same shape — `dict_hit` selects the
        // branch, never `frequency == 0` (Codex pre-impl Q2). RC0: the
        // OOV side is `8 * OOV_PER_CHAR_PENALTY`, the dict side is
        // `ln`-scale.
        assert!(
            oov(3, 8) > ec(0, 3, 8),
            "OOV {} must exceed freq-0 dict pricing {}",
            oov(3, 8),
            ec(0, 3, 8)
        );
    }

    #[test]
    fn oov_cost_is_per_char_and_length_independent_of_syllables() {
        // RC0: khiin `segment_min_cost:198-206` prices an uncovered
        // span as `BIG` per advanced char. So OOV cost is exactly
        // `OOV_PER_CHAR_PENALTY * toneless_len`, strictly increasing in
        // char length and INDEPENDENT of syllable count (the S7
        // `10^syllable_count` decay is gone — syllable_count is OOV
        // metadata only). This is what structurally stops a long OOV
        // blob from ever undercutting a dict-coverable path (RC0).
        assert_eq!(oov(1, 6), OOV_PER_CHAR_PENALTY * 6.0);
        assert_eq!(oov(1, 8), oov(2, 8));
        assert_eq!(oov(2, 8), oov(3, 8));
        assert!(oov(1, 6) < oov(1, 7), "OOV cost must rise per char");
    }

    #[test]
    fn rc0_long_dict_path_beats_whole_buffer_oov_blob_any_length() {
        // RC0 core invariant at the cost level: a dict-covering path
        // of MANY edges (here 7 single-char dict words, the
        // `ginalangtsiahpngbesai`→囡仔人食飯袂使 shape) must stay
        // cheaper than a single whole-buffer OOV blob spanning the same
        // chars — length-independently. The pre-RC0 smooth OOV pricing
        // let the blob win past ~6 edges (the reproduced bug).
        let blob_chars = 21usize; // ≈ "ginalangtsiahpngbesai"
        let oov_blob = oov(7, blob_chars);
        // 7 modest-frequency dict edges (~3 chars each).
        let dict_path: f64 = (0..7).map(|_| ec(500, 1, 3)).sum();
        assert!(
            dict_path < oov_blob,
            "dict_path={dict_path} must stay < oov_blob={oov_blob}"
        );
        // …and a 20-edge sentence still loses to one OOV char.
        let huge_dict: f64 = (0..20).map(|_| ec(50, 1, 3)).sum();
        assert!(
            huge_dict < oov(1, 1),
            "even a 20-edge dict path ({huge_dict}) < one OOV char ({})",
            oov(1, 1)
        );
    }

    #[test]
    fn longer_spelling_is_cheaper_at_equal_frequency() {
        // khiin letter bias: `/ word_len^0.2`. At equal freq & syllable
        // count a longer toneless spelling is the cheaper edge.
        assert!(ec(100, 1, 6) < ec(100, 1, 3));
    }

    #[test]
    fn user_preference_discounts_multi_syllable_cost() {
        // A phrase edge with a positive decayed delta costs strictly
        // less than the same edge with no user history; the discount is
        // exactly `ln(1 + delta)` in log space.
        let neutral = ec(50, 2, 4);
        let preferred = edge_cost(50, 2, 4, 1.0, true);
        assert!(
            preferred < neutral,
            "preferred={preferred} neutral={neutral}"
        );
        assert!((preferred - (neutral - 1.0f64.ln_1p())).abs() < 1e-9);
    }

    #[test]
    fn single_syllable_user_weight_is_damped_to_zero_discount() {
        // The Q4c BLOCK guard, preserved through the log-space rewrite:
        // a hot single char with a huge decayed delta gets ln(1)=0
        // discount → identical to neutral (SCALE = 0.0).
        assert_eq!(WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE, 0.0);
        let neutral = ec(184_693, 1, 1);
        let with_huge_delta = edge_cost(184_693, 1, 1, 4.0, true);
        assert_eq!(
            with_huge_delta, neutral,
            "single-syllable edge must ignore the user delta at SCALE=0.0"
        );
    }

    #[test]
    fn user_discount_never_raises_cost_on_malformed_delta() {
        // δ is clamped `>= 0`; a (contractually impossible) negative
        // delta must not turn the discount into a penalty.
        let neutral = ec(50, 2, 4);
        assert!(edge_cost(50, 2, 4, -1.0, true) <= neutral + 1e-12);
        assert!((edge_cost(50, 2, 4, -1.0, true) - neutral).abs() < 1e-9);
    }

    #[test]
    fn cost_is_finite_at_frequency_and_syllable_extremes() {
        for &(f, s, l) in &[
            (0u32, 0u8, 0usize),
            (0, 1, 1),
            (184_693, 8, 12),
            (u32::MAX, u8::MAX, 64),
        ] {
            // Both pricing branches must stay finite at the extremes.
            // The OOV branch is `OOV_PER_CHAR_PENALTY * len`; at
            // `len = 64` that is `6.4e11`, far below f64's range.
            for dict_hit in [true, false] {
                let c = edge_cost(f, s, l, 4.0, dict_hit);
                assert!(c.is_finite(), "f={f} s={s} l={l} dict_hit={dict_hit} → {c}");
            }
        }
    }
}
