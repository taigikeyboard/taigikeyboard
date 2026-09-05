//! v3.5.9 D / C-3b — runtime TPS-notone derivation parity test.
//!
//! `matches_continuous_tps_toneless_key` derives `tps_notone` for each
//! `record.tl` at runtime via `phonetics::tps_notone_from_tl`, which
//! mirrors the build pipeline chain `dictionary/build/merge_csv.py:251-265`:
//!
//! 1. Per-token split on `[-, whitespace]`.
//! 2. `phonetics::to_tone_number` (mirrors `convert_tl_to_tps_strict`'s
//!    diacritic→numeric input expectation; the Node bridge accepts both
//!    diacritic and numeric TL — our Rust `to_zhuyin` expects numeric).
//! 3. `phonetics::tps::to_zhuyin(token, encode_safe=false,
//!    or_maps_to_er=false)` — same call shape the Node bridge uses
//!    with the toggle-OFF default.
//! 4. Drop the 8 TPS tone marks per `phonetics::is_tps_tone_mark`.
//! 5. Concat tokens fused (no separator).
//!
//! The production `dictionary.fst` is built from the precomputed
//! `record.tps_notone` column in `dictionary/output/dictionary.csv` (see
//! `dictionary/build/create_fst.py:140` + `merge_csv.py:265`). If the
//! Rust runtime derivation drifts from the build-pipeline derivation,
//! the continuous-input guard silently rejects every TPS family hit
//! whose `record.tl ↔ tps_notone` mapping diverges — and a stale
//! Rust port loses an abbreviation-collision protection without
//! visible signal.
//!
//! This parity test reads the shipped CSV and asserts byte-match for
//! every row whose `tl` and `tps_notone` are both non-empty. If the CSV
//! is not present (lean checkout) the test soft-skips. Pure-Bopomofo
//! `tps_notone` is expected — rows with stray ASCII / Latin in
//! `tps_notone` come from upstream build-pipeline anomalies (residue
//! the Node bridge could not convert cleanly) that the runtime guard
//! cannot reach via a `tps:<bopomofo>` lookup key anyway and are
//! skipped.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn dictionary_csv_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // engine
    path.pop(); // repo root
    path.push("dictionary");
    path.push("output");
    path.push("dictionary.csv");
    path
}

/// Parse `dictionary.csv` returning `(tl, tps_notone, tps_notone_var)`
/// per row. C-5 extended the row tuple to include the variant column
/// for the er↔or dual-emit parity test below.
fn read_rows(path: &std::path::Path) -> std::io::Result<Vec<(String, String, String)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let header = lines.next().expect("CSV header line present")?;
    let columns: Vec<&str> = header.split(',').collect();
    let tl_idx = columns
        .iter()
        .position(|c| *c == "tl")
        .expect("`tl` column present in CSV header");
    let tps_notone_idx = columns
        .iter()
        .position(|c| *c == "tps_notone")
        .expect("`tps_notone` column present in CSV header");
    let tps_notone_var_idx = columns
        .iter()
        .position(|c| *c == "tps_notone_var")
        .expect("`tps_notone_var` column present in CSV header");

    let mut rows = Vec::new();
    let max_idx = tl_idx.max(tps_notone_idx).max(tps_notone_var_idx);
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() <= max_idx {
            continue;
        }
        rows.push((
            fields[tl_idx].to_string(),
            fields[tps_notone_idx].to_string(),
            fields[tps_notone_var_idx].to_string(),
        ));
    }
    Ok(rows)
}

/// True when every char of `s` is a Bopomofo / Bopomofo Extended scalar
/// (the shape `tps_notone` should have for any continuous-input-reachable
/// row). Mirrors `phonetics::is_tps_char` predicate inline so the test
/// stays self-contained.
fn is_pure_bopomofo(s: &str) -> bool {
    !s.is_empty() && s.chars().all(phonetics::is_tps_char)
}

#[test]
fn runtime_tps_notone_matches_build_pipeline_for_every_row() {
    let path = dictionary_csv_path();
    if !path.exists() {
        eprintln!(
            "skipping TPS-notone parity test: {} not present (lean checkout)",
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

    for (tl, tps_notone, _var) in &rows {
        total += 1;
        if tl.is_empty() || tps_notone.is_empty() {
            continue;
        }
        // Upstream build-pipeline anomalies (residue Node bridge could
        // not cleanly convert) leave non-Bopomofo glyphs in
        // `tps_notone`. No continuous TPS user input can produce a
        // `tps:<non-bopomofo>` lookup key, so the guard cannot reach
        // them. Skip from the parity gate.
        if !is_pure_bopomofo(tps_notone) {
            anomalies += 1;
            continue;
        }
        compared += 1;
        let derived = phonetics::tps_notone_from_tl(tl);
        if derived != *tps_notone {
            drift.push((tl.clone(), tps_notone.clone(), derived));
            if drift.len() >= 10 {
                break;
            }
        }
    }

    assert!(
        drift.is_empty(),
        "runtime TPS-notone derivation drifted from build pipeline on \
         {} of {} compared rows (first {} shown):\n{}",
        drift.len(),
        compared,
        drift.len(),
        drift
            .iter()
            .map(|(tl, expected, got)| format!(
                "  tl={tl:?} expected tps_notone={expected:?} got {got:?}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    eprintln!(
        "TPS-notone parity OK: {compared}/{total} rows compared \
         ({anomalies} non-Bopomofo anomaly rows skipped)."
    );
}

/// v3.5.9 D / C-3b — Codex post-impl BLOCK pin. C-3a's build pipeline
/// dual-emits `tps:<tps_notone>` (primary, ㄜ for er/or) AND
/// `tps:<tps_notone_var>` (ㄛ variant) per row whose primary contains
/// ㄜ. The continuous toneless-key guard must accept BOTH forms or it
/// silently rejects valid ㄛ-form input.
///
/// This test exercises the variant helper byte-shape against a small
/// curated set drawn from CSV (rows known to carry ㄜ in their primary
/// `tps_notone`). The full-row parity is covered by the parent test —
/// this one pins the variant substitution semantics in isolation so a
/// future refactor cannot quietly drop the ㄜ→ㄛ swap.
#[test]
fn tps_notone_or_variant_substitutes_er_to_or_glyph() {
    // Primary form (toggle ON / er-glyph) → variant form (toggle OFF / or-glyph).
    let cases = [
        ("\u{3109}\u{311c}", "\u{3109}\u{311b}"), // ㄉㄜ → ㄉㄛ (tor / tór)
        ("\u{3110}\u{311c}", "\u{3110}\u{311b}"), // ㄋㄜ → ㄋㄛ (lor)
        ("\u{310e}\u{311c}\u{31b7}", "\u{310e}\u{311b}\u{31b7}"), // ㄏㄜㆷ → ㄏㄛㆷ (orh)
        // Multi-er: every ㄜ swaps (no single-occurrence shortcut).
        (
            "\u{3109}\u{311c}\u{3110}\u{311c}",
            "\u{3109}\u{311b}\u{3110}\u{311b}",
        ),
    ];
    for (primary, expected) in cases {
        let got = phonetics::tps_notone_or_variant(primary);
        assert_eq!(
            got, expected,
            "tps_notone_or_variant({primary:?}) expected {expected:?} got {got:?}",
        );
    }
    // Bodies without ㄜ return empty (caller skips redundant variant emit).
    assert_eq!(phonetics::tps_notone_or_variant("\u{3109}\u{311e}"), ""); // ㄉㄞ
    assert_eq!(phonetics::tps_notone_or_variant(""), "");
}

/// v3.5.9 D / C-5 — full-CSV `tps_notone_var` parity (D capstone). C-3a
/// dual-emits `tps:<tps_notone>` AND `tps:<tps_notone_var>` per row whose
/// primary contains ㄜ (1107 rows in the shipped CSV). The runtime
/// helper [`phonetics::tps_notone_or_variant`] derives the variant from
/// the primary via ㄜ→ㄛ substitution; this test asserts byte-match
/// against the shipped column row-for-row.
///
/// Drift here means the C-3a build-pipeline `apply_or_dialect_variant`
/// and the Rust runtime mirror diverged — the continuous toneless-key
/// guard would silently reject the ㄛ variant for divergent rows.
#[test]
fn runtime_tps_notone_var_matches_build_pipeline_for_every_row() {
    let path = dictionary_csv_path();
    if !path.exists() {
        eprintln!(
            "skipping TPS-notone-var parity test: {} not present (lean checkout)",
            path.display()
        );
        return;
    }

    let rows = read_rows(&path).expect("read dictionary.csv rows");
    // Iterate every row with a non-empty pure-Bopomofo primary, compute
    // the runtime-derived variant, and compare to the CSV variant —
    // INCLUDING the empty-string case (no ㄜ → no variant). Skipping
    // empty-var rows would mask a regression where the build pipeline
    // stops populating `tps_notone_var` entirely. Codex post-impl
    // BLOCK 2026-05-26.
    let mut compared = 0usize;
    let mut anomalies = 0usize;
    let mut nonempty_vars = 0usize;
    let mut drift: Vec<(String, String, String)> = Vec::new();

    for (_tl, notone, var) in &rows {
        if notone.is_empty() {
            continue;
        }
        if !is_pure_bopomofo(notone) || (!var.is_empty() && !is_pure_bopomofo(var)) {
            anomalies += 1;
            continue;
        }
        compared += 1;
        let derived = phonetics::tps_notone_or_variant(notone);
        if !var.is_empty() {
            nonempty_vars += 1;
        }
        if derived != *var {
            drift.push((notone.clone(), var.clone(), derived));
            if drift.len() >= 10 {
                break;
            }
        }
    }

    assert!(
        drift.is_empty(),
        "runtime tps_notone_or_variant(tps_notone) drifted from build \
         pipeline tps_notone_var on {} of {} compared rows (first {} \
         shown):\n{}",
        drift.len(),
        compared,
        drift.len(),
        drift
            .iter()
            .map(|(notone, expected, got)| format!(
                "  tps_notone={notone:?} expected tps_notone_var={expected:?} got {got:?}"
            ))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // Coverage gate (Codex post-impl Finding 1): if the build pipeline
    // ever stops emitting variants entirely, the column-level parity
    // above stays empty-vs-empty consistent and prints "0 drift"
    // misleadingly. The shipped CSV currently has 1107 non-empty
    // `tps_notone_var` rows (per C-3a build pipeline + or-vowel
    // inventory); accept any non-zero count as a regression alarm
    // boundary rather than pinning the exact number (which would force
    // a parity-test re-pin on every dictionary refresh).
    assert!(
        nonempty_vars > 0,
        "expected the shipped CSV to contain at least one non-empty \
         `tps_notone_var` row (C-3a build pipeline emits dual-form for \
         er/or-vowel rows). Found 0 — build pipeline regressed or the \
         CSV column is unpopulated."
    );

    eprintln!(
        "TPS-notone-var parity OK: {compared} rows compared, \
         {nonempty_vars} non-empty variants ({anomalies} anomaly rows skipped)."
    );
}
