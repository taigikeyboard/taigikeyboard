//! `build-syllables` subcommand — build the v3.5.8 Phase 2 TL syllable
//! inventory FST from canonical dictionary `tl_num` strings.
//!
//! Stdin protocol: one raw `tl_num` per line, e.g.
//!     tai5gi2
//!     so͘3choa7
//!     peⁿ5peⁿ5
//!     tsiuⁿ7thuan5
//! Empty lines are skipped. Each line is split into syllable tokens at
//! ASCII tone digits `1..=9`; non-ASCII chars (`ⁿ`, `o͘`, combining
//! marks) inside a token are passed through to `phonetics::canonicalize_syllable`,
//! which folds them to canonical TL via `normalize_to_tl`.
//!
//! Residue policy (per Codex pre-impl review Q3): a token without a
//! closing tone digit OR a leading digit before any letters is a build
//! anomaly and aborts the run with a diagnostic. Phonotactic invalidity
//! (e.g. malformed dual-marked `tn̄g6`) is per-syllable: we count and
//! sample-log to stderr but keep building, since dictionary source rows
//! occasionally carry typos that should not block the inventory.
//!
//! Output: `fst::Set` containing two key forms per valid syllable —
//! canonical numeric (`tsua7`) and canonical toneless (`tsua`). Sort +
//! dedup happens before insert.

// 中文: build-syllables 子命令 — 從 tl_num 取出音節、正規化成 TL canonical,
// 中文: 同時寫 numeric (tsua7) 與 toneless (tsua) 兩種 key 進 fst::Set。
// 中文: residue (token 沒以聲調數字結尾) 視為 build anomaly,直接 abort。
// 中文: phonotactic 不合法 (如 tn̄g6) 計數+取樣 log 後跳過,不擋 build。

use std::fs::File;
use std::io::{self, BufRead, BufWriter};

use phonetics::canonicalize_syllable;

const INVALID_SAMPLE_LIMIT: usize = 10;

pub struct SyllableBuildStats {
    pub lines_in: usize,
    pub syllables_extracted: usize,
    pub valid_syllables: usize,
    pub invalid_skipped: usize,
    pub distinct_keys_out: usize,
}

pub(crate) fn run_build(output_path: &str) -> Result<SyllableBuildStats, String> {
    let stdin = io::stdin();
    let mut lines_in: usize = 0;
    let mut syllables_extracted: usize = 0;
    let mut valid_syllables: usize = 0;
    let mut invalid_samples: Vec<String> = Vec::new();
    let mut keys: Vec<String> = Vec::new();

    for line_res in stdin.lock().lines() {
        let line = line_res.map_err(|e| format!("stdin read error: {}", e))?;
        if line.is_empty() {
            continue;
        }
        lines_in += 1;
        let tokens = split_into_syllables(&line)
            .map_err(|e| format!("line {} (`{}`): {}", lines_in, line, e))?;
        syllables_extracted += tokens.len();

        for token in &tokens {
            match canonicalize_syllable(token) {
                Some((canonical_toneless, tone)) => {
                    valid_syllables += 1;
                    if tone.is_empty() {
                        keys.push(canonical_toneless);
                    } else {
                        keys.push(format!("{}{}", canonical_toneless, tone));
                        keys.push(canonical_toneless);
                    }
                }
                None => {
                    if invalid_samples.len() < INVALID_SAMPLE_LIMIT {
                        invalid_samples.push(token.clone());
                    }
                }
            }
        }
    }

    if syllables_extracted == 0 {
        return Err("no syllables extracted from stdin".to_string());
    }
    // Hard-fail when every extracted token is phonotactically invalid;
    // without this gate a structural regression silently writes an empty
    // FST and exits success.
    if valid_syllables == 0 {
        return Err(format!(
            "all {} extracted syllables failed phonotactic validation — \
             check input shape or canonicalize_syllable",
            syllables_extracted,
        ));
    }

    let invalid_skipped = syllables_extracted - valid_syllables;
    if invalid_skipped > 0 {
        eprintln!(
            "[fst-builder] phonotactic-invalid syllables skipped: {} (samples: {:?})",
            invalid_skipped, invalid_samples,
        );
    }

    keys.sort_unstable();
    keys.dedup();
    let distinct_keys_out = keys.len();

    let file =
        File::create(output_path).map_err(|e| format!("create output `{}`: {}", output_path, e))?;
    let writer = BufWriter::new(file);
    let mut builder =
        fst::SetBuilder::new(writer).map_err(|e| format!("fst SetBuilder init: {}", e))?;
    for key in &keys {
        builder
            .insert(key.as_bytes())
            .map_err(|e| format!("fst insert `{}`: {}", key, e))?;
    }
    builder.finish().map_err(|e| format!("fst finish: {}", e))?;

    Ok(SyllableBuildStats {
        lines_in,
        syllables_extracted,
        valid_syllables,
        invalid_skipped,
        distinct_keys_out,
    })
}

/// Split a `tl_num` line into syllable tokens at ASCII tone digits.
/// Each token includes its trailing tone digit. Returns `Err` on
/// trailing residue (token never closed) or leading-digit anomalies.
// 中文: 把 tl_num 行依 ASCII 聲調數字 1..=9 切成 token,trailing 不收尾或開頭即為數字算 anomaly。
fn split_into_syllables(line: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_has_letter = false;

    for ch in line.chars() {
        if ('1'..='9').contains(&ch) {
            if !current_has_letter {
                return Err(format!("tone digit `{}` with no preceding letters", ch));
            }
            current.push(ch);
            tokens.push(std::mem::take(&mut current));
            current_has_letter = false;
        } else {
            // `is_alphabetic()` is Unicode-aware: ASCII letters AND `ⁿ`
            // (U+207F, Lm) count; bare combining marks (`\u{0358}`,
            // `\u{0304}`, all Mn) do not, so a syllable that starts
            // with a stray combining mark trips the leading-digit
            // anomaly when its first real char is a tone digit.
            if ch.is_alphabetic() {
                current_has_letter = true;
            }
            current.push(ch);
        }
    }

    if !current.is_empty() {
        return Err(format!(
            "trailing residue `{}` — every syllable must end with a tone digit 1..=9",
            current
        ));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_pure_ascii_multi_syllable() {
        let toks = split_into_syllables("tai5gi2").expect("split");
        assert_eq!(toks, vec!["tai5", "gi2"]);
    }

    #[test]
    fn split_with_combining_marks() {
        let toks = split_into_syllables("so͘3choa7").expect("split");
        assert_eq!(toks, vec!["so͘3", "choa7"]);
    }

    #[test]
    fn split_with_superscript_n() {
        let toks = split_into_syllables("tsiuⁿ7thuan5").expect("split");
        assert_eq!(toks, vec!["tsiuⁿ7", "thuan5"]);
    }

    #[test]
    fn split_residue_errors() {
        let err = split_into_syllables("tai").expect_err("should error");
        assert!(err.contains("trailing residue"));
    }

    #[test]
    fn split_leading_digit_errors() {
        let err = split_into_syllables("5tai").expect_err("should error");
        assert!(err.contains("no preceding letters"));
    }

    /// End-to-end check that exercises split + canonicalize + key
    /// emission for one stdin line — pins splitter regressions that the
    /// lexicon integration test (which starts from canonicalized pairs)
    /// cannot catch.
    #[test]
    fn pipeline_emits_canonical_pairs_for_real_tl_num_lines() {
        let cases: &[(&str, &[&str])] = &[
            // (raw tl_num, expected sorted+deduped canonical keys)
            ("tai5gi2", &["gi", "gi2", "tai", "tai5"]),
            ("so͘3choa7", &["soo", "soo3", "tsua", "tsua7"]),
            ("peⁿ5peⁿ5", &["penn", "penn5"]),
            ("tsiuⁿ7thuan5", &["thuan", "thuan5", "tsiunn", "tsiunn7"]),
        ];
        for (line, expected) in cases {
            let tokens = split_into_syllables(line).expect("split");
            let mut keys: Vec<String> = Vec::new();
            for token in &tokens {
                if let Some((canonical, tone)) = phonetics::canonicalize_syllable(token) {
                    if tone.is_empty() {
                        keys.push(canonical);
                    } else {
                        keys.push(format!("{}{}", canonical, tone));
                        keys.push(canonical);
                    }
                }
            }
            keys.sort();
            keys.dedup();
            let expected_owned: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(keys, expected_owned, "pipeline mismatch for `{line}`");
        }
    }

    #[test]
    fn split_leading_combining_mark_followed_by_digit_errors() {
        // Combining mark before any letter must NOT count as letter
        // content — the first ASCII tone digit then triggers the
        // leading-digit anomaly. Pins the `is_alphabetic()` heuristic.
        let err = split_into_syllables("\u{0358}5").expect_err("should error");
        assert!(err.contains("no preceding letters"));
    }
}
