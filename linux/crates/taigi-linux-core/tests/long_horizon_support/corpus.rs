#![allow(dead_code)]
//! Real-corpus loader (`corpus/taigi-typing/exampleSentences.js`, read-only per
//! `corpus/README.md`) + synthetic fallback + authoritative engine-backed
//! keystroke encoders for TL, POJ, Telex, and TPS.

use std::path::{Path, PathBuf};

use prost::Message;
use protos::engine::{
    phonetics_request, phonetics_response, request, response, ExternalLookupDigitForm,
    PhoneticsRequest, Request, Response,
};
use serde::Deserialize;
use taigi_desktop_core::engine::{tl_display_to_tps, tl_to_poj, TPS_OR_MAPS_TO_ER};

#[derive(Clone, Debug, Deserialize)]
pub struct RawCorpusItem {
    pub hanji: String,
    pub tailo: String,
    #[serde(default)]
    pub dic_url: String,
}

#[derive(Clone, Debug)]
pub struct CorpusSentence {
    pub index: usize,
    pub hanji: String,
    pub tailo: String,
    pub dic_url: String,
    pub segments: Vec<SentenceSegment>,
}

#[derive(Clone, Debug)]
pub enum SentenceSegment {
    Word(EncodedWord),
    Punct(String),
}

#[derive(Clone, Debug)]
pub struct EncodedWord {
    pub display_tl: String,
    /// Hyphen-joined syllables with `--` preserved for khinsiann and digits 2/3/5/6/7/8/9.
    pub digit_tl: String,
    /// Individual syllables (with `(toneless_tl, Option<u8>, preceded_by_khinsiann)`).
    pub syllables: Vec<SyllableSpec>,
    /// Physical keyboard strokes for TPS (`InputMode::Tps`).
    pub tps_keys: Vec<TpsStroke>,
}

#[derive(Clone, Debug)]
pub struct SyllableSpec {
    pub toneless_tl: String,
    /// ASCII keystrokes in POJ with `oo`/`nn` double-tap on (`o\u{0358}` -> `oo`, `\u{207f}` -> `nn`).
    pub toneless_poj: String,
    /// Explicit tone digit from `ExternalLookupDigitForm` (`None` means tone 1 or 4).
    pub tone_digit: Option<u8>,
    /// True if preceded by `--` (khinsiann) rather than single `-` or word start.
    pub khinsiann_before: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpsStroke {
    pub ch: char,
    pub shift: bool,
}

#[derive(Clone, Debug)]
pub struct LoadedCorpus {
    pub source_label: String,
    pub is_real_corpus: bool,
    pub sentences: Vec<CorpusSentence>,
}

/// Exact inverse of `desktop/crates/taigi-desktop-core/src/keys/tps_layout.rs` `KEYS`.
const TPS_LAYOUT_KEYS: &[(char, &str)] = &[
    ('1', "\u{3105}"),
    ('!', "\u{31a0}"),
    ('q', "\u{3106}"),
    ('a', "\u{3107}"),
    ('2', "\u{3109}"),
    ('w', "\u{310a}"),
    ('s', "\u{310b}"),
    ('x', "\u{310c}"),
    ('e', "\u{310d}"),
    ('E', "\u{31a3}"),
    ('d', "\u{310e}"),
    ('D', "\u{312b}"),
    ('c', "\u{310f}"),
    ('r', "\u{3110}"),
    ('R', "\u{31a2}"),
    ('f', "\u{3111}"),
    ('v', "\u{3112}"),
    ('y', "\u{3117}"),
    ('Y', "\u{31a1}"),
    ('h', "\u{3118}"),
    ('n', "\u{3119}"),
    ('8', "\u{311a}"),
    ('*', "\u{31a9}"),
    ('i', "\u{311b}"),
    ('k', "\u{311c}"),
    ('o', "\u{31a4}"),
    ('O', "\u{31a5}"),
    ('m', "\u{31a8}"),
    ('.', "\u{31a6}"),
    ('>', "\u{31a7}"),
    (',', "\u{311d}"),
    ('u', "\u{3127}"),
    ('U', "\u{31aa}"),
    ('j', "\u{3128}"),
    ('J', "\u{31ab}"),
    ('9', "\u{311e}"),
    ('(', "\u{31ae}"),
    ('l', "\u{3120}"),
    ('L', "\u{31af}"),
    ('0', "\u{3122}"),
    (')', "\u{31b0}"),
    (';', "\u{3124}"),
    ('-', "\u{31b2}"),
    ('_', "\u{31b1}"),
    ('p', "\u{3123}"),
    ('P', "\u{31ac}"),
    ('/', "\u{3125}"),
    ('?', "\u{31ad}"),
    ('t', "\u{31b4}"),
    ('g', "\u{31b5}"),
    ('b', "\u{31bb}"),
    ('z', "\u{31b7}"),
    ('4', "\u{02cb}"),
    ('3', "\u{02ea}"),
    ('6', "\u{02ca}"),
    ('#', "\u{02c7}"),
    ('5', "\u{02eb}"),
    ('7', "\u{02d9}"),
    ('^', "\u{02c6}"),
    ('=', "-"),
];

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

pub fn load_corpus(max_encode: Option<usize>) -> LoadedCorpus {
    if let Some((path, raw_items)) = try_load_real_corpus() {
        let limit = max_encode.unwrap_or(raw_items.len()).min(raw_items.len());
        let sentences = raw_items
            .into_iter()
            .take(limit)
            .enumerate()
            .filter_map(|(index, item)| encode_sentence(index, item))
            .collect();
        return LoadedCorpus {
            source_label: path.display().to_string(),
            is_real_corpus: true,
            sentences,
        };
    }

    let sentences = synthetic_items()
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| encode_sentence(index, item))
        .collect();
    LoadedCorpus {
        source_label: "synthetic-fallback".to_owned(),
        is_real_corpus: false,
        sentences,
    }
}

fn candidate_corpus_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(explicit) = std::env::var("TAIGI_TYPING_CORPUS_PATH") {
        if !explicit.trim().is_empty() {
            paths.push(PathBuf::from(explicit));
        }
    }
    let root = repo_root();
    paths.push(root.join("corpus/taigi-typing/exampleSentences.js"));
    paths
}

fn try_load_real_corpus() -> Option<(PathBuf, Vec<RawCorpusItem>)> {
    for path in candidate_corpus_paths() {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Some(items) = parse_example_sentences_js(&contents) {
                if !items.is_empty() {
                    return Some((canonical_or_self(&path), items));
                }
            }
        }
    }
    None
}

fn canonical_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn parse_example_sentences_js(js: &str) -> Option<Vec<RawCorpusItem>> {
    let start = js.find('[')?;
    let end = js.rfind(']')?;
    if end <= start {
        return None;
    }
    let cleaned = strip_json_trailing_commas(&js[start..=end]);
    let nested: Vec<Vec<RawCorpusItem>> = serde_json::from_str(&cleaned).ok()?;
    Some(nested.into_iter().flatten().collect())
}

/// Removes `,` tokens outside JSON strings when followed only by whitespace and `]` or `}`.
fn strip_json_trailing_commas(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in src.char_indices() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch == ',' {
            let mut look = idx + 1;
            while look < bytes.len() && bytes[look].is_ascii_whitespace() {
                look += 1;
            }
            if look < bytes.len() && (bytes[look] == b']' || bytes[look] == b'}') {
                continue;
            }
        }
        out.push(ch);
    }
    out
}

fn external_lookup_digit_form(reading: &str) -> Option<String> {
    let req = Request {
        id: 1,
        generation: 0,
        config_snapshot: None,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            method: Some(phonetics_request::Method::ExternalLookupDigitForm(
                ExternalLookupDigitForm {
                    reading: reading.to_owned(),
                },
            )),
        })),
    };
    let resp_bytes = dispatch::process_request(&req.encode_to_vec());
    let resp = Response::decode(resp_bytes.as_slice()).ok()?;
    match resp.payload? {
        response::Payload::Phonetics(phon) => match phon.result? {
            phonetics_response::Result::StringResult(s) => Some(s.output),
            _ => None,
        },
        _ => None,
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphabetic()
        || ch == '-'
        || ('\u{0300}'..='\u{036F}').contains(&ch)
        || ch == '\u{207F}'
        || ch == '\u{1D3A}'
}

fn encode_sentence(index: usize, item: RawCorpusItem) -> Option<CorpusSentence> {
    let mut segments = Vec::new();
    let mut current_word = String::new();
    let mut current_punct = String::new();

    for ch in item.tailo.chars() {
        if is_word_char(ch) {
            if !current_punct.is_empty() {
                segments.push(SentenceSegment::Punct(std::mem::take(&mut current_punct)));
            }
            current_word.push(ch);
        } else {
            if !current_word.is_empty() {
                let word = std::mem::take(&mut current_word);
                if let Some(encoded) = encode_word(&word) {
                    segments.push(SentenceSegment::Word(encoded));
                }
            }
            if !ch.is_whitespace() {
                current_punct.push(ch);
            } else if !current_punct.is_empty() {
                segments.push(SentenceSegment::Punct(std::mem::take(&mut current_punct)));
            }
        }
    }
    if !current_word.is_empty() {
        if let Some(encoded) = encode_word(&current_word) {
            segments.push(SentenceSegment::Word(encoded));
        }
    }
    if !current_punct.is_empty() {
        segments.push(SentenceSegment::Punct(current_punct));
    }

    let has_word = segments
        .iter()
        .any(|seg| matches!(seg, SentenceSegment::Word(_)));
    if !has_word {
        return None;
    }

    Some(CorpusSentence {
        index,
        hanji: item.hanji,
        tailo: item.tailo,
        dic_url: item.dic_url,
        segments,
    })
}

fn encode_word(raw_display: &str) -> Option<EncodedWord> {
    let trimmed = raw_display.trim_matches('-');
    if trimmed.is_empty() {
        return None;
    }
    // Preserve `--` khinsiann boundaries across `ExternalLookupDigitForm`.
    let mut syllables = Vec::new();
    let mut digit_groups = Vec::new();
    for (group_idx, group) in trimmed.split("--").enumerate() {
        if group.is_empty() {
            continue;
        }
        let digit_group = external_lookup_digit_form(group)?;
        if digit_group.is_empty() {
            continue;
        }
        for (syl_idx, syl) in digit_group.split('-').enumerate() {
            if syl.is_empty() {
                continue;
            }
            let (toneless_tl, tone_digit) = split_trailing_tone(syl);
            if toneless_tl.is_empty() || !toneless_tl.chars().all(|c| c.is_ascii_alphabetic()) {
                continue;
            }
            let toneless_poj = derive_poj_ascii_keys(&toneless_tl);
            syllables.push(SyllableSpec {
                toneless_tl,
                toneless_poj,
                tone_digit,
                khinsiann_before: group_idx > 0 && syl_idx == 0,
            });
        }
        digit_groups.push(digit_group);
    }
    if syllables.is_empty() {
        return None;
    }
    let digit_tl = digit_groups.join("--");
    let tps_keys = derive_tps_strokes(trimmed);
    Some(EncodedWord {
        display_tl: trimmed.to_owned(),
        digit_tl,
        syllables,
        tps_keys,
    })
}

fn split_trailing_tone(syl: &str) -> (String, Option<u8>) {
    if let Some(last) = syl.as_bytes().last().copied() {
        if (b'1'..=b'9').contains(&last) {
            return (syl[..syl.len() - 1].to_owned(), Some(last - b'0'));
        }
    }
    (syl.to_owned(), None)
}

fn derive_poj_ascii_keys(toneless_tl: &str) -> String {
    let Some(poj) = tl_to_poj(toneless_tl) else {
        return toneless_tl.to_owned();
    };
    let mut ascii = String::with_capacity(poj.len() + 2);
    for ch in poj.chars() {
        match ch {
            '\u{0358}' => ascii.push('o'), // `o` + `\u{0358}` -> `oo` double-tap
            '\u{207F}' => ascii.extend(['n', 'n']),
            c if c.is_ascii_alphabetic() => ascii.push(c.to_ascii_lowercase()),
            _ => {}
        }
    }
    if ascii.is_empty() {
        toneless_tl.to_owned()
    } else {
        ascii
    }
}

fn derive_tps_strokes(display_tl: &str) -> Vec<TpsStroke> {
    let Some(tps_text) = tl_display_to_tps(display_tl, TPS_OR_MAPS_TO_ER) else {
        return Vec::new();
    };
    let mut strokes = Vec::new();
    // Split by hyphen / khinsiann so unmarked tone 1/4 syllables get a Space pin
    // before the next syllable when needed.
    let parts: Vec<&str> = tps_text.split('-').collect();
    for (idx, part) in parts.iter().enumerate() {
        if idx > 0 {
            // `-` is typed with `=` on the TPS layout (`--` -> `==`).
            strokes.push(TpsStroke {
                ch: '=',
                shift: false,
            });
        }
        if part.is_empty() {
            continue;
        }
        let mut last_was_tone = false;
        for ch in part.chars() {
            let mut buf = [0u8; 4];
            let needle = ch.encode_utf8(&mut buf);
            if let Some(&(key_ch, _)) = TPS_LAYOUT_KEYS.iter().find(|(_, glyph)| *glyph == needle) {
                let shift = key_ch.is_ascii_uppercase()
                    || matches!(key_ch, '!' | '*' | '>' | '(' | ')' | '_' | '?' | '#' | '^');
                let base_ch = key_ch;
                strokes.push(TpsStroke { ch: base_ch, shift });
                last_was_tone = matches!(
                    ch,
                    '\u{02cb}'
                        | '\u{02ea}'
                        | '\u{02ca}'
                        | '\u{02c7}'
                        | '\u{02eb}'
                        | '\u{02d9}'
                        | '\u{02c6}'
                );
            }
        }
        // Tone 1 / 4 in TPS carries no explicit mark; Space pins the syllable's unmarked tone.
        if !last_was_tone && idx + 1 < parts.len() && !parts[idx + 1].is_empty() {
            strokes.push(TpsStroke {
                ch: ' ',
                shift: false,
            });
        }
    }
    strokes
}

/// Authoritative Telex key for a tone digit (`telex_guide_rows.rs` `ROWS`).
pub fn telex_key_for_tone(tone: Option<u8>, explicit_tone_1_4: bool) -> Option<char> {
    match tone {
        None | Some(1) | Some(4) => explicit_tone_1_4.then_some('x'),
        Some(2) | Some(8) => Some('v'),
        Some(3) => Some('y'),
        Some(5) => Some('d'),
        Some(6) => Some('z'),
        Some(7) => Some('w'),
        Some(9) => Some('q'),
        _ => None,
    }
}

fn synthetic_items() -> Vec<RawCorpusItem> {
    let pairs = [
        (
            "今仔日日頭真炎，愛記得啉水。",
            "Kin-á-ji̍t ji̍t-thâu tsin iām, ài kì-tit lim tsuí.",
        ),
        (
            "阿公佇門口埕泡茶配土豆。",
            "A-kong tī mn̂g-kháu-tiânn phàu tê phuè thôo-tāu.",
        ),
        (
            "食飽未？今仔日天氣真好。",
            "Tsia̍h-pá--buē? Kin-á-ji̍t thinn-khì tsin hó.",
        ),
        (
            "台灣是一個真媠的島嶼。",
            "Tâi-uân sī tsi̍t ê tsin suí ê tó-sū.",
        ),
        (
            "大家來講台語，愈講愈輪轉。",
            "Ta̍k-ke lâi kóng Tâi-gí, jú kóng jú lūn-tsuán.",
        ),
        ("阿母佇灶跤煮暗頓。", "A-bú tī tsàu-kha tsú àm-tǹg."),
        (
            "緊轉來食飯，莫閣耍矣！",
            "Kín tńg--lâi tsia̍h-pn̄g, mài koh sńg--ah!",
        ),
        (
            "這領衫穿起來真四序。",
            "Tsit niá sann tshīng--khí-lâi tsin sù-sī.",
        ),
        (
            "外口咧落雨，出門愛記得紮雨傘。",
            "Guā-kháu teh lo̍h-hōo, tshut-mn̂g ài kì-tit tsah hōo-suànn.",
        ),
        (
            "囡仔人有耳無喙，恬恬仔聽。",
            "Gín-á-lâng ū hīnn bô tshuì, tiām-tiām-á thiann.",
        ),
        (
            "你有閒無？咱來去啉咖啡。",
            "Lí ū-îng--bô? Lán lâi-khì lim ka-phi.",
        ),
        (
            "火車欲開矣，行較緊咧。",
            "Hué-tshia beh khui--ah, kiânn khah kín--leh.",
        ),
    ];
    pairs
        .into_iter()
        .enumerate()
        .map(|(idx, (hanji, tailo))| RawCorpusItem {
            hanji: hanji.to_owned(),
            tailo: tailo.to_owned(),
            dic_url: format!("synthetic://{idx}"),
        })
        .collect()
}
