//! The two web dictionaries a search result can be looked up in — 教典 and
//! 台語辭典 (ChhoeTaigi) — and the digit-tone spelling of a TL reading their
//! query strings take. Port of `ExternalLookupURLBuilder.swift`.

// 外部辭典連結 — 教典 / ChhoeTaigi 的查詢網址,與其要求的數字調拼法。

use super::phonetics::{nfd_preprocess_for_lookup, strip_tone};

/// 教典's search URL for `tl`, or `None` when the reading spells nothing.
pub fn moe_url(tl: &str) -> Option<String> {
    url(
        "https://sutian.moe.edu.tw/zh-hant/tshiau/",
        &[("lui", "tai_su")],
        "tsha",
        tl,
    )
}

/// ChhoeTaigi's search URL for `tl`.
pub fn chhoe_url(tl: &str) -> Option<String> {
    url(
        "https://chhoe.taigi.info/s",
        &[("s", "su"), ("f", "e"), ("lmjf", "ki")],
        "lmj",
        tl,
    )
}

fn url(
    base: &str,
    fixed_query: &[(&str, &str)],
    reading_parameter: &str,
    tl: &str,
) -> Option<String> {
    let digit_tone = digit_tone_form(tl);
    if digit_tone.is_empty() {
        return None;
    }
    let mut query: Vec<String> = fixed_query
        .iter()
        .map(|(name, value)| format!("{name}={}", percent_encode(value)))
        .collect();
    query.push(format!(
        "{reading_parameter}={}",
        percent_encode(&digit_tone)
    ));
    Some(format!("{base}?{}", query.join("&")))
}

/// Percent-encoding for a query value: unreserved characters kept,
/// everything else (including `+`, `/`, `&`, `=` and non-ASCII) encoded as
/// UTF-8. Stricter than Foundation's `URLQueryItem` (which leaves `+` and
/// `/` alone) — the readings this ever carries are letters, digits and
/// hyphens after `digit_tone_form`, where the two agree.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// A TL reading in the digit-tone spelling the web dictionaries search by:
/// lowercased, syllable by syllable (empty syllables kept so the hyphens
/// survive), the nasal marks as `nn`, the tone as a trailing digit with
/// tones 1 and 4 omitted (`ExternalLookupURLBuilder.digitToneForm`).
pub fn digit_tone_form(tl: &str) -> String {
    tl.to_lowercase()
        .split('-')
        .map(syllable_in_digit_tone)
        .collect::<Vec<_>>()
        .join("-")
}

fn syllable_in_digit_tone(syllable: &str) -> String {
    if syllable.is_empty() {
        return String::new();
    }
    let with_nasal = syllable.replace(['\u{207F}', '\u{1D3A}'], "nn");
    if let Some(last) = with_nasal.chars().last().filter(char::is_ascii_digit) {
        let normalized = nfd_preprocess_for_lookup(&with_nasal).unwrap_or(with_nasal);
        return if last == '1' || last == '4' {
            normalized[..normalized.len() - 1].to_owned()
        } else {
            normalized
        };
    }
    let Some((bare, tone)) = nfd_preprocess_for_lookup(&with_nasal).and_then(|p| strip_tone(&p))
    else {
        return with_nasal;
    };
    if tone.is_empty() || tone == "1" || tone == "4" {
        bare
    } else {
        format!("{bare}{tone}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_tone_form_omits_tones_one_and_four_and_keeps_hyphens() {
        // trace: tâi-gí → tai5-gi2; tsia̍h → tsiah8; kau (tone 1) → kau;
        // ah4 (numeric) → ah; a leading empty syllable keeps its hyphen.
        assert_eq!(digit_tone_form("Tâi-gí"), "tai5-gi2");
        assert_eq!(digit_tone_form("tsia̍h"), "tsiah8");
        assert_eq!(digit_tone_form("kau"), "kau");
        assert_eq!(digit_tone_form("ah4"), "ah");
        assert_eq!(digit_tone_form("--ah"), "--ah");
        assert_eq!(digit_tone_form("tiⁿ"), "tinn");
    }

    #[test]
    fn the_two_urls_carry_the_fixed_query_and_the_encoded_reading() {
        assert_eq!(
            moe_url("Tâi-gí").as_deref(),
            Some("https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=tai5-gi2")
        );
        assert_eq!(
            chhoe_url("tsia̍h").as_deref(),
            Some("https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=tsiah8")
        );
        assert_eq!(moe_url(""), None);
    }
}
