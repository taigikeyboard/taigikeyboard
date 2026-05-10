//! TPS syllabifier — O(n) scan returning every byte offset that lies
//! immediately after a TPS syllable terminator (tone mark, entering
//! coda, or 8th-tone dot).
//!
//! Tone marks are unambiguous syllable terminators in TPS (per
//! `docs/roadmap.md` line 95). Tone-1 (no mark) syllables produce no
//! ending here — Phase 4's dispatcher handles implicit boundaries via
//! the "next initial seen" rule.

// 中文: TPS 音節切分器 — 線性掃描,在每個聲調符號 / 入聲韻尾 / 第 8 聲點之後回報切點。
// 中文: 第 1 聲沒有調號,本掃描不偵測,改由 Phase 4 在看到下一個聲母字符時補回邊界。

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

/// Return every byte offset `e > pos` such that `input[..e]` ends on
/// a TPS syllable terminator. An entering-coda small letter followed
/// immediately by an 8th-tone dot terminates AT the dot (one ending,
/// not two); a stray dot terminates standalone.
///
/// Contract:
/// - Returns ascending byte offsets (`char_indices` is monotonic, so
///   no separate dedup is needed).
/// - Returns empty `Vec` when `pos >= input.len()` or `pos` is not on
///   a UTF-8 char boundary.
// 中文: 線性掃描 input[pos..],在每個 tone-mark / 入聲韻尾 / 第 8 聲點之後回報切點 (天然遞增)。
pub fn valid_span_endings(input: &str, pos: usize) -> Vec<usize> {
    if pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }

    let mut endings: Vec<usize> = Vec::new();
    let mut iter = input[pos..].char_indices().peekable();

    while let Some((rel, ch)) = iter.next() {
        let abs = pos + rel + ch.len_utf8();
        match classify(ch) {
            Some(Terminator::Mark) | Some(Terminator::Tone8Dot) => endings.push(abs),
            Some(Terminator::EnteringCoda) => {
                // Coalesce a following 8th-tone dot into the same ending.
                if let Some(&(_, next_ch)) = iter.peek() {
                    if matches!(classify(next_ch), Some(Terminator::Tone8Dot)) {
                        iter.next();
                        endings.push(abs + next_ch.len_utf8());
                        continue;
                    }
                }
                endings.push(abs);
            }
            None => {}
        }
    }

    endings
}
