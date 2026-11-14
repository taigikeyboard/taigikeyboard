//! Hanji neutral-tone (`--` / `-`) aligner, display transform, and stripper.
//!
//! Builtin pure-Hanji dictionary entries store plain Hanji without `--` or `-`
//! neutral-tone markers. When `khinsiann_in_hanji` is enabled, [`khinsiann_hanji_display`]
//! (and its borrowed fast-path wrapper [`render_hanji_khinsiann`]) aligns each
//! Hanji/Han-Lo syllable against `canonical_tl` and injects LKK / BONG報 style
//! neutral-tone markers (`--` for neutral boundaries, `-` for subsequent
//! syllables inside a neutral tail, reset on word spaces).
//!
//! The tokenization and formatting rules mirror `tokenize_hanji_syllables` and
//! `format_khinsiann_hanji` in `dictionary/common/cleanup.py`, kept equal by the
//! `khinsiann_pipeline_parity` test in `engine/lexicon/tests/prod.rs`.
//!
//! [`strip_khinsiann_hanji`] and [`learning_hanji_key`] strip `--`-introduced
//! neutral-tone hyphens from pure-Hanji strings while preserving Han-Lo entries,
//! pure-roman entries, and lone single-`-` entries (`一-六`) that carry no `--`.

use std::borrow::Cow;

use crate::hanji::{is_cjk_char, is_kana};
use unicode_normalization::char::is_combining_mark;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HanjiTokenKind {
    IdeographOrKana,
    Roman,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HanjiToken<'a> {
    pub leading_sep: &'a str,
    pub token: &'a str,
    pub kind: HanjiTokenKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlToken<'a> {
    pub leading_sep: &'a str,
    pub syllable: &'a str,
}

fn is_hanzi_separator(c: char) -> bool {
    matches!(c, ' ' | '\u{3000}' | '-')
}

fn is_roman_syllable_char(c: char) -> bool {
    (c.is_alphabetic() || is_combining_mark(c) || ('\u{0300}'..='\u{036F}').contains(&c))
        && !is_cjk_char(c)
        && !is_kana(c)
}

fn tokenize_hanji(hanji: &str) -> Option<(Vec<HanjiToken<'_>>, &str)> {
    let mut tokens = Vec::new();
    let mut sep_start = 0usize;
    let mut iter = hanji.char_indices().peekable();
    while let Some((idx, c)) = iter.next() {
        if is_hanzi_separator(c) {
            continue;
        }
        let leading_sep = &hanji[sep_start..idx];
        if is_cjk_char(c) || is_kana(c) {
            let end = idx + c.len_utf8();
            tokens.push(HanjiToken {
                leading_sep,
                token: &hanji[idx..end],
                kind: HanjiTokenKind::IdeographOrKana,
            });
            sep_start = end;
        } else if is_roman_syllable_char(c) {
            let mut end = idx + c.len_utf8();
            while let Some(&(next_idx, next_c)) = iter.peek() {
                if is_roman_syllable_char(next_c) {
                    end = next_idx + next_c.len_utf8();
                    iter.next();
                } else {
                    break;
                }
            }
            tokens.push(HanjiToken {
                leading_sep,
                token: &hanji[idx..end],
                kind: HanjiTokenKind::Roman,
            });
            sep_start = end;
        } else {
            return None;
        }
    }
    Some((tokens, &hanji[sep_start..]))
}

fn tokenize_tl(canonical_tl: &str) -> Vec<TlToken<'_>> {
    let mut tokens = Vec::new();
    let mut sep_start = 0usize;
    let mut iter = canonical_tl.char_indices().peekable();
    while let Some((idx, c)) = iter.next() {
        if c == ' ' || c == '-' {
            continue;
        }
        let leading_sep = &canonical_tl[sep_start..idx];
        let mut end = idx + c.len_utf8();
        while let Some(&(next_idx, next_c)) = iter.peek() {
            if next_c != ' ' && next_c != '-' {
                end = next_idx + next_c.len_utf8();
                iter.next();
            } else {
                break;
            }
        }
        tokens.push(TlToken {
            leading_sep,
            syllable: &canonical_tl[idx..end],
        });
        sep_start = end;
    }
    tokens
}

/// Align `hanji` and `canonical_tl` 1:1 into syllable tokens.
/// Returns `None` if `hanji` is empty, contains no ideograph/kana char, contains
/// unalignable characters (digits, punctuation, etc.), or has a different
/// syllable count than `canonical_tl`.
pub fn align_hanji_syllables<'a>(
    hanji: &'a str,
    canonical_tl: &'a str,
) -> Option<(Vec<HanjiToken<'a>>, &'a str, Vec<TlToken<'a>>)> {
    if hanji.is_empty() || !hanji.chars().any(|c| is_cjk_char(c) || is_kana(c)) {
        return None;
    }
    let (htoks, htrail) = tokenize_hanji(hanji)?;
    let ttoks = tokenize_tl(canonical_tl);
    if htoks.len() != ttoks.len() {
        return None;
    }
    Some((htoks, htrail, ttoks))
}

/// Apply LKK / BONG報 neutral-tone hyphenation (`--` and `-`) to `hanji` using
/// `canonical_tl` when `enabled` is true, borrowing `hanji` without allocation
/// whenever disabled or unchanged.
pub fn render_hanji_khinsiann<'a>(
    hanji: &'a str,
    canonical_tl: &str,
    enabled: bool,
) -> Cow<'a, str> {
    if !enabled || hanji.is_empty() || hanji.contains("--") || !canonical_tl.contains("--") {
        return Cow::Borrowed(hanji);
    }
    let Some((htoks, htrail, ttoks)) = align_hanji_syllables(hanji, canonical_tl) else {
        return Cow::Borrowed(hanji);
    };

    let mut out = String::with_capacity(hanji.len() + 8);
    let mut in_neutral = false;
    for (i, (htok, ttok)) in htoks.iter().zip(ttoks.iter()).enumerate() {
        let tsep = ttok.leading_sep;
        if let Some((_, suffix)) = tsep.rsplit_once(' ') {
            if suffix.contains("--") {
                in_neutral = true;
                out.push_str("--");
            } else {
                in_neutral = false;
                out.push_str(htok.leading_sep);
            }
        } else {
            let dash_count = tsep.bytes().filter(|&b| b == b'-').count();
            if dash_count >= 2 {
                in_neutral = true;
                out.push_str("--");
            } else if dash_count == 1 {
                if in_neutral && i > 0 {
                    out.push('-');
                } else {
                    out.push_str(htok.leading_sep);
                }
            } else {
                out.push_str(htok.leading_sep);
            }
        }
        out.push_str(htok.token);
    }
    out.push_str(htrail);
    if out == hanji {
        Cow::Borrowed(hanji)
    } else {
        Cow::Owned(out)
    }
}

/// Apply LKK / BONG報 neutral-tone hyphenation (`--` and `-`) to `hanji` using
/// `canonical_tl`. Returns `hanji` unchanged if it already contains `--`, if
/// `canonical_tl` has no `--`, or if 1:1 syllable alignment fails.
pub fn khinsiann_hanji_display(hanji: &str, canonical_tl: &str) -> String {
    render_hanji_khinsiann(hanji, canonical_tl, true).into_owned()
}

/// Append `segment` to an assembled Hanji sentence buffer `out`, inserting `"--"`
/// between words when `khinsiann_before` is true and neither side already has a
/// boundary hyphen.
pub fn push_hanji_with_interword_khinsiann(out: &mut String, segment: &str, khinsiann_before: bool) {
    if !out.is_empty()
        && khinsiann_before
        && !out.ends_with('-')
        && !segment.starts_with('-')
    {
        out.push_str("--");
    }
    out.push_str(segment);
}

fn is_pure_hanji(s: &str) -> bool {
    s.chars().any(|c| is_cjk_char(c) || is_kana(c))
        && s.chars()
            .all(|c| is_hanzi_separator(c) || is_cjk_char(c) || is_kana(c))
}

/// Borrowed fast-path wrapper around [`strip_khinsiann_hanji`] for learning and
/// ranking key lookups.
pub fn learning_hanji_key(hanji: &str) -> Cow<'_, str> {
    if !hanji.contains("--") || !is_pure_hanji(hanji) {
        return Cow::Borrowed(hanji);
    }
    let mut out = String::with_capacity(hanji.len());
    let mut chars = hanji.chars().peekable();
    let mut in_neutral = false;
    while let Some(c) = chars.next() {
        if c == '-' {
            let mut run = 1usize;
            while chars.peek() == Some(&'-') {
                chars.next();
                run += 1;
            }
            if run >= 2 {
                in_neutral = true;
            } else if !in_neutral {
                out.push('-');
            }
        } else {
            if matches!(c, ' ' | '\u{3000}') {
                in_neutral = false;
            }
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// Strip `--`-introduced neutral-tone hyphens (`--` and subsequent `-` inside the
/// neutral tail until a space boundary) from pure-Hanji spellings.
/// Han-Lo entries, pure-roman entries, and single-`-` entries without `--` (`一-六`)
/// are returned unchanged.
pub fn strip_khinsiann_hanji(hanji: &str) -> String {
    learning_hanji_key(hanji).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_transform_matches_lkk_bong_rule() {
        assert_eq!(khinsiann_hanji_display("會記得", "ē-kì--tit"), "會記--得");
        assert_eq!(khinsiann_hanji_display("轉去", "--tńg-khì"), "--轉-去");
        assert_eq!(khinsiann_hanji_display("矣", "--ah"), "--矣");
        assert_eq!(khinsiann_hanji_display("轉去", "tńg--khì"), "轉--去");
        assert_eq!(khinsiann_hanji_display("轉去", "tńg-khì"), "轉去");
        assert_eq!(
            khinsiann_hanji_display("記起來", "kì--khí-lâi"),
            "記--起-來"
        );
        assert_eq!(
            khinsiann_hanji_display("九月重陽", "káu--ge̍h tiông-iông"),
            "九--月重陽"
        );
        assert_eq!(
            khinsiann_hanji_display("做人矣", "tsuè--lâng--ah"),
            "做--人--矣"
        );
        assert_eq!(
            khinsiann_hanji_display("有的無的", "ū--ê-bô--ê"),
            "有--的-無--的"
        );
        assert_eq!(
            khinsiann_hanji_display("用sai的", "iōng sai--ê"),
            "用sai--的"
        );
        assert_eq!(khinsiann_hanji_display("tso̍h日", "tso̍h--ji̍t"), "tso̍h--日");
        assert_eq!(khinsiann_hanji_display("邊a", "pinn--a"), "邊--a");
        assert!(matches!(
            render_hanji_khinsiann("會記得", "ē-kì--tit", false),
            Cow::Borrowed("會記得")
        ));
    }

    #[test]
    fn display_transform_preserves_already_marked_and_unalignable() {
        assert_eq!(khinsiann_hanji_display("邊--a", "pinn--a"), "邊--a");
        assert_eq!(khinsiann_hanji_display("尾--à", "bué--à"), "尾--à");
        assert_eq!(
            khinsiann_hanji_display("食飽--bē", "tsia̍h-pá--bē"),
            "食飽--bē"
        );
        assert_eq!(khinsiann_hanji_display("tso̍h--日", "tso̍h--ji̍t"), "tso̍h--日");
        assert_eq!(khinsiann_hanji_display("--tńg-去", "--tńg-khì"), "--tńg-去");
        assert_eq!(khinsiann_hanji_display("會--記得", "ē-kì--tit"), "會--記得");
        assert_eq!(
            khinsiann_hanji_display("ē-kì--tit", "ē-kì--tit"),
            "ē-kì--tit"
        );
        // Mismatched syllable count or unalignable char -> unchanged
        assert_eq!(khinsiann_hanji_display("轉", "tńg--khì"), "轉");
        assert_eq!(khinsiann_hanji_display("轉/去", "tńg--khì"), "轉/去");
        assert_eq!(
            khinsiann_hanji_display("有二步\u{f777}", "ū--lī-pōo--pueh"),
            "有二步\u{f777}"
        );
        // Single-hyphen IDS entry without -- in TL
        assert_eq!(khinsiann_hanji_display("大-片", "tuā-pue"), "大-片");
        // Empty
        assert_eq!(khinsiann_hanji_display("", "ē-kì--tit"), "");
    }

    #[test]
    fn strip_khinsiann_hanji_only_strips_neutral_tone_pure_hanji() {
        assert_eq!(strip_khinsiann_hanji("--轉-去"), "轉去");
        assert_eq!(strip_khinsiann_hanji("--轉去"), "轉去");
        assert_eq!(strip_khinsiann_hanji("記--起-來"), "記起來");
        assert_eq!(strip_khinsiann_hanji("--一-下"), "一下");
        assert_eq!(strip_khinsiann_hanji("有--的-無--的"), "有的無的");
        assert_eq!(strip_khinsiann_hanji("九--月重陽"), "九月重陽");
        assert_eq!(strip_khinsiann_hanji("九--月 重-陽"), "九月 重-陽");
        // Lone single hyphens without `--` are preserved (matches SQLite `LIKE '%--%'`).
        assert_eq!(strip_khinsiann_hanji("一-六"), "一-六");
        assert!(matches!(learning_hanji_key("一-六"), Cow::Borrowed("一-六")));
        // Han-Lo and roman-only entries are preserved.
        assert_eq!(strip_khinsiann_hanji("大--ê"), "大--ê");
        assert_eq!(strip_khinsiann_hanji("矮--ê"), "矮--ê");
        assert_eq!(strip_khinsiann_hanji("邊--a"), "邊--a");
        assert_eq!(strip_khinsiann_hanji("尾--à"), "尾--à");
        assert_eq!(strip_khinsiann_hanji("tso̍h--日"), "tso̍h--日");
        assert_eq!(strip_khinsiann_hanji("--tńg-去"), "--tńg-去");
        assert_eq!(strip_khinsiann_hanji("ē-kì--tit"), "ē-kì--tit");
    }

    #[test]
    fn interword_khinsiann_join_avoids_duplicate_hyphens() {
        let mut out = String::new();
        push_hanji_with_interword_khinsiann(&mut out, "走", true);
        push_hanji_with_interword_khinsiann(&mut out, "矣", true);
        assert_eq!(out, "走--矣");

        let mut leading = String::from("走");
        push_hanji_with_interword_khinsiann(&mut leading, "--轉-去", true);
        assert_eq!(leading, "走--轉-去");
    }
}
