// Pure shadow pipeline — mode-aware canonicalize (POJ→POJ ASCII under POJ
// mode, POJ→TL ASCII under TL/English mode; v3.5.9 B-2 PR #309), hyphen
// strip, lattice build, and derived span / partial-prefix / custom-key
// helpers.

use lexicon::{ConsumedSpan, SyllableInventory};
use phonetics::InputMode;
use unicode_normalization::UnicodeNormalization;

use crate::lattice::{build_lattice, Lattice};
use crate::syllabifier::valid_span_endings_lowered;

/// v3.5.9 B-2 — map `mode` to its FST key family prefix. The tagged-single-FST
/// (`syllables.fst` + `dictionary.fst`) carries `tl:` / `poj:` / `tps:`
/// families; English shares the TL family because English buffers do not
/// have their own inventory and the syllabifier is not invoked there.
///
/// v3.5.9 D / C-3b — TPS promoted to a first-class family. The continuous
/// walker now emits `tps:<bopomofo_toneless>` keys against the C-0 emit
/// of `dictionary.fst`; the legacy `build_keys_tps` path (which folded
/// TPS into `tl:` keys via `phonetics::tps_to_tl`) is retired.
// 中文: B-2 — 將 mode 映射到 FST key 家族前綴。`syllables.fst` / `dictionary.fst` 為
// 中文:   tagged-single-FST,共存 `tl:` / `poj:` / `tps:` 三家族。English 沿用 `tl:` 家族
// 中文:   (英文 buffer 不會走音節切分)。
// 中文: D / C-3b — TPS first-class,連續 walker 改發 tps:<bopomofo_toneless>;
// 中文:   舊 build_keys_tps(經 tps_to_tl 折成 tl: 鍵)退役。
pub(crate) fn mode_key_prefix(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Poj => "poj",
        InputMode::Tps => "tps",
        InputMode::Tl | InputMode::English => "tl",
    }
}

/// v3.5.9 D / C-3b — mode-aware tone-mark stripping. Generalises the
/// TL/POJ-only [`strip_ascii_tone_digits`] so a TPS shadow slice strips
/// its 8 Bopomofo tone scalars (per `phonetics::tps::is_tps_tone_mark`)
/// instead of (no-op) ASCII digits, producing a key body that matches the
/// `tps:<tps_notone>` family. Used by every shadow-derived key / roman
/// emit site: `left_anchored_keys_from_lattice`, the walker edge key in
/// `composing::continuous::fetch_walker_slot0_inner`, the walker's
/// no-dict roman synth, `custom_toneless_key`, and `build_partial_prefix_key`.
///
/// TL/POJ/English: ASCII-digit strip — byte-identical to the legacy
/// [`strip_ascii_tone_digits`] path.
/// TPS: TPS tone-mark strip — drops the 8 scalars listed in
/// `phonetics::tps::is_tps_tone_mark` (U+02C6 ˆ, U+02C7 ˇ, U+02CA ˊ,
/// U+02CB ˋ, U+02D9 ˙, U+02EA ˪, U+02EB ˫, U+0307 combining dot above).
/// Hyphens / whitespace are NOT stripped here because
/// [`build_hyphen_shadow`] already drops ASCII `-` upstream and the TPS
/// buffer rarely contains them; matches `_TPS_TONE_AND_SEP_RE`'s tone
/// half (build pipeline `dictionary/common/notone.py::remove_tps_tone`).
// 中文: D / C-3b — mode-aware tone-mark 剝除。對 TL/POJ/English 與舊
// 中文:   strip_ascii_tone_digits byte-identical (ASCII 數字),對 TPS 剝除
// 中文:   phonetics::tps::is_tps_tone_mark 列的 8 個 Bopomofo 聲調符號。
// 中文:   不剝連字號/空白 — shadow 早已剝、TPS buffer 罕見;對應
// 中文:   _TPS_TONE_AND_SEP_RE 的聲調半邊。
pub(crate) fn strip_tones_for_mode(s: &str, mode: InputMode) -> String {
    match mode {
        InputMode::Tps => s
            .chars()
            .filter(|c| !phonetics::is_tps_tone_mark(*c))
            .collect(),
        InputMode::Tl | InputMode::Poj | InputMode::English => strip_ascii_tone_digits(s),
    }
}

/// True iff `span` is a fully-toned TL/POJ reading: a non-empty sequence
/// of `(letter+ digit)` groups — every syllable carries an ASCII tone
/// digit (e.g. `tai5`, `tai5gi2`, `kak4`) — with no orphan/leading digit
/// and a trailing tone digit. Such a span maps verbatim onto the
/// digit-separated, hyphenless `tl_num` / `poj_num` FST key family
/// (`dictionary/build/create_fst.py:127-130` emits `tl:<tl_num>` for
/// every record), so an exact lookup on the verbatim span filters
/// candidates to exactly the typed tone(s).
///
/// Text-only by design. The span-local + walker callers pass an
/// already-syllabified lattice edge (syllable validity guaranteed
/// upstream); the partial-prefix caller ([`build_partial_prefix_key`])
/// passes the raw whole-buffer shadow, which may be an INCOMPLETE
/// syllable. Either way this answers only "did the user fully tone it?",
/// a pure property of the text — it never claims the span is a real word,
/// so it needs no inventory and no re-segmentation (avoiding the greedy
/// dead-end trap [`greedy_longest_syllabification`] documents). Safety for
/// the unvalidated partial case: a fully-toned-LOOKING but nonexistent
/// body (`abc1`) yields a verbatim key that simply MISSES the FST and
/// returns zero candidates — never a wrong-tone hit. A mixed/partial-tone
/// span (`tai5bak`, `taigi2`, `tai5g`) ends in a letter → returns false,
/// so the caller keeps it on the toneless key, preserving the
/// toneless-input "show all tones" behavior with no regression.
// 中文: 判斷 span 是否為「全含調」TL/POJ 讀法 = `([字母]+[數字])+`(每音節皆帶 ASCII 聲調數字,尾端為數字,無孤兒數字)。
// 中文:   此形 verbatim 對齊 tl_num/poj_num FST 家族 → exact lookup 即按聲調過濾。
// 中文:   純文字判定:span-local/walker 來自已切音節的 lattice edge;partial-prefix 傳整個 raw shadow(可能非完整音節),
// 中文:   但純文字只答「是否全含調」、不宣稱是真詞,故不需 inventory、不重切音節(避開 greedy dead-end)。
// 中文:   未驗證 partial 之安全性:全含調但不存在的 body(abc1)只會 verbatim key miss FST 回空,不會錯調命中。
// 中文:   混合/部分含調(tai5bak / taigi2 / tai5g 尾為字母)回 false → caller 留在去調鍵,無回歸。
fn span_is_fully_toned_ascii(span: &str) -> bool {
    if span.is_empty() {
        return false;
    }
    let mut group_has_letter = false;
    for b in span.bytes() {
        if b.is_ascii_digit() {
            if !group_has_letter {
                return false; // orphan digit — no letter opened this group (`5tai`, `tai55`)
            }
            group_has_letter = false; // tone digit closes the current syllable group
        } else if b.is_ascii_alphabetic() {
            group_has_letter = true;
        } else {
            return false; // leaked hyphen / non-ASCII — not a clean toned reading
        }
    }
    // A trailing letter leaves a group open (un-toned) → not fully toned.
    !group_has_letter
}

/// FST lookup body for a continuous-input span. When `span` is a
/// fully-toned TL/POJ reading ([`span_is_fully_toned_ascii`]), return it
/// verbatim (digits kept) so `lookup_exact` / `lookup_prefix` filters by
/// the typed tone — the fix for the bug where explicit `tai5` surfaced
/// every tone of `tai`. Otherwise return the toneless form
/// ([`strip_tones_for_mode`]): toneless continuous input intentionally
/// surfaces all tones, and mixed/partial-tone spans have no fully-toned
/// FST key family. This is a key-SELECTION rule (one branch per span
/// shape), NOT a runtime fallback — there is no "toned miss → retry
/// toneless" path.
///
/// Only TL and POJ are tone-eligible. `English` mode has no tone
/// semantics (a trailing digit in an English buffer is not a tone), so it
/// keeps the legacy digit-strip — excluded here to avoid changing English
/// continuous behavior. TPS tones are Bopomofo scalars, not ASCII digits,
/// so [`span_is_fully_toned_ascii`] returns false for any non-ASCII
/// content and TPS always takes the toneless branch unchanged.
// 中文: 連續輸入 span 的 FST 查詢主體:全含調 TL/POJ → verbatim(保留數字)讓 lookup 按聲調過濾;
// 中文:   否則回去調形。為「鍵選擇規則」非 runtime fallback(無 toned-miss→retry-toneless)。
// 中文:   僅 TL/POJ 可含調;English 無聲調語意(尾端數字非聲調)維持去調;TPS 為注音聲調符非 ASCII 數字,恆走去調。
pub(crate) fn fst_body_for_span(span: &str, mode: InputMode) -> String {
    if matches!(mode, InputMode::Tl | InputMode::Poj) && span_is_fully_toned_ascii(span) {
        span.to_string()
    } else {
        strip_tones_for_mode(span, mode)
    }
}

/// Cap on syllabifier BFS depth for Phase 6 fetches. Matches the
/// `max_syllables=8` budget called out in `docs/releases/v3.5.8/plan.md` § Phase 3 — Performance and
/// keeps the worst-case lookup at O(n × 3 × 8) FST hits. Owned by
/// [`build_shadow_lattice`] (lattice BFS budget); since v3.5.9 D / C-3b
/// the legacy per-mode `continuous::build_keys_tps` is retired and all
/// four modes (TL/POJ/English/TPS) share the unified shadow → lattice
/// path through this single cap.
// 中文: syllabifier BFS 深度上限,對應 roadmap §Phase 3 的 8 syllable 估算。
// 中文: build_shadow_lattice 共用此 cap;v3.5.9 D / C-3b 退役 build_keys_tps 後,
// 中文:   四模式共用同一 shadow → lattice 路徑,FST 查詢複雜度仍由此值有界。
pub(crate) const MAX_SYLLABLES: usize = 8;

/// Run the v3.5.8 Items 8 + 9 canonicalize → hyphen-shadow pipeline
/// and build the segmentation lattice over the resulting shadow.
/// Returns `(shadow, shadow_to_raw_end, lattice)`. Shared by
/// `dispatch::build_keys_tl_with_inventory` (left-anchored projection —
/// its output is byte-identical to pre-S1, the S1 pinning tests guard
/// this) and `continuous::fetch_walker_slot0_inner` (S2 whole-sentence walker)
/// so the shadow + offset map + DAG are constructed exactly once per
/// fetch and the two consumers cannot drift.
// 中文: 跑 Item 8/9 canonicalize → hyphen-shadow 並建 lattice;回 (shadow, shadow→raw map, lattice)。
// 中文: build_keys (左錨投影,byte-identical 於 pre-S1) 與 fetch_walker_slot0 (S2 walker) 共用。
pub(crate) fn build_shadow_lattice(
    raw: &str,
    inv: &SyllableInventory,
    mode: InputMode,
) -> (String, Vec<usize>, Lattice) {
    let lower = raw.to_ascii_lowercase();
    let (canonical, canonical_to_raw_end) = canonicalize_poj_shadow(&lower, mode);
    let (shadow, shadow_to_canonical_end) = build_hyphen_shadow(&canonical);
    let shadow_to_raw_end: Vec<usize> = shadow_to_canonical_end
        .iter()
        .map(|&c| canonical_to_raw_end[c])
        .collect();
    // v3.5.9 B-2 — thread `mode` into the lattice builder; the inventory is
    // mode-aware (`SyllableInventory::contains_in(mode, …)`), so a POJ-mode
    // shadow now resolves against the `poj:` family of `syllables.fst` and
    // emits POJ-shaped syllable boundaries (`chiah`, `goa`, …) rather than
    // collapsing onto the TL forms.
    // 中文: B-2 — mode 透傳至 lattice builder;inventory 為 mode-aware,
    // 中文:   POJ 模式下走 `poj:` 家族,辨識 POJ 拼寫的音節邊界而非塌成 TL 形。
    let lattice = build_lattice(&shadow, inv, mode, MAX_SYLLABLES);
    (shadow, shadow_to_raw_end, lattice)
}

/// v3.5.9 A1 — extracted from the pre-A1 `build_keys_tl_with_inventory`
/// loop. Emit ONLY the lattice's left-anchored (`start == 0`)
/// projection as keys, byte-identical to the pre-S1 single-start
/// `valid_span_endings(shadow, 0, …)` output (`build_lattice` sorts
/// edges so the `start == 0` ones come first in ascending-`end`
/// order), so the span-local candidate / commit path is unchanged
/// from S1.
///
/// Interior (`start > 0`) edges are NOT emitted as user-facing keys:
/// under Model B (`docs/engine/continuous-input-ranking.md`
/// §10.3/§10.4) commit is forward-only `pending[..consumed_bytes]`,
/// so an independently tappable interior candidate has no
/// Model-B-consistent commit. S2's whole-sentence walker consumes
/// the interior edges INTERNALLY (via [`build_shadow_lattice`] +
/// `crate::lattice::walk_best` in `fetch_walker_slot0`) and emits one
/// synthesized full-buffer best path explicitly prepended at slot 0
/// by `handle_fetch_at_pos`; the user-facing commit span stays
/// `(0, end)`. Interior `台語`-style words remain reachable as the
/// next path-step after the prefix is nailed (Codex pre-impl S2
/// Q1c = option ii, 2026-05-16; `docs/releases/v3.5.8/plan.md` §整句 lattice + walker).
// 中文: A1 抽出 — 只發左錨投影 (start==0) 為 key,與 S1 前逐 byte 相同 → span-local 不變。
// 中文: 內段 (start>0) 不發為可點 key:Model B forward-only commit 無對應語意;
// 中文:   S2 walker 內部吃內段邊、合成單一全 buffer 最佳路徑由 handle_fetch_at_pos
// 中文:   explicit prepend 到 slot 0,commit span 維持 (0,end)。
pub(crate) fn left_anchored_keys_from_lattice(
    shadow: &str,
    shadow_to_raw_end: &[usize],
    lattice: &Lattice,
    inv: &SyllableInventory,
    mode: InputMode,
) -> Vec<(ConsumedSpan, String)> {
    // v3.5.9 B-2 — `mode` selects the FST key family the emitted keys are
    // namespaced into. The lattice itself was already built against the
    // matching `SyllableInventory` family ([`build_shadow_lattice`] →
    // [`build_lattice`]), so the syllabification and the key namespace
    // come from a single mode parameter — they cannot drift.
    // 中文: B-2 — mode 同時決定 lattice 走的 inventory 家族與此處 key 命名空間,單一參數
    // 中文:   貫穿,音節切分與 key 前綴不會由不同來源分歧。
    let prefix = mode_key_prefix(mode);

    // Longest-match prefix suppression (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`,
    // USER 2026-05-31「免調也壓制」): among the SINGLE-syllable spans anchored at
    // offset 0, surface only the LONGEST. A shorter single syllable that is a
    // strict prefix of a longer one (`ta`⊂`tai`⊂`tai5`, `tsu`⊂`tsua`) is
    // dropped — fixing the reported bug where typing a complete syllable
    // (`tai` / `tai5`) surfaced 2-letter `ta` candidates. Keys on span length,
    // not tone, so it covers toned + toneless alike and subsumes the
    // explicit-tone case (§17).
    //
    // The single-syllable ends are recomputed via the SAME `max_syllables = 1`
    // primitive `build_lattice` chains (`lattice.edges()` flattens depth, so a
    // `(0, end)` edge cannot be told apart from a chain reaching `end` —
    // re-running the depth-1 walk is the only way to isolate true single
    // syllables). Lowercasing once mirrors `build_lattice`, which also
    // re-lowercases the shadow before walking.
    //
    // An end is suppressed only when it is (a) a single-syllable end, (b) not
    // the longest single syllable, AND (c) has NO multi-syllable phrase
    // reading — i.e. no interior edge `(m, end)` with `m > 0` reaches it. (c)
    // is the safety guard: a shorter span that ALSO parses as a phrase
    // (`a`+`i` ending where `ai` is a single syllable too) is a legitimate
    // different-word candidate and must survive. In practice (b)+(c) coincide
    // for the reported bug (`ta`/`tsu` have no interior predecessor), but (c)
    // makes the rule provably never drop a phrase candidate. Display layer
    // only: the lattice keeps every edge, so the walker
    // (`fetch_walker_slot0_inner`) + min-hop `span_min_syllable_count` still
    // see every split (non-greedy `ta`+`nia` recovery, Codex PR #290 P1,
    // unaffected).
    // 中文: longest-match 前綴壓制 — 只壓「單音節且非最長且無多音節 phrase 讀法」的較短端;
    // 中文:   (ta⊂tai⊂tai5、tsu⊂tsua);含調免調皆壓 (USER 2026-05-31「免調也壓制」)。
    // 中文: (c) phrase 守門 — 同 end 另有 phrase 讀法 (內段 edge (m,end), m>0 可達) 即合法異詞候選,保留;
    // 中文:   保證絕不誤刪 phrase。lattice.edges() 攤平深度,故以 max_syllables=1 重走辨識單音節端。
    // 中文: 純顯示層 — lattice edge 不動,walker / span_min_syllable_count 仍見全部切法 (不退化 #290)。
    let lowered = shadow.to_ascii_lowercase();
    let single_ends = valid_span_endings_lowered(&lowered, 0, inv, mode, 1);
    let max_single_end = single_ends.iter().copied().max();
    let has_phrase_reading = |end: usize| lattice.edges().iter().any(|&(s, e)| e == end && s > 0);

    let mut out = Vec::with_capacity(lattice.edges().len());
    for &(start, end) in lattice.edges() {
        if start != 0 {
            continue;
        }
        // Guards mirror the pre-S1 single-start loop exactly (`start`
        // is 0 here, so the slice / offset map is identical).
        if end == 0 || end > shadow.len() || !shadow.is_char_boundary(end) {
            continue;
        }
        // Drop a strictly-shorter single-syllable-only prefix span (see the
        // (a)/(b)/(c) rule above). Longest single, phrase ends, and
        // phrase-reachable shorter spans are all kept.
        // 中文: 丟掉「較短且僅單音節且無 phrase 讀法」的前綴;最長單音節、phrase 端、可作 phrase 的較短端皆保留。
        if single_ends.contains(&end) && Some(end) != max_single_end && !has_phrase_reading(end) {
            continue;
        }
        // Explicit-tone fix — tone-aware lookup body. A fully-toned span
        // (`tai5`, `tai5gi2`) keeps its digits so `lookup_exact` filters
        // to the typed tone; a toneless / mixed span strips to the fused
        // toneless key (all-tone surface), preserving the toneless-input
        // behavior. See [`fst_body_for_span`].
        // v3.5.9 D / C-3b — the underlying strip is mode-aware: TL/POJ
        // drop ASCII digits, TPS drops Bopomofo tone marks (matches the
        // `tps:<tps_notone>` FST family from C-0; TPS always toneless here).
        // 中文: 明確聲調修正 — tone-aware 查詢主體。全含調 span 保留數字 → lookup_exact 按聲調過濾;
        // 中文:   去調/混合 span 走去調 fused key(全聲調),維持去調輸入行為。見 fst_body_for_span。
        let body = fst_body_for_span(&shadow[..end], mode);
        if body.is_empty() {
            continue;
        }
        let raw_end = shadow_to_raw_end[end];
        out.push(((0u32, raw_end as u32), format!("{prefix}:{body}")));
    }
    out
}

/// v3.5.8 S5 (Codex pre-impl Q2, 2026-05-17) — greedy longest-syllable
/// segmentation of `shadow`, the no-dict carve-out's user-facing
/// romanization reading.
///
/// From each offset, take the **longest** valid single syllable
/// (`valid_span_endings_lowered(.., max_syllables = 1)` → max ending)
/// and advance. This is the canonical romanization reading (khiin
/// longest-match family, `references/khiin-rs/khiin/src/data/segmenter.rs`):
/// `taiuantai → [tai, uan, tai]`. Literal *maximal-syllable-count*
/// would instead over-split into sub-syllables (`ta i u an …`),
/// reproducing the very over-segmentation S5 removes — so greedy-LONGEST
/// is deliberate, not max-segment.
///
/// Returns `None` when some offset has no valid syllable (the buffer
/// cannot be cleanly read syllable-by-syllable) — the caller then
/// suppresses the slot-0 synth and leaves the span-local list
/// untouched (pre-S2 behavior, same contract as
/// `continuous::synth_consumed_span`'s trailing-hyphen suppression).
/// `shadow` is ASCII-lowercased here; lowercasing is byte-length and
/// char-boundary preserving, so the returned offsets index `shadow`
/// identically.
// 中文: S5 — greedy 最長音節切分,no-dict carve-out 的羅馬字讀法。
// 中文:   每步取最長合法單音節 → taiuantai=[tai,uan,tai]。
pub(crate) fn greedy_longest_syllabification(
    shadow: &str,
    inv: &SyllableInventory,
    mode: InputMode,
) -> Option<Vec<(usize, usize)>> {
    let lowered = shadow.to_ascii_lowercase();
    let mut segs: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0usize;
    while pos < lowered.len() {
        // v3.5.9 B-2: `mode` selects the inventory family that gates the
        // single-syllable step. The shadow is already in the matching
        // family's canonical ASCII form (B-2 reshape of
        // `canonicalize_poj_shadow` preserves POJ ASCII when `mode ==
        // Poj`), so a TL shadow walks the `tl:` family and a POJ shadow
        // walks the `poj:` family — both produce shadow-aligned offsets
        // because the inventory family's syllable boundaries match the
        // shadow form.
        // 中文: B-2 — mode 決定走哪一家族;shadow 已是該家族的 canonical ASCII
        // 中文:   形式 (canonicalize_poj_shadow POJ 模式保 POJ),家族與切點對齊。
        let end = valid_span_endings_lowered(&lowered, pos, inv, mode, 1)
            .into_iter()
            .max()?;
        segs.push((pos, end));
        pos = end;
    }
    // Forward-only single-syllable steps land exactly on `len`; the
    // guard is belt-and-suspenders.
    (pos == lowered.len()).then_some(segs)
}

/// v3.5.8 OOV-cost fix (Codex PR #290 P1 `r3255035136`, 2026-05-18) —
/// the **guaranteed** syllable count of `shadow_span`: the minimum
/// number of single-syllable hops to cover it. Each hop is one valid
/// syllable from `valid_span_endings_lowered(.., max_syllables = 1)` —
/// the same per-syllable step `lattice::builder::build_lattice` relies
/// on (it calls `valid_span_endings_lowered(.., max_syllables = 8)`,
/// whose internal BFS itself advances exactly one valid syllable per
/// depth level, so an emitted edge is a chain of these single hops).
///
/// Sets the no-dict edge's `syllable_count`. **v3.5.8 RC0**: the OOV
/// edge *cost* is now `OOV_PER_CHAR_PENALTY * toneless_len`
/// (char-keyed, khiin's per-char `BIG`), so `syllable_count` no longer
/// feeds OOV pricing — it is metadata that flows into the synthesized
/// slot-0 candidate's syllable sum. Kept honest (not a hardcoded `1`)
/// anyway so that sum stays correct and the dispatch invariant is not
/// weakened (Codex pre-impl RC0 Q3). Replaces
/// `greedy_longest_syllabification(span).len()`: greedy-longest is not
/// a global segmentation guarantee — it can dead-end (`None`) on a
/// span still lattice-syllabifiable via a *non-greedy* split, and the
/// old `unwrap_or(1)` then under-counted a multi-syllable OOV span. A
/// min-hop BFS over the builder's own single-syllable steps cannot
/// dead-end on a real lattice edge: `build_lattice` emits
/// `(start, end)` only by chaining exactly those hops, so a hop-path
/// `0 → len` provably exists and the BFS returns `Some(>= 1)`. Min-hop
/// (not greedy / not max) is the fewest-syllable valid reading — it is
/// `1` only when the whole span is itself one valid syllable
/// (correct), never collapsing a genuinely multi-syllable span to `1`.
///
/// `None` only if `len` is not single-syllable-reachable at all —
/// impossible for an edge this same `build_lattice` produced over the
/// same `shadow`/`inv` (Codex pre-impl Q1/Q2 OK); the caller treats
/// `None` as a broken edge/provider invariant and fail-closed **drops
/// the edge** rather than mispricing it (the buffer is still spanned
/// via finer edges).
// 中文: shadow_span 的「保證」音節數 = 覆蓋它所需的最少單音節 hop 數。
pub(crate) fn span_min_syllable_count(
    shadow_span: &str,
    inv: &SyllableInventory,
    mode: InputMode,
) -> Option<usize> {
    let lowered = shadow_span.to_ascii_lowercase();
    let end = lowered.len();
    if end == 0 {
        return None;
    }
    // Unweighted shortest path (in #hops) from offset 0 to `end`. Each
    // hop is one valid syllable from
    // `valid_span_endings_lowered(.., pos, inv, mode, 1)` — the
    // single-hop primitive the lattice builder chains under the same
    // `mode`. v3.5.9 B-2 plumbs `mode` through here so a POJ shadow
    // walks the `poj:` family for hop counts, matching the family used
    // by [`build_lattice`] to produce the edge in the first place; the
    // hop-count invariant (`build_lattice` emits `(start, end)` only by
    // chaining single hops) holds per-family.
    // 中文: B-2 — mode 同步透傳;POJ shadow 走 `poj:` 家族的單音節 hop,
    // 中文:   與 build_lattice 同家族,保持「edge 必由 single-hop 鏈組成」不變式。
    use std::collections::{BTreeMap, VecDeque};
    let mut dist: BTreeMap<usize, usize> = BTreeMap::new();
    dist.insert(0, 0);
    let mut queue: VecDeque<usize> = VecDeque::new();
    queue.push_back(0);
    while let Some(pos) = queue.pop_front() {
        let hops = dist[&pos];
        if pos == end {
            return Some(hops);
        }
        for nxt in valid_span_endings_lowered(&lowered, pos, inv, mode, 1) {
            if nxt > pos && nxt <= end && !dist.contains_key(&nxt) {
                dist.insert(nxt, hops + 1);
                queue.push_back(nxt);
            }
        }
    }
    None
}

/// Build a hyphenless shadow of `raw` paired with a byte-indexed map
/// from shadow byte offsets back to raw byte offsets, mirroring the
/// hyphen half of `dictionary/common/notone.py::remove_tone` regex
/// `[\d\-]`. The digit half stays at [`strip_ascii_tone_digits`].
///
/// Contract:
/// - `shadow` is `raw` with every ASCII `-` (U+002D) removed; all other
///   bytes (including non-ASCII bytes from accidental POJ diacritics)
///   pass through unchanged.
/// - `shadow_to_raw_end` has length `shadow.len() + 1`; index `k` is the
///   raw byte offset RIGHT AFTER the last raw char that contributed the
///   `k`-th shadow byte. `shadow_to_raw_end[0] = 0`.
/// - Leading hyphens before the first surviving raw char ARE folded
///   into the consumed prefix: every shadow ending whose raw mapping
///   passes byte index 0 inherits the preceding hyphens in its
///   `consumed_span`.
/// - Trailing hyphens AFTER the last surviving raw char are NOT
///   folded: a shadow ending at `shadow.len()` maps to the raw byte
///   AFTER the last non-hyphen char, leaving any trailing hyphen in the
///   pending raw buffer for platform UI to retain post-commit.
///
/// Examples:
/// - `"tai-bak"` → shadow `"taibak"`, map `[0, 1, 2, 3, 5, 6, 7]`
/// - `"-tai"`    → shadow `"tai"`,    map `[0, 2, 3, 4]`
/// - `"tai-"`    → shadow `"tai"`,    map `[0, 1, 2, 3]`
/// - `"goa--si"` → shadow `"goasi"`,  map `[0, 1, 2, 3, 6, 7]`
/// - `"---"`     → shadow `""`,       map `[0]`
// 中文: 把 raw 內所有 ASCII `-` 拿掉成 shadow,並建立 shadow byte → raw byte 的對照表。
// 中文: leading `-` 算進前綴消耗;trailing `-` 留在 pending buffer 不被吃掉。
pub(crate) fn build_hyphen_shadow(raw: &str) -> (String, Vec<usize>) {
    let mut shadow = String::with_capacity(raw.len());
    let mut shadow_to_raw_end: Vec<usize> = Vec::with_capacity(raw.len() + 1);
    shadow_to_raw_end.push(0);
    for (raw_idx, ch) in raw.char_indices() {
        if ch == '-' {
            continue;
        }
        let raw_end_after_ch = raw_idx + ch.len_utf8();
        for _ in 0..ch.len_utf8() {
            shadow_to_raw_end.push(raw_end_after_ch);
        }
        shadow.push(ch);
    }
    (shadow, shadow_to_raw_end)
}

/// Drop every ASCII digit from `s`. Equivalent to the digit half of
/// `dictionary/common/notone.py::remove_tone()` regex `[\d\-]` under
/// the canonical-ASCII TL input contract — Python `\d` matches every
/// Unicode decimal digit, but TL canonical input only ever uses
/// ASCII `0..=9`, so `is_ascii_digit()` is sound here. Hyphens are
/// stripped one layer up by [`build_hyphen_shadow`] (Phase 9 Item 8),
/// so callers feed this fn a hyphenless shadow slice already.
// 中文: 對應 notone.py [\d\-] 中的 \d (ASCII contract 等價);hyphen 半邊由 build_hyphen_shadow 上一層處理 (Phase 9 Item 8)。
pub(crate) fn strip_ascii_tone_digits(s: &str) -> String {
    s.chars().filter(|c| !c.is_ascii_digit()).collect()
}

/// v3.5.8 S6 (Codex pre-impl S6 Q2, 2026-05-17, BLOCK condition) —
/// derive the walker lattice-edge match key for a
/// `custom_dictionary.db` entry's romanization, or `None` when the
/// derived form does not land in `[a-z]+` after canonicalization.
///
/// MUST produce a key byte-identical to the one
/// `continuous::fetch_walker_slot0_inner`'s edge provider builds for a
/// syllable span (`<prefix>:{toneless}` with `prefix ∈ {tl, poj}` per
/// `mode_key_prefix(mode)`; v3.5.9 B-2 PR #309 promoted POJ to a
/// first-class FST key family, pre-B-2 every key prefixed `tl:`).
/// `toneless` is the hyphen-stripped, mode-canonicalized,
/// tone-digit-stripped shadow slice. This helper therefore reuses the
/// **same three shadow helpers in the same order** —
/// [`canonicalize_poj_shadow`] → [`build_hyphen_shadow`] →
/// [`strip_ascii_tone_digits`] — as the single normalization source.
/// Codex pre-impl S6 Q2 **BLOCK**ed a plain `strip_ascii_tone_digits`:
/// it cannot fold a POJ/diacritic custom roman (`tâi-uân`, `tâi-gí`)
/// into `taiuan` / `taigi`; the canonicalize pass is load-bearing. The
/// offset maps the helpers also return are irrelevant here (the custom
/// roman is keyed whole, never sliced against raw), so they are
/// discarded.
///
/// **Roman-only** (Codex pre-impl S6 Q2): `hanji` is edge *payload*
/// resolved after the edge is chosen, never an edge key — the lattice
/// is keyed by toneless romanization spans. Returns `None` for an
/// empty result or any residue outside ASCII `a..=z`: punctuation /
/// CJK / digit-only custom roman, or a POJ-shape token that does not
/// canonicalize cleanly, can never equal a syllabifier-built lattice
/// edge key, so it simply stays a span-local candidate and never
/// enters the walker.
///
/// `mode` MUST be the same value `continuous::fetch_walker_slot0_inner`
/// passes to [`build_shadow_lattice`] for this fetch: the canonicalize
/// step is mode-gated — TL/English mode folds POJ→TL (`ch→ts`,
/// `oa→ua`, ...) and emits `tl:`, POJ mode keeps POJ ASCII (no fold)
/// and emits `poj:`. A mismatch would make a custom roman key
/// `poj:chiah` while the lattice edge keys `tl:tsiah` (or the
/// converse), silently breaking the S6 byte-identity match.
// 中文: 由 custom_dictionary.db entry 的羅馬字推導 walker lattice-edge 比對 key(canonicalize 後不落 [a-z]+ → None)。
// 中文: 必須與 edge provider 的 <prefix>:{toneless} byte-identical(B-2 後 prefix ∈ {tl, poj})→
// 中文:   重用同一組 shadow helper 同順序、共用同一 mode。
pub(crate) fn custom_toneless_key(roman: &str, mode: InputMode) -> Option<String> {
    let lower = roman.to_ascii_lowercase();
    let (canonical, _) = canonicalize_poj_shadow(&lower, mode);
    let (shadow, _) = build_hyphen_shadow(&canonical);
    let toneless = strip_tones_for_mode(&shadow, mode);
    if toneless.is_empty() {
        return None;
    }
    // v3.5.9 D / C-3b — gate body shape by mode:
    // - TL/POJ/English: body must be all ASCII lowercase a..=z
    //   (no Bopomofo / digit / punctuation residue can collide with a
    //   syllabifier-built TL/POJ lattice edge key).
    // - TPS: body must be all Bopomofo (TPS char) AND carry no leftover
    //   TPS tone mark (strip should have caught them; the guard is
    //   defensive — non-Bopomofo residue cannot equal a `tps:<tps_notone>`
    //   edge key built from the same shadow pipeline).
    // 中文: D / C-3b — 依 mode 守 body 形狀。TL/POJ/English 仍要求純 ASCII 小寫 a..=z;
    // 中文:   TPS 要求純 Bopomofo 且無殘留聲調符號 — strip 已處理,守門為防呆;
    // 中文:   非 Bopomofo 殘留無法等於 tps:<tps_notone> edge 鍵。
    let body_ok = match mode {
        InputMode::Tps => toneless
            .chars()
            .all(|c| phonetics::is_tps_char(c) && !phonetics::is_tps_tone_mark(c)),
        InputMode::Tl | InputMode::Poj | InputMode::English => {
            toneless.bytes().all(|b| b.is_ascii_lowercase())
        }
    };
    if !body_ok {
        return None;
    }
    // v3.5.9 B-2 — mode-aware family prefix. The contract still requires
    // byte-identity with the edge provider's emitted key (the S6
    // invariant), so this MUST consume the same `mode` and the same
    // shadow pipeline — both are now mode-aware in lockstep.
    // 中文: B-2 — mode-aware 前綴。S6 byte-identity 不變:walker edge 與此處共用同一 mode
    // 中文:   + 同一 shadow pipeline,前綴一致。
    let prefix = mode_key_prefix(mode);
    Some(format!("{prefix}:{toneless}"))
}

/// Canonicalize POJ-display input (`pe̍h-ōe-jī`, `chóa`, `peⁿ`, `so͘`)
/// into ASCII spelling for its mode's FST key family, paired with a
/// byte-indexed map from canonical byte offsets back to original `input`
/// byte offsets. v3.5.8 Phase 9 Item 9; v3.5.9 B-2 reshape so the output
/// is **POJ ASCII** under POJ mode (`chiah` stays `chiah`) and **TL literal**
/// under TL mode (POJ-shaped input is NOT folded into the TL chain).
///
/// Mode-gated Phase 2 rule list (v3.5.9 B-2, refined by PR #309 Codex
/// P1 `r3276402303`; TL-literal pass added 2026-06-05):
/// - `mode == InputMode::Poj` → [`phonetics::NORMALIZE_TO_POJ_GLYPH_RULES`]:
///   the glyph-only subset (`o͘→oo`, `ⁿ→nn`, `ᴺ→nn`). The legacy
///   `ou→oo` alias is **excluded** because it would mis-fire across
///   syllable boundaries on hyphenless multi-syllable user input — e.g.
///   typing `toui` for POJ `tó-uī` (indexed `poj_notone=toui`) would
///   get folded to `tooi` and lose the lattice match. Per-syllable
///   callers (build-pipeline `canonicalize_poj_syllable`, runtime
///   `derive_poj_notone_for_match`) still consume the full
///   [`phonetics::NORMALIZE_TO_POJ_RULES`] — they apply per-token so
///   the `ou` alias only ever sees a single syllable.
/// - `mode != InputMode::Poj` (TL / TPS / English) →
///   [`phonetics::TL_ENCODING_RULES`]: encoding-only (`o͘→oo`, `ⁿ→nn`,
///   `ᴺ→nn`, `oonn→onn`), **no POJ→TL spelling fold**. TL input is taken
///   literally so a valid TL special final `eng` [ɛŋ] survives (not
///   collapsed to `ing` [iŋ]) and POJ-shaped TL-mode input (`teng`/`goa`/
///   `chiah`) is not auto-corrected into a `tl:` hit. The ASCII branch is
///   still identity (F3C gate); since the encoding rules are no-ops on
///   ASCII, ASCII identity and `TL_ENCODING_RULES` agree on ASCII input.
///
/// Phase 1 — char-level NFD walk over the original input. Each NFD
/// scalar that is one of the 8 tone-mark combining codepoints in
/// `engine/phonetics/src/tables.rs::COMBINING_TO_TONE_NUM`
/// (`U+0300, U+0301, U+0302, U+0304, U+0306, U+030B, U+030C, U+030D`) is
/// dropped, with its UTF-8 byte width absorbed into the preceding base
/// char's `raw_end` so the offset map stays anchored at the right of
/// each consumed run. Other NFD scalars pass through unchanged
/// (including `\u{0358}` and `\u{207f}` / `\u{1d3a}`, which Phase 2
/// turns into ASCII regardless of which rule list is active — both
/// lists carry the same encoding rules).
///
/// Phase 2 — apply the mode-selected rule list with offset-aware
/// substring replace. Shrinking rules (`o\u{0358}→oo`, `\u{207f}|\u{1d3a}
/// →nn`, `oonn→onn` for TL only) drain the dropped trailing byte's map
/// entry; byte-count-preserving rules leave the offset map invariant.
///
/// Output contract (mirrors [`build_hyphen_shadow`]):
/// - `canonical` is ASCII (after Phase 2 all non-ASCII codepoints have
///   been replaced with ASCII spellings).
/// - `canonical_to_raw_end` has length `canonical.len() + 1`. Index `k`
///   is the original-input byte offset right after the last original
///   byte that contributed the first `k` canonical bytes.
///   `canonical_to_raw_end[0] = 0`.
///
/// Examples (NFC inputs):
/// - `"pe\u{030d}h"` (5 bytes) → `"peh"`, map `[0, 1, 4, 5]` — the
///   dropped `\u{030d}` (2 bytes) is folded into the preceding `e`'s
///   raw_end.
/// - `"so\u{0358}"` (4 bytes) → `"soo"`, map `[0, 1, 4, 4]` — the
///   `o\u{0358}→oo` substitution emits two ASCII bytes for the original
///   non-ASCII pair, with EVERY new byte anchored at raw_end 4 so a
///   partial-prefix syllabifier hit (`so` toneless) still consumes the
///   whole `o\u{0358}` source spelling. Without this fold a `tl:so`
///   candidate against the live `dictionary.csv` `so` / `soo` sibling
///   pair would commit leaving `\u{0358}` dangling in the pending
///   buffer (Codex post-impl P1 2026-05-15).
/// - `"pe\u{207f}"` (5 bytes) → `"penn"`, map `[0, 1, 2, 5, 5]` — the
///   `\u{207f}→nn` substitution starts AFTER the `pe`, so the
///   atomic-fold rule only touches map indices strictly inside the
///   substitution span (`map[3]` and `map[4]`). The byte BEFORE the
///   substitution (`map[2] = 2`) is left alone — a syllabifier match
///   ending exactly at the substitution boundary (e.g. `pe` toneless)
///   legitimately consumes only `pe` raw bytes and leaves `\u{207f}`
///   pending; the user's deliberate tap on the shorter candidate
///   opted into that.
// 中文: POJ-display 輸入 → 該模式 FST 家族對應的 ASCII 拼寫(POJ 模式→POJ ASCII,TL 模式→TL ASCII;v3.5.9 B-2 PR #309),
// 中文:   並建立 canonical byte → raw byte 對照表。
pub(crate) fn canonicalize_poj_shadow(input: &str, mode: InputMode) -> (String, Vec<usize>) {
    if input.is_ascii() {
        // Identity offset map: Phase 1's NFD walk is a no-op for ASCII,
        // so every canonical byte maps straight back to its own raw
        // offset regardless of which branch we take below.
        let map: Vec<usize> = (0..=input.len()).collect();
        if matches!(mode, InputMode::Poj) {
            // v3.5.9 B-2 — POJ mode: apply the glyph-only POJ rule
            // subset. All 3 rules are non-ASCII → ASCII substitutions
            // that are no-ops on already-ASCII input, so the ASCII
            // POJ path is now **identity** — `chiah` stays `chiah`,
            // `toui` stays `toui` (PR #309 Codex P1 `r3276402303`:
            // applying the legacy `ou→oo` alias whole-buffer mis-fired
            // on multi-syllable hyphenless typing like `toui` for
            // POJ `tó-uī`). Dirty-row `ou` protection moves to the
            // per-syllable build pipeline where it is structurally
            // safe (one syllable per application).
            // 中文: B-2 PR #309 — POJ 模式 ASCII 改走 identity (glyph-only 規則對 ASCII 為 no-op)。
            // 中文:   `toui` 保持 `toui`,讓 lattice 切出 `poj:to` + `poj:ui` 對齊 `tó-uī` 索引;
            // 中文:   `ou→oo` 髒資料防護下放到逐音節 build pipeline,單音節下不會誤觸發。
            return apply_normalize_with_offsets(
                input.to_owned(),
                map,
                phonetics::NORMALIZE_TO_POJ_GLYPH_RULES,
            );
        }
        // TL / non-POJ: F3C identity fast-path (protects `toui`).
        return (input.to_owned(), map);
    }

    // Phase 1: NFD walk per original char so we can pair every NFD scalar
    // with the byte range of the original char it came from.
    let mut intermediate = String::with_capacity(input.len());
    let mut map: Vec<usize> = Vec::with_capacity(input.len() + 1);
    map.push(0);
    let mut nfd_buf = [0u8; 4];

    for (raw_idx, ch) in input.char_indices() {
        let raw_end_after = raw_idx + ch.len_utf8();
        for nfd_ch in ch.nfd() {
            if is_tone_combining_mark(nfd_ch) {
                // Drop: absorb the dropped scalar's raw bytes into the
                // preceding emitted byte's raw_end so platform commit
                // does not leave a dangling combining mark in the
                // pending buffer. Guard against a leading standalone
                // combining mark (map has only the baseline `0` entry,
                // no emitted byte yet to absorb into): leave the
                // baseline at `0` per the documented contract; the
                // next emitted char's `raw_end_after` will already
                // account for the dropped mark's byte width via its
                // own `raw_idx + len_utf8()` (Codex post-impl P3
                // 2026-05-15).
                if map.len() > 1 {
                    if let Some(last) = map.last_mut() {
                        *last = raw_end_after;
                    }
                }
                continue;
            }
            let nfd_str = nfd_ch.encode_utf8(&mut nfd_buf);
            intermediate.push_str(nfd_str);
            for _ in 0..nfd_ch.len_utf8() {
                map.push(raw_end_after);
            }
        }
    }

    // Lowercase the ASCII letters that survived the NFD walk so the
    // Phase 2 substitutions actually match. Non-ASCII bytes left over
    // (`\u{0358}`, `\u{207f}`, `\u{1d3a}`) are unaffected by
    // `to_ascii_lowercase` and get replaced into ASCII by Phase 2 below.
    let intermediate_lower = intermediate.to_ascii_lowercase();

    // v3.5.9 B-2 (PR #309 Codex P1 `r3276402303` refinement) — mode
    // selects Phase 2 rule list. POJ mode runs the glyph-only POJ
    // subset (no `ou→oo` alias, no `ch→ts` chain) so non-ASCII POJ
    // input like `pe\u{030d}h` / `chia\u{030d}h` / `so\u{0358}` lands
    // as POJ ASCII (`peh` / `chiah` / `soo`). The `ou→oo` alias is
    // dropped here too because the same hyphenless multi-syllable
    // failure mode applies to non-ASCII input (e.g. `t\u{f3}u\u{12b}`
    // typed without a hyphen would have folded `toui → tooi`). TL /
    // TPS / English keep the pre-B-2 chain so dictionary hits routed
    // through the TL family stay byte-identical.
    // 中文: B-2 PR #309 — mode 決定 Phase 2 rule list。POJ 模式跑 glyph-only POJ 子集
    // 中文:   (無 `ou→oo` alias、無 ch→ts 鏈),避免跨音節邊界誤觸發 (同 ASCII 分支理由)。
    // 中文:   非-ASCII POJ 輸入仍落到 POJ ASCII 而非 TL ASCII。
    // POJ keeps POJ shape (glyph-only); TL / English / TPS take input
    // literally — encoding-only normalization, NO POJ→TL spelling fold — so a
    // valid TL special final `eng` [ɛŋ] is not collapsed to `ing` [iŋ] and a
    // POJ-spelled syllable typed in TL mode (`teng`/`goa`/`chiah`) is not
    // auto-corrected into a `tl:` family hit. English non-ASCII stays literal
    // (`hello` not reinterpreted as Taigi); TPS Bopomofo never matches these
    // Latin rules.
    // 中文: POJ 保 POJ 形 (glyph-only);TL/English/TPS 字面化 — 純編碼,無 POJ→TL 拼寫摺疊。
    let rules = if matches!(mode, InputMode::Poj) {
        phonetics::NORMALIZE_TO_POJ_GLYPH_RULES
    } else {
        phonetics::TL_ENCODING_RULES
    };
    apply_normalize_with_offsets(intermediate_lower, map, rules)
}

/// True for the 8 combining tone-mark scalars listed in
/// `engine/phonetics/src/tables.rs::COMBINING_TO_TONE_NUM`. Codex
/// pre-impl flagged that `\u{0358}` (combining dot above right, part of
/// POJ `o\u{0358}` for `oo`) must NOT be dropped here — it has to
/// survive Phase 1 so the Phase 2 `o\u{0358}→oo` substitution can fire.
// 中文: 只認 phonetics tables.rs 的 8 個聲調 combining 符號;`\u{0358}` 留給 Phase 2 處理。
fn is_tone_combining_mark(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'  // grave (tone 3)
            | '\u{0301}'  // acute (tone 2)
            | '\u{0302}'  // circumflex (tone 5)
            | '\u{0304}'  // macron (tone 7)
            | '\u{0306}'  // breve (POJ tone 9)
            | '\u{030b}'  // double acute (TL tone 9)
            | '\u{030c}'  // caron (tone 6)
            | '\u{030d}' // vertical line above (tone 8)
    )
}

/// Apply an ordered list of `(find, replace)` rules with offset-map
/// maintenance, returning the mutated string + updated map. v3.5.9 B-2
/// generalization of the pre-B-2 `apply_normalize_to_tl_with_offsets`:
/// the caller now passes the rule list. The two production rule lists
/// reaching this entry are [`phonetics::TL_ENCODING_RULES`]
/// (TL / English / TPS mode — encoding-only, no POJ→TL spelling fold) and
/// [`phonetics::NORMALIZE_TO_POJ_GLYPH_RULES`]
/// (POJ mode — the glyph-only subset of `NORMALIZE_TO_POJ_RULES` without
/// the `ou→oo` alias, which would mis-fire across syllable boundaries
/// at whole-buffer scope; see B-2 PR #309 Codex P1 `r3276402303`). This
/// makes `canonicalize_poj_shadow` mode-aware without duplicating the
/// offset-map maintenance loop. Same in-order iteration + same patterns
/// as the rule lists in `phonetics::syllable`; non-shrinking rules
/// leave the offset map invariant, shrinking rules drain the dropped
/// trailing byte's map entry instead of producing a new `String`.
// 中文: B-2 — 將代換鏈 + offset-map 維護泛化,呼叫端傳 rule list;
// 中文:   TL/English/TPS 用 NORMALIZE_TO_TL_RULES,POJ 模式用 NORMALIZE_TO_POJ_GLYPH_RULES
// 中文:   (glyph-only,不含 ou→oo 別名,whole-buffer 跨音節安全);
// 中文:   shrinking 規則由 offset_aware_replace 處理 map 收縮,其餘規則 map invariant。
pub(crate) fn apply_normalize_with_offsets(
    s: String,
    map: Vec<usize>,
    rules: &[(&str, &str)],
) -> (String, Vec<usize>) {
    let mut s = s;
    let mut map = map;
    for (find, repl) in rules {
        offset_aware_replace(&mut s, &mut map, find, repl);
    }
    (s, map)
}

/// Walk `s` left-to-right, replacing every occurrence of `find` with
/// `repl`, and update `map` so each post-replacement byte still points
/// at the correct original-input `raw_end`. Used by
/// [`apply_normalize_with_offsets`].
///
/// Semantics:
/// - Equal-length replacements (`ch→ts`, `oa→ua`, ...) leave `map`
///   untouched at every byte position because the replaced bytes
///   inherit the same `raw_end` slots.
/// - Shrinking replacements (`o\u{0358}→oo`, `\u{207f}→nn`, `oonn→onn`)
///   drain `map[pos + repl_len .. pos + find_len]` so the new last
///   byte of the replacement inherits the original `find`'s trailing
///   `raw_end` — i.e. the commit consumes everything `find` covered.
/// - Right-to-left replace order keeps already-computed positions
///   stable while we mutate `s` / `map`.
///
/// Caller must ensure `find` is non-empty and `repl.len() <=
/// find.len()` (asserted in debug builds — Codex pre-impl scope guard
/// 2026-05-15: we only need shrinking here; an expanding rule would
/// require allocating new map entries and is YAGNI).
fn offset_aware_replace(s: &mut String, map: &mut Vec<usize>, find: &str, repl: &str) {
    debug_assert!(!find.is_empty(), "offset_aware_replace: empty find pattern");
    debug_assert!(
        repl.len() <= find.len(),
        "offset_aware_replace: expanding replacement {find:?}→{repl:?} not supported"
    );
    let find_len = find.len();
    let repl_len = repl.len();
    if find_len == 0 || !s.contains(find) {
        return;
    }
    // Collect all match start byte positions left-to-right without
    // overlap (mirrors `str::replace`).
    let mut positions: Vec<usize> = Vec::new();
    let mut start = 0;
    while let Some(pos) = s[start..].find(find) {
        let abs = start + pos;
        positions.push(abs);
        start = abs + find_len;
    }
    for &pos in positions.iter().rev() {
        s.replace_range(pos..pos + find_len, repl);
        if find_len != repl_len {
            // `map[k]` = raw_end AFTER canonical byte k-1, so map has
            // length canonical.len() + 1. To preserve the load-bearing
            // contract that no syllabifier match ever leaves an
            // upstream-substituted source codepoint dangling in the
            // pending buffer (Codex post-impl P1 2026-05-15, against
            // `dictionary/output/dictionary.csv` `so` + `soo` siblings):
            //   1. Drain the trailing `(find_len - repl_len)` interior
            //      map entries inside the matched range — these are
            //      the bytes the substitution dropped.
            //   2. Force every surviving interior entry inside the
            //      match span (`map[pos + 1 .. pos + repl_len + 1]`)
            //      to the original `raw_end_of_match`. The replacement
            //      now represents the FULL match atomically, so any
            //      partial-prefix syllable candidate that lands at a
            //      shadow_end inside the substituted span still
            //      consumes every raw byte of the source spelling.
            let raw_end_of_match = map[pos + find_len];
            map.drain(pos + repl_len..pos + find_len);
            for slot in map.iter_mut().take(pos + repl_len + 1).skip(pos + 1) {
                *slot = raw_end_of_match;
            }
        }
    }
}

/// v3.5.8 Phase 9 Item 10 / v3.5.9 B-2 + D Fork 7b — partial-prefix
/// mode-aware key builder. Runs the
/// `lowercase → canonicalize_poj_shadow → build_hyphen_shadow →
/// strip_tones_for_mode` chain (mode-aware tone strip since v3.5.9 D / C-3b:
/// TL/POJ/English drop ASCII tone digits, TPS drops the 8 Bopomofo tone
/// scalars per `phonetics::is_tps_tone_mark`) as the production
/// [`left_anchored_keys_from_lattice`] / walker edge providers but
/// **skips the syllabifier** (the partial-prefix path is reached
/// precisely because the syllabifier returned no valid ending — TL `g`,
/// TPS `ㄉ`, etc.). Emits the `tl:` / `poj:` / `tps:` family prefix
/// matching `mode` via [`mode_key_prefix`] so the byte-range scan in
/// [`crate::continuous::fetch_via_lexicon_partial_inner`] hits the right
/// FST family. Returns `None` when the resulting toneless key is empty
/// (raw was hyphen-only / digit-only for TL/POJ, bare tone mark for TPS)
/// so the caller can short-circuit without firing an unbounded prefix
/// scan.
///
/// `consumed_span` is fixed to `(0, raw.len())` — partial-prefix
/// candidates always final-commit per Q15.4 (the offset maps from
/// Items 8 + 9 are intentionally discarded here because there is no
/// per-syllable mid-commit semantics to preserve).
// 中文: Item 10 / B-2 — partial-prefix mode-aware key 構造,沿用 Item 8/9 chain;不走 syllabifier。
// 中文:   consumed_span 固定 (0, raw.len()),配合 Q15.4 partial-prefix 一律 final-commit。
// 中文: B-2 rename:`_tl` 後綴脫去,emit 改為 mode-aware (POJ 走 `poj:` 家族)。
pub(crate) fn build_partial_prefix_key(
    raw: &str,
    mode: InputMode,
) -> Option<(ConsumedSpan, String)> {
    if raw.is_empty() {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    let (canonical, _canonical_to_raw_end) = canonicalize_poj_shadow(&lower, mode);
    let (shadow, _shadow_to_canonical_end) = build_hyphen_shadow(&canonical);
    // Explicit-tone fix — tone-aware body, same rule as
    // `left_anchored_keys_from_lattice` / the walker edge: a fully-toned
    // whole buffer (`tai5`) yields the verbatim `tl:tai5` prefix so the
    // Step 4b `lookup_prefix` extension scan only surfaces tone-5-initial
    // keys, never the all-tone `tl:tai` range. Toneless / mixed buffers
    // keep the toneless prefix (the partial-prefix path's normal "typing
    // toward the first boundary" behavior). See [`fst_body_for_span`].
    // v3.5.9 D / C-3b + D Fork 7b — TPS reaches this builder via the
    // unified `assemble_candidates` empty-keys fallthrough and always
    // takes the toneless branch (`is_tps_tone_mark` strip), so a raw
    // `ㄉㄧˊ` shadow still yields the `tps:ㄉㄧ` toneless key.
    // 中文: 明確聲調修正 — tone-aware 主體,與 left_anchored_keys_from_lattice / walker edge 同規則。
    // 中文:   全含調 buffer(tai5)→ verbatim `tl:tai5` 前綴,Step 4b lookup_prefix 只撈 tone-5 開頭鍵;
    // 中文:   去調/混合 buffer 維持去調前綴。TPS 恆走去調分支(is_tps_tone_mark 剝除)。
    let body = fst_body_for_span(&shadow, mode);
    if body.is_empty() {
        return None;
    }
    let prefix = mode_key_prefix(mode);
    Some(((0u32, raw.len() as u32), format!("{prefix}:{body}")))
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure shadow pipeline. Dispatch-level integration
    //! (decode round-trip, degraded paths) lives in
    //! `engine/composing/tests/dispatch_continuous.rs`; the byte-exact
    //! cross-slice golden lives in `engine/composing/tests/golden_fetch_at_pos.rs`.

    use super::*;

    #[test]
    fn strip_ascii_tone_digits_drops_all_ascii_digits() {
        // Equivalent to digit half of `notone.py::remove_tone()` regex
        // `[\d\-]` under the canonical-ASCII TL contract.
        assert_eq!(strip_ascii_tone_digits("tsua"), "tsua");
        assert_eq!(strip_ascii_tone_digits("tsua7"), "tsua");
        assert_eq!(strip_ascii_tone_digits("tai1bak4"), "taibak");
        // '0' is not a tone marker per phonetics::syllable.rs:18-20 but
        // notone.py drops every ASCII digit; mirror that here.
        assert_eq!(strip_ascii_tone_digits("a0b"), "ab");
        // Hyphen NOT stripped at this layer — `build_hyphen_shadow`
        // (Phase 9 Item 8) handles the `[\d\-]` regex's hyphen half
        // upstream, so by the time a slice reaches this fn it is
        // already hyphenless. The literal-passthrough assertion stays
        // as a behavioural pin so a refactor cannot quietly fold the
        // hyphen strip into both layers.
        assert_eq!(strip_ascii_tone_digits("tai-bak"), "tai-bak");
    }

    // ----- v3.5.8 S6 — custom_toneless_key (缺口 1) -----

    #[test]
    fn custom_toneless_key_numeric_tl_strips_tone_digits() {
        assert_eq!(
            custom_toneless_key("tai5gi2", InputMode::Tl).as_deref(),
            Some("tl:taigi")
        );
    }

    #[test]
    fn custom_toneless_key_numeric_with_hyphen_strips_both() {
        // Same `tl:taigi` key the edge provider builds for the `taigi`
        // span — proves the byte-identical-match contract.
        assert_eq!(
            custom_toneless_key("tai5-gi2", InputMode::Tl).as_deref(),
            Some("tl:taigi")
        );
    }

    #[test]
    fn custom_toneless_key_tl_strips_tone_keeps_literal_spelling() {
        // The canonicalize pass still strips tone marks + folds glyph
        // encoding, so a diacritic custom roman keys to the same toneless
        // form as the numeric one when there is NO spelling difference:
        // `tâi-gí` → `tl:taigi` == numeric `tai5gi2`.
        assert_eq!(
            custom_toneless_key("tâi-gí", InputMode::Tl).as_deref(),
            Some("tl:taigi"),
            "tone-mark + hyphen strip (no spelling change) → tl:taigi"
        );
        // TL-literal (2026-06-05): the POJ→TL SPELLING fold is NOT applied,
        // so POJ-spelled `oân` (oa) keys to `tl:taioan`, distinct from the
        // TL-spelled `uan5` → `tl:taiuan`. Both the lattice edge key and this
        // custom key run the SAME `canonicalize_poj_shadow(mode)`, so they
        // stay byte-identical and the continuous custom match still holds.
        assert_eq!(
            custom_toneless_key("tâi-oân", InputMode::Tl).as_deref(),
            Some("tl:taioan"),
        );
        assert_eq!(
            custom_toneless_key("tai5uan5", InputMode::Tl).as_deref(),
            Some("tl:taiuan"),
        );
    }

    #[test]
    fn custom_toneless_key_rejects_empty_and_non_tl_residue() {
        // Empty / whitespace / punctuation / CJK / digit-only custom
        // roman can never equal a syllabifier-built lattice edge key,
        // so it returns None and stays a span-local-only candidate.
        assert_eq!(custom_toneless_key("", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("   ", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("!!!", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("123", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("台語", InputMode::Tl), None);
    }

    #[test]
    fn custom_toneless_key_poj_ascii_matches_walker_edge_key() {
        // S6 byte-identity: a custom-dict roman `chiah` keyed under the
        // SAME `mode` the walker edge provider uses must equal the
        // lattice edge key for that mode. v3.5.9 B-2 — POJ now emits
        // `poj:` family keys preserving POJ ASCII (pre-B-2 this folded
        // to TL `tl:tsiah`; B-2 keeps POJ first-class via the `poj:`
        // family of the tagged-single-FST).
        assert_eq!(
            custom_toneless_key("chiah", InputMode::Poj).as_deref(),
            Some("poj:chiah"),
        );
        // TL mode keeps the F3C identity (un-canonicalized).
        assert_eq!(
            custom_toneless_key("chiah", InputMode::Tl).as_deref(),
            Some("tl:chiah"),
        );
    }

    // ----- v3.5.8 Phase 9 Item 8 — hyphen-shadow contract pins -----

    #[test]
    fn build_hyphen_shadow_no_hyphen_is_identity() {
        let (shadow, map) = build_hyphen_shadow("taibak");
        assert_eq!(shadow, "taibak");
        // No hyphens means every shadow byte maps to its own raw position
        // — pinning this protects existing hyphenless `consumed_span` values
        // from any future drift.
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn build_hyphen_shadow_internal_hyphen_collapses() {
        // `tai-bak` collapses to `taibak`; shadow byte 3 (`b`) maps to
        // raw end 5 because raw byte 4 was the consumed `-`.
        let (shadow, map) = build_hyphen_shadow("tai-bak");
        assert_eq!(shadow, "taibak");
        assert_eq!(map, vec![0, 1, 2, 3, 5, 6, 7]);
    }

    #[test]
    fn build_hyphen_shadow_leading_hyphen_consumes_into_prefix() {
        // Leading `-` is consumed into the prefix: shadow byte 0 (`t`)
        // maps to raw end 2 so commits at any shadow ending cover the
        // leading hyphen.
        let (shadow, map) = build_hyphen_shadow("-tai");
        assert_eq!(shadow, "tai");
        assert_eq!(map, vec![0, 2, 3, 4]);
    }

    #[test]
    fn build_hyphen_shadow_trailing_hyphen_is_not_consumed() {
        // Trailing `-` stays in pending: shadow ending at len() maps to
        // raw byte AFTER the last non-hyphen char, never to raw_len.
        let (shadow, map) = build_hyphen_shadow("tai-");
        assert_eq!(shadow, "tai");
        // map.last() = raw end after the final `i`, NOT raw_len.
        assert_eq!(map, vec![0, 1, 2, 3]);
    }

    #[test]
    fn build_hyphen_shadow_double_hyphen_collapses() {
        // `goa--si` → `goasi`; shadow byte 3 (`s`) maps to raw end 6
        // because raw bytes 3 & 4 were the consumed `--` pair.
        let (shadow, map) = build_hyphen_shadow("goa--si");
        assert_eq!(shadow, "goasi");
        assert_eq!(map, vec![0, 1, 2, 3, 6, 7]);
    }

    #[test]
    fn build_hyphen_shadow_all_hyphens_yields_empty_shadow() {
        let (shadow, map) = build_hyphen_shadow("---");
        assert_eq!(shadow, "");
        // Only the baseline entry survives (`map[0] = 0`).
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn build_hyphen_shadow_empty_input_yields_single_baseline() {
        let (shadow, map) = build_hyphen_shadow("");
        assert_eq!(shadow, "");
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn build_hyphen_shadow_trailing_hyphen_after_internal_hyphen_excluded() {
        // `tai-bak-`: the internal `-` folds into the consumed prefix,
        // but the trailing `-` stays pending — so map.last() = 7 (after
        // `k`), NOT 8 (the raw_len). Catches a regression on the
        // off-by-one risk Codex flagged in pre-impl consult.
        let (shadow, map) = build_hyphen_shadow("tai-bak-");
        assert_eq!(shadow, "taibak");
        assert_eq!(map, vec![0, 1, 2, 3, 5, 6, 7]);
    }

    // ----- v3.5.8 Phase 9 Item 9 — canonicalize_poj_shadow contract pins -----

    #[test]
    fn canonicalize_poj_shadow_pure_ascii_is_identity_fast_path() {
        // The F3C gate: pure-ASCII TL input MUST pass through with an
        // identity offset map, otherwise downstream Item 8 contract pins
        // (offset map = vec![0, 1, 2, ...]) regress.
        let (canonical, map) = canonicalize_poj_shadow("tai-bak", InputMode::Tl);
        assert_eq!(canonical, "tai-bak");
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn canonicalize_poj_shadow_empty_ascii_is_identity() {
        let (canonical, map) = canonicalize_poj_shadow("", InputMode::Tl);
        assert_eq!(canonical, "");
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn canonicalize_poj_shadow_combining_tone_mark_drops_and_absorbs() {
        // `pe\u{030d}h` (POJ `pe̍h` for 白): combining tone-8 mark on
        // `e`; canonical drops it and `e`'s raw_end inherits the 2
        // bytes the mark would have consumed.
        let (canonical, map) = canonicalize_poj_shadow("pe\u{030d}h", InputMode::Tl);
        assert_eq!(canonical, "peh");
        // `p` stays at raw_end 1; `e` jumps to 4 (skipping the
        // 2-byte `\u{030d}`); `h` lands at 5.
        assert_eq!(map, vec![0, 1, 4, 5]);
    }

    #[test]
    fn canonicalize_poj_shadow_o_with_dot_above_right_emits_oo() {
        // `so\u{0358}` (POJ `so͘` for 嫂): combining dot-above-right is
        // NOT a tone mark per is_tone_combining_mark; it survives
        // Phase 1 and Phase 2 collapses `o\u{0358}` → `oo`.
        let (canonical, map) = canonicalize_poj_shadow("so\u{0358}", InputMode::Tl);
        assert_eq!(canonical, "soo");
        // Both new `o` bytes anchor at raw_end 4 (after the full
        // `o\u{0358}` source spelling). Codex post-impl P1: partial-
        // prefix `so` against the live dictionary's `so` / `soo`
        // sibling pair must still consume the full source spelling.
        assert_eq!(map, vec![0, 1, 4, 4]);
    }

    #[test]
    fn canonicalize_poj_shadow_leading_combining_mark_preserves_baseline_zero() {
        // Standalone leading combining mark (no base char to absorb
        // into): Codex post-impl P3 — baseline map[0] = 0 stays, the
        // next emitted char accounts for the dropped mark's bytes via
        // its own raw_idx + len_utf8.
        let (canonical, map) = canonicalize_poj_shadow("\u{030d}h", InputMode::Tl);
        assert_eq!(canonical, "h");
        assert_eq!(map, vec![0, 3]);
    }

    #[test]
    fn canonicalize_poj_shadow_superscript_nasal_marker_emits_nn() {
        // `pe\u{207f}` (POJ `peⁿ`): superscript-n collapses to `nn`.
        let (canonical, map) = canonicalize_poj_shadow("pe\u{207f}", InputMode::Tl);
        assert_eq!(canonical, "penn");
        // `p` → 1, `e` → 2, both new `n` bytes anchor at raw_end 5
        // (after `\u{207f}`).
        assert_eq!(map, vec![0, 1, 2, 5, 5]);
    }

    #[test]
    fn canonicalize_poj_shadow_tl_non_ascii_is_literal_no_spelling_fold() {
        // TL-literal (2026-06-05): non-ASCII `chóa` (POJ glyph for 紙) keeps
        // its literal spelling after the tone-2 acute drop — NO `ch→ts` /
        // `oa→ua` POJ→TL fold. Pre-2026-06-05 this folded to `tsua`.
        let (canonical, _map) = canonicalize_poj_shadow("ch\u{f3}a", InputMode::Tl);
        assert_eq!(canonical, "choa");
    }

    #[test]
    fn canonicalize_poj_shadow_precomposed_uppercase_lowercases_via_phase2() {
        // `\u{00d3}` (`Ó`, precomposed UPPERCASE) → NFD `O\u{0301}` →
        // drop combining → `O` (uppercase) → Phase 2 lowercase pass
        // makes it `o`. This pins the ordering: NFD walk must come
        // BEFORE the lowercase pass, otherwise uppercase precomposed
        // diacritic chars would survive into Phase 2 substitutions.
        // TL-literal (2026-06-05): no `oa→ua` spelling fold, so the result
        // is `oa` — still proving the uppercase `Ó` lowercased to `o`.
        let (out, _map) = canonicalize_poj_shadow("\u{00d3}a", InputMode::Tl);
        assert_eq!(out, "oa", "{out:?}");
    }

    #[test]
    fn canonicalize_poj_shadow_oonn_shrinks_with_offset_drain() {
        // Synthetic regression: simulate Phase 1 emitting `oonn`
        // (e.g. via `o\u{0358}\u{207f}` upstream). Phase 2 `oonn→onn`
        // is the only 4→3 shrinking rule and must drain exactly one
        // map entry. Input `o\u{0358}\u{207f}` itself: `o` (1) +
        // `\u{0358}` (2) + `\u{207f}` (3) = 6 bytes.
        let (out, map) = canonicalize_poj_shadow("o\u{0358}\u{207f}", InputMode::Tl);
        assert_eq!(out, "onn", "{out:?}");
        // After `oonn→onn` collapse, the final byte's raw_end must
        // equal the full input length (6).
        assert_eq!(*map.last().unwrap(), 6, "{map:?}");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_preserves_poj_shape() {
        // v3.5.9 B-2 (PR #309 refinement) — POJ mode applies the
        // glyph-only NORMALIZE_TO_POJ_GLYPH_RULES subset (`o\u{0358}
        // →oo`, `\u{207f}→nn`, `\u{1d3a}→nn`; no `ou→oo` alias
        // whole-buffer). Pure ASCII POJ syllables stay POJ-shaped —
        // they are then looked up against the `poj:` family of the
        // tagged-single-FST.
        let (poj, map) = canonicalize_poj_shadow("chiah", InputMode::Poj);
        assert_eq!(poj, "chiah", "POJ mode preserves POJ ASCII (no ch→ts fold)");
        // ASCII identity map: Phase 1 NFD is identity for ASCII; no
        // POJ rule fires on `chiah` (no `ou`, no non-ASCII chars), so
        // the map is invariant.
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5]);
        // TL mode keeps the F3C identity unchanged from pre-B-2.
        let (tl, _) = canonicalize_poj_shadow("chiah", InputMode::Tl);
        assert_eq!(tl, "chiah");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_no_tl_chain_substitutions() {
        // v3.5.9 B-2 — none of the POJ→TL chain rules (`chh→tsh`,
        // `oa→ua`, `oe→ue`, `eng→ing`, `ek→ik`) fire in POJ mode:
        // NORMALIZE_TO_POJ_RULES is encoding-only. Each input below
        // stays as itself, routes to the `poj:` family.
        for input in ["chhia", "goa", "hoe", "peng", "tek"] {
            let (out, _) = canonicalize_poj_shadow(input, InputMode::Poj);
            assert_eq!(out, input, "POJ `{input}` must stay POJ-shaped");
        }
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_ou_is_boundary_preserving() {
        // v3.5.9 B-2 PR #309 (Codex P1 `r3276402303`) — shadow
        // canonicalize MUST NOT apply the `ou → oo` alias whole-buffer:
        // hyphenless POJ user input like `toui` (intended POJ `tó-uī`,
        // indexed `poj_notone=toui`) would mis-fold to `tooi` and lose
        // the lattice match. Under the glyph-only rule subset, ASCII
        // POJ input is identity — the lattice + syllabifier handle
        // boundaries via the `poj:` family inventory.
        let (toui, toui_map) = canonicalize_poj_shadow("toui", InputMode::Poj);
        assert_eq!(
            toui, "toui",
            "POJ `toui` must stay `toui` (boundary preserved)"
        );
        // ASCII identity → offset map is the identity sequence.
        assert_eq!(toui_map, vec![0, 1, 2, 3, 4]);
        // Same boundary-preservation guarantee for typical dirty
        // single-syllable inputs (`sou`, `kou`): these now stay as
        // themselves (pre-fix they folded). Per-syllable dirty-row
        // protection lives in the build pipeline + per-token matching
        // guard derive — both consume the full NORMALIZE_TO_POJ_RULES
        // and remain safe (one syllable per application).
        let (sou, _) = canonicalize_poj_shadow("sou", InputMode::Poj);
        assert_eq!(sou, "sou");
        let (kou, _) = canonicalize_poj_shadow("kou", InputMode::Poj);
        assert_eq!(kou, "kou");
        // TL mode unchanged — F3C identity fast-path on ASCII.
        let (tl_sou, _) = canonicalize_poj_shadow("sou", InputMode::Tl);
        assert_eq!(tl_sou, "sou", "TL ASCII path stays identity (F3C)");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_non_ascii_keeps_poj_shape() {
        // v3.5.9 B-2 (PR #309 refinement) — non-ASCII POJ-display input
        // under POJ mode runs Phase 1 (NFD + tone-mark drop) then
        // Phase 2 with NORMALIZE_TO_POJ_GLYPH_RULES (glyph encoding
        // only, no `ou→oo` alias whole-buffer). Result preserves POJ
        // shape — `chia\u{030d}h` (POJ `chia̍h` for 食) stays `chiah`,
        // NOT folded to TL `tsiah`.
        let (out, _) = canonicalize_poj_shadow("chia\u{030d}h", InputMode::Poj);
        assert_eq!(
            out, "chiah",
            "POJ mode keeps POJ shape after tone-mark drop"
        );
        // TL mode is literal too (2026-06-05) — no `ch→ts` spelling fold;
        // both modes converge to `chiah` on this glyph-only input.
        let (tl, _) = canonicalize_poj_shadow("chia\u{030d}h", InputMode::Tl);
        assert_eq!(tl, "chiah", "TL literal: no ch→ts spelling fold");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_non_ascii_o_with_dot_above_right() {
        // v3.5.9 B-2 SHOULD #2 (Codex pre-impl) — the shrinking
        // `o\u{0358}→oo` rule fires in BOTH lists; verify the offset
        // map drain stays consistent under POJ mode (commit-span
        // contract): the two emitted `o` bytes both anchor at raw_end
        // 4 so a partial-prefix `so` candidate still consumes the full
        // `o\u{0358}` source spelling on commit.
        let (poj, poj_map) = canonicalize_poj_shadow("so\u{0358}", InputMode::Poj);
        assert_eq!(poj, "soo");
        assert_eq!(poj_map, vec![0, 1, 4, 4]);
        // TL mode produces the same shape (the rule is shared) — pin
        // both to lock cross-mode byte-identity on this commit-span-
        // sensitive shrinking rule.
        let (tl, tl_map) = canonicalize_poj_shadow("so\u{0358}", InputMode::Tl);
        assert_eq!(tl, "soo");
        assert_eq!(tl_map, poj_map);
    }

    #[test]
    fn canonicalize_poj_shadow_poj_non_ascii_superscript_nasal() {
        // v3.5.9 B-2 SHOULD #2 — `\u{207f}→nn` (POJ `ⁿ` → ASCII `nn`)
        // shrinks from 3 bytes to 2. Verify the POJ-mode offset map
        // matches the TL-mode one byte-for-byte (commit-span contract).
        let (poj, poj_map) = canonicalize_poj_shadow("pe\u{207f}", InputMode::Poj);
        assert_eq!(poj, "penn");
        assert_eq!(poj_map, vec![0, 1, 2, 5, 5]);
        let (tl, tl_map) = canonicalize_poj_shadow("pe\u{207f}", InputMode::Tl);
        assert_eq!(tl, "penn");
        assert_eq!(tl_map, poj_map);
    }

    #[test]
    fn canonicalize_poj_shadow_tl_ascii_chiah_stays_identity_f3c_guard() {
        // Regression guard for the F3C gate: the SAME ASCII input in TL
        // mode (`mode = InputMode::Tl`) must NOT be rewritten, so the `tó-uī`
        // (`toui`) class of real TL entries is never garbled.
        let (out, map) = canonicalize_poj_shadow("chiah", InputMode::Tl);
        assert_eq!(out, "chiah", "TL-mode ASCII must stay identity");
        assert_eq!(map, (0..="chiah".len()).collect::<Vec<_>>());
        // `toui` (佗位) must survive — `ou→oo` must NOT fire in TL mode.
        let (toui, _) = canonicalize_poj_shadow("toui", InputMode::Tl);
        assert_eq!(toui, "toui", "F3C: TL-mode `toui` must not become `tooi`");
    }

    #[test]
    fn canonicalize_poj_shadow_non_ascii_both_modes_literal() {
        // 2026-06-05 TL-literal pass — non-ASCII POJ-display input now keeps
        // its literal shape in BOTH modes (POJ glyph-only, TL encoding-only,
        // neither runs the POJ→TL spelling chain). `chóa` (POJ for 紙) →
        // `choa` either way. The pre-B-2 TL `tsua` fold AND the B-2-era
        // mode divergence are both gone.
        let (poj, _) = canonicalize_poj_shadow("ch\u{f3}a", InputMode::Poj);
        let (tl, _) = canonicalize_poj_shadow("ch\u{f3}a", InputMode::Tl);
        assert_eq!(poj, "choa", "POJ keeps POJ ASCII shape");
        assert_eq!(tl, "choa", "TL literal: no ch→ts / oa→ua fold");
    }

    #[test]
    fn is_tone_combining_mark_covers_all_eight_tones() {
        // Pins parity with `engine/phonetics/src/tables.rs::COMBINING_TO_TONE_NUM`.
        // If a new tone mark is added there, this assertion must be
        // updated in lockstep — the comment list above guards the
        // mapping.
        for c in [
            '\u{0300}', '\u{0301}', '\u{0302}', '\u{0304}', '\u{0306}', '\u{030b}', '\u{030c}',
            '\u{030d}',
        ] {
            assert!(is_tone_combining_mark(c), "{c:?} should be a tone mark");
        }
    }

    #[test]
    fn is_tone_combining_mark_excludes_non_tone_combiners() {
        // `\u{0358}` (combining dot above right) is in the
        // U+0300-U+036F combining block but is NOT a tone mark; it has
        // to survive Phase 1 so Phase 2 `o\u{0358}→oo` can fire.
        assert!(!is_tone_combining_mark('\u{0358}'));
        // ASCII letters / digits / hyphens / common Latin diacritics
        // must obviously not be flagged either.
        for c in ['a', '0', '-', '\u{00e2}', '\u{014d}'] {
            assert!(!is_tone_combining_mark(c), "{c:?} must not be a tone mark");
        }
    }

    #[test]
    fn offset_aware_replace_same_length_leaves_map_invariant() {
        let mut s = String::from("choa");
        let mut map = vec![0, 1, 2, 3, 4];
        offset_aware_replace(&mut s, &mut map, "ch", "ts");
        assert_eq!(s, "tsoa");
        assert_eq!(map, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn offset_aware_replace_shrinking_drains_middle_entries() {
        // Synthetic: `xxoonnyy` shrinks `oonn` (4 bytes) → `onn` (3),
        // dropping one map entry from inside the matched range. The
        // entry that survives at the new end position must equal the
        // original map[pos + find_len] (raw_end of the full match).
        let mut s = String::from("xxoonnyy");
        let mut map = vec![0, 10, 20, 30, 40, 50, 60, 70, 80];
        offset_aware_replace(&mut s, &mut map, "oonn", "onn");
        assert_eq!(s, "xxonnyy");
        // Post-replace map length = canonical.len() + 1 = 8.
        // Slot for "after the entire `onn` collapse" (map index 5)
        // must equal the original map[6] = 60 (raw_end of the full
        // `oonn` match).
        assert_eq!(map.len(), 8);
        assert_eq!(map[0], 0);
        assert_eq!(map[2], 20, "byte before `oo` must be unchanged");
        assert_eq!(
            map[5], 60,
            "byte after `onn` must equal raw_end of full match"
        );
        assert_eq!(map[6], 70, "tail must shift left by one slot");
    }

    #[test]
    fn offset_aware_replace_skips_when_pattern_absent() {
        let mut s = String::from("xyz");
        let mut map = vec![0, 1, 2, 3];
        offset_aware_replace(&mut s, &mut map, "ab", "cd");
        assert_eq!(s, "xyz");
        assert_eq!(map, vec![0, 1, 2, 3]);
    }

    // ----- v3.5.8 Phase 9 Item 10 — partial-prefix key derivation -----

    #[test]
    fn build_partial_prefix_key_passes_ascii_through() {
        let (span, key) = build_partial_prefix_key("gu", InputMode::Tl).unwrap();
        // partial-prefix candidates always final-commit (Q15.4) →
        // consumed_span covers the whole pending tail.
        assert_eq!(span, (0u32, 2u32));
        assert_eq!(key, "tl:gu");
    }

    #[test]
    fn build_partial_prefix_key_tone_aware_lowercases() {
        // Explicit-tone fix — a fully-toned partial buffer KEEPS its tone
        // digit so the prefix scan filters to the typed tone: `GU5` →
        // `tl:gu5` (was `tl:gu` pre-fix). Toneless input still strips to
        // the all-tone fused prefix: `GU` → `tl:gu`.
        // trace: "GU5" → lower "gu5" → shadow "gu5" → fully-toned (`gu`+`5`)
        //   → verbatim body "gu5" → "tl:gu5".
        let (_, key) = build_partial_prefix_key("GU5", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:gu5");
        let (_, key) = build_partial_prefix_key("GU", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:gu");
    }

    #[test]
    fn build_partial_prefix_key_strips_internal_hyphen_via_item8_shadow() {
        // Item 8's hyphen-shadow chain runs on the partial-prefix path
        // too — `tai-` collapses to `tai` (trailing `-` stays in the
        // pending raw buffer per build_hyphen_shadow contract), and
        // `-tai` collapses to `tai`.
        let (_, key) = build_partial_prefix_key("tai-", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:tai");
        let (_, key) = build_partial_prefix_key("-tai", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:tai");
    }

    #[test]
    fn build_partial_prefix_key_canonicalizes_poj_diacritic_via_item9() {
        // Item 9's canonicalize chain runs on partial-prefix input too
        // — `pe\u{030d}` (POJ `pe̍h` minus the trailing `h`) folds to
        // `pe` after the tone-mark drop, giving FST key `tl:pe`.
        let (_, key) = build_partial_prefix_key("pe\u{030d}", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:pe");
    }

    #[test]
    fn build_partial_prefix_key_returns_none_for_empty_after_strip() {
        // Hyphen-only or digit-only raw produces an empty toneless
        // key — return None so the caller skips the FST scan.
        assert!(build_partial_prefix_key("", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("-", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("--", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("5", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("-5-", InputMode::Tl).is_none());
    }

    #[test]
    fn build_partial_prefix_key_tone_policy_pins_unvalidated_partial() {
        // Explicit-tone fix — the partial-prefix builder skips the
        // syllabifier (it is reached precisely when no valid ending
        // exists), so it can receive a NON-validated whole buffer. Pin
        // the tone-policy on each shape so the safety reasoning in
        // `fst_body_for_span` stays honest:
        //   - fully-toned → verbatim toned prefix (filters by tone).
        let (_, k) = build_partial_prefix_key("tai5", InputMode::Tl).unwrap();
        assert_eq!(k, "tl:tai5");
        //   - mixed (trailing letter) → toneless prefix (no regression).
        let (_, k) = build_partial_prefix_key("tai5g", InputMode::Tl).unwrap();
        assert_eq!(k, "tl:taig");
        //   - fully-toned-LOOKING but non-existent → still verbatim; it
        //     just misses the FST and returns zero candidates (NEVER a
        //     wrong-tone hit), which is the safe outcome for junk input.
        let (_, k) = build_partial_prefix_key("abc1", InputMode::Tl).unwrap();
        assert_eq!(k, "tl:abc1");
    }

    #[test]
    fn build_partial_prefix_key_poj_emits_poj_family() {
        // v3.5.9 B-2 — POJ mode emits `poj:` family keys preserving POJ
        // ASCII spelling (pre-B-2 this test asserted `tl:tsi` after a
        // POJ→TL fold; B-2 makes POJ first-class so `chi` stays `chi`
        // and routes to the `poj:` family of the FST).
        let (_, key) = build_partial_prefix_key("chi", InputMode::Poj).unwrap();
        assert_eq!(key, "poj:chi");
        let (_, tl_key) = build_partial_prefix_key("chi", InputMode::Tl).unwrap();
        assert_eq!(tl_key, "tl:chi", "TL mode keeps F3C identity");
    }

    #[test]
    fn build_partial_prefix_key_tps_emits_tps_family_for_leading_initial() {
        // v3.5.9 D Fork 7b — TPS partial-prefix activated. A leading
        // lone Bopomofo initial `ㄉ` (3 bytes UTF-8) emits `tps:ㄉ` so
        // `prefix_index.n("tps:ㄉ")` byte-range scans every dictionary
        // row whose `tps_notone` starts with `ㄉ` (佇/著/丁/同/單/...).
        let (span, key) = build_partial_prefix_key("\u{3109}", InputMode::Tps).unwrap();
        assert_eq!(span, (0u32, 3u32));
        assert_eq!(key, "tps:\u{3109}");
    }

    #[test]
    fn build_partial_prefix_key_tps_strips_tone_marks() {
        // `ㄉㄞˊ` (with U+02CA tone-5 mark) → `tps:ㄉㄞ`. The 8 Bopomofo
        // tone scalars (per `phonetics::is_tps_tone_mark`) are stripped
        // by `strip_tones_for_mode(_, Tps)` exactly like ASCII digits
        // are stripped for TL/POJ.
        let (_, key) =
            build_partial_prefix_key("\u{3109}\u{311e}\u{02ca}", InputMode::Tps).unwrap();
        assert_eq!(key, "tps:\u{3109}\u{311e}");
    }

    #[test]
    fn build_partial_prefix_key_tps_bare_tone_mark_returns_none() {
        // A bare tone-mark only buffer (no Bopomofo body) strips to
        // empty → None, preventing an unbounded `tps:` namespace scan.
        // Mirrors the TL `digit-only` / `hyphen-only` guard above.
        assert!(build_partial_prefix_key("\u{02ca}", InputMode::Tps).is_none());
        assert!(build_partial_prefix_key("\u{02cb}", InputMode::Tps).is_none());
        assert!(build_partial_prefix_key("\u{0307}", InputMode::Tps).is_none());
    }

    // ----- v3.5.8 S5 — no-dict carve-out (greedy + min-hop helpers) -----

    // Hermetic `SyllableInventory` builder — same inline pattern as the
    // `lattice::builder` unit tests (inline duplication preferred over a
    // shared test-utils crate). Pins the carve-out helper without the
    // `LexiconHandle` singleton (Codex post-impl S5 P3, 2026-05-17).
    fn build_inventory(samples: &[&str]) -> SyllableInventory {
        use std::path::PathBuf;

        use fst::SetBuilder;
        use phonetics::canonicalize_syllable;

        let mut keys: Vec<String> = Vec::new();
        for s in samples {
            let (canonical, tone) = canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"));
            if tone.is_empty() {
                keys.push(format!("tl:{canonical}"));
            } else {
                keys.push(format!("tl:{canonical}{tone}"));
                keys.push(format!("tl:{canonical}"));
            }
        }
        keys.sort();
        keys.dedup();

        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf = std::env::temp_dir().join(format!(
            "taigi_shadow_carveout_{}_{n}.fst",
            std::process::id()
        ));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in &keys {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
        SyllableInventory::open(&path).expect("open inventory")
    }

    /// v3.5.9 B-2 — POJ-only inventory builder. Emits `poj:<canonical>`
    /// keys via `phonetics::canonicalize_poj_syllable` (which preserves
    /// POJ ASCII shape, distinct from `canonicalize_syllable`'s TL fold).
    /// Used by the B-2 mode-aware unit tests to prove POJ shadow helpers
    /// route to the `poj:` family of the tagged-single-FST.
    // 中文: B-2 — POJ-only inventory builder。emit `poj:` 前綴 + POJ ASCII canonical,
    // 中文:   驗證 mode-aware shadow helpers 路由到 `poj:` 家族。
    fn build_poj_inventory(samples: &[&str]) -> SyllableInventory {
        use std::path::PathBuf;

        use fst::SetBuilder;
        use phonetics::canonicalize_poj_syllable;

        let mut keys: Vec<String> = Vec::new();
        for s in samples {
            let (canonical, tone) = canonicalize_poj_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_poj_syllable"));
            if tone.is_empty() {
                keys.push(format!("poj:{canonical}"));
            } else {
                keys.push(format!("poj:{canonical}{tone}"));
                keys.push(format!("poj:{canonical}"));
            }
        }
        keys.sort();
        keys.dedup();

        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf = std::env::temp_dir().join(format!(
            "taigi_shadow_carveout_poj_{}_{n}.fst",
            std::process::id()
        ));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in &keys {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
        SyllableInventory::open(&path).expect("open inventory")
    }

    #[test]
    fn greedy_longest_syllabification_taiuantai_reads_tai_uan_tai() {
        // The documented no-dict carve-out expectation: `taiuantai`
        // (no dict hit anywhere) must read as the canonical
        // longest-syllable segmentation `tai uan tai`, NOT the
        // sub-syllable over-split `ta i u an ta i` (literal
        // max-syllable-count) and NOT the min-cost fewest-edge blob.
        let inv = build_inventory(&["tai1", "uan1", "ta1", "i1", "u1", "an1"]);
        let segs = greedy_longest_syllabification("taiuantai", &inv, InputMode::Tl)
            .expect("buffer is fully syllabifiable");
        assert_eq!(segs, vec![(0, 3), (3, 6), (6, 9)]);
        let roman = segs
            .iter()
            .map(|&(s, e)| strip_ascii_tone_digits(&"taiuantai"[s..e]))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(roman, "tai uan tai");
    }

    #[test]
    fn greedy_longest_syllabification_returns_none_when_unsegmentable() {
        // A trailing byte with no valid syllable → `None`, so the
        // caller suppresses the slot-0 synth and leaves the span-local
        // list untouched (pre-S2 behavior).
        let inv = build_inventory(&["tai1"]);
        assert!(greedy_longest_syllabification("taix", &inv, InputMode::Tl).is_none());
    }

    #[test]
    fn greedy_longest_syllabification_poj_mode_uses_poj_family() {
        // v3.5.9 B-2 — under POJ mode the carve-out walks the `poj:`
        // family of the inventory. With a `poj:`-only inventory (no
        // matching `tl:` entries), TL-mode probing must FAIL while
        // POJ-mode probing succeeds — proves the mode parameter selects
        // the right family end-to-end.
        let inv = build_poj_inventory(&["chiah4", "goa2"]);
        let segs = greedy_longest_syllabification("chiahgoa", &inv, InputMode::Poj)
            .expect("POJ shadow must syllabify against `poj:` family");
        assert_eq!(segs, vec![(0, 5), (5, 8)]);
        // TL mode against the same inventory finds nothing — the `tl:`
        // family is empty, so the very first step has no valid ending.
        assert!(
            greedy_longest_syllabification("chiahgoa", &inv, InputMode::Tl).is_none(),
            "TL mode must NOT see `poj:`-only inventory entries"
        );
    }

    // ----- v3.5.8 OOV-cost fix — span_min_syllable_count
    //       (Codex PR #290 P1 r3255035136) -----

    #[test]
    fn span_min_syllable_count_simple_spans() {
        let inv = build_inventory(&["tai1", "uan1", "ta1"]);
        // Whole span is itself one valid syllable → 1 (correct, not a
        // collapse).
        assert_eq!(span_min_syllable_count("tai", &inv, InputMode::Tl), Some(1));
        // `taiuanta` = tai|uan|ta → 3 (the synth syllable-sum metadata).
        assert_eq!(
            span_min_syllable_count("taiuanta", &inv, InputMode::Tl),
            Some(3)
        );
        // Not single-syllable-reachable → None (caller fail-closes).
        assert_eq!(span_min_syllable_count("taix", &inv, InputMode::Tl), None);
        assert_eq!(span_min_syllable_count("", &inv, InputMode::Tl), None);
    }

    #[test]
    fn span_min_syllable_count_recovers_a_greedy_dead_end() {
        // The exact P1: greedy-longest dead-ends but the span IS
        // lattice-syllabifiable via a non-greedy split, so the old
        // `greedy…unwrap_or(1)` mispriced this 2-syllable OOV blob as
        // ONE syllable. Inventory = {ta, tan, nia}; input `tania`:
        //   greedy from 0 takes the LONGEST prefix `tan` → tail `ia`
        //   has no valid syllable → greedy = None → old code = 1.
        //   non-greedy `ta` then `nia` spans it → real count = 2.
        let inv = build_inventory(&["ta1", "tan1", "nia1"]);
        assert!(
            greedy_longest_syllabification("tania", &inv, InputMode::Tl).is_none(),
            "precondition: greedy-longest must dead-end on this span"
        );
        assert_eq!(
            span_min_syllable_count("tania", &inv, InputMode::Tl),
            Some(2),
            "min-hop walk must recover the real 2-syllable count, not collapse to 1"
        );
    }

    #[test]
    fn span_min_syllable_count_poj_mode_uses_poj_family() {
        // v3.5.9 B-2 — POJ mode min-hop walks the `poj:` family. A
        // `poj:`-only inventory: POJ-mode hop count succeeds while
        // TL-mode probe must return None.
        let inv = build_poj_inventory(&["chiah4", "goa2"]);
        assert_eq!(
            span_min_syllable_count("chiahgoa", &inv, InputMode::Poj),
            Some(2),
        );
        assert_eq!(
            span_min_syllable_count("chiahgoa", &inv, InputMode::Tl),
            None,
            "TL mode must NOT see `poj:`-only entries"
        );
    }
}
