//! TPS syllabifier — O(n) scan returning every byte offset that ends a
//! TPS syllable. Two boundary kinds:
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
    if pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }

    let mut endings: Vec<usize> = Vec::new();
    let mut nucleus_seen = false;
    let mut iter = input[pos..].char_indices().peekable();

    while let Some((rel, ch)) = iter.next() {
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

    if nucleus_seen {
        endings.push(input.len());
    }

    endings
}
