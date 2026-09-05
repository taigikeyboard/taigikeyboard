//! TPS / Zhuyin conversion — ported from `taigi-converter/src/zhuyin.js`.
//! The Zhuyin lookup tables and TPS punctuation map live in this module
//! because TPS is the only domain that owns them: this file consumes
//! all six tables and `tps_adjust` consumes the two tone tables. Keeping
//! them next to their primary consumer keeps `tables.rs` focused on
//! cross-module shared data (TL / tone diacritics).

// TPS / 注音雙向轉換 (TL ↔ TPS) 與表音符號偵測;六張查找表都收在此檔,因為只有 TPS 領域使用。

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Lookup tables — ported 1:1 from `taigi-converter/src/zhuyin.js`. Order is
// significant: longer keys appear first so `tsh` matches before `t`.
// ---------------------------------------------------------------------------

pub(crate) const ZHUYIN_INITIALS: &[(&str, &str)] = &[
    ("tshi", "\u{3111}\u{3127}"),
    ("tsi", "\u{3110}\u{3127}"),
    ("tsh", "\u{3118}"),
    ("ph", "\u{3106}"),
    ("th", "\u{310a}"),
    ("ts", "\u{3117}"),
    ("si", "\u{3112}\u{3127}"),
    ("ji", "\u{31a2}\u{3127}"),
    ("kh", "\u{310e}"),
    ("ng", "\u{312b}"),
    ("p", "\u{3105}"),
    ("m", "\u{3107}"),
    ("b", "\u{31a0}"),
    ("t", "\u{3109}"),
    ("n", "\u{310b}"),
    ("l", "\u{310c}"),
    ("s", "\u{3119}"),
    ("j", "\u{31a1}"),
    ("k", "\u{310d}"),
    ("g", "\u{31a3}"),
    ("h", "\u{310f}"),
];

pub(crate) const ZHUYIN_VOWELS: &[(&str, &str)] = &[
    ("ainn", "\u{31ae}"),
    ("aunn", "\u{31af}"),
    ("ann", "\u{31a9}"),
    ("enn", "\u{31a5}"),
    ("inn", "\u{31aa}"),
    ("onn", "\u{31a7}"),
    ("unn", "\u{31ab}"),
    ("ang", "\u{3124}"),
    ("ong", "\u{31b2}"),
    ("oo", "\u{31a6}"),
    ("ee", "\u{311d}"),
    ("er", "\u{311c}"),
    // `or` defaults to ㄛ at runtime; this ㄜ entry is the toggle-ON form
    // selected by `to_zhuyin(_, _, or_maps_to_er = true)`.
    ("or", "\u{311c}"),
    ("ir", "\u{31a8}"),
    ("ai", "\u{311e}"),
    ("au", "\u{3120}"),
    ("am", "\u{31b0}"),
    ("an", "\u{3122}"),
    ("om", "\u{31b1}"),
    ("ng", "\u{31ad}"),
    ("a", "\u{311a}"),
    ("e", "\u{31a4}"),
    ("i", "\u{3127}"),
    ("o", "\u{311b}"),
    ("u", "\u{3128}"),
    ("m", "\u{31ac}"),
    ("n", "\u{3123}"),
];

/// True when `c` is a TPS nucleus glyph that can START a syllable body
/// after an onset — a base vowel / medial / nasalized vowel / precomposed
/// nasal-coda final (ㄚㄧㄨㄛㆦㆤㄜㄝㆨㄞㄠ, the ㆩㆥㆪㆧㆫㆮㆯ nasalized set,
/// and the ㆰㄢㄤㆱㆲ am/an/ang/om/ong finals). Derived from
/// [`ZHUYIN_VOWELS`] minus the three syllabic/coda-nasal forms `ㆬ`(m) /
/// `ㄣ`(n) / `ㆭ`(ng), which are codas, not nuclei.
///
/// Used by the continuous-input de-fold predicate (`composing::shadow`):
/// a folded coda glyph is only de-folded to an onset when the FOLLOWING
/// char is vowel material — i.e. the coda is positioned where a real
/// onset could begin the next syllable (`ㄍㆤㆷ|ㄧㄥ`), never before a tone
/// mark, separator, or another coda.
// 判斷 c 是否為「可接在聲母後起始音節主體」的 TPS 韻核 glyph(母音/介音/鼻化母音/
//   precomposed 鼻韻尾 am/an/ang/om/ong)。源自 ZHUYIN_VOWELS 扣掉三個自鳴/韻尾鼻音
//   ㆬ(m)/ㄣ(n)/ㆭ(ng)。供 de-fold predicate:韻尾後接韻核才反摺,絕不在聲調符/分隔符/韻尾前反摺。
pub fn is_tps_vowel_material(c: char) -> bool {
    // The syllabic / coda nasal forms are codas, not nuclei — exclude them
    // even though they live in `ZHUYIN_VOWELS`.
    if matches!(c, '\u{31ac}' | '\u{3123}' | '\u{31ad}') {
        return false;
    }
    ZHUYIN_VOWELS
        .iter()
        .any(|(_, glyph)| glyph.chars().next() == Some(c))
}

pub(crate) const ZHUYIN_TONES: &[(&str, &str)] = &[
    ("1", " "),
    ("2", "\u{02cb}"),
    ("3", "\u{02ea}"),
    ("p4", "\u{31b4}"),
    ("t4", "\u{31b5}"),
    ("k4", "\u{31bb}"),
    ("h4", "\u{31b7}"),
    ("5", "\u{02ca}"),
    ("6", "\u{02c7}"),
    ("7", "\u{02eb}"),
    ("p8", "\u{31b4}\u{0307}"),
    ("t8", "\u{31b5}\u{0307}"),
    ("k8", "\u{31bb}\u{0307}"),
    ("h8", "\u{31b7}\u{0307}"),
    ("8", "\u{0307}"),
    ("9", "\u{02c6}"),
];

pub(crate) const ZHUYIN_TONES_ENCODE_SAFE: &[(&str, &str)] = &[
    ("1", " "),
    ("2", "\u{02cb}"),
    ("3", "\u{02ea}"),
    ("p4", "\u{31b4}"),
    ("t4", "\u{31b5}"),
    ("k4", "\u{31bb}"),
    ("h4", "\u{31b7}"),
    ("5", "\u{02ca}"),
    ("6", "\u{02c7}"),
    ("7", "\u{02eb}"),
    ("p8", "\u{31b4}\u{02d9}"),
    ("t8", "\u{31b5}\u{02d9}"),
    ("k8", "\u{31bb}\u{02d9}"),
    ("h8", "\u{31b7}\u{02d9}"),
    ("8", "\u{02d9}"),
    ("9", "\u{02c6}"),
];

const PUNCTUATION_CHARS: &[&str] = &[
    "\u{ff0e}", "\u{300c}", "\u{300d}", "\u{ff0c}", "\u{3002}", "\u{ff1f}", "--", ",", ".", "?",
    "\"",
];

const PUNCTUATION_PAIRS: &[(&str, &str)] = &[
    ("\u{3002}", ". "),
    ("\u{3002}", "."),
    ("\u{300c}", "\""),
    ("\u{300d}", "\""),
    ("\u{ff0c}", ", "),
    ("\u{ff0c}", ","),
    ("\u{ff1f}", "? "),
    ("\u{ff1f}", "?"),
    ("\u{ff0e}", "\u{00b7} "),
    ("\u{ff0e}", "\u{00b7}"),
];

static ZHUYIN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("[\u{3100}-\u{312f}\u{31a0}-\u{31bf}]").unwrap());

// Reverse-direction tables (TPS → TL). Built once at first use; `from_zhuyin`
// previously rebuilt these on every call (~150 String allocations per syllable).
static REV_INITIALS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut rev: Vec<(&'static str, &'static str)> = ZHUYIN_INITIALS
        .iter()
        .map(|(tl, tps)| (*tps, *tl))
        .collect();
    rev.extend([
        ("\u{3110}", "ts"),
        ("\u{3111}", "tsh"),
        ("\u{3112}", "s"),
        ("\u{31a2}", "j"),
    ]);
    rev.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    rev
});

static REV_VOWELS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut rev: Vec<(&'static str, &'static str)> =
        ZHUYIN_VOWELS.iter().map(|(tl, tps)| (*tps, *tl)).collect();
    rev.push(("\u{3125}", "ng"));
    rev.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    rev
});

static REV_TONES: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, &'static str> = HashMap::new();
    for table in [ZHUYIN_TONES, ZHUYIN_TONES_ENCODE_SAFE] {
        for (tl, tps) in table {
            map.entry(*tps).or_insert(*tl);
        }
    }
    let mut entries: Vec<(&'static str, &'static str)> = map.into_iter().collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    entries
});

pub(crate) fn is_zhuyin(text: &str) -> bool {
    ZHUYIN_RE.is_match(text)
}

/// True when `ch` lies in the Bopomofo (U+3100–U+312F) or Bopomofo
/// Extended (U+31A0–U+31BF) block — the same range `ZHUYIN_RE` scans.
/// Single-`char` companion to [`is_zhuyin`] for callers that walk a
/// stream char-by-char (the composing TPS syllabifier) and must not
/// allocate a `&str` per code point.
// 判斷單一字元是否落在注音 / 注音擴充區塊 (與 ZHUYIN_RE 同範圍);供逐字掃描的呼叫端用,免每字配置字串。
pub fn is_tps_char(ch: char) -> bool {
    matches!(ch, '\u{3100}'..='\u{312f}' | '\u{31a0}'..='\u{31bf}')
}

/// True when `ch` is the leading consonant of a TPS initial. Derived
/// from [`ZHUYIN_INITIALS`] (the first `char` of each Bopomofo value),
/// so it tracks edits to that table with no parallel const set to keep
/// in sync. The four `REV_INITIALS` extras (ㄐ ㄑ ㄒ ㆢ) are already the
/// first chars of the `tsi` / `tshi` / `si` / `ji` two-symbol initials,
/// so iterating `ZHUYIN_INITIALS` alone covers the full set.
///
/// Used by the composing TPS syllabifier's "next initial seen" rule to
/// infer a tone-1 syllable boundary: an initial appearing after a
/// nucleus has been consumed starts a new syllable.
// 判斷字元是否為 TPS 聲母的首字;由 ZHUYIN_INITIALS 推導 (取每筆注音值首字),無平行常數表需同步維護。
// REV_INITIALS 的四個額外項 (ㄐㄑㄒㆢ) 本就是 tsi/tshi/si/ji 兩符聲母的首字,故只走 ZHUYIN_INITIALS 即涵蓋全集。
pub fn is_tps_initial(ch: char) -> bool {
    ZHUYIN_INITIALS.iter().any(|(_, tps)| tps.starts_with(ch))
}

/// True when `body` is non-empty and every char is a TPS initial glyph —
/// the shape of a `tps_abbrev` acronym key (one leading consonant per
/// syllable, e.g. `ㄍㄅ` for `ka-pi`, `ㄐㄅ` for a `tsi-p…` word since
/// [`tps_abbrev_from_tl`] keeps only the leading glyph `ㄐ`). A full
/// syllabic reading always carries a vowel / medial (家 → `ㄍㄚ`) or a
/// coda / syllabic-nasal glyph (毋 → `ㆬ`), none of which are initials, so
/// a real reading is never flagged.
///
/// Used by the continuous partial-prefix path to drop acronym key
/// surfaces *before* the hydrate cap: Bopomofo orders all initials
/// (consonants `U+3105..U+3119`) ahead of all vowels (`U+311A+`), so the
/// `tps_abbrev` keys byte-sort in front of every full-reading key and
/// would otherwise consume the whole budget, starving single-char
/// readings out of the candidate pool.
///
/// Deliberately conservative: a **vowel-initial** word's abbrev (e.g.
/// `ㄚㄅ` for an `a-…` first syllable) starts with a vowel glyph and is
/// NOT flagged here. Fully separating the `tps_abbrev` family from the
/// continuous lookup needs an FST family tag (out of scope for this
/// engine-only fix). The record-level guard
/// `lexicon::continuous::matches_continuous_tps_toneless_prefix_key`
/// still validates every surviving rowid.
// body 非空且每個字皆 TPS 聲母字 = tps_abbrev 縮寫形狀 (每音節留首字聲母,
//   顎化 tsi→ㄐ 也是聲母)。完整讀音必帶母音/介音或自鳴/韻尾鼻音字 (非聲母),
//   故不誤判。供連續 partial-prefix 在 hydrate cap 之前剔除縮寫 key surface
//   (注音子音 byte 序在母音前,縮寫 key 會把完整讀音單字擠出預算)。
// 刻意保守:母音開頭詞的縮寫 (如 ㄚㄅ) 不在此剔除;完整分離 tps_abbrev 家族需
//   FST family tag (本 engine-only 修法範圍外)。record 層 guard 仍逐一驗證存活 rowid。
pub fn is_tps_initial_only(body: &str) -> bool {
    !body.is_empty() && body.chars().all(is_tps_initial)
}

/// Standalone TPS tone marks: `\u{02c6}` ˆ tone-9, `\u{02c7}` ˇ tone-6,
/// `\u{02ca}` ́ tone-5, `\u{02cb}` ̀ tone-2, `\u{02d9}` ˙ encode-safe
/// tone-8 dot, `\u{02ea}` ˪ tone-3, `\u{02eb}` ˫ tone-7, `\u{0307}`
/// combining dot for tone-8. Stop codas (`\u{31b4-7,b}`) are part of the
/// toneless syllable body, not tone marks proper — tone-4 stop syllables
/// carry no trailing mark, tone-8 stops carry a trailing dot.
///
/// v3.5.9 D / C-3b — public so `composing::shadow::strip_tones_for_mode`
/// and `lexicon::continuous::matches_continuous_tps_toneless_key` can
/// reuse the single canonical set. Pre-C-3b this was private with an
/// `is_tps_tone_mark_pub` thunk re-export; simplifier-flagged
/// triple-naming collapsed to one public symbol.
// TPS 聲調符號集合;入聲韻尾本身屬音節主體,聲調 4 無尾標,聲調 8 在韻尾後加點。
// D / C-3b — 改 pub,讓 composing::shadow / lexicon::continuous 共用單一真相;
//   砍掉之前 thunk 三重命名(simplifier 提醒)。
pub fn is_tps_tone_mark(ch: char) -> bool {
    matches!(
        ch,
        '\u{02c6}'
            | '\u{02c7}'
            | '\u{02ca}'
            | '\u{02cb}'
            | '\u{02d9}'
            | '\u{02ea}'
            | '\u{02eb}'
            | '\u{0307}'
    )
}

/// Canonicalize a single TPS tone-8 scalar: platform keyboards type the
/// standalone modifier-letter dot `U+02D9` (˙), but the build pipeline
/// stores tone-8 as the combining dot above `U+0307` in every `tps:<tps_num>`
/// FST key (see [`ZHUYIN_TONES`] vs [`ZHUYIN_TONES_ENCODE_SAFE`]). Map
/// `U+02D9 → U+0307` so a raw-buffer TPS span matches the stored toned key;
/// every other char (including the already-combining `U+0307`) passes through.
///
/// Char-level shared source for the key-shaping sites that build a `tps:` key
/// from raw buffer input: `lexicon::key_normalizer` (Tab3 search),
/// `custom_search` (custom-dictionary keys), and `composing::shadow`
/// (continuous explicit-tone key). The continuous syllabifier probe
/// (`composing::syllabifier::tps`) applies the same `U+02D9 → U+0307`
/// substitution inline at the string level (allocating only when the buffer
/// contains `U+02D9`) — same mapping, kept separate for that hot-path's
/// allocation tuning.
// TPS tone-8 調號正規化 — 鍵盤打獨立點 U+02D9,build pipeline 的 tps:<tps_num> 鍵用組合點 U+0307;
//   把 U+02D9 換成 U+0307 讓原始 buffer 對齊已存 toned key,其餘字元原樣通過。
//   char 級共用源:三個由 raw buffer 組 tps: 鍵的點 (key_normalizer / custom_search / shadow);
//   syllabifier probe 為熱路徑配置調校,以字串級 inline 做同一替換 (僅含 U+02D9 才配置)。
pub fn normalize_tps_tone8_scalar(ch: char) -> char {
    if ch == '\u{02d9}' {
        '\u{0307}'
    } else {
        ch
    }
}

/// Split a TPS syllable token into `(canonical_toneless, tone_mark)`.
///
/// Mirrors [`crate::canonicalize_syllable`] (TL) and
/// [`crate::canonicalize_poj_syllable`] (POJ) for the TPS family of
/// `syllables.fst`. The input is one TPS syllable already produced by
/// the dictionary build pipeline (`convert(_, "tl", "zhuyin")`), so
/// the phonotactic validity is guaranteed upstream by the TL CSV;
/// this function only:
///
/// 1. rejects empty or non-Bopomofo inputs,
/// 2. rejects any non-Bopomofo / non-tone-mark char in the body,
/// 3. rejects internal tone marks (only the trailing char may be a
///    tone — e.g. `ㄅˋˊ` is rejected),
/// 4. splits the trailing tone mark off if present,
/// 5. returns the toneless TPS body + the tone mark (empty for
///    tone-1).
///
/// Returns `None` on any of the above rejection conditions.
// 為 syllables.fst 的 TPS 家族切出 (toneless, tone);輸入由 Python 端透過
//   `convert(_, "tl", "zhuyin")` 預先產生,音韻有效性上游已保證,此處
//   再做 (1) 注音範圍守門 (2) 內部不准混入 ASCII 等非注音 (3) 聲調符號
//   僅允許出現於末位 — 防呼叫端誤餵其他形態 token。
pub fn canonicalize_tps_syllable(token: &str) -> Option<(String, String)> {
    if token.is_empty() {
        return None;
    }
    let chars: Vec<char> = token.chars().collect();
    let first = *chars.first()?;
    if !is_tps_char(first) {
        return None;
    }
    let last = *chars.last()?;
    let body_end = if is_tps_tone_mark(last) {
        chars.len() - 1
    } else {
        chars.len()
    };
    if body_end == 0 {
        return None;
    }
    // Body chars must all be Bopomofo — no ASCII letter / digit / stray
    // tone marks inside. The trailing tone (if any) was already split
    // off above.
    for ch in &chars[..body_end] {
        if !is_tps_char(*ch) {
            return None;
        }
    }
    let tone_len = if body_end < chars.len() {
        last.len_utf8()
    } else {
        0
    };
    let toneless_len = token.len() - tone_len;
    let toneless = &token[..toneless_len];
    let tone = &token[toneless_len..];
    Some((toneless.to_string(), tone.to_string()))
}

/// v3.5.9 D / C-3b — runtime mirror of the build pipeline chain
/// `dictionary/build/merge_csv.py:251-265`:
///   `tps_num   = concat( convert_tl_to_tps_strict(tok) for tok in tl.split([-,ws]) )`
///   `tps_notone = remove_tps_tone(tps_num)  # strip the 8 tone marks + hyphens + whitespace`
///
/// Used by `lexicon::continuous::matches_continuous_tps_toneless_key` to
/// derive the expected `tps_notone` surface from a dictionary record's
/// `tl` field at runtime, so a continuous-input `tps:<body>` walker key
/// can be rejected when the FST hit is actually a `tps_abbrev` collision
/// (the same shape of guard B-2 added for `tl:` and `poj:`).
///
/// Per-syllable: TL tokens are split on `-` and whitespace (mirrors the
/// Python `re.split(r"[-\s]+", tl)`), each token is converted via
/// [`crate::to_tone_number`] to numeric-tone form (the Node bridge accepts
/// diacritic input but our Rust [`to_zhuyin`] expects numeric), passed to
/// [`to_zhuyin`] with `encode_safe = false` + `or_maps_to_er = true`
/// (the Node bridge default — `convert_tl_to_tps_strict` builds the
/// primary `tps_notone` column with this toggle), then tone marks +
/// hyphen + whitespace dropped via [`is_tps_tone_mark`] + the
/// `_TPS_TONE_AND_SEP_RE` regex equivalent. The resulting tokens
/// concatenate fused (no separator), exactly what `merge_csv.py`'s
/// `"".join(tps_per_syllable)` produces.
///
/// Encoding-only — no phonotactic gating — same posture as the POJ analog
/// (`engine/lexicon/src/continuous.rs::derive_poj_notone_for_match`): the
/// build pipeline does not gate, gating here would silently reject any
/// legitimate dictionary row whose TL shape the Rust port misses but the
/// Node bridge accepts. The non-golden parity test
/// `engine/lexicon/tests/tps_notone_parity.rs` pins runtime derivation
/// against `dictionary/output/dictionary.csv` row-for-row (C-5 scope) so
/// drift is caught loud.
// D / C-3b — `tps_notone` 的 runtime mirror。對應 build pipeline
//   merge_csv.py 既有鏈:逐音節 TL → numeric tone → to_zhuyin → 去 tone marks → concat。
//   給 lexicon::continuous::matches_continuous_tps_toneless_key 使用,
//   濾掉 tps_abbrev 碰撞 (與 B-2 為 tl:/poj: 加上的 guard 同 shape)。
// encoding-only 不做 phonotactic gate(姿態與 derive_poj_notone_for_match 一致),
//   建置端不 gate,runtime gate 會誤殺合法 dict 行。
pub fn tps_notone_from_tl(record_tl: &str) -> String {
    tps_notone_collecting(record_tl, None)
}

/// [`tps_notone_from_tl`] plus the byte offset each syllable ENDS at in the
/// returned string. Sibling of [`crate::tl_num_syllable_ends_from_tl`]; a
/// caller measuring how far a typed prefix reaches into a reading needs the
/// boundaries, not the syllables themselves.
// tps_notone_from_tl + 每個音節的結束位移;量測輸入前綴走多遠的呼叫端
//   需要的是邊界而不是音節本身。
pub fn tps_notone_syllable_ends_from_tl(record_tl: &str) -> (String, Vec<u32>) {
    let mut ends = Vec::new();
    let notone = tps_notone_collecting(record_tl, Some(&mut ends));
    (notone, ends)
}

fn tps_notone_collecting(record_tl: &str, mut ends: Option<&mut Vec<u32>>) -> String {
    let mut out = String::with_capacity(record_tl.len() * 3);
    for token in tl_syllable_tokens(record_tl) {
        let numeric = crate::api::to_tone_number(token);
        // `or_maps_to_er = true` matches the build pipeline default. The
        // build chain calls `convert_tl_to_tps_strict` (Node bridge with
        // its `or_maps_to_er` default ON), which emits ㄜ for both TL
        // `er` and `or` tokens — i.e. the `tps_notone` column in
        // `dictionary.csv` always carries the er-glyph form. The
        // separate `tps_notone_var` column (built by C-3a's
        // `apply_or_dialect_variant`) carries the ㄛ alternate; this
        // runtime helper produces only the primary `tps_notone` form
        // because the guard compares against that column.
        // or_maps_to_er=true 對齊 build pipeline 預設 — Node bridge 預設將 TL er/or 都映射為 ㄜ;
        //   `tps_notone` 欄一律 ㄜ-glyph,ㄛ 變體在 `tps_notone_var` (C-3a),
        //   此 runtime helper 只需產出主欄即可比對 guard。
        let before = out.len();
        out.push_str(&tps_notone_from_numeric_token(&numeric));
        if out.len() != before {
            if let Some(ends) = ends.as_deref_mut() {
                ends.push(out.len() as u32);
            }
        }
    }
    out
}

/// The TL syllable tokens of a record reading. Splits on the three
/// separators the build pipeline treats as syllable boundaries (ASCII
/// hyphen, space, tab) and drops the empty runs a 輕聲 `--` produces.
/// Single source for every per-token TPS derivation below.
// record reading 的 TL 音節 token — 依 build pipeline 的三種分隔符 (連字號/空白/tab) 切,
//   丟掉輕聲 `--` 產生的空 token。以下逐音節 TPS 衍生皆共用此來源。
pub(crate) fn tl_syllable_tokens(record_tl: &str) -> impl Iterator<Item = &str> {
    record_tl.split(['-', ' ', '\t']).filter(|t| !t.is_empty())
}

/// One numeric-tone TL token → its fused TPS notone surface.
///
/// `_TPS_TONE_AND_SEP_RE` in `dictionary/common/notone.py:43` strips not
/// only the 8 tone marks but ALSO ASCII hyphen + any whitespace.
/// [`to_zhuyin`] for tone-1 inputs emits a trailing space marker; without
/// the whitespace strip the runtime derivation gains a stray ` `
/// (U+0020) that the build pipeline's `tps_notone` column does not have.
/// Match the regex exactly to keep runtime ↔ build pipeline byte-identical.
// 單一 numeric TL token → fused TPS 去調面。notone.py 的 _TPS_TONE_AND_SEP_RE 同時剝
//   聲調符號 + ASCII 連字號 + 空白;to_zhuyin 對第 1 聲會帶尾空白,須一併剝除。
fn tps_notone_from_numeric_token(numeric_token: &str) -> String {
    to_zhuyin(numeric_token, false, true)
        .chars()
        .filter(|&ch| !is_tps_tone_mark(ch) && ch != '-' && !ch.is_whitespace())
        .collect()
}

/// A3 (§41) — the numeric tone of the TL syllable whose fused TPS notone
/// prefix ends EXACTLY at `notone_prefix`, or `None` when no syllable
/// boundary lands there.
///
/// This is the alignment primitive the space-pinned tone filter needs: a
/// TPS keyboard space closes the syllable the user just typed, so the
/// candidate is only eligible when its reading has a boundary at that
/// same point AND the syllable ending there carries the tone the space
/// means (1 for an open rime, 4 for a stop coda — the two tones TPS
/// writes with no mark). Callers own the tone predicate; this fn only
/// answers "which syllable does the typed prefix end on, and what is its
/// tone".
///
/// Accepts the C-3a or→er dialect variant of the accumulated prefix for
/// the same reason [`matches_continuous_tps_toneless_key`] does: a user
/// typing the ㄛ form reaches the row through `tps_notone_var`, so a
/// primary-only comparison would reject a legitimate hit.
// A3 (§41) — 回傳「fused TPS 去調前綴剛好在 notone_prefix 收尾」的那個 TL 音節的數字聲調;
//   無音節邊界落在該處回 None。TPS 空白關閉剛打完的音節,故候選必須在同一點有音節邊界,
//   且該音節的聲調等於空白所代表的無調號調(開音節 1、入聲尾 4)。聲調判斷留給呼叫端。
//   同 matches_continuous_tps_toneless_key,接受 C-3a or→er 變體形(使用者打 ㄛ 形經 var 鍵命中)。
pub fn tps_notone_prefix_boundary_tone(record_tl: &str, notone_prefix: &str) -> Option<char> {
    if notone_prefix.is_empty() {
        return None;
    }
    let mut accumulated = String::with_capacity(notone_prefix.len());
    for token in tl_syllable_tokens(record_tl) {
        let numeric = crate::api::to_tone_number(token);
        let tone = numeric.chars().next_back().filter(char::is_ascii_digit)?;
        accumulated.push_str(&tps_notone_from_numeric_token(&numeric));
        if accumulated.len() > notone_prefix.len() {
            return None;
        }
        let variant = tps_notone_or_variant(&accumulated);
        if accumulated == notone_prefix || (!variant.is_empty() && variant == notone_prefix) {
            return Some(tone);
        }
    }
    None
}

/// v3.5.9 D / C-3b — runtime mirror of
/// `dictionary/common/notone.py::apply_or_dialect_variant`. Returns the
/// `or_maps_to_er = false` variant of a primary TPS notone surface by
/// substituting every ㄜ (U+311C) with ㄛ (U+311B). Returns an empty
/// string when the input has no ㄜ (callers skip a redundant variant
/// emit per build pipeline behavior).
///
/// Why this lives here at runtime: the C-3a build pipeline dual-emits
/// `tps:<tps_notone>` AND `tps:<tps_notone_var>` per row whose primary
/// form contains ㄜ. The continuous-input toneless guard
/// (`lexicon::matches_continuous_tps_toneless_key`) must therefore
/// accept BOTH forms — otherwise a user typing the ㄛ form (e.g.
/// `ㄉㄛ` for TL `tor`/`tór`) hits the FST via `tps_notone_var` but
/// the guard's primary-only derivation rejects it as an
/// abbrev-collision. Codex post-impl BLOCK 2026-05-25.
// D / C-3b — `apply_or_dialect_variant` 的 Rust runtime 鏡像。
//   C-3a build pipeline 對含 ㄜ 的 row dual-emit `tps_notone` + `tps_notone_var`
//   (ㄜ→ㄛ);連續輸入 guard 必須同時接受兩形,否則用戶打 ㄛ 形會被誤殺。
pub fn tps_notone_or_variant(notone: &str) -> String {
    if !notone.contains('\u{311c}') {
        return String::new();
    }
    notone.replace('\u{311c}', "\u{311b}")
}

/// v3.6.1 R3 — TPS num (tone-marked, fused) sibling of [`tps_notone_from_tl`].
/// Per-token TL → numeric → [`to_zhuyin`], KEEPING the tone marks and dropping
/// only hyphen + whitespace so the fused form matches a TPS continuous input
/// that carries tone marks. Used as the `tps:num` custom-dictionary search key.
// R3 — tps_notone_from_tl 的「保留聲調符號」版本,供自訂詞 tps:num 搜尋鍵;
//   逐音節 TL → numeric → to_zhuyin,只剝連字號 / 空白,聲調符號保留。
pub fn tps_num_from_tl(record_tl: &str) -> String {
    tps_num_collecting(record_tl, None)
}

/// [`tps_num_from_tl`] plus per-syllable end offsets — the tone-marked sibling
/// of [`tps_notone_syllable_ends_from_tl`], same contract.
// tps_num_from_tl + 每個音節的結束位移,契約同 tps_notone_syllable_ends_from_tl。
pub fn tps_num_syllable_ends_from_tl(record_tl: &str) -> (String, Vec<u32>) {
    let mut ends = Vec::new();
    let num = tps_num_collecting(record_tl, Some(&mut ends));
    (num, ends)
}

fn tps_num_collecting(record_tl: &str, mut ends: Option<&mut Vec<u32>>) -> String {
    let mut out = String::with_capacity(record_tl.len() * 3);
    for token in tl_syllable_tokens(record_tl) {
        let numeric = crate::api::to_tone_number(token);
        let tps = to_zhuyin(&numeric, false, true);
        let before = out.len();
        for ch in tps.chars() {
            if ch == '-' || ch.is_whitespace() {
                continue;
            }
            out.push(ch);
        }
        if out.len() != before {
            if let Some(ends) = ends.as_deref_mut() {
                ends.push(out.len() as u32);
            }
        }
    }
    out
}

/// v3.6.1 R3 — TPS abbrev mirror of build pipeline
/// `dictionary/common/abbrev.py::extract_tps_abbrev`. First Bopomofo glyph
/// (leading initial / vowel) per TL syllable; "" for fewer than 2 syllables.
/// Used as the `tps:abbrev` custom-dictionary search key.
// R3 — extract_tps_abbrev 的 runtime 鏡像;每個 TL 音節取首個注音字母 (聲母/韻母),
//   少於兩音節回空字串。供自訂詞 tps:abbrev 搜尋鍵。
pub fn tps_abbrev_from_tl(record_tl: &str) -> String {
    let syllables: Vec<&str> = record_tl
        .split(['-', ' ', '\t'])
        .filter(|s| !s.is_empty())
        .collect();
    if syllables.len() < 2 {
        return String::new();
    }
    let mut out = String::new();
    for syllable in syllables {
        let numeric = crate::api::to_tone_number(syllable);
        let tps = to_zhuyin(&numeric, false, true);
        match tps
            .chars()
            .find(|&c| c != '-' && !c.is_whitespace() && !is_tps_tone_mark(c))
        {
            Some(glyph) => out.push(glyph),
            None => return String::new(),
        }
    }
    out
}

/// Convert a single TL token (with tone digit) to TPS.
///
/// - `encode_safe = true` substitutes `\u{02d9}` for `\u{0307}` so TPS
///   round-trips through systems that strip combining marks.
/// - `or_maps_to_er = false` (default) renders the vowel `or` as ㄛ
///   (`\u{311b}`); `true` renders it as ㄜ (`\u{311c}`), matching the iOS
///   `orMapsToER` toggle. Override is per-token (only applied when the
///   matched vowel slot is exactly `"or"`); other vowels are unaffected.
///
/// Single-token: caller pre-splits multi-token TL on `[-\s]+`. v3.5.9 D /
/// C-5 widened to `pub` so lexicon parity tests + composing golden
/// fixtures can emit per-syllable tone-marked TPS samples without
/// rebuilding the `Method::TlNumericToTps` proto plumbing.
// 把單一 TL token (含聲調數字) 轉成 TPS;`encode_safe` 用獨立空白點符號讓 TPS 能通過會剝組合符號的系統,`or_maps_to_er` 切換母音 `or` 的渲染。
// D / C-5 — 為 lexicon parity 測試 + composing golden fixture 之需,放寬至 pub;
//   呼叫端負責先用 `[-\s]+` 拆 token 再逐 token 呼叫。
pub fn to_zhuyin(text: &str, encode_safe: bool, or_maps_to_er: bool) -> String {
    let mut remaining: String = text.to_lowercase();
    let mut pre_punct = String::new();
    let mut consonant = String::new();
    let mut vowel = String::new();
    let mut tone = String::new();

    loop {
        let mut matched = false;
        for punct in PUNCTUATION_CHARS {
            if remaining.starts_with(punct) {
                pre_punct.push_str(punct);
                remaining = remaining[punct.len()..].to_string();
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }

    for (tl, tps) in ZHUYIN_INITIALS {
        if remaining.starts_with(tl) {
            // The palatalized sibilant/affricate initials (`tsi`/`tshi`/`si`/
            // `ji`, all ending in the medial `i` → ㄧ) must NOT match when the
            // `i` is actually the head of the `ir` [ɨ] central vowel — i.e. the
            // next char is `r` (`tsir`/`sir`/`tshir`/`jir`). Taking the
            // palatalized form there strands `r` as residue and the `ir` final
            // never forms (the build pipeline then drops the row's `tps_num`).
            // Fall through to the bare initial (`ts`/`tsh`/`s`/`j`) so `ir`
            // matches. `r` is not a TL initial, so `<sibilant>i` + `r` is
            // unambiguously the `ir` vowel. Mirrors `taigi-converter/src/zhuyin.js`.
            if tl.ends_with('i') && remaining[tl.len()..].starts_with('r') {
                continue;
            }
            consonant.push_str(tps);
            remaining = remaining[tl.len()..].to_string();
            break;
        }
    }

    loop {
        let mut matched = false;
        for (tl, tps) in ZHUYIN_VOWELS {
            if remaining.starts_with(tl) {
                // Per-token override: when toggle is OFF, the vowel `or`
                // renders as ㄛ (matches iOS default). With toggle ON, fall
                // through to the table value (ㄜ).
                let effective_tps: &str = if *tl == "or" && !or_maps_to_er {
                    "\u{311b}"
                } else {
                    tps
                };
                vowel.push_str(effective_tps);
                remaining = remaining[tl.len()..].to_string();
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }

    let tone_table: &[(&str, &str)] = if encode_safe {
        ZHUYIN_TONES_ENCODE_SAFE
    } else {
        ZHUYIN_TONES
    };
    for (tl, tps) in tone_table {
        if remaining.starts_with(tl) {
            tone.push_str(tps);
            remaining = remaining[tl.len()..].to_string();
            break;
        }
    }

    // Idiosyncratic TPS adjustments — see zhuyin.js:173-185.
    if vowel.is_empty() && consonant == "\u{3107}" {
        vowel = "\u{31ac}".to_string();
        consonant.clear();
    }
    if vowel.is_empty() && consonant == "\u{312b}" {
        vowel = "\u{31ad}".to_string();
        consonant.clear();
    }
    if vowel == "\u{3125}" && consonant.is_empty() {
        vowel = "\u{31ad}".to_string();
    }
    let cv = format!("{consonant}{vowel}");
    if cv.ends_with('\u{31ad}') && cv.chars().rev().nth(1) == Some('\u{3127}') {
        vowel = vowel.replace('\u{31ad}', "\u{3125}");
    }
    if consonant.ends_with('\u{3127}') && vowel == "\u{3123}\u{3123}" {
        consonant.pop();
        vowel = "\u{31aa}".to_string();
    }
    if vowel.contains('\u{311b}') && !tone.is_empty() {
        if let Some(first) = tone.chars().next() {
            if "\u{31b4}\u{31b5}\u{31bb}".contains(first) {
                vowel = vowel.replace('\u{311b}', "\u{31a6}");
            }
        }
    }

    let mut result = format!("{pre_punct}{consonant}{vowel}{tone}{remaining}");

    loop {
        let mut matched = false;
        for (tps_punct, tl_punct) in PUNCTUATION_PAIRS {
            if result.contains(tl_punct) {
                result = result.replace(tl_punct, tps_punct);
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }

    result.replace("--", "\u{00b7}")
}

/// Convert a TPS string to a TL tone-numbered string. Mirrors `fromZhuyin` in
/// `zhuyin.js`. Word segmentation is **not** performed here — that is the
/// segmenter's job, which belongs with the Lexicon slice.
// 把 TPS 字串轉回 TL 聲調數字形;此處不做斷詞,斷詞屬於 Lexicon 的職責。
pub fn from_zhuyin(text: &str) -> String {
    let rev_punct = [
        ("\u{3002}", "."),
        ("\u{300c}", "\""),
        ("\u{300d}", "\""),
        ("\u{ff0c}", ","),
        ("\u{ff1f}", "?"),
        ("\u{ff0e}", "\u{00b7}"),
    ];
    let mut input = text.to_string();
    for (tps, ascii) in rev_punct {
        input = input.replace(tps, ascii);
    }

    let mut parts: Vec<Part> = Vec::new();
    let mut remaining = input;
    let mut initial = String::new();
    let mut vowel = String::new();

    while !remaining.is_empty() {
        let mut matched = false;

        for (tps, tl) in REV_TONES.iter() {
            if remaining.starts_with(*tps) {
                remaining = remaining[tps.len()..].to_string();
                if !initial.is_empty() || !vowel.is_empty() {
                    let mut syl = format!("{initial}{vowel}");
                    if initial == "m" && vowel == "m" {
                        syl = "m".to_string();
                    }
                    if syl.contains("oo") && is_pt_or_k_stop(tl) {
                        syl = syl.replacen("oo", "o", 1);
                    }
                    parts.push(Part::Syllable(format!("{syl}{tl}")));
                }
                initial.clear();
                vowel.clear();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        if initial.is_empty() && vowel.is_empty() {
            for (tps, tl) in REV_INITIALS.iter() {
                if remaining.starts_with(*tps) {
                    initial = (*tl).to_string();
                    remaining = remaining[tps.len()..].to_string();
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
        }

        for (tps, tl) in REV_VOWELS.iter() {
            if remaining.starts_with(*tps) {
                vowel.push_str(tl);
                remaining = remaining[tps.len()..].to_string();
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        if !initial.is_empty() || !vowel.is_empty() {
            let mut syl = format!("{initial}{vowel}");
            if initial == "m" && vowel == "m" {
                syl = "m".to_string();
            }
            parts.push(Part::Syllable(syl));
            initial.clear();
            vowel.clear();
        }
        if let Some(c) = remaining.chars().next() {
            parts.push(Part::Other(c));
            remaining = remaining[c.len_utf8()..].to_string();
        }
    }

    if !initial.is_empty() || !vowel.is_empty() {
        let mut syl = format!("{initial}{vowel}");
        if initial == "m" && vowel == "m" {
            syl = "m".to_string();
        }
        parts.push(Part::Syllable(syl));
    }

    let mut out = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            if let (Part::Syllable(_), Part::Syllable(_)) = (&parts[i - 1], part) {
                out.push('-');
            }
        }
        match part {
            Part::Syllable(s) => out.push_str(s),
            Part::Other(c) => out.push(*c),
        }
    }
    out
}

enum Part {
    Syllable(String),
    Other(char),
}

fn is_pt_or_k_stop(tl: &str) -> bool {
    matches!(tl, "p4" | "t4" | "k4" | "p8" | "t8" | "k8")
}

#[cfg(test)]
mod tests {
    // A3 (§41) — `tps_notone_prefix_boundary_tone` unit pins. Traces:
    //   "si"    → to_tone_number "si1"    → notone ㄒㄧ,   tone 1 (open rime, unmarked)
    //   "sī"    → to_tone_number "si7"    → notone ㄒㄧ,   tone 7 (marked)
    //   "tsit"  → to_tone_number "tsit4"  → notone ㄐㄧㆵ, tone 4 (stop coda, unmarked)
    //   "tsi̍t"  → to_tone_number "tsit8"  → notone ㄐㄧㆵ, tone 8 (same coda + dot)
    // A3 (§41) — tps_notone_prefix_boundary_tone 單元釘定(上方為逐步推導)。
    #[test]
    fn boundary_tone_reports_unmarked_open_rime_as_tone_one() {
        assert_eq!(tps_notone_prefix_boundary_tone("si", "ㄒㄧ"), Some('1'));
    }

    #[test]
    fn boundary_tone_reports_marked_syllable_tone() {
        assert_eq!(tps_notone_prefix_boundary_tone("sī", "ㄒㄧ"), Some('7'));
        assert_eq!(tps_notone_prefix_boundary_tone("sí", "ㄒㄧ"), Some('2'));
    }

    #[test]
    fn boundary_tone_separates_stop_coda_tone_four_from_tone_eight() {
        // The 這 (tsit4) vs 一 (tsit8) split the bug report hit: identical
        // notone surface, different tone.
        assert_eq!(tps_notone_prefix_boundary_tone("tsit", "ㄐㄧㆵ"), Some('4'));
        assert_eq!(tps_notone_prefix_boundary_tone("tsi̍t", "ㄐㄧㆵ"), Some('8'));
    }

    #[test]
    fn boundary_tone_walks_to_the_syllable_the_prefix_ends_on() {
        // 交代 kau1-tài: a prefix ending on the FIRST syllable reports that
        // syllable's tone (1), the full body reports the last one (3).
        let first = tps_notone_from_tl("kau");
        let full = tps_notone_from_tl("kau-tài");
        assert_eq!(
            tps_notone_prefix_boundary_tone("kau-tài", &first),
            Some('1')
        );
        assert_eq!(tps_notone_prefix_boundary_tone("kau-tài", &full), Some('3'));
    }

    #[test]
    fn boundary_tone_rejects_a_prefix_that_ends_mid_syllable() {
        // ㄍ alone is half of ㄍㄠ — no syllable boundary lands there, so the
        // reading is not eligible for a pin at that point.
        assert_eq!(tps_notone_prefix_boundary_tone("kau-tài", "ㄍ"), None);
    }

    #[test]
    fn boundary_tone_rejects_a_prefix_longer_than_the_reading() {
        let longer = format!("{}{}", tps_notone_from_tl("kau-tài"), "ㄒㄧ");
        assert_eq!(tps_notone_prefix_boundary_tone("kau-tài", &longer), None);
    }

    #[test]
    fn boundary_tone_rejects_an_empty_prefix() {
        assert_eq!(tps_notone_prefix_boundary_tone("si", ""), None);
    }

    #[test]
    fn boundary_tone_accepts_the_or_dialect_variant_surface() {
        // C-3a: a row whose primary notone carries ㄜ is reachable through
        // the ㄛ variant key, so the boundary walk must accept both.
        let primary = tps_notone_from_tl("ter");
        let variant = tps_notone_or_variant(&primary);
        if !variant.is_empty() {
            assert_eq!(
                tps_notone_prefix_boundary_tone("ter", &variant),
                tps_notone_prefix_boundary_tone("ter", &primary),
            );
        }
    }

    use super::*;

    /// Exact-output pin for the `phonetics::tps_to_tl` re-export consumed by
    /// `lexicon::classify_input`. The lexicon-side test only asserts ASCII /
    /// non-raw; this guards the literal romanization.
    #[test]
    fn from_zhuyin_basic_round_trip() {
        assert_eq!(from_zhuyin("ㄉㄧㄠˊ"), "tiau5");
    }

    /// `ir` [ɨ] central vowel after a sibilant/affricate initial must NOT
    /// palatalize — the greedy `tsi`/`tshi`/`si`/`ji` initial used to eat the
    /// `i` and strand `r`, emptying `tps_num` for ~426 dict rows (自 tsir7,
    /// 事 sir7, …). Parity with `taigi-converter/src/zhuyin.js`.
    #[test]
    fn to_zhuyin_ir_after_sibilant_no_palatalization() {
        // ts/tsh/s/j + ir → bare initial + ㆨ (no medial ㄧ, no stray `r`).
        assert_eq!(to_zhuyin("tsir7", false, true), "ㄗㆨ˫");
        assert_eq!(to_zhuyin("sir7", false, true), "ㄙㆨ˫");
        assert_eq!(to_zhuyin("tshir1", false, true), "ㄘㆨ ");
        assert_eq!(to_zhuyin("jir5", false, true), "ㆡㆨˊ");
        // ir-family codas (irinn / irng) ride the same fix.
        assert_eq!(to_zhuyin("tsirinn5", false, true), "ㄗㆨㆪˊ");
        assert_eq!(to_zhuyin("sirng1", false, true), "ㄙㆨㆭ ");
        // Non-sibilant `ir` was always fine; genuine palatalization (non-`r`
        // medial) is unchanged — the guard fires only before `r`.
        assert_eq!(to_zhuyin("kir1", false, true), "ㄍㆨ ");
        assert_eq!(to_zhuyin("tsinn5", false, true), "ㄐㆪˊ");
        assert_eq!(to_zhuyin("tsia2", false, true), "ㄐㄧㄚˋ");
    }

    #[test]
    fn is_tps_vowel_material_accepts_nuclei_rejects_coda_nasals() {
        // Base vowels / medials / nasalized vowels / precomposed nasal-coda
        // finals are nucleus material (can follow a de-folded onset).
        for c in [
            'ㄚ', 'ㄧ', 'ㄨ', 'ㄛ', 'ㆦ', 'ㆤ', 'ㄞ', 'ㄠ', 'ㆩ', 'ㄢ', 'ㄤ',
        ] {
            assert!(is_tps_vowel_material(c), "{c} should be vowel material");
        }
        // The syllabic / coda nasal forms are codas, NOT nuclei.
        for c in ['ㆬ', 'ㄣ', 'ㆭ'] {
            assert!(
                !is_tps_vowel_material(c),
                "{c} (coda nasal) must be rejected"
            );
        }
        // Onsets / stop codas / tone marks / space are not vowel material.
        for c in ['ㄍ', 'ㆷ', 'ㆵ', '\u{02cb}', ' '] {
            assert!(!is_tps_vowel_material(c), "{c} must be rejected");
        }
    }

    #[test]
    fn is_tps_initial_only_flags_abbrev_shapes_keeps_full_readings() {
        // tps_abbrev acronym shapes — every glyph is an initial.
        for body in ["ㄍ", "ㄍㄅ", "ㄍㄅㄐ", "ㄉㄎ", "ㆠㄍ"] {
            assert!(is_tps_initial_only(body), "{body:?} should be initial-only");
        }
        // Full readings always carry a vowel / medial …
        for body in ["ㄍㄚ", "ㄍㆦ", "ㄍㄠ", "ㄐㄧ", "ㄐㄧㄠ"] {
            assert!(
                !is_tps_initial_only(body),
                "{body:?} (full reading) must NOT be initial-only"
            );
        }
        // … or a coda / syllabic-nasal final glyph (毋 ㆬ, 黃 ㆭ, n-coda ㄣ),
        // which are NOT initials → never flagged.
        for body in ["ㆬ", "ㆭ", "ㄣ", "ㄍㄢ", "ㄍㄤ"] {
            assert!(
                !is_tps_initial_only(body),
                "{body:?} (carries coda/nasal nucleus) must NOT be initial-only"
            );
        }
        // Empty body is never an abbrev.
        assert!(!is_tps_initial_only(""));
    }

    /// `canonicalize_tps_syllable` splits a trailing tone mark off a
    /// well-formed TPS syllable. Pins the `(toneless, tone)` contract
    /// consumed by `fst-builder build-syllables` Family::Tps.
    #[test]
    fn canonicalize_tps_splits_tone_mark() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "\u{3110}\u{3127}\u{311b}\u{02cb}",
                "\u{3110}\u{3127}\u{311b}",
                "\u{02cb}",
            ),
            ("\u{310d}\u{3127}\u{02ca}", "\u{310d}\u{3127}", "\u{02ca}"),
            ("\u{3105}\u{31a4}\u{02ea}", "\u{3105}\u{31a4}", "\u{02ea}"),
        ];
        for (input, want_toneless, want_tone) in cases {
            let got = canonicalize_tps_syllable(input).expect("split");
            assert_eq!(got.0, *want_toneless, "toneless mismatch for {input:?}");
            assert_eq!(got.1, *want_tone, "tone mismatch for {input:?}");
        }
    }

    /// Tone-1 syllables have no trailing mark (`to_zhuyin` emits the
    /// table value `" "` then trimming downstream strips it). Canonical
    /// shape: full input as toneless, empty tone.
    #[test]
    fn canonicalize_tps_tone_one_has_empty_tone() {
        let got = canonicalize_tps_syllable("\u{3105}\u{31a4}").expect("split");
        assert_eq!(got.0, "\u{3105}\u{31a4}");
        assert_eq!(got.1, "");
    }

    /// Tone-8 entering tones use the combining dot `\u{0307}` after a
    /// stop coda (`\u{31b4-7,b}`). The dot is the tone mark; the stop
    /// stays with the toneless body.
    #[test]
    fn canonicalize_tps_tone_eight_keeps_stop_coda_with_toneless() {
        let got = canonicalize_tps_syllable("\u{3105}\u{31a4}\u{31b5}\u{0307}").expect("split");
        assert_eq!(got.0, "\u{3105}\u{31a4}\u{31b5}");
        assert_eq!(got.1, "\u{0307}");
    }

    /// Encode-safe tone-8 variant uses `\u{02d9}` instead of `\u{0307}`.
    /// Both must canonicalize through the same `(toneless, tone)` shape;
    /// engine-side normalization to the canonical form is C-3b's job.
    #[test]
    fn canonicalize_tps_tone_eight_encode_safe_dot() {
        let got = canonicalize_tps_syllable("\u{3105}\u{31a4}\u{31b5}\u{02d9}").expect("split");
        assert_eq!(got.0, "\u{3105}\u{31a4}\u{31b5}");
        assert_eq!(got.1, "\u{02d9}");
    }

    /// Non-Bopomofo input (e.g. accidental ASCII leak from upstream)
    /// must reject — the FST builder's `valid_syllables > 0` gate
    /// counts on this to fail loud if Python ever passes raw `tl_num`.
    #[test]
    fn canonicalize_tps_rejects_non_bopomofo_first_char() {
        assert!(canonicalize_tps_syllable("tai5").is_none());
        assert!(canonicalize_tps_syllable("").is_none());
        assert!(canonicalize_tps_syllable("\u{02cb}").is_none());
    }

    /// ASCII chars mid-syllable (e.g. mixed-shape `ㄅa\u{02cb}`) must
    /// reject — Codex post-impl Q4 hardening. The Python build pipeline
    /// already filters this via `convert_tl_to_tps_strict`'s residue
    /// check, but a permissive public helper would let downstream
    /// callers ship malformed syllables silently.
    #[test]
    fn canonicalize_tps_rejects_ascii_in_body() {
        // `ㄅa˫` — ASCII 'a' between Bopomofo first and tone-7 mark.
        assert!(canonicalize_tps_syllable("\u{3105}a\u{02eb}").is_none());
        // `ㄅa` without trailing tone — body has ASCII.
        assert!(canonicalize_tps_syllable("\u{3105}a").is_none());
    }

    /// Internal tone marks (only the trailing char may be a tone)
    /// must reject — Codex post-impl Q4 hardening. `ㄅ˫ˊ` (two
    /// tone-7 marks back-to-back, or any double-tone) is malformed.
    #[test]
    fn canonicalize_tps_rejects_internal_tone_marks() {
        // `ㄅ˫ˊ` — tone-7 mid-syllable then tone-5 at end.
        assert!(canonicalize_tps_syllable("\u{3105}\u{02eb}\u{02ca}").is_none());
        // `ㄅˋㄉˊ` — internal tone-2 followed by Bopomofo then tone-5.
        assert!(canonicalize_tps_syllable("\u{3105}\u{02cb}\u{3109}\u{02ca}").is_none());
    }
}
