//! v3.5.8 Phase 3 — TPS syllabifier scanner tests.
//!
//! Verifies that tone-mark spacing modifiers, entering-coda small
//! letters, and 8th-tone dot variants all produce ascending byte
//! offsets. Tone-1 (no mark) syllables must yield no ending — Phase 4
//! handles those via the "next initial seen" rule.

// 中文: Phase 3 TPS syllabifier 測試 — 確認聲調 mark / 入聲韻尾 / 第 8 聲點都會回報切點。
// 中文: 第 1 聲沒有 mark,本掃描不偵測;由 Phase 4 在看到下一個聲母時補回邊界。

use composing::syllabifier::tps::valid_span_endings;

#[test]
fn single_tone5_mark_terminates_one_syllable() {
    // ㄉㄧㄠˊ — `tiau5`. Bytes: ㄉ(3) ㄧ(3) ㄠ(3) ˊ(2) = 11.
    let input = "\u{3109}\u{3127}\u{3120}\u{02ca}";
    assert_eq!(valid_span_endings(input, 0), vec![input.len()]);
}

#[test]
fn two_syllable_chain_yields_running_endings() {
    // ㄉㄚˋ ㄎㄧˊ — tone2 + tone5 marks. Two endings.
    let s1 = "\u{3109}\u{311a}\u{02cb}"; // ㄉㄚˋ — 8 bytes
    let s2 = "\u{310e}\u{3127}\u{02ca}"; // ㄎㄧˊ — 8 bytes
    let input = format!("{s1}{s2}");
    let endings = valid_span_endings(&input, 0);
    assert_eq!(endings, vec![s1.len(), input.len()]);
}

#[test]
fn entering_coda_p4_terminates_without_following_dot() {
    // ㄎㄚㆴ — k4. No 8th-tone dot follows.
    let input = "\u{310e}\u{311a}\u{31b4}";
    assert_eq!(valid_span_endings(input, 0), vec![input.len()]);
}

#[test]
fn entering_coda_with_combining_dot_coalesces_into_single_ending() {
    // ㄎㄚㆴ + U+0307 → tone-8 stop. One ending at end-of-dot.
    let input = "\u{310e}\u{311a}\u{31b4}\u{0307}";
    assert_eq!(valid_span_endings(input, 0), vec![input.len()]);
}

#[test]
fn entering_coda_with_encode_safe_dot_coalesces() {
    // ㄎㄚㆴ + U+02D9 (encode-safe variant) → one ending.
    let input = "\u{310e}\u{311a}\u{31b4}\u{02d9}";
    assert_eq!(valid_span_endings(input, 0), vec![input.len()]);
}

#[test]
fn standalone_combining_dot_terminates() {
    // Stray U+0307 mid-stream. The encode-safe table at
    // phonetics/tps.rs:109 lists ("8", U+02D9) as a valid standalone
    // tone-8 entry; the NFD U+0307 mirror is symmetric.
    let input = "\u{3109}\u{311a}\u{0307}\u{310e}\u{3127}\u{02ca}";
    let endings = valid_span_endings(input, 0);
    let dot_end = "\u{3109}\u{311a}\u{0307}".len();
    assert_eq!(endings, vec![dot_end, input.len()]);
}

#[test]
fn tone1_without_mark_yields_no_ending() {
    // ㄉㄚ — pure tone-1, no terminator. Phase 3 returns nothing;
    // Phase 4 will detect the boundary when the next initial appears.
    let input = "\u{3109}\u{311a}";
    assert!(valid_span_endings(input, 0).is_empty());
}

#[test]
fn pos_offset_into_input_walks_only_remaining_suffix() {
    // ㄉㄚˋ ㄎㄧˊ — start scan after the first tone-2 mark.
    let s1 = "\u{3109}\u{311a}\u{02cb}";
    let s2 = "\u{310e}\u{3127}\u{02ca}";
    let input = format!("{s1}{s2}");
    let endings = valid_span_endings(&input, s1.len());
    assert_eq!(endings, vec![input.len()]);
}

#[test]
fn pos_at_input_end_returns_empty() {
    let input = "\u{3109}\u{311a}\u{02cb}";
    assert!(valid_span_endings(input, input.len()).is_empty());
}

#[test]
fn empty_input_returns_empty() {
    assert!(valid_span_endings("", 0).is_empty());
}

#[test]
fn pos_at_non_char_boundary_returns_empty_safely() {
    // ㄉ is 3 bytes; pos=1 is mid-codepoint. Must not panic.
    let input = "\u{3109}\u{311a}\u{02cb}";
    assert!(valid_span_endings(input, 1).is_empty());
}

#[test]
fn endings_are_strictly_ascending() {
    let s1 = "\u{310e}\u{311a}\u{31b4}\u{0307}"; // tone-8 stop
    let s2 = "\u{3109}\u{311a}\u{02cb}"; // tone-2
    let input = format!("{s1}{s2}");
    let endings = valid_span_endings(&input, 0);
    for w in endings.windows(2) {
        assert!(
            w[0] < w[1],
            "endings must be strictly ascending: {endings:?}"
        );
    }
}
