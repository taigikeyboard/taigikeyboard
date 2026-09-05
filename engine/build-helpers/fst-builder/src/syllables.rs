//! `build-syllables` subcommand — build the v3.5.9 D / C-0 tagged-single-FST
//! syllable inventory (`syllables.fst`) carrying the `tl:`, `poj:`, and
//! `tps:` families in one set.
//!
//! CLI: `fst-builder build-syllables <output.fst> --tl-input <path> --poj-input <path> --tps-input <path>`
//!
//! TL / POJ input files hold one raw `tl_num` / `poj_num` per line:
//!     tai5gi2
//!     so͘3choa7
//!     peⁿ5peⁿ5
//!     tsiuⁿ7thuan5
//!
//! TPS input holds ONE pre-canonicalized TPS syllable per line (Python
//! pre-splits hyphenated TL and converts per-syllable via the
//! `taigi-converter` bridge, so each line is already one syllable —
//! the digit-based splitter does not apply to Bopomofo input).
//!
//! Empty lines are skipped. TL / POJ lines are split into syllable
//! tokens at ASCII tone digits `1..=9`; non-ASCII chars (`ⁿ`, `o͘`,
//! combining marks) inside a token are passed through to the per-family
//! canonicalizer:
//!   - `tl:` family → `phonetics::canonicalize_syllable` (POJ→TL spelling
//!     fold + `o͘`→`oo` + `ⁿ`→`nn`), then prefix keys with `tl:`
//!   - `poj:` family → `phonetics::canonicalize_poj_syllable`
//!     (encoding-only fold; POJ ASCII spelling preserved), then prefix
//!     keys with `poj:`
//!   - `tps:` family → `phonetics::canonicalize_tps_syllable`
//!     (tone-mark split; phonotactic validity guaranteed upstream by
//!     the TL CSV + `taigi-converter`), then prefix keys with `tps:`
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

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;

use phonetics::{
    canonicalize_poj_syllable, canonicalize_syllable, canonicalize_tps_syllable,
    nasal_oo_alias_spelling,
};

const INVALID_SAMPLE_LIMIT: usize = 10;

#[derive(Debug)]
pub struct SyllableBuildStats {
    pub tl_lines_in: usize,
    pub poj_lines_in: usize,
    pub tps_lines_in: usize,
    pub syllables_extracted: usize,
    pub valid_syllables: usize,
    pub invalid_skipped: usize,
    pub distinct_keys_out: usize,
}

/// Family discriminant for the tagged-single-FST syllable inventory.
/// The string form is consumed as a key prefix (`tl:` / `poj:`); the
/// callback dispatches to the right phonetics canonicalizer.
#[derive(Debug, Clone, Copy)]
enum Family {
    Tl,
    Poj,
    Tps,
}

impl Family {
    fn prefix(self) -> &'static str {
        match self {
            Family::Tl => "tl:",
            Family::Poj => "poj:",
            Family::Tps => "tps:",
        }
    }

    fn canonicalize(self, token: &str) -> Option<(String, String)> {
        match self {
            Family::Tl => canonicalize_syllable(token),
            Family::Poj => canonicalize_poj_syllable(token),
            Family::Tps => canonicalize_tps_syllable(token),
        }
    }

    /// TPS input is already one pre-canonicalized syllable per line —
    /// the digit-based [`split_into_syllables`] does not apply to
    /// Bopomofo. TL / POJ inputs hold one `tl_num` / `poj_num` row per
    /// line and need digit-based splitting.
    fn line_is_single_syllable(self) -> bool {
        matches!(self, Family::Tps)
    }
}

pub(crate) fn run_build(
    output_path: &str,
    tl_input_path: &str,
    poj_input_path: &str,
    tps_input_path: &str,
) -> Result<SyllableBuildStats, String> {
    let mut keys: Vec<String> = Vec::new();
    let mut invalid_samples: Vec<String> = Vec::new();

    let tl_counts = ingest_family(Family::Tl, tl_input_path, &mut keys, &mut invalid_samples)?;
    let poj_counts = ingest_family(Family::Poj, poj_input_path, &mut keys, &mut invalid_samples)?;
    let tps_counts = ingest_family(Family::Tps, tps_input_path, &mut keys, &mut invalid_samples)?;

    // Per-family non-empty + valid > 0 gates (Codex pre-impl B-1 SHOULD/BLOCK,
    // extended to TPS in C-0): aggregate gates allowed a regressed POJ
    // canonicalizer or an empty `--poj-input` to silently land a POJ-empty
    // inventory and ship as green. B-1 is B-2's hard prerequisite, C-0 is
    // C-3b's hard prerequisite — both MUST fail loud.
    for (label, c) in &[
        ("tl", &tl_counts),
        ("poj", &poj_counts),
        ("tps", &tps_counts),
    ] {
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

    let syllables_extracted = tl_counts.syllables_extracted
        + poj_counts.syllables_extracted
        + tps_counts.syllables_extracted;
    let valid_syllables =
        tl_counts.valid_syllables + poj_counts.valid_syllables + tps_counts.valid_syllables;
    let tl_lines_in = tl_counts.lines_in;
    let poj_lines_in = poj_counts.lines_in;
    let tps_lines_in = tps_counts.lines_in;

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
        tps_lines_in,
        syllables_extracted,
        valid_syllables,
        invalid_skipped,
        distinct_keys_out,
    })
}

/// Per-family running counters; returned by [`ingest_family`] so
/// `run_build` can apply per-family non-empty + valid > 0 gates before
/// writing the FST (Codex pre-impl B-1 SHOULD/BLOCK guard).
#[derive(Default)]
struct FamilyCounts {
    lines_in: usize,
    syllables_extracted: usize,
    valid_syllables: usize,
}

/// Read one family's input file, accumulate prefixed keys into `keys`,
/// and return the family's per-stream counters.
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
        let tokens: Vec<String> = if family.line_is_single_syllable() {
            vec![line.clone()]
        } else {
            split_into_syllables(&line).map_err(|e| {
                format!(
                    "{} input `{}` line {} (`{}`): {}",
                    family.prefix(),
                    input_path,
                    counts.lines_in,
                    line,
                    e
                )
            })?
        };
        counts.syllables_extracted += tokens.len();

        for token in &tokens {
            match family.canonicalize(token) {
                Some((canonical_toneless, tone)) => {
                    counts.valid_syllables += 1;
                    let prefix = family.prefix();
                    // Nasal-`oo` spelling alias — the `o͘ⁿ` rendering of the
                    // nasal final, ASCII `oonn`, indexed beside the canonical
                    // `onn` so a user who spells it that way still segments.
                    // Emitted HERE, where `token` is one syllable, because
                    // that is the only place the boundaries are still known
                    // (see `phonetics::nasal_oo_alias_spelling`). TPS is
                    // Bopomofo and never matches.
                    let spellings = std::iter::once(canonical_toneless.clone())
                        .chain(nasal_oo_alias_spelling(&canonical_toneless));
                    for toneless in spellings {
                        if tone.is_empty() {
                            keys.push(format!("{}{}", prefix, toneless));
                        } else {
                            keys.push(format!("{}{}{}", prefix, toneless, tone));
                            keys.push(format!("{}{}", prefix, toneless));
                        }
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
        let tokens: Vec<String> = if family.line_is_single_syllable() {
            vec![raw.to_string()]
        } else {
            split_into_syllables(raw)?
        };
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
        let tps = write_temp("tps", "\u{3110}\u{3127}\u{02cb}\n");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
            tps.to_str().unwrap(),
        )
        .expect_err("must reject empty POJ input");
        assert!(
            err.contains("poj") && err.contains("no syllables extracted"),
            "expected poj per-family gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(tps);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn run_build_rejects_empty_tl_input() {
        let tl = write_temp("tl", "");
        let poj = write_temp("poj", "chit8\n");
        let tps = write_temp("tps", "\u{3110}\u{3127}\u{02cb}\n");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
            tps.to_str().unwrap(),
        )
        .expect_err("must reject empty TL input");
        assert!(
            err.contains("tl") && err.contains("no syllables extracted"),
            "expected tl per-family gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(tps);
        let _ = std::fs::remove_file(out);
    }

    /// v3.5.9 D / C-0 per-family gate extended to TPS: an empty
    /// `--tps-input` must fail loud so a regressed Python derivation
    /// (`convert_tl_to_tps_strict` returning empty / converter bridge
    /// down) cannot ship a TPS-empty syllables.fst that silently breaks
    /// C-3b's `SyllableInventory::contains_in(Mode::Tps, ..)`.
    #[test]
    fn run_build_rejects_empty_tps_input() {
        let tl = write_temp("tl", "tai5\n");
        let poj = write_temp("poj", "chit8\n");
        let tps = write_temp("tps", "");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
            tps.to_str().unwrap(),
        )
        .expect_err("must reject empty TPS input");
        assert!(
            err.contains("tps") && err.contains("no syllables extracted"),
            "expected tps per-family gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(tps);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn run_build_rejects_all_phonotactic_invalid_in_family() {
        // POJ input has tokens that all fail phonotactic validation
        // (`xyz1`, `qq2`); TL/TPS are healthy. Per-family
        // `valid_syllables == 0` gate must fire on the POJ side.
        let tl = write_temp("tl-valid", "tai5\n");
        let poj = write_temp("poj-invalid", "xyz1\nqq2\n");
        let tps = write_temp("tps-valid", "\u{3110}\u{3127}\u{02cb}\n");
        let out = write_temp("out", "");
        let err = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
            tps.to_str().unwrap(),
        )
        .expect_err("must reject when POJ family has zero phonotactic-valid syllables");
        assert!(
            err.contains("poj") && err.contains("phonotactic validation"),
            "expected poj valid>0 gate, got: {err}"
        );
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(tps);
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn run_build_succeeds_with_minimal_triple_input() {
        // Sanity: tiny well-formed triple input writes an FST containing
        // all three families' canonical pairs.
        let tl = write_temp("tl-ok", "tai5\n");
        let poj = write_temp("poj-ok", "chit8\n");
        // TPS `ㄐㄧˋ` (tsi2-shape with tone-2) — pre-canonicalized one
        // syllable per line per C-0 contract.
        let tps = write_temp("tps-ok", "\u{3110}\u{3127}\u{02cb}\n");
        let out = write_temp("out-ok", "");
        let stats = run_build(
            out.to_str().unwrap(),
            tl.to_str().unwrap(),
            poj.to_str().unwrap(),
            tps.to_str().unwrap(),
        )
        .expect("triple build should succeed");
        assert_eq!(stats.tl_lines_in, 1);
        assert_eq!(stats.poj_lines_in, 1);
        assert_eq!(stats.tps_lines_in, 1);
        // tl: 2 keys (tai, tai5), poj: 2 (chit, chit8), tps: 2 (toneless, +tone) → 6 distinct.
        assert_eq!(stats.distinct_keys_out, 6);
        let bytes = std::fs::read(&out).expect("read out");
        let set = fst::Set::new(bytes).expect("parse fst");
        assert!(set.contains(b"tl:tai"));
        assert!(set.contains(b"tl:tai5"));
        assert!(set.contains(b"poj:chit"));
        assert!(set.contains(b"poj:chit8"));
        assert!(set.contains("tps:\u{3110}\u{3127}".as_bytes()));
        assert!(set.contains("tps:\u{3110}\u{3127}\u{02cb}".as_bytes()));
        let _ = std::fs::remove_file(tl);
        let _ = std::fs::remove_file(poj);
        let _ = std::fs::remove_file(tps);
        let _ = std::fs::remove_file(out);
    }

    /// TPS family must use the one-syllable-per-line ingest path
    /// (`Family::line_is_single_syllable`), NOT `split_into_syllables`
    /// which only understands ASCII tone digits. Pins the contract that
    /// Python pre-splits hyphenated TL into per-syllable TPS lines.
    #[test]
    fn tps_family_keeps_full_line_as_one_syllable() {
        let keys = collect_family_keys_from_stdin_for_test(Family::Tps, "\u{3110}\u{3127}\u{02cb}")
            .expect("collect");
        assert_eq!(
            keys,
            vec![
                "tps:\u{3110}\u{3127}".to_string(),
                "tps:\u{3110}\u{3127}\u{02cb}".to_string(),
            ]
        );
    }
}
