//! `build-syllables` subcommand — build the v3.5.9 B-1 tagged-single-FST
//! syllable inventory (`syllables.fst`) carrying BOTH the `tl:` and `poj:`
//! families in one set.
//!
//! CLI: `fst-builder build-syllables <output.fst> --tl-input <path> --poj-input <path>`
//!
//! Each input file holds one raw `tl_num` / `poj_num` per line, e.g.
//!     tai5gi2
//!     so͘3choa7
//!     peⁿ5peⁿ5
//!     tsiuⁿ7thuan5
//!
//! Empty lines are skipped. Each line is split into syllable tokens at
//! ASCII tone digits `1..=9`; non-ASCII chars (`ⁿ`, `o͘`, combining
//! marks) inside a token are passed through to the per-family
//! canonicalizer:
//!   - `tl:` family → `phonetics::canonicalize_syllable` (POJ→TL spelling
//!     fold + `o͘`→`oo` + `ⁿ`→`nn`), then prefix keys with `tl:`
//!   - `poj:` family → `phonetics::canonicalize_poj_syllable`
//!     (encoding-only fold; POJ ASCII spelling preserved), then prefix
//!     keys with `poj:`
//!
//! Residue policy (per Codex pre-impl review Q3, carried from v3.5.8
//! Phase 2): a token without a closing tone digit OR a leading digit
//! before any letters is a build anomaly and aborts the run with a
//! diagnostic. Phonotactic invalidity (e.g. malformed dual-marked
//! `tn̄g6`) is per-syllable: we count and sample-log to stderr but keep
//! building, since dictionary source rows occasionally carry typos that
//! should not block the inventory.
//!
//! Output: `fst::Set` containing two key forms per valid syllable per
//! family — canonical numeric (`tl:tsua7` / `poj:choa7`) and canonical
//! toneless (`tl:tsua` / `poj:choa`). Sort + dedup happens before
//! insert.

// 中文: build-syllables 子命令 — 從 tl_num 與 poj_num 兩條 input 流取出音節,
// 中文:   分別走 canonicalize_syllable (TL fold) 與 canonicalize_poj_syllable (POJ ASCII),
// 中文:   keys 加上 `tl:` / `poj:` 前綴併入單一 fst::Set。
// 中文: residue (token 沒以聲調數字結尾) 視為 build anomaly,直接 abort。
// 中文: phonotactic 不合法 (如 tn̄g6) 計數+取樣 log 後跳過,不擋 build。

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;

use phonetics::{canonicalize_poj_syllable, canonicalize_syllable};

const INVALID_SAMPLE_LIMIT: usize = 10;

#[derive(Debug)]
pub struct SyllableBuildStats {
    pub tl_lines_in: usize,
    pub poj_lines_in: usize,
    pub syllables_extracted: usize,
    pub valid_syllables: usize,
    pub invalid_skipped: usize,
    pub distinct_keys_out: usize,
}

/// Family discriminant for the tagged-single-FST syllable inventory.
/// The string form is consumed as a key prefix (`tl:` / `poj:`); the
/// callback dispatches to the right phonetics canonicalizer.
// 中文: tagged-single-FST 的家族標籤;字串形式作為 key 前綴,callback 決定 canonicalize 走 TL 或 POJ。
#[derive(Debug, Clone, Copy)]
enum Family {
    Tl,
    Poj,
}

impl Family {
    fn prefix(self) -> &'static str {
        match self {
            Family::Tl => "tl:",
            Family::Poj => "poj:",
        }
    }

    fn canonicalize(self, token: &str) -> Option<(String, String)> {
        match self {
            Family::Tl => canonicalize_syllable(token),
            Family::Poj => canonicalize_poj_syllable(token),
        }
    }
}

pub(crate) fn run_build(
    output_path: &str,
    tl_input_path: &str,
    poj_input_path: &str,
) -> Result<SyllableBuildStats, String> {
    let mut keys: Vec<String> = Vec::new();
    let mut invalid_samples: Vec<String> = Vec::new();

    let tl_counts = ingest_family(Family::Tl, tl_input_path, &mut keys, &mut invalid_samples)?;
    let poj_counts = ingest_family(Family::Poj, poj_input_path, &mut keys, &mut invalid_samples)?;

    // Per-family non-empty + valid > 0 gates (Codex pre-impl B-1 SHOULD/BLOCK):
    // aggregate gates allowed a regressed POJ canonicalizer or an empty
    // `--poj-input` to silently land a POJ-empty inventory and ship as
    // green. B-1 is B-2's hard prerequisite, so this MUST fail loud.
    // 中文: 每家族 non-empty + valid > 0 雙閘 — 防止 POJ 端 canonicalizer 退化或
    // 中文:   POJ input 漏接時,builder 仍從 TL 端綠燈走完寫出 POJ 空的 FST。
    for (label, c) in &[("tl", &tl_counts), ("poj", &poj_counts)] {
        if c.syllables_extracted == 0 {
            return Err(format!(
                "{label}: no syllables extracted from input — empty file or missing input"
            ));
        }
        if c.valid_syllables == 0 {
            return Err(format!(
                "{label}: all {} extracted syllables failed phonotactic validation \
                 — check input shape or canonicalize_*_syllable",
                c.syllables_extracted,
            ));
        }
    }

    let syllables_extracted = tl_counts.syllables_extracted + poj_counts.syllables_extracted;
    let valid_syllables = tl_counts.valid_syllables + poj_counts.valid_syllables;
    let tl_lines_in = tl_counts.lines_in;
    let poj_lines_in = poj_counts.lines_in;

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
        tl_lines_in,
        poj_lines_in,
        syllables_extracted,
        valid_syllables,
        invalid_skipped,
        distinct_keys_out,
    })
}

/// Per-family running counters; returned by [`ingest_family`] so
/// `run_build` can apply per-family non-empty + valid > 0 gates before
/// writing the FST (Codex pre-impl B-1 SHOULD/BLOCK guard).
// 中文: 單一家族的計數,讓 run_build 在寫 FST 前對 TL / POJ 各自閘門。
#[derive(Default)]
struct FamilyCounts {
    lines_in: usize,
    syllables_extracted: usize,
    valid_syllables: usize,
}

/// Read one family's input file, accumulate prefixed keys into `keys`,
/// and return the family's per-stream counters.
// 中文: 讀一條家族 input,把帶 prefix 的 key 累積進 keys,並回該家族的計數。
fn ingest_family(
    family: Family,
    input_path: &str,
    keys: &mut Vec<String>,
    invalid_samples: &mut Vec<String>,
) -> Result<FamilyCounts, String> {
    let file = File::open(Path::new(input_path))
        .map_err(|e| format!("open {} input `{}`: {}", family.prefix(), input_path, e))?;
    let mut counts = FamilyCounts::default();
    for line_res in BufReader::new(file).lines() {
        let line = line_res
            .map_err(|e| format!("read {} input `{}`: {}", family.prefix(), input_path, e))?;
        if line.is_empty() {
            continue;
        }
        counts.lines_in += 1;
        let tokens = split_into_syllables(&line).map_err(|e| {
            format!(
                "{} input `{}` line {} (`{}`): {}",
                family.prefix(),
                input_path,
                counts.lines_in,
                line,
                e
            )
        })?;
        counts.syllables_extracted += tokens.len();

        for token in &tokens {
            match family.canonicalize(token) {
                Some((canonical_toneless, tone)) => {
                    counts.valid_syllables += 1;
                    let prefix = family.prefix();
                    if tone.is_empty() {
                        keys.push(format!("{}{}", prefix, canonical_toneless));
                    } else {
                        keys.push(format!("{}{}{}", prefix, canonical_toneless, tone));
                        keys.push(format!("{}{}", prefix, canonical_toneless));
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
    Ok(counts)
}

/// Read one family's lines from `stdin` (legacy/test convenience). The
/// production driver writes to disk and uses [`run_build`]; this exists
/// so unit tests can exercise the split + canonicalize + emit pipeline
/// without temp-file scaffolding.
#[cfg(test)]
fn collect_family_keys_from_stdin_for_test(
    family: Family,
    input: &str,
) -> Result<Vec<String>, String> {
    let mut keys: Vec<String> = Vec::new();
    let mut syllables_extracted: usize = 0;
    let mut valid_syllables: usize = 0;
    let mut invalid_samples: Vec<String> = Vec::new();
    for raw in input.lines() {
        if raw.is_empty() {
            continue;
        }
        let tokens = split_into_syllables(raw)?;
        syllables_extracted += tokens.len();
        for token in &tokens {
            match family.canonicalize(token) {
                Some((canonical_toneless, tone)) => {
                    valid_syllables += 1;
                    let prefix = family.prefix();
                    if tone.is_empty() {
                        keys.push(format!("{}{}", prefix, canonical_toneless));
                    } else {
                        keys.push(format!("{}{}{}", prefix, canonical_toneless, tone));
                        keys.push(format!("{}{}", prefix, canonical_toneless));
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
    let _ = (syllables_extracted, valid_syllables, invalid_samples);
    keys.sort_unstable();
    keys.dedup();
    Ok(keys)
}

/// Split a `tl_num` / `poj_num` line into syllable tokens at ASCII tone
/// digits. Each token includes its trailing tone digit. Returns `Err` on
/// trailing residue (token never closed) or leading-digit anomalies.
// 中文: 把 tl_num/poj_num 行依 ASCII 聲調數字 1..=9 切成 token,trailing 不收尾或開頭即為數字算 anomaly。
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
    /// emission for one TL stdin line — pins splitter regressions that
    /// the lexicon integration test (which starts from canonicalized
    /// pairs) cannot catch.
    #[test]
    fn pipeline_emits_canonical_tl_pairs() {
        let cases: &[(&str, &[&str])] = &[
            // (raw tl_num, expected sorted+deduped canonical keys WITH tl: prefix)
            ("tai5gi2", &["tl:gi", "tl:gi2", "tl:tai", "tl:tai5"]),
            ("so͘3choa7", &["tl:soo", "tl:soo3", "tl:tsua", "tl:tsua7"]),
            ("peⁿ5peⁿ5", &["tl:penn", "tl:penn5"]),
            (
                "tsiuⁿ7thuan5",
                &["tl:thuan", "tl:thuan5", "tl:tsiunn", "tl:tsiunn7"],
            ),
        ];
        for (line, expected) in cases {
            let keys = collect_family_keys_from_stdin_for_test(Family::Tl, line).expect("collect");
            let expected_owned: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(keys, expected_owned, "TL pipeline mismatch for `{line}`");
        }
    }

    /// POJ family preserves POJ ASCII shape (no `ch→ts` etc. fold).
    /// Pins the v3.5.9 B-1 contract that `chiah` / `goa` / `che` etc.
    /// land as their own syllable boundaries in the inventory.
    #[test]
    fn pipeline_emits_canonical_poj_pairs() {
        let cases: &[(&str, &[&str])] = &[
            // POJ-shaped rows that DIFFER from TL fold
            (
                "chit8goa2",
                &["poj:chit", "poj:chit8", "poj:goa", "poj:goa2"],
            ),
            ("toa7", &["poj:toa", "poj:toa7"]),
            ("che1", &["poj:che", "poj:che1"]),
            (
                "koe1peng5",
                &["poj:koe", "poj:koe1", "poj:peng", "poj:peng5"],
            ),
            // Non-ASCII POJ collapses to ASCII via encoding-only rules
            ("so͘3", &["poj:soo", "poj:soo3"]),
            ("peⁿ5peⁿ5", &["poj:penn", "poj:penn5"]),
        ];
        for (line, expected) in cases {
            let keys = collect_family_keys_from_stdin_for_test(Family::Poj, line).expect("collect");
            let expected_owned: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(keys, expected_owned, "POJ pipeline mismatch for `{line}`");
        }
    }

    #[test]
    fn pipeline_distinguishes_tl_and_poj_families_for_divergent_row() {
        // `chit8` (POJ) → `poj:chit` + `poj:chit8`
        // `tsit8` (TL  ) → `tl:tsit`  + `tl:tsit8`
        let tl_keys = collect_family_keys_from_stdin_for_test(Family::Tl, "tsit8").expect("tl");
        let poj_keys = collect_family_keys_from_stdin_for_test(Family::Poj, "chit8").expect("poj");
        assert_eq!(tl_keys, vec!["tl:tsit", "tl:tsit8"]);
        assert_eq!(poj_keys, vec!["poj:chit", "poj:chit8"]);
        // The two sets are disjoint — tagged-single-FST property.
        for k in &tl_keys {
            assert!(!poj_keys.contains(k), "TL key `{k}` leaked into POJ family");
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

    fn write_temp(name: &str, body: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fst-builder-gate-{}-{}-{}.txt",
            std::process::id(),
            name,
            n
        ));
        std::fs::write(&path, body).expect("write temp");
        path
    }

    /// v3.5.9 B-1 (Codex pre-impl SHOULD/BLOCK) — `run_build` must fail
    /// loud when either family's input is empty. The earlier aggregate
    /// gate let an empty `--poj-input` ship a POJ-empty inventory while
    /// the TL side carried the build, which would silently break B-2.
    #[test]
    fn run_build_rejects_empty_poj_input() {
        let tl = write_temp("tl", "tai5gi2\n");
        let poj = write_temp("poj", "");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
        )
        .expect_err("must reject empty POJ input");
        assert!(
            err.contains("poj") && err.contains("no syllables extracted"),
            "expected poj per-family gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn run_build_rejects_empty_tl_input() {
        let tl = write_temp("tl", "");
        let poj = write_temp("poj", "chit8\n");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
        )
        .expect_err("must reject empty TL input");
        assert!(
            err.contains("tl") && err.contains("no syllables extracted"),
            "expected tl per-family gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn run_build_rejects_all_phonotactic_invalid_in_family() {
        // POJ input has tokens that all fail phonotactic validation
        // (`xyz1`, `qq2`); TL is healthy. Per-family `valid_syllables == 0`
        // gate must fire on the POJ side.
        let tl = write_temp("tl-valid", "tai5\n");
        let poj = write_temp("poj-invalid", "xyz1\nqq2\n");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
        )
        .expect_err("must reject when POJ family has zero phonotactic-valid syllables");
        assert!(
            err.contains("poj") && err.contains("phonotactic validation"),
            "expected poj valid>0 gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn run_build_succeeds_with_minimal_dual_input() {
        // Sanity: tiny well-formed dual input writes an FST containing
        // both families' canonical pairs.
        let tl = write_temp("tl-ok", "tai5\n");
        let poj = write_temp("poj-ok", "chit8\n");
        let out = write_temp("out-ok", "");
        let stats = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
        )
        .expect("dual build should succeed");
        assert_eq!(stats.tl_lines_in, 1);
        assert_eq!(stats.poj_lines_in, 1);
        // tl: 2 keys (tai, tai5), poj: 2 keys (chit, chit8) → 4 distinct.
        assert_eq!(stats.distinct_keys_out, 4);
        let bytes = std::fs::read(&out).expect("read out");
        let set = fst::Set::new(bytes).expect("parse fst");
        assert!(set.contains(b"tl:tai"));
        assert!(set.contains(b"tl:tai5"));
        assert!(set.contains(b"poj:chit"));
        assert!(set.contains(b"poj:chit8"));
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(out);
    }
}
