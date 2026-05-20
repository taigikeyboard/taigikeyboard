// Pure shadow pipeline — POJ→TL canonicalize, hyphen strip, lattice build, and derived span / partial-prefix / custom-key helpers.

use lexicon::{ConsumedSpan, SyllableInventory};
use unicode_normalization::UnicodeNormalization;

use crate::lattice::{build_lattice, Lattice};
use crate::syllabifier::tl as tl_syll;

/// Cap on syllabifier BFS depth for Phase 6 fetches. Matches the
/// `max_syllables=8` budget called out in `docs/roadmap.md:231` and
/// keeps the worst-case lookup at O(n × 3 × 8) FST hits. Shared by
/// [`build_shadow_lattice`] (lattice BFS budget) and
/// `continuous::build_keys_tps` (TPS cumulative-key cap) — they must
/// agree to keep per-keystroke FST lookup and candidate scoring
/// complexity bounded across modes.
// 中文: TL syllabifier BFS 深度上限,對應 roadmap §Phase 3 的 8 syllable 估算。
// 中文: build_shadow_lattice 與 continuous::build_keys_tps 共用,確保各模式每鍵 FST 查詢複雜度有界。
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
    is_poj: bool,
) -> (String, Vec<usize>, Lattice) {
    let lower = raw.to_ascii_lowercase();
    let (canonical, canonical_to_raw_end) = canonicalize_poj_shadow(&lower, is_poj);
    let (shadow, shadow_to_canonical_end) = build_hyphen_shadow(&canonical);
    let shadow_to_raw_end: Vec<usize> = shadow_to_canonical_end
        .iter()
        .map(|&c| canonical_to_raw_end[c])
        .collect();
    let lattice = build_lattice(&shadow, inv, MAX_SYLLABLES);
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
/// Q1c = option ii, 2026-05-16; `docs/roadmap.md` §整句 lattice).
// 中文: A1 抽出 — 只發左錨投影 (start==0) 為 key,與 S1 前逐 byte 相同 → span-local 不變。
// 中文: 內段 (start>0) 不發為可點 key:Model B forward-only commit 無對應語意;
// 中文:   S2 walker 內部吃內段邊、合成單一全 buffer 最佳路徑由 handle_fetch_at_pos
// 中文:   explicit prepend 到 slot 0,commit span 維持 (0,end)。
pub(crate) fn left_anchored_keys_from_lattice(
    shadow: &str,
    shadow_to_raw_end: &[usize],
    lattice: &Lattice,
) -> Vec<(ConsumedSpan, String)> {
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
        let toneless = strip_ascii_tone_digits(&shadow[..end]);
        if toneless.is_empty() {
            continue;
        }
        let raw_end = shadow_to_raw_end[end];
        out.push(((0u32, raw_end as u32), format!("tl:{toneless}")));
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
) -> Option<Vec<(usize, usize)>> {
    let lowered = shadow.to_ascii_lowercase();
    let mut segs: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0usize;
    while pos < lowered.len() {
        let end = tl_syll::valid_span_endings_lowered(&lowered, pos, inv, 1)
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
pub(crate) fn span_min_syllable_count(shadow_span: &str, inv: &SyllableInventory) -> Option<usize> {
    let lowered = shadow_span.to_ascii_lowercase();
    let end = lowered.len();
    if end == 0 {
        return None;
    }
    // Unweighted shortest path (in #hops) from offset 0 to `end`. Each
    // hop is one valid syllable from
    // `valid_span_endings_lowered(.., pos, inv, 1)` (the single-hop
    // primitive the lattice builder chains). FIFO BFS + a visited
    // distance map = min hops; only strictly-forward steps are
    // enqueued so it terminates in <= `end` iterations.
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
        for nxt in tl_syll::valid_span_endings_lowered(&lowered, pos, inv, 1) {
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
/// `custom_dictionary.db` entry's romanization, or `None` when it is
/// not TL-shaped.
///
/// MUST produce a key byte-identical to the one
/// `continuous::fetch_walker_slot0_inner`'s edge provider builds for a
/// syllable span (`tl:{toneless}`, where `toneless` is the
/// hyphen-stripped, POJ-canonicalized, tone-digit-stripped shadow
/// slice). It therefore reuses the **same three shadow helpers in the
/// same order** — [`canonicalize_poj_shadow`] → [`build_hyphen_shadow`]
/// → [`strip_ascii_tone_digits`] — as the single normalization source.
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
/// CJK / digit-only / non-TL custom roman can never equal a
/// syllabifier-built lattice edge key, so it simply stays a span-local
/// candidate and never enters the walker.
///
/// `is_poj` MUST be the same value `continuous::fetch_walker_slot0_inner`
/// passes to [`build_shadow_lattice`] for this fetch: the
/// canonicalize step is mode-gated (toneless ASCII POJ folds `ch→ts`
/// only when `is_poj`), so a mismatch would make a custom roman key
/// `tl:chiah` while the lattice edge keys `tl:tsiah`, silently
/// breaking the S6 byte-identity match.
// 中文: 由 custom_dictionary.db entry 的羅馬字推導 walker lattice-edge 比對 key(非 TL-shaped → None)。
// 中文: 必須與 edge provider 的 tl:{toneless} byte-identical → 重用同一組 shadow helper 同順序。
pub(crate) fn custom_toneless_key(roman: &str, is_poj: bool) -> Option<String> {
    let lower = roman.to_ascii_lowercase();
    let (canonical, _) = canonicalize_poj_shadow(&lower, is_poj);
    let (shadow, _) = build_hyphen_shadow(&canonical);
    let toneless = strip_ascii_tone_digits(&shadow);
    if toneless.is_empty() || !toneless.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    Some(format!("tl:{toneless}"))
}

/// Canonicalize POJ-display input (`pe̍h-ōe-jī`, `chóa`, `peⁿ`, `so͘`)
/// into ASCII TL spelling, paired with a byte-indexed map from canonical
/// byte offsets back to original `input` byte offsets. v3.5.8 Phase 9
/// Item 9.
///
/// Pure-ASCII handling is **mode-gated** on `is_poj`:
/// - `is_poj == false` (TL / non-POJ): passed through unchanged with an
///   identity offset map — the F3C gate from the pre-impl Codex consult
///   (2026-05-15), preserving every Item 8 hyphen-shadow contract pin.
///   Without it, `phonetics::normalize_to_tl`'s ASCII substitutions
///   (`ou→oo`, `oa→ua`, ...) would mis-rewrite real dictionary entries
///   like `tó-uī` (`dictionary/output/dictionary.csv:1984`,
///   `tl_notone=toui`) into `tooi` once the hyphen collapses the two
///   syllables together.
/// - `is_poj == true` (POJ mode): toneless ASCII POJ never carries the
///   non-ASCII tone diacritics that would otherwise route it through the
///   Phase 2 chain, so the identity fast-path would leave `chiah` /
///   `goa` / `che` keyed as `tl:chiah` (zero FST hits — the dictionary
///   stores `tl:tsiah`). For ASCII the Phase 1 NFD walk is identity, so
///   we build the identity offset map and run Phase 2 directly. The
///   `tó-uī`→`toui`→`tooi` ambiguity above re-applies here, but it is
///   the SAME pre-existing class the non-ASCII POJ path already has
///   ([`apply_normalize_to_tl_with_offsets`] runs unconditionally
///   there) and is far less severe than every ch-/oa-/oe- POJ word
///   returning zero candidates. Per-syllable POJ disambiguation is a
///   separate concern (Codex pre-impl Q3, 2026-05-20).
///
/// Non-ASCII inputs run the two-phase canonicalize:
///
/// Phase 1 — char-level NFD walk over the original input. Each NFD
/// scalar that is one of the 8 tone-mark combining codepoints in
/// `engine/phonetics/src/tables.rs::COMBINING_TO_TONE_NUM`
/// (`U+0300, U+0301, U+0302, U+0304, U+0306, U+030B, U+030C, U+030D`) is
/// dropped, with its UTF-8 byte width absorbed into the preceding base
/// char's `raw_end` so the offset map stays anchored at the right of
/// each consumed run. Other NFD scalars pass through unchanged
/// (including `\u{0358}` and `\u{207f}` / `\u{1d3a}`, which Phase 2
/// turns into ASCII).
///
/// Phase 2 — apply the [`phonetics::normalize_to_tl`] substitution
/// chain ([`phonetics::NORMALIZE_TO_TL_RULES`] is the single ordered
/// source consumed by both sides) with offset-aware substring replace.
/// All substitutions other than `o\u{0358}→oo`, `\u{207f}|\u{1d3a}→nn`,
/// and `oonn→onn` are byte-count-preserving so the offset map is
/// invariant; the three shrinking rules drain the dropped trailing
/// byte's map entry.
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
// 中文: POJ-display 輸入 → 純 ASCII 標準 TL 拼寫,並建立 canonical byte → raw byte 對照表。
pub(crate) fn canonicalize_poj_shadow(input: &str, is_poj: bool) -> (String, Vec<usize>) {
    if input.is_ascii() {
        // Identity offset map: Phase 1's NFD walk is a no-op for ASCII,
        // so every canonical byte maps straight back to its own raw
        // offset regardless of which branch we take below.
        let map: Vec<usize> = (0..=input.len()).collect();
        if is_poj {
            // POJ mode: fold POJ→TL spelling even for toneless ASCII so
            // `chiah`→`tsiah`, `goa`→`gua`, … reach the TL-keyed FST.
            return apply_normalize_to_tl_with_offsets(input.to_owned(), map);
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
    // Phase 2 substitutions (`ch→ts`, `oa→ua`, ...) actually match.
    // Non-ASCII bytes left over (`\u{0358}`, `\u{207f}`, `\u{1d3a}`) are
    // unaffected by `to_ascii_lowercase` and get replaced into ASCII by
    // Phase 2 below.
    let intermediate_lower = intermediate.to_ascii_lowercase();

    apply_normalize_to_tl_with_offsets(intermediate_lower, map)
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

/// Apply [`phonetics::NORMALIZE_TO_TL_RULES`] (the D2 single source the
/// `phonetics::normalize_to_tl` body also consumes) with offset-map
/// maintenance. Same order + same patterns — keeping the two in
/// lockstep is a hard prerequisite (Codex pre-impl note 2026-05-15);
/// post-D2 the rule list is the single source so divergence is no
/// longer possible by editing one side. The three shrinking rules
/// (`o\u{0358}→oo`, `\u{207f}|\u{1d3a}→nn`, `oonn→onn`) drain the
/// dropped trailing byte's map entry instead of producing a new
/// `String`.
// 中文: 套 phonetics::NORMALIZE_TO_TL_RULES 的代換鏈,同時維護 offset map;
// 中文: D2 後 rule list 為單一來源,兩端不會由單側修改而漂移。
pub(crate) fn apply_normalize_to_tl_with_offsets(
    s: String,
    map: Vec<usize>,
) -> (String, Vec<usize>) {
    let mut s = s;
    let mut map = map;
    for (find, repl) in phonetics::NORMALIZE_TO_TL_RULES {
        offset_aware_replace(&mut s, &mut map, find, repl);
    }
    (s, map)
}

/// Walk `s` left-to-right, replacing every occurrence of `find` with
/// `repl`, and update `map` so each post-replacement byte still points
/// at the correct original-input `raw_end`. Used by
/// [`apply_normalize_to_tl_with_offsets`].
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

/// v3.5.8 Phase 9 Item 10 — partial-prefix TL/POJ key builder. Runs
/// the same `lowercase → canonicalize_poj_shadow → build_hyphen_shadow
/// → strip_ascii_tone_digits` chain as
/// `dispatch::build_keys_tl_with_inventory` but **skips the
/// syllabifier** (the partial-prefix path is reached precisely because
/// `tl_syll::valid_span_endings` returned empty). Returns `None` when
/// the resulting toneless key is empty (raw was hyphen-only /
/// digit-only) so the caller can short-circuit without firing an
/// unbounded `tl:` prefix scan.
///
/// `consumed_span` is fixed to `(0, raw.len())` — partial-prefix
/// candidates always final-commit per Q15.4 (the offset maps from
/// Items 8 + 9 are intentionally discarded here because there is no
/// per-syllable mid-commit semantics to preserve).
// 中文: Item 10 — partial-prefix TL/POJ key 構造,沿用 Item 8/9 chain;不走 syllabifier。
// 中文:   consumed_span 固定 (0, raw.len()),配合 Q15.4 partial-prefix 一律 final-commit。
pub(crate) fn build_partial_prefix_key_tl(
    raw: &str,
    is_poj: bool,
) -> Option<(ConsumedSpan, String)> {
    if raw.is_empty() {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    let (canonical, _canonical_to_raw_end) = canonicalize_poj_shadow(&lower, is_poj);
    let (shadow, _shadow_to_canonical_end) = build_hyphen_shadow(&canonical);
    let toneless = strip_ascii_tone_digits(&shadow);
    if toneless.is_empty() {
        return None;
    }
    Some(((0u32, raw.len() as u32), format!("tl:{toneless}")))
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
            custom_toneless_key("tai5gi2", false).as_deref(),
            Some("tl:taigi")
        );
    }

    #[test]
    fn custom_toneless_key_numeric_with_hyphen_strips_both() {
        // Same `tl:taigi` key the edge provider builds for the `taigi`
        // span — proves the byte-identical-match contract.
        assert_eq!(
            custom_toneless_key("tai5-gi2", false).as_deref(),
            Some("tl:taigi")
        );
    }

    #[test]
    fn custom_toneless_key_poj_diacritic_is_canonicalized_first() {
        // Codex pre-impl S6 Q2 BLOCK: a POJ/diacritic custom roman must
        // fold to the same toneless key a numeric/typed `taigi` span
        // produces. A plain tone-digit strip would NOT do this — the
        // `canonicalize_poj_shadow` pass is load-bearing.
        assert_eq!(
            custom_toneless_key("tâi-gí", false).as_deref(),
            Some("tl:taigi"),
            "POJ diacritic must canonicalize → same key as numeric tai5gi2"
        );
        // POJ `oa`/`ou`-style display also canonicalizes (Item 9 chain).
        assert_eq!(
            custom_toneless_key("tâi-oân", false).as_deref(),
            custom_toneless_key("tai5uan5", false).as_deref(),
        );
    }

    #[test]
    fn custom_toneless_key_rejects_empty_and_non_tl_residue() {
        // Empty / whitespace / punctuation / CJK / digit-only custom
        // roman can never equal a syllabifier-built lattice edge key,
        // so it returns None and stays a span-local-only candidate.
        assert_eq!(custom_toneless_key("", false), None);
        assert_eq!(custom_toneless_key("   ", false), None);
        assert_eq!(custom_toneless_key("!!!", false), None);
        assert_eq!(custom_toneless_key("123", false), None);
        assert_eq!(custom_toneless_key("台語", false), None);
    }

    #[test]
    fn custom_toneless_key_poj_ascii_matches_walker_edge_key() {
        // S6 byte-identity: a custom-dict roman `chiah` keyed under the
        // SAME is_poj the walker edge provider uses must equal the
        // lattice edge key `tl:tsiah` (split-brain would drop the
        // custom match in POJ mode).
        assert_eq!(
            custom_toneless_key("chiah", true).as_deref(),
            Some("tl:tsiah"),
        );
        // TL mode keeps the F3C identity (un-canonicalized).
        assert_eq!(
            custom_toneless_key("chiah", false).as_deref(),
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
        let (canonical, map) = canonicalize_poj_shadow("tai-bak", false);
        assert_eq!(canonical, "tai-bak");
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn canonicalize_poj_shadow_empty_ascii_is_identity() {
        let (canonical, map) = canonicalize_poj_shadow("", false);
        assert_eq!(canonical, "");
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn canonicalize_poj_shadow_combining_tone_mark_drops_and_absorbs() {
        // `pe\u{030d}h` (POJ `pe̍h` for 白): combining tone-8 mark on
        // `e`; canonical drops it and `e`'s raw_end inherits the 2
        // bytes the mark would have consumed.
        let (canonical, map) = canonicalize_poj_shadow("pe\u{030d}h", false);
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
        let (canonical, map) = canonicalize_poj_shadow("so\u{0358}", false);
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
        let (canonical, map) = canonicalize_poj_shadow("\u{030d}h", false);
        assert_eq!(canonical, "h");
        assert_eq!(map, vec![0, 3]);
    }

    #[test]
    fn canonicalize_poj_shadow_superscript_nasal_marker_emits_nn() {
        // `pe\u{207f}` (POJ `peⁿ`): superscript-n collapses to `nn`.
        let (canonical, map) = canonicalize_poj_shadow("pe\u{207f}", false);
        assert_eq!(canonical, "penn");
        // `p` → 1, `e` → 2, both new `n` bytes anchor at raw_end 5
        // (after `\u{207f}`).
        assert_eq!(map, vec![0, 1, 2, 5, 5]);
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ch_initial_substitutes_to_ts() {
        // Non-ASCII path with `ch` initial: `chóa` (POJ `tsuá`) →
        // canonical `tsua` with the tone-2 acute dropped.
        let (canonical, _map) = canonicalize_poj_shadow("ch\u{f3}a", false);
        assert_eq!(canonical, "tsua");
    }

    #[test]
    fn canonicalize_poj_shadow_precomposed_uppercase_lowercases_via_phase2() {
        // `\u{00d3}` (`Ó`, precomposed UPPERCASE) → NFD `O\u{0301}` →
        // drop combining → `O` (uppercase) → Phase 2 lowercase pass
        // makes it `o`. This pins the ordering: NFD walk must come
        // BEFORE the lowercase pass, otherwise uppercase precomposed
        // diacritic chars would survive into Phase 2 substitutions.
        let (out, _map) = canonicalize_poj_shadow("\u{00d3}a", false);
        assert_eq!(out, "ua", "{out:?}");
    }

    #[test]
    fn canonicalize_poj_shadow_oonn_shrinks_with_offset_drain() {
        // Synthetic regression: simulate Phase 1 emitting `oonn`
        // (e.g. via `o\u{0358}\u{207f}` upstream). Phase 2 `oonn→onn`
        // is the only 4→3 shrinking rule and must drain exactly one
        // map entry. Input `o\u{0358}\u{207f}` itself: `o` (1) +
        // `\u{0358}` (2) + `\u{207f}` (3) = 6 bytes.
        let (out, map) = canonicalize_poj_shadow("o\u{0358}\u{207f}", false);
        assert_eq!(out, "onn", "{out:?}");
        // After `oonn→onn` collapse, the final byte's raw_end must
        // equal the full input length (6).
        assert_eq!(*map.last().unwrap(), 6, "{map:?}");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_chiah_folds_ch_to_ts() {
        // PR #300 root cause: in POJ mode, toneless ASCII POJ must run
        // Phase 2 — `chiah` → `tsiah` so the FST `tl:tsiah` lookup hits.
        // TL mode keeps the F3C identity (`chiah` → `chiah`).
        let (poj, map) = canonicalize_poj_shadow("chiah", true);
        assert_eq!(poj, "tsiah");
        // ASCII identity map (Phase 1 NFD is identity for ASCII;
        // `ch→ts` is byte-length-preserving so the map is invariant).
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5]);
        let (tl, _) = canonicalize_poj_shadow("chiah", false);
        assert_eq!(tl, "chiah");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_chhia_oa_oe_eng_ek_fold() {
        // All the equal-length POJ→TL ASCII substitutions fire under
        // is_poj: chh→tsh, oa→ua, oe→ue, eng→ing, ek→ik.
        for (input, expected) in [
            ("chhia", "tshia"), // 車
            ("goa", "gua"),     // 我  oa→ua
            ("hoe", "hue"),     // oe→ue
            ("peng", "ping"),   // eng→ing
            ("tek", "tik"),     // ek→ik
        ] {
            let (out, _) = canonicalize_poj_shadow(input, true);
            assert_eq!(out, expected, "is_poj `{input}` → `{expected}`");
        }
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_oonn_shrinks_with_offset_drain() {
        // The only ASCII-reachable shrinking rule (`oonn→onn`) must
        // still drain exactly one map entry under the POJ ASCII path.
        let (out, map) = canonicalize_poj_shadow("oonn", true);
        assert_eq!(out, "onn", "{out:?}");
        assert_eq!(
            *map.last().unwrap(),
            "oonn".len(),
            "shrunk final byte inherits the full source raw_end: {map:?}",
        );
    }

    #[test]
    fn canonicalize_poj_shadow_tl_ascii_chiah_stays_identity_f3c_guard() {
        // Regression guard for the F3C gate: the SAME ASCII input in TL
        // mode (is_poj = false) must NOT be rewritten, so the `tó-uī`
        // (`toui`) class of real TL entries is never garbled.
        let (out, map) = canonicalize_poj_shadow("chiah", false);
        assert_eq!(out, "chiah", "TL-mode ASCII must stay identity");
        assert_eq!(map, (0..="chiah".len()).collect::<Vec<_>>());
        // `toui` (佗位) must survive — `ou→oo` must NOT fire in TL mode.
        let (toui, _) = canonicalize_poj_shadow("toui", false);
        assert_eq!(toui, "toui", "F3C: TL-mode `toui` must not become `tooi`");
    }

    #[test]
    fn canonicalize_poj_shadow_non_ascii_is_mode_independent() {
        // Non-ASCII POJ-display input has always run Phase 2; mode
        // gating only changes the ASCII path. `chóa` canonicalizes to
        // `tsua` in BOTH modes (Phase 1 strips tone-2, then Phase 2
        // runs `ch→ts`, `oa→ua` regardless of is_poj for non-ASCII
        // inputs).
        let (poj, _) = canonicalize_poj_shadow("ch\u{f3}a", true);
        let (tl, _) = canonicalize_poj_shadow("ch\u{f3}a", false);
        assert_eq!(poj, "tsua");
        assert_eq!(tl, "tsua");
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
    fn build_partial_prefix_key_tl_passes_ascii_through() {
        let (span, key) = build_partial_prefix_key_tl("gu", false).unwrap();
        // partial-prefix candidates always final-commit (Q15.4) →
        // consumed_span covers the whole pending tail.
        assert_eq!(span, (0u32, 2u32));
        assert_eq!(key, "tl:gu");
    }

    #[test]
    fn build_partial_prefix_key_tl_strips_tone_digits_and_lowercases() {
        // The digit half of `notone.py::remove_tone` still applies on
        // the partial-prefix path so `gu5` and `gu` produce the same
        // FST prefix key.
        let (_, key) = build_partial_prefix_key_tl("GU5", false).unwrap();
        assert_eq!(key, "tl:gu");
    }

    #[test]
    fn build_partial_prefix_key_tl_strips_internal_hyphen_via_item8_shadow() {
        // Item 8's hyphen-shadow chain runs on the partial-prefix path
        // too — `tai-` collapses to `tai` (trailing `-` stays in the
        // pending raw buffer per build_hyphen_shadow contract), and
        // `-tai` collapses to `tai`.
        let (_, key) = build_partial_prefix_key_tl("tai-", false).unwrap();
        assert_eq!(key, "tl:tai");
        let (_, key) = build_partial_prefix_key_tl("-tai", false).unwrap();
        assert_eq!(key, "tl:tai");
    }

    #[test]
    fn build_partial_prefix_key_tl_canonicalizes_poj_diacritic_via_item9() {
        // Item 9's canonicalize chain runs on partial-prefix input too
        // — `pe\u{030d}` (POJ `pe̍h` minus the trailing `h`) folds to
        // `pe` after the tone-mark drop, giving FST key `tl:pe`.
        let (_, key) = build_partial_prefix_key_tl("pe\u{030d}", false).unwrap();
        assert_eq!(key, "tl:pe");
    }

    #[test]
    fn build_partial_prefix_key_tl_returns_none_for_empty_after_strip() {
        // Hyphen-only or digit-only raw produces an empty toneless
        // key — return None so the caller skips the FST scan.
        assert!(build_partial_prefix_key_tl("", false).is_none());
        assert!(build_partial_prefix_key_tl("-", false).is_none());
        assert!(build_partial_prefix_key_tl("--", false).is_none());
        assert!(build_partial_prefix_key_tl("5", false).is_none());
        assert!(build_partial_prefix_key_tl("-5-", false).is_none());
    }

    #[test]
    fn build_partial_prefix_key_tl_poj_ascii_canonicalizes() {
        // Partial-prefix path (syllabifier yielded no ending) must also
        // fold POJ ASCII so a half-typed `chi` keys `tl:tsi`, not the
        // dead `tl:chi`.
        let (_, key) = build_partial_prefix_key_tl("chi", true).unwrap();
        assert_eq!(key, "tl:tsi");
        let (_, tl_key) = build_partial_prefix_key_tl("chi", false).unwrap();
        assert_eq!(tl_key, "tl:chi", "TL mode keeps F3C identity");
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
                keys.push(canonical);
            } else {
                keys.push(format!("{canonical}{tone}"));
                keys.push(canonical);
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

    #[test]
    fn greedy_longest_syllabification_taiuantai_reads_tai_uan_tai() {
        // The documented no-dict carve-out expectation: `taiuantai`
        // (no dict hit anywhere) must read as the canonical
        // longest-syllable segmentation `tai uan tai`, NOT the
        // sub-syllable over-split `ta i u an ta i` (literal
        // max-syllable-count) and NOT the min-cost fewest-edge blob.
        let inv = build_inventory(&["tai1", "uan1", "ta1", "i1", "u1", "an1"]);
        let segs = greedy_longest_syllabification("taiuantai", &inv)
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
        assert!(greedy_longest_syllabification("taix", &inv).is_none());
    }

    // ----- v3.5.8 OOV-cost fix — span_min_syllable_count
    //       (Codex PR #290 P1 r3255035136) -----

    #[test]
    fn span_min_syllable_count_simple_spans() {
        let inv = build_inventory(&["tai1", "uan1", "ta1"]);
        // Whole span is itself one valid syllable → 1 (correct, not a
        // collapse).
        assert_eq!(span_min_syllable_count("tai", &inv), Some(1));
        // `taiuanta` = tai|uan|ta → 3 (the synth syllable-sum metadata).
        assert_eq!(span_min_syllable_count("taiuanta", &inv), Some(3));
        // Not single-syllable-reachable → None (caller fail-closes).
        assert_eq!(span_min_syllable_count("taix", &inv), None);
        assert_eq!(span_min_syllable_count("", &inv), None);
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
            greedy_longest_syllabification("tania", &inv).is_none(),
            "precondition: greedy-longest must dead-end on this span"
        );
        assert_eq!(
            span_min_syllable_count("tania", &inv),
            Some(2),
            "min-hop walk must recover the real 2-syllable count, not collapse to 1"
        );
    }
}
