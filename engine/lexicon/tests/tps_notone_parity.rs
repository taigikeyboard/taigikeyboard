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

// 中文: D / C-3b — runtime TPS-notone derive 與 build pipeline 平行性測試。
// 中文:   matches_continuous_tps_toneless_key 採 phonetics::tps_notone_from_tl(record.tl)
// 中文:   推導,須與 dictionary.csv 內 build pipeline 預算的 tps_notone 一致。
// 中文:   非純 Bopomofo `tps_notone` 為 build pipeline 上游異常,連續輸入觸不到,跳過。

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

/// Parse `dictionary.csv` returning `(tl, tps_notone)` per row.
fn read_rows(path: &std::path::Path) -> std::io::Result<Vec<(String, String)>> {
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

    let mut rows = Vec::new();
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() <= tps_notone_idx.max(tl_idx) {
            continue;
        }
        rows.push((
            fields[tl_idx].to_string(),
            fields[tps_notone_idx].to_string(),
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

    for (tl, tps_notone) in &rows {
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
        ("\u{3109}\u{311c}", "\u{3109}\u{311b}"),                   // ㄉㄜ → ㄉㄛ (tor / tór)
        ("\u{3110}\u{311c}", "\u{3110}\u{311b}"),                   // ㄋㄜ → ㄋㄛ (lor)
        ("\u{310e}\u{311c}\u{31b7}", "\u{310e}\u{311b}\u{31b7}"),  // ㄏㄜㆷ → ㄏㄛㆷ (orh)
        // Multi-er: every ㄜ swaps (no single-occurrence shortcut).
        ("\u{3109}\u{311c}\u{3110}\u{311c}", "\u{3109}\u{311b}\u{3110}\u{311b}"),
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
