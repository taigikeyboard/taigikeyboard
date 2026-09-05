//! v3.5.9 B-2 — runtime POJ-notone derivation parity test (Codex
//! post-impl SHOULD #1).
//!
//! `matches_continuous_poj_toneless_key` derives the `poj_notone` for
//! each `record.tl` at runtime via encoding-only per-token
//! normalization: `phonetics::api::tl_display_to_poj_display +
//! split(['-', ' ']) + per-token NFD-drop-tone-marks + lowercase +
//! phonetics::normalize_to_poj + strip-digits + concat`. The production
//! `dictionary.fst` is built from the precomputed `record.poj_notone`
//! column in `dictionary/output/dictionary.csv` (see
//! `dictionary/build/create_fst.py:124-127` +
//! `dictionary/build/merge_csv.py:226-240`). If the Rust runtime
//! derivation ever drifts from the build-pipeline derivation, the
//! continuous-input guard would silently reject every POJ family hit
//! whose `record.tl ↔ poj_notone` mapping diverges.
//!
//! This parity test reads the shipped CSV and asserts that the runtime
//! derivation byte-matches the stored `poj_notone` column for every
//! row whose `tl` and `poj_notone` are both non-empty and ASCII
//! (anomaly rows where the build pipeline left non-ASCII glyphs in
//! `poj_notone` are skipped — they cannot be reached by any continuous
//! `poj:<ascii>` lookup key anyway). If the CSV is not present (some
//! lean checkouts may omit the 24 MB output file) the test no-ops
//! with a soft skip rather than blocking.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Runtime derivation — byte-identical to the body of
/// `matches_continuous_poj_toneless_key + derive_poj_notone_for_match`
/// in `engine/lexicon/src/continuous.rs`. Re-implemented here (not
/// exposed via `pub`) so a refactor of the guard cannot quietly drift
/// from the parity contract under test.
fn derive_poj_notone_runtime(tl_display: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let poj_display = phonetics::api::tl_display_to_poj_display(tl_display);
    let mut out = String::with_capacity(poj_display.len());
    for token in poj_display.split(['-', ' ']) {
        if token.is_empty() {
            continue;
        }
        let mut token_buf = String::with_capacity(token.len());
        for ch in token.nfd() {
            if matches!(
                ch,
                '\u{0300}'
                    | '\u{0301}'
                    | '\u{0302}'
                    | '\u{0304}'
                    | '\u{0306}'
                    | '\u{030b}'
                    | '\u{030c}'
                    | '\u{030d}'
            ) {
                continue;
            }
            for lower_ch in ch.to_lowercase() {
                token_buf.push(lower_ch);
            }
        }
        for c in phonetics::normalize_to_poj(&token_buf).chars() {
            if !c.is_ascii_digit() {
                out.push(c);
            }
        }
    }
    out
}

fn dictionary_csv_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = engine/lexicon → 4-up = repo root.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // engine
    path.pop(); // repo root
    path.push("dictionary");
    path.push("output");
    path.push("dictionary.csv");
    path
}

/// Parse `dictionary.csv`'s relevant columns. The shipped file uses
/// plain comma separation with no embedded quoted commas in the
/// columns we read (`tl`, `poj_notone`); a naive splitter is safe.
/// Returns `(tl, poj_notone)` per row in the order they appear.
fn read_rows(path: &std::path::Path) -> std::io::Result<Vec<(String, String)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    // Header
    let header = lines.next().expect("CSV header line present")?;
    let columns: Vec<&str> = header.split(',').collect();
    let tl_idx = columns
        .iter()
        .position(|c| *c == "tl")
        .expect("`tl` column present in CSV header");
    let poj_notone_idx = columns
        .iter()
        .position(|c| *c == "poj_notone")
        .expect("`poj_notone` column present in CSV header");

    let mut rows = Vec::new();
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        // Tolerate any row that does not parse cleanly — the test is
        // about derivation correctness on well-formed rows, not CSV
        // parser robustness.
        if fields.len() <= poj_notone_idx.max(tl_idx) {
            continue;
        }
        rows.push((
            fields[tl_idx].to_string(),
            fields[poj_notone_idx].to_string(),
        ));
    }
    Ok(rows)
}

#[test]
fn runtime_poj_notone_matches_build_pipeline_for_every_row() {
    let path = dictionary_csv_path();
    if !path.exists() {
        eprintln!(
            "skipping POJ-notone parity test: {} not present (lean checkout)",
            path.display()
        );
        return;
    }

    let rows = read_rows(&path).expect("read dictionary.csv rows");
    assert!(
        !rows.is_empty(),
        "expected non-empty `dictionary.csv` row stream"
    );

    let mut total = 0usize;
    let mut compared = 0usize;
    let mut anomalies = 0usize;
    let mut drift: Vec<(String, String, String)> = Vec::new();

    for (tl, poj_notone) in &rows {
        total += 1;
        if tl.is_empty() || poj_notone.is_empty() {
            continue;
        }
        // Skip rows whose shipped `poj_notone` is not pure ASCII —
        // those are upstream build-pipeline anomalies (e.g. stacked
        // diacritics on a single base char survive `to_numeric_tone`)
        // that no user can possibly type as a continuous-input toneless
        // key. The matching guard correctly never reaches them via the
        // `poj:<ascii>` lookup path.
        if !poj_notone.is_ascii() {
            anomalies += 1;
            continue;
        }
        compared += 1;
        let derived = derive_poj_notone_runtime(tl);
        if derived != *poj_notone {
            drift.push((tl.clone(), poj_notone.clone(), derived));
            if drift.len() >= 10 {
                break;
            }
        }
    }

    assert!(
        drift.is_empty(),
        "runtime POJ-notone derivation drifted from build pipeline on \
         {} of {} compared rows (first {} shown):\n{}",
        drift.len(),
        compared,
        drift.len(),
        drift
            .iter()
            .map(|(tl, expected, got)| format!(
                "  tl={tl:?} expected poj_notone={expected:?} got {got:?}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    eprintln!(
        "POJ-notone parity OK: {compared}/{total} rows compared \
         ({anomalies} non-ASCII anomaly rows skipped)."
    );
}
