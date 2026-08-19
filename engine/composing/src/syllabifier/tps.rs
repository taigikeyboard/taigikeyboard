//! TPS syllabifier — BFS over a `SyllableInventory` returning every
//! span ending reachable from `pos` by a chain of 1..=`max_syllables`
//! valid TPS Bopomofo syllables.
//!
//! Mirrors the TL syllabifier shape (`syllabifier::tl::valid_span_endings_lowered`)
//! exactly. The pre-fix structural pre-scan (next-initial-seen rule)
//! could not split toneless `ㄉㄞ|ㄨㄢ` because the medial vowel `ㄨ`
//! is not a TPS initial, so no boundary was proposed between the two
//! syllables — and inventory probes of the fused 2-syllable slice
//! `tps:ㄉㄞㄨㄢ` always missed. The inv-driven BFS here probes every
//! byte position `(cur+1)..=cur+MAX_SYLLABLE_BYTES_TPS`, accepts any
//! slice the `tps:` family of `syllables.fst` recognises, and lets
//! `SyllableInventory::contains_in(Tps, …)` itself act as the
//! garbage-Bopomofo filter (a malformed span has no inventory hit and
//! produces no edge).
//!
//! Direction-first alignment per CLAUDE.md Core Principle #6 + Codex
//! pre-impl 2026-05-27: TL, POJ, TPS now share one segmenter shape,
//! matching librime DAG (`references/librime/src/rime/algo/syllabifier.cc`),
//! khiin-rs DP over known words
//! (`references/khiin-rs/khiin/src/data/segmenter.rs`), and McBopomofo
//! ReadingGrid unigram-backed spans
//! (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp`).

// 中文: TPS 音節切分器 — 對 SyllableInventory 跑 inv-driven BFS,回報深度 ≤ max_syllables 鏈可達的所有 span ending。
// 中文: 修復前的 structural pre-scan (next-initial-seen) 無法切 ㄉㄞ|ㄨㄢ — ㄨ 是介音非聲母,無切點建議,
// 中文:   inv probe 撞到融合 2 音節的 tps:ㄉㄞㄨㄢ 永遠 miss。改成 inv-driven BFS 直接以
// 中文:   contains_in(Tps, slice) 作邊界判定;與 TL syllabifier 同型,亦對齊 librime / khiin-rs / McBopomofo。

use std::collections::{BTreeSet, VecDeque};

use lexicon::SyllableInventory;
use phonetics::InputMode;

/// Upper bound on a single TPS syllable's byte length. Bopomofo
/// (U+3100..U+312F) + Bopomofo Extended (U+31A0..U+31BF) chars are 3
/// bytes each in UTF-8; tone-mark spacing modifiers
/// (U+02C6/02C7/02CA/02CB/02D9/02EA/02EB) + combining dot (U+0307) are
/// 2 bytes each. Worst-case syllable from `engine/phonetics/src/tps.rs`
/// (`ZHUYIN_INITIALS` + `ZHUYIN_VOWELS` + `ZHUYIN_TONES`): compound
/// initial 2 chars × 3 + vowel chain 3 chars × 3 + entering coda 1 ×
/// 3 + tone-8 dot 1 × 2 = 20 bytes. Set to 24 for a small headroom and
/// to match the order-of-magnitude of TL's tight `MAX_SYLLABLE_BYTES`.
// 中文: TPS 單音節最大 byte 上限 — Bopomofo / 擴展區 = 3 bytes,聲調符 = 2 bytes;
// 中文:   結構最大 20 bytes;設 24 留小幅 headroom,與 TL 的 10 同量級。
const MAX_SYLLABLE_BYTES_TPS: usize = 24;

/// Return every byte offset `e > pos` reachable from `pos` by a chain
/// of 1..=`max_syllables` syllables, where each chain link
/// `lowered[cur..end]` is a member of the `tps:` family branch of the
/// v3.5.9 B-1 tagged-single-FST syllable inventory.
///
/// Contract:
/// - Returns ascending, deduplicated byte offsets.
/// - Returns empty `Vec` when `pos >= lowered.len()`, `max_syllables
///   == 0`, or `pos` is not on a UTF-8 char boundary.
/// - `_mode` is taken for signature parity with the TL syllabifier so
///   the [`crate::syllabifier::valid_span_endings_lowered`] dispatcher
///   can forward to either by `InputMode`; this function always probes
///   the `tps:` family regardless of `_mode` (the dispatcher routes
///   only TPS callers here).
/// - `is_false_toneless_boundary_tps` suppresses a toneless ending
///   that sits immediately before a TPS tone mark, mirroring TL's
///   `is_false_toneless_boundary` so the longer numeric-tone form
///   (`tps:ㄉㄞˊ`) wins over the shorter toneless form when the user
///   typed a tone mark.
/// - Tone-8 dot `U+02D9` (encode-safe modifier-letter dot typed by
///   the platform keyboards per `engine/lexicon/src/key_normalizer.rs`)
///   is substituted to the canonical combining `U+0307` before the
///   BFS, so the inventory probe hits the build-pipeline-emitted form
///   (`dictionary/build/merge_csv.py` writes `U+0307`). Both code
///   points are 2 bytes UTF-8 so the substitution is byte-length
///   preserving — returned offsets index the original `lowered`
///   unchanged.
///
/// Algorithm: FIFO BFS using `endings` itself as the visited set —
/// `BTreeSet::insert` returns `true` only on first arrival, which
/// under unit edge costs is also the minimum depth. Time:
/// O(n × MAX_SYLLABLE_BYTES_TPS) FST lookups, each O(syllable_len).
// 中文: 從 pos 出發,以 1..=max_syllables 條 tps: 家族音節鏈走訪,回傳所有可達 byte 位移 (遞增去重)。
// 中文: false-toneless guard 壓掉「短的 toneless edge」當下一字是 TPS 聲調符 (含 U+02D9 / U+0307),
// 中文:   讓 numeric-tone 長形 (例 tps:ㄉㄞˊ) 勝出,對齊 TL 的 is_false_toneless_boundary 行為。
// 中文: BFS 前把鍵盤端 U+02D9 (encode-safe 第 8 聲點) 取代為 build pipeline 使用的 U+0307 (組合形),
// 中文:   兩者皆 2 bytes UTF-8 → byte 偏移不變,回傳值仍以原始 lowered 為座標。
pub(crate) fn valid_span_endings_lowered(
    lowered: &str,
    pos: usize,
    inv: &SyllableInventory,
    _mode: InputMode,
    max_syllables: usize,
    barriers: &[usize],
) -> Vec<usize> {
    if max_syllables == 0 || pos >= lowered.len() || !lowered.is_char_boundary(pos) {
        return Vec::new();
    }

    // U+02D9 → U+0307 byte-length-preserving substitution for inv-probe
    // canonicalization. Allocate only when the buffer contains U+02D9;
    // common TPS input via Bopomofo tone marks (ˋ ˊ ˫ ˪ ˇ ˆ) is unaffected.
    let normalized: Option<String> = if lowered.contains('\u{02D9}') {
        Some(lowered.replace('\u{02D9}', "\u{0307}"))
    } else {
        None
    };
    let probe = normalized.as_deref().unwrap_or(lowered);

    let mut endings: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    queue.push_back((pos, 0));

    while let Some((cur, depth)) = queue.pop_front() {
        if depth >= max_syllables {
            continue;
        }
        let upper = (cur + MAX_SYLLABLE_BYTES_TPS).min(probe.len());
        for end in (cur + 1)..=upper {
            if !probe.is_char_boundary(end) {
                continue;
            }
            // §35 barrier contract, part (a): a stripped separator / 連字
            // is a mandatory syllable cut — no SINGLE syllable may cross
            // it (`cur < barrier < end`). Chains may still span it link
            // by link, which is exactly the §31 soft-separator behavior
            // (`ㄍㄠ`␣`ㄉㄞ` → 交代 via two links meeting AT the barrier).
            // 中文: §35 barrier (a) — 分隔符為強制切點,單一音節不可跨越(鏈可逐節跨,
            // 中文:   即 §31 軟分隔符行為:交代 = 兩節在 barrier 相接)。
            if barriers.iter().any(|&b| cur < b && b < end) {
                continue;
            }
            // §35 ambiguity expansion: the probe accepts any reading of
            // the span under the TPS ambiguity families. Part (b) of the
            // barrier contract: a syllable ENDING at a barrier has its
            // last glyph restricted to Final-role readings.
            // 中文: §35 展開探測 — span 任一讀法命中即成 edge;結束於 barrier 的音節,
            // 中文:   末 glyph 只許 Final 形(契約 (b))。
            let final_only = span_final_only_offsets(probe, cur, end, barriers);
            if inv.contains_in_tps_readings(&probe[cur..end], &final_only)
                && !is_false_toneless_boundary_tps(probe, end)
                && endings.insert(end)
            {
                queue.push_back((end, depth + 1));
            }
        }
    }

    endings.into_iter().collect()
}

/// True when the inventory-accepted toneless TPS syllable at `..end`
/// sits immediately before a TPS tone-mark char — the longer numeric
/// form (e.g. `tps:ㄉㄞˊ` for tone-5) is the correct match and the
/// FST contains both. Reuses the canonical tone-mark set from
/// `phonetics::is_tps_tone_mark` so the eight scalars
/// (ˆˇˊˋ˙˪˫ + combining dot above) stay in one source of truth.
///
/// Stop codas (ㆴㆵㆻㆷ U+31B4/B5/BB/B7) are syllable body, not tone
/// marks; tone-4 stops carry no trailing mark and tone-8 stops carry
/// a trailing dot. Per `dictionary/common/notone.py:34-45 remove_tps_tone`,
/// stop codas are NOT stripped — they stay in `tps_notone` keys. So
/// they are deliberately absent from the guard set; the inv probe of
/// the longer slice that ends ON the coda is the correct match.
// 中文: 把「toneless TPS 音節邊界後接聲調符」判為假邊界 — 較長的 numeric-tone 形是正解,
// 中文:   FST 兩形都收。直接重用 phonetics::is_tps_tone_mark (含 U+02D9 + U+0307);
// 中文:   入聲韻尾 ㆴㆵㆻㆷ 屬音節主體,不在 guard 內。
/// Byte offsets (relative to the span `probe[cur..end]`) of glyphs whose
/// pattern slot must be Final-only: the span's LAST glyph when the span
/// ends exactly at a barrier or at a trailing barrier (= end of shadow
/// where a separator was stripped). Interior glyphs are never
/// barrier-adjacent here because part (a) already refuses crossing
/// spans.
// 中文: span 內須限 Final 形的 glyph 偏移(相對 span):span 恰結束於 barrier 時的末 glyph。
// 中文:   內部 glyph 不會鄰接 barrier(契約 (a) 已拒絕跨越)。
fn span_final_only_offsets(probe: &str, cur: usize, end: usize, barriers: &[usize]) -> Vec<usize> {
    if !barriers.contains(&end) {
        return Vec::new();
    }
    // Find the last char's start offset within the span.
    let span = &probe[cur..end];
    match span.char_indices().last() {
        Some((last_start, _)) => vec![last_start],
        None => Vec::new(),
    }
}

fn is_false_toneless_boundary_tps(lowered: &str, end: usize) -> bool {
    lowered[end..]
        .chars()
        .next()
        .is_some_and(phonetics::is_tps_tone_mark)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fst::SetBuilder;
    use lexicon::SyllableInventory;
    use phonetics::InputMode;

    use super::valid_span_endings_lowered;

    const MAX_SYLLABLES: usize = 8;

    /// Hermetic `tps:` family inventory builder. Mirrors the pattern in
    /// `composing::tests::build_keys_tps` — keys carry the `tps:`
    /// prefix so `SyllableInventory::contains_in(Tps, ..)` finds them.
    /// Each input syllable is recorded in BOTH toneless and the supplied
    /// optional tone-marked form to mirror the build pipeline's
    /// dual-emit (`dictionary/build/create_syllables_fst.py` §toneless
    /// + §numeric).
    fn build_tps_inventory(samples: &[(&str, Option<&str>)]) -> SyllableInventory {
        let mut keys: Vec<String> = Vec::new();
        for &(toneless, numeric) in samples {
            keys.push(format!("tps:{toneless}"));
            if let Some(n) = numeric {
                keys.push(format!("tps:{n}"));
            }
        }
        keys.sort();
        keys.dedup();

        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf =
            std::env::temp_dir().join(format!("taigi_tps_syl_unit_{}_{n}.fst", std::process::id()));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in &keys {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
        SyllableInventory::open(&path).expect("open inventory")
    }

    #[test]
    fn medial_led_second_syllable_splits_via_inventory() {
        // ㄉㄞ|ㄨㄢ — toneless 台灣. `ㄨ` is a medial vowel, not a TPS
        // initial; the pre-fix structural scanner could not propose a
        // boundary between ㄉㄞ and ㄨㄢ. The inv-driven BFS does, because
        // both `tps:ㄉㄞ` and `tps:ㄨㄢ` are in the inventory.
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", None), ("\u{3128}\u{3122}", None)]);
        let dai_len = "\u{3109}\u{311e}".len();
        let input = "\u{3109}\u{311e}\u{3128}\u{3122}";
        let endings =
            valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert_eq!(endings, vec![dai_len, input.len()]);
    }

    #[test]
    fn user_bug_taiuantaigi_full_chain_reachable() {
        // ㄉㄞㄨㄢㄉㄞㆣㄧ — exact reported bug input. Every single-
        // syllable hit + every chain end must be reachable.
        let inv = build_tps_inventory(&[
            ("\u{3109}\u{311e}", None), // ㄉㄞ
            ("\u{3128}\u{3122}", None), // ㄨㄢ
            ("\u{31a3}\u{3127}", None), // ㆣㄧ
        ]);
        let dai = "\u{3109}\u{311e}";
        let uan = "\u{3128}\u{3122}";
        let gi = "\u{31a3}\u{3127}";
        let input = format!("{dai}{uan}{dai}{gi}");
        let endings =
            valid_span_endings_lowered(&input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        let e1 = dai.len();
        let e2 = dai.len() + uan.len();
        let e3 = dai.len() + uan.len() + dai.len();
        assert_eq!(endings, vec![e1, e2, e3, input.len()]);
    }

    #[test]
    fn false_toneless_boundary_before_tone_mark_suppresses_short_form() {
        // ㄉㄞ + ˊ (tone 5). FST has BOTH the toneless `tps:ㄉㄞ` and
        // the numeric `tps:ㄉㄞˊ`. Without the guard, BFS would accept
        // (0, 6) (toneless, leaving orphan ˊ) AND (0, 8) (numeric).
        // The guard suppresses the toneless ending so commit semantics
        // never leave a dangling tone mark.
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", Some("\u{3109}\u{311e}\u{02ca}"))]);
        let input = "\u{3109}\u{311e}\u{02ca}";
        let endings =
            valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert_eq!(endings, vec![input.len()]);
    }

    #[test]
    fn false_toneless_boundary_before_tone8_dot_combining_suppresses() {
        // ㄎㄚㆴ̇ — tone-8 stop with combining U+0307 dot. FST has both
        // the toneless `tps:ㄎㄚㆴ` (tone-4) and the tone-8 numeric
        // `tps:ㄎㄚㆴ̇`. Guard must suppress the toneless ending so an
        // orphan U+0307 cannot be commit-stranded.
        let inv = build_tps_inventory(&[(
            "\u{310e}\u{311a}\u{31b4}",
            Some("\u{310e}\u{311a}\u{31b4}\u{0307}"),
        )]);
        let input = "\u{310e}\u{311a}\u{31b4}\u{0307}";
        let endings =
            valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert_eq!(endings, vec![input.len()]);
    }

    #[test]
    fn encode_safe_dot_u02d9_normalizes_to_u0307_for_inv_probe() {
        // ㄎㄚㆴ˙ — tone-8 stop with the encode-safe U+02D9 modifier-
        // letter dot that platform keyboards (iOS `TaigiLayouts.swift`,
        // Android `tps.json`) emit. The build pipeline writes the
        // canonical combining U+0307 into `dictionary.fst` /
        // `syllables.fst` (`dictionary/build/merge_csv.py`). The
        // syllabifier substitutes U+02D9 → U+0307 internally so the
        // inv probe hits the canonical numeric form, while the
        // false-toneless guard fires on either dot variant (both in
        // `phonetics::is_tps_tone_mark`). Result: ONLY the full
        // tone-8 ending — no orphan-dot short ending, and the full
        // numeric form is not silently dropped.
        let inv = build_tps_inventory(&[(
            "\u{310e}\u{311a}\u{31b4}",
            Some("\u{310e}\u{311a}\u{31b4}\u{0307}"),
        )]);
        let input = "\u{310e}\u{311a}\u{31b4}\u{02d9}";
        let endings =
            valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert_eq!(endings, vec![input.len()]);
    }

    #[test]
    fn entering_coda_alone_stays_a_valid_ending() {
        // ㄎㄚㆴ — tone-4 stop, no trailing dot. The coda IS the body;
        // guard MUST NOT suppress this — there is no tone mark after.
        let inv = build_tps_inventory(&[("\u{310e}\u{311a}\u{31b4}", None)]);
        let input = "\u{310e}\u{311a}\u{31b4}";
        let endings =
            valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert_eq!(endings, vec![input.len()]);
    }

    #[test]
    fn max_syllables_caps_bfs_depth() {
        // Chain of 3 single-syllable inv hits but max_syllables = 2 →
        // only the 1- and 2-syllable endings reachable.
        let inv = build_tps_inventory(&[
            ("\u{3109}\u{311e}", None), // ㄉㄞ
            ("\u{3128}\u{3122}", None), // ㄨㄢ
            ("\u{31a3}\u{3127}", None), // ㆣㄧ
        ]);
        let input = "\u{3109}\u{311e}\u{3128}\u{3122}\u{31a3}\u{3127}";
        let endings = valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, 2, &[]);
        let e1 = "\u{3109}\u{311e}".len();
        let e2 = "\u{3109}\u{311e}\u{3128}\u{3122}".len();
        assert_eq!(endings, vec![e1, e2]);
    }

    #[test]
    fn non_bopomofo_suffix_does_not_extend_chain_past_inv_hit() {
        // ㄉㄞXY — the prefix ㄉㄞ (3+3 bytes) hits `tps:ㄉㄞ`; the XY
        // suffix is non-Bopomofo and inv-absent, so BFS finds no edge
        // continuing from end=6. inv-only filtering is sufficient — no
        // structural garbage-rejection filter needed.
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", None)]);
        let input = "\u{3109}\u{311e}XY";
        let endings =
            valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert_eq!(endings, vec!["\u{3109}\u{311e}".len()]);
    }

    #[test]
    fn empty_input_returns_empty() {
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", None)]);
        let endings = valid_span_endings_lowered("", 0, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert!(endings.is_empty());
    }

    #[test]
    fn pos_at_input_end_returns_empty() {
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", None)]);
        let input = "\u{3109}\u{311e}";
        let endings = valid_span_endings_lowered(
            input,
            input.len(),
            &inv,
            InputMode::Tps,
            MAX_SYLLABLES,
            &[],
        );
        assert!(endings.is_empty());
    }

    #[test]
    fn pos_at_non_char_boundary_returns_empty_safely() {
        // ㄉ is 3 bytes; pos=1 is mid-codepoint. Must not panic.
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", None)]);
        let input = "\u{3109}\u{311e}";
        let endings =
            valid_span_endings_lowered(input, 1, &inv, InputMode::Tps, MAX_SYLLABLES, &[]);
        assert!(endings.is_empty());
    }

    #[test]
    fn max_syllables_zero_returns_empty() {
        let inv = build_tps_inventory(&[("\u{3109}\u{311e}", None)]);
        let input = "\u{3109}\u{311e}";
        let endings = valid_span_endings_lowered(input, 0, &inv, InputMode::Tps, 0, &[]);
        assert!(endings.is_empty());
    }
}
