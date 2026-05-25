//! TPS syllabifier — O(n) scan returning every byte offset that ends a
//! TPS syllable.
//!
//! v3.5.9 D / C-3b — gained an `_lowered` variant matching the TL
//! syllabifier signature (`inv`, `mode`, `max_syllables`) so the
//! [`crate::syllabifier::valid_span_endings_lowered`] dispatcher can
//! route TPS through the same lattice builder. The inventory gate is
//! load-bearing per Codex pre-impl Fork 3 amendment: each candidate
//! ending is verified via `inv.contains_in(InputMode::Tps, slice)` so
//! a malformed TPS span (residue, unknown Bopomofo combination) cannot
//! leak into the walker's no-dict carve-out as a synth roman.
//! Two boundary kinds:
//!
//! 1. **Explicit terminator** — a tone mark (tone 2/3/5/6/7/9), an
//!    entering-coda small letter (tone 4, optionally coalesced with a
//!    following tone-8 dot), or a standalone tone-8 dot. These are
//!    unambiguous in TPS (per `docs/releases/v3.5.8/plan.md` § 走查範例 Case D — TPS).
//! 2. **Implicit tone-1 boundary** ("next initial seen" rule, v3.5.8
//!    Phase 9 Item 7) — tone-1 syllables carry no mark, so a boundary
//!    is inferred when a new initial consonant appears after a nucleus
//!    has already been consumed, and at end-of-input when a tone-1
//!    nucleus is still pending. This is implemented HERE in the
//!    syllabifier, not in the dispatcher.
//!
//! Without rule 2, a pure tone-1 buffer such as `ㄉㄞㆣㄧ` ("tâi-gí")
//! yields zero terminators → no FST keys → empty candidate strip.

// 中文: TPS 音節切分器 — 線性掃描,回報每個音節結束的 byte 位移,含兩種邊界:
// 中文: (1) 顯式終止符:聲調符號 / 入聲韻尾 / 第 8 聲點;
// 中文: (2) 隱式第 1 聲邊界 (next-initial-seen,v3.5.8 Phase 9 Item 7):第 1 聲無調號,
// 中文:     在已吃過韻核後又見到新聲母、或輸入結尾仍有未結的第 1 聲時補回邊界。規則實作於本切分器,非 dispatcher。

/// Roles a TPS character can play in syllable termination. Sourced
/// from `engine/phonetics/src/tps.rs:75-111` (`ZHUYIN_TONES` and
/// `ZHUYIN_TONES_ENCODE_SAFE`).
// 中文: TPS 字符的終止角色;對應 phonetics::tps 的 ZHUYIN_TONES 與 ZHUYIN_TONES_ENCODE_SAFE 兩張表。
enum Terminator {
    /// Spacing modifier tone marks (tone 2/3/5/6/7/9) — each marks the
    /// end of one syllable. Tone 1 is implicit (no mark) and excluded.
    Mark,
    /// Bopomofo Extended entering-coda small letter (p/t/k/h stop).
    /// Stands alone as tone 4; may be coalesced with a following dot
    /// for tone 8.
    EnteringCoda,
    /// 8th-tone dot — `U+0307` (NFD combining) or `U+02D9` (encode-safe
    /// spacing variant). Treated as a standalone terminator because the
    /// encode-safe table at `tps.rs:109` lists it as a complete tone-8
    /// entry on its own; ignoring a stray dot mid-stream would silently
    /// drop a syllable boundary.
    Tone8Dot,
}

fn classify(ch: char) -> Option<Terminator> {
    match ch {
        '\u{02cb}'  // tone 2 ˋ
        | '\u{02ea}'  // tone 3 ˪
        | '\u{02ca}'  // tone 5 ˊ
        | '\u{02c7}'  // tone 6 ˇ
        | '\u{02eb}'  // tone 7 ˫
        | '\u{02c6}'  // tone 9 ˆ
            => Some(Terminator::Mark),
        '\u{31b4}'  // ㆴ p
        | '\u{31b5}'  // ㆵ t
        | '\u{31bb}'  // ㆻ k
        | '\u{31b7}'  // ㆷ h
            => Some(Terminator::EnteringCoda),
        '\u{0307}'  // combining dot above (NFD)
        | '\u{02d9}'  // ˙ modifier letter dot above (encode-safe)
            => Some(Terminator::Tone8Dot),
        _ => None,
    }
}

/// Return every byte offset `e > pos` such that `input[..e]` ends a
/// TPS syllable, covering both explicit terminators and implicit
/// tone-1 boundaries (module-level rules 1 and 2).
///
/// - **Explicit terminator**: an entering-coda small letter followed
///   immediately by an 8th-tone dot terminates AT the dot (one ending,
///   not two); a stray dot terminates standalone.
/// - **Implicit tone-1** ("next initial seen"): once a nucleus has
///   been consumed, the next initial consonant starts a new syllable,
///   so the previous tone-1 syllable ends at the byte offset *before*
///   that initial. End-of-input with a pending tone-1 nucleus ends the
///   trailing syllable at `input.len()`. An explicit terminator clears
///   the pending nucleus, so an initial immediately after a tone mark
///   is not a false boundary.
///
/// "Nucleus" is any Bopomofo-range char ([`phonetics::is_tps_char`])
/// that is neither an initial nor a terminator — tolerant of medials
/// and extended vowels without enumerating the vowel table. Non-TPS
/// chars (Latin, punctuation, space) never set the nucleus flag.
///
/// Contract:
/// - Returns strictly ascending byte offsets. Pushes are monotonic by
///   construction (terminator/EOI offsets only grow; an implicit push
///   needs an intervening nucleus that advances the cursor), so no
///   separate dedup is needed.
/// - Returns empty `Vec` when `pos >= input.len()` or `pos` is not on
///   a UTF-8 char boundary. Scan state is fresh from `pos`.
// 中文: 線性掃描 input[pos..],回報每個音節結束位移,含顯式終止符與隱式第 1 聲邊界 (天然遞增)。
// 中文: 韻核 = 非聲母非終止符的注音字元;見到新聲母 (前面已有韻核) 或輸入結尾仍有未結第 1 聲時補切點;終止符會清除待結韻核。
pub fn valid_span_endings(input: &str, pos: usize) -> Vec<usize> {
    raw_endings_from(input, pos, usize::MAX)
}

/// v3.5.9 D / C-3b — pre-lowered variant matching the TL syllabifier
/// signature so the [`crate::syllabifier::valid_span_endings_lowered`]
/// dispatcher can route TPS through `lattice::build_lattice` without
/// per-mode special cases. TPS is case-insensitive (Bopomofo has no
/// case axis), so `lowered` is just `input` passed through; the
/// per-keystroke `to_ascii_lowercase` allocation upstream is harmless
/// for non-Latin chars and is consistent with the TL path.
///
/// Algorithm:
/// 1. Walk the buffer with the existing terminator-scan to collect the
///    candidate ending byte offsets from `pos` (each ending closes one
///    structural TPS syllable — tone-marked or implicit tone-1).
/// 2. **Inventory-gate** each candidate: chain forward from `pos` using
///    `inv.contains_in(InputMode::Tps, lowered[cur..end])` on each hop's
///    syllable slice. The first depth-1 endings reachable that pass the
///    inventory filter become depth-1 results; the BFS then continues
///    forward from each accepted ending until the chain is exhausted or
///    `max_syllables` is reached.
/// 3. Cap result depth at `max_syllables` (mirrors the TL BFS depth
///    bound). The inventory gate rejects malformed Bopomofo that the
///    structural classifier alone would accept as a "syllable" (Codex
///    pre-impl Fork 3 amendment — without this an unknown Bopomofo span
///    would slip into walker OOV synth as a no-dict carve-out).
///
/// `_inv` and `_mode` are taken for signature parity; this function
/// always probes the `tps:` family via `inv.contains_in(InputMode::Tps,
/// ...)` regardless of the caller's `mode` (the dispatcher in
/// `syllabifier::mod.rs` is responsible for routing only TPS callers
/// here; an unexpected non-TPS `mode` reaching this fn would not change
/// the family probed).
// 中文: D / C-3b — 與 TL syllabifier signature 對齊的 pre-lowered 變體。
// 中文:   先以結構化終止符掃描蒐集候選 ending,再以 inv.contains_in(Tps, slice)
// 中文:   逐跳過濾,深度上限 max_syllables(對齊 TL BFS bound)。Codex 修訂:
// 中文:   不過濾會讓不合法 Bopomofo 跳進 walker OOV synth。
pub(crate) fn valid_span_endings_lowered(
    lowered: &str,
    pos: usize,
    inv: &lexicon::SyllableInventory,
    _mode: phonetics::InputMode,
    max_syllables: usize,
) -> Vec<usize> {
    if max_syllables == 0 || pos >= lowered.len() || !lowered.is_char_boundary(pos) {
        return Vec::new();
    }
    // The structural scanner enumerates syllable boundaries from `pos`
    // forward (terminator or implicit tone-1). Cap collection at
    // `max_syllables` so each call walks O(max_syllables × syllable_bytes)
    // bytes, not O(N) bytes to end-of-input. The BFS below caps depth at
    // `max_syllables` anyway, so any candidate past the Nth boundary
    // would never be inserted into `endings` — truncating the candidate
    // list is byte-identical for output. Without the cap,
    // `build_lattice` calls this once per reachable start (Codex PR
    // #337 r3298575431) and the per-start full-suffix rescan becomes
    // O(N²) on long TPS buffers, matching the cost shape the TL path
    // explicitly avoids via `MAX_SYLLABLE_BYTES`.
    // 中文: 結構化掃描以 max_syllables 為上限收集候選 ending(原本 usize::MAX 會走到 EOI,
    // 中文:   per-start 全 suffix 重掃 → O(N²),Codex r3298575431 指出)。下方 BFS 深度也限
    // 中文:   max_syllables,候選超過此上限的位置永遠不會插入 endings → 截斷對輸出 byte-identical。
    let candidates = raw_endings_from(lowered, pos, max_syllables);
    if candidates.is_empty() {
        return Vec::new();
    }
    // BFS over candidate endings under the inventory gate. Mirrors the
    // TL syllabifier's depth-1..=max_syllables chain semantics so the
    // lattice builder's left-anchored phrase edges (`build_lattice`
    // emits them from the BFS visit of every reachable start) come out
    // the same shape for TPS.
    use std::collections::{BTreeSet, VecDeque};
    let mut endings: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    queue.push_back((pos, 0));
    while let Some((cur, depth)) = queue.pop_front() {
        if depth >= max_syllables {
            continue;
        }
        for &end in &candidates {
            if end <= cur {
                continue;
            }
            if !lowered.is_char_boundary(end) {
                continue;
            }
            let slice = &lowered[cur..end];
            if !inv.contains_in(phonetics::InputMode::Tps, slice) {
                continue;
            }
            if endings.insert(end) {
                queue.push_back((end, depth + 1));
            }
        }
    }
    endings.into_iter().collect()
}

// Internal structural scan — returns every TPS syllable ending byte
// offset reachable from `pos` by walking terminators + implicit tone-1
// boundaries forward. Caller cap via `limit` keeps the public
// [`valid_span_endings`] (no cap) and the inventory-gated
// [`valid_span_endings_lowered`] using ONE scan loop.
// 中文: 結構化掃描內核 — 從 pos 往前以終止符 + 隱式第 1 聲收集所有候選 ending byte 位移;
// 中文:   呼叫端透過 limit 控上限(公開 API 為無上限,gated 變體在掃描後依 max_syllables 過濾)。
fn raw_endings_from(input: &str, pos: usize, limit: usize) -> Vec<usize> {
    if pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }

    let mut endings: Vec<usize> = Vec::new();
    let mut nucleus_seen = false;
    let mut iter = input[pos..].char_indices().peekable();

    while let Some((rel, ch)) = iter.next() {
        if endings.len() >= limit {
            break;
        }
        let start = pos + rel;
        let abs = start + ch.len_utf8();
        match classify(ch) {
            Some(Terminator::Mark) | Some(Terminator::Tone8Dot) => {
                endings.push(abs);
                nucleus_seen = false;
            }
            Some(Terminator::EnteringCoda) => {
                // Coalesce a following 8th-tone dot into the same ending.
                if let Some(&(_, next_ch)) = iter.peek() {
                    if matches!(classify(next_ch), Some(Terminator::Tone8Dot)) {
                        iter.next();
                        endings.push(abs + next_ch.len_utf8());
                        nucleus_seen = false;
                        continue;
                    }
                }
                endings.push(abs);
                nucleus_seen = false;
            }
            None => {
                if phonetics::is_tps_initial(ch) {
                    // A new initial after a nucleus closes the pending
                    // tone-1 syllable just before this initial.
                    if nucleus_seen {
                        endings.push(start);
                        nucleus_seen = false;
                    }
                } else if phonetics::is_tps_char(ch) {
                    nucleus_seen = true;
                }
            }
        }
    }

    if nucleus_seen && endings.len() < limit {
        endings.push(input.len());
    }

    endings
}
