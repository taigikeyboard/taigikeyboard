//! Hanji neutral-tone (`--` / `-`) aligner, display transform, and stripper.
//!
//! Builtin pure-Hanji dictionary entries store plain Hanji without `--` or `-`
//! neutral-tone markers. When `khinsiann_in_hanji` is enabled, [`render_hanji_khinsiann`]
//! aligns each Hanji/Han-Lo syllable against `canonical_tl` and injects LKK / BONG報 style
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

pub(crate) fn is_roman_syllable_char(c: char) -> bool {
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

/// Neutral-tone continuation context entering a Hanji word from a preceding
/// typed separator (`--`, or `-` continuing an open neutral tail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadingKhinsiann {
    /// No preceding neutral-tone boundary (only mark `--` present in `canonical_tl`).
    None,
    /// Prepended with `--` mid-composing: opens a neutral-tone span (`--`
    /// before syllable 0, `-` between hyphen-joined syllables until a word space).
    DoubleDash,
    /// Follows a typed `-` while already inside an open neutral-tone span
    /// (`-` before syllable 0 and between hyphen-joined syllables until a word space).
    ContinueHyphen,
}

/// Apply LKK / BONG報 neutral-tone hyphenation (`--` and `-`) to `hanji` using
/// `canonical_tl` when `enabled` is true, borrowing `hanji` without allocation
/// whenever disabled or unchanged.
pub fn render_hanji_khinsiann<'a>(
    hanji: &'a str,
    canonical_tl: &str,
    enabled: bool,
) -> Cow<'a, str> {
    render_hanji_khinsiann_with_leading(hanji, canonical_tl, enabled, LeadingKhinsiann::None)
}

/// [`render_hanji_khinsiann`] with an optional [`LeadingKhinsiann`] boundary
/// typed immediately before the word.
///
/// When `leading` is [`LeadingKhinsiann::DoubleDash`] or
/// [`LeadingKhinsiann::ContinueHyphen`], syllable 0 opens with `--` or `-`
/// and hyphen-joined syllables inside the same word carry `-` until a word
/// space resets neutral tone; any internal `--` in `canonical_tl` or `hanji`
/// is kept as `--`.
pub fn render_hanji_khinsiann_with_leading<'a>(
    hanji: &'a str,
    canonical_tl: &str,
    enabled: bool,
    leading: LeadingKhinsiann,
) -> Cow<'a, str> {
    if !enabled || hanji.is_empty() {
        return Cow::Borrowed(hanji);
    }
    if leading == LeadingKhinsiann::None {
        if hanji.contains("--") || !canonical_tl.contains("--") {
            return Cow::Borrowed(hanji);
        }
    } else if hanji.starts_with('-') {
        return Cow::Borrowed(hanji);
    }
    let Some((htoks, htrail, ttoks)) = align_hanji_syllables(hanji, canonical_tl) else {
        return match leading {
            LeadingKhinsiann::None => Cow::Borrowed(hanji),
            LeadingKhinsiann::DoubleDash => Cow::Owned(format!("--{hanji}")),
            LeadingKhinsiann::ContinueHyphen => Cow::Owned(format!("-{hanji}")),
        };
    };

    let has_existing_dd = hanji.contains("--");
    let mut preserve_rest = false;
    let mut out = String::with_capacity(hanji.len() + 8);
    let mut in_neutral = false;
    for (i, (htok, ttok)) in htoks.iter().zip(ttoks.iter()).enumerate() {
        if i == 0 && leading != LeadingKhinsiann::None {
            in_neutral = true;
            match leading {
                LeadingKhinsiann::DoubleDash => out.push_str("--"),
                LeadingKhinsiann::ContinueHyphen => out.push('-'),
                LeadingKhinsiann::None => {}
            }
            out.push_str(htok.token);
            continue;
        }
        if has_existing_dd && (preserve_rest || htok.leading_sep.contains("--")) {
            preserve_rest = true;
            out.push_str(htok.leading_sep);
            out.push_str(htok.token);
            continue;
        }
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

/// Whether a word with reading `canonical_tl` entered under `leading` ends
/// inside an open neutral-tone span (no space reset after the last neutral
/// marker).
pub fn ends_in_khinsiann(canonical_tl: &str, leading: LeadingKhinsiann) -> bool {
    if let Some((_, last_word)) = canonical_tl.rsplit_once(' ') {
        last_word.contains("--")
    } else {
        leading != LeadingKhinsiann::None || canonical_tl.contains("--")
    }
}

/// Append `segment` to an assembled Hanji sentence buffer `out`, inserting `"--"`
/// between words when `khinsiann_before` is true and neither side already has a
/// boundary hyphen.
pub fn push_hanji_with_interword_khinsiann(
    out: &mut String,
    segment: &str,
    khinsiann_before: bool,
) {
    if !out.is_empty() && khinsiann_before && !out.ends_with('-') && !segment.starts_with('-') {
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

    /// [`render_hanji_khinsiann`] with the `--` / `-` markers on.
    fn khinsiann_hanji_display(hanji: &str, canonical_tl: &str) -> String {
        render_hanji_khinsiann(hanji, canonical_tl, true).into_owned()
    }

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
        assert!(matches!(
            learning_hanji_key("一-六"),
            Cow::Borrowed("一-六")
        ));
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

    #[test]
    fn double_dash_leading_marks_non_khinsiann_and_preserves_internal_double_dash() {
        let dd = |hanji, tl| {
            render_hanji_khinsiann_with_leading(hanji, tl, true, LeadingKhinsiann::DoubleDash)
                .into_owned()
        };
        // Non-khinsiann multi-syllable words get `--` + interior `-`.
        assert_eq!(dd("出來", "tshut-lâi"), "--出-來");
        assert_eq!(dd("一點仔", "tsi̍t-tiám-á"), "--一-點-仔");
        assert_eq!(dd("落去", "lo̍h-khì"), "--落-去");
        assert_eq!(dd("我", "guá"), "--我");
        // Already `--`-opening or pre-marked.
        assert_eq!(dd("轉去", "--tńg-khì"), "--轉-去");
        assert_eq!(dd("--轉-去", "--tńg-khì"), "--轉-去");
        // Internal `--` is kept separate, never flattened to `-`.
        assert_eq!(dd("轉去", "tńg--khì"), "--轉--去");
        assert_eq!(dd("會記得", "ē-kì--tit"), "--會-記--得");
        assert_eq!(dd("過來", "kuè--lâi"), "--過--來");
        // Han-Lo and custom entries with internal `--`.
        assert_eq!(dd("四腳--ê", "sì-kha--ê"), "--四-腳--ê");
        assert_eq!(dd("啥mih", "siánn-mih"), "--啥-mih");
        assert_eq!(dd("邊--a", "pinn--a"), "--邊--a");
        assert_eq!(dd("會--記得", "ē-kì--tit"), "--會--記得");
        assert_eq!(dd("tso̍h日", "tso̍h--ji̍t"), "--tso̍h--日");
        // Word spaces reset neutral tone.
        assert_eq!(dd("放重利", "pàng tāng-lāi"), "--放重利");
        assert_eq!(dd("台北人", "tâi-pak lâng"), "--台-北人");
        assert_eq!(dd("台北 人", "tâi-pak lâng"), "--台-北 人");
        assert_eq!(dd("九月重陽", "káu--ge̍h tiông-iông"), "--九--月重陽");
        // Unalignable or non-Hanji entries still take the leading `--`.
        assert_eq!(dd("轉/去", "tńg-khì"), "--轉/去");
        assert_eq!(dd("OK", "o-khe"), "--OK");
        assert_eq!(dd("", "tshut-lâi"), "");
        assert!(matches!(
            render_hanji_khinsiann_with_leading(
                "出來",
                "tshut-lâi",
                false,
                LeadingKhinsiann::DoubleDash
            ),
            Cow::Borrowed("出來")
        ));
    }

    #[test]
    fn continue_hyphen_chains_inside_open_neutral_span() {
        let cont = |hanji, tl| {
            render_hanji_khinsiann_with_leading(hanji, tl, true, LeadingKhinsiann::ContinueHyphen)
                .into_owned()
        };
        assert_eq!(cont("聲", "siann"), "-聲");
        assert_eq!(cont("出來", "tshut-lâi"), "-出-來");
        assert_eq!(cont("會記得", "ē-kì--tit"), "-會-記--得");
        assert_eq!(cont("台北人", "tâi-pak lâng"), "-台-北人");
        assert_eq!(cont("--轉-去", "--tńg-khì"), "--轉-去");
        assert_eq!(cont("-聲", "siann"), "-聲");
        assert_eq!(cont("OK", "o-khe"), "-OK");
    }

    #[test]
    fn ends_in_khinsiann_tracks_open_neutral_spans_and_space_resets() {
        use LeadingKhinsiann::{ContinueHyphen, DoubleDash, None};
        assert!(!ends_in_khinsiann("siá", None));
        assert!(!ends_in_khinsiann("tshut-lâi", None));
        assert!(ends_in_khinsiann("tsi̍t", DoubleDash));
        assert!(ends_in_khinsiann("tshut-lâi", DoubleDash));
        assert!(ends_in_khinsiann("siann", ContinueHyphen));
        assert!(ends_in_khinsiann("ē-kì--tit", None));
        assert!(!ends_in_khinsiann("pàng tāng-lāi", DoubleDash));
        assert!(!ends_in_khinsiann("káu--ge̍h tiông-iông", DoubleDash));
        assert!(ends_in_khinsiann("iōng sai--ê", None));
    }
}
