//! §43 — runtime `tl_num` / `poj_num` derivation parity test.
//!
//! `SyllableReach` (`lexicon::continuous`) decides whether a strict-prefix
//! extension may be offered by measuring how far the typed body reaches into
//! the record's reading, ON THE KEY SURFACE the row was found under. For a
//! tone-bearing body that surface is the `tl:<tl_num>` / `poj:<poj_num>` family
//! `create_fst.py:141` emits from the precomputed `tl_num` / `poj_num` columns
//! of `dictionary/output/dictionary.csv`, and the runtime mirrors of those
//! columns are `phonetics::tl_num_syllable_ends_from_tl` /
//! `phonetics::poj_num_syllable_ends_from_tl`.
//!
//! Drift is SILENT: a mirror that disagrees with the column no longer matches
//! its own key, `SyllableReach` fails open, and the over-length candidates
//! §43 exists to remove come back for every affected reading — with no error,
//! no panic and no failing unit test, because a hermetic fixture builds its
//! keys from the same mirror it is testing. This file is the only thing that
//! catches it, so it reads the shipped CSV and asserts a byte-match for every
//! row.
//!
//! Two conventions, deliberately: the `tl_num` column keeps the reading's own
//! glyphs (`thò͘-sái` → `tho͘3sai2`) while `poj_num` is ASCII-folded
//! (`khuànn` → `khoann3`). Applying either convention to both columns costs
//! tens of thousands of divergences; this is why the two mirrors are separate
//! functions rather than one with a mode flag.
//!
//! Soft-skips when the CSV is absent (lean checkout), like its
//! `tps_notone_parity` / `poj_notone_parity` siblings.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn dictionary_csv_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // engine
    path.pop(); // repo root
    path.push("dictionary");
    path.push("output");
    path.push("dictionary.csv");
    path
}

/// One CSV row's reading plus the four precomputed romanization key columns
/// `create_fst.py` emits key families from.
struct Row {
    tl: String,
    tl_num: String,
    poj_num: String,
    tl_notone: String,
    poj_notone: String,
}

fn read_rows(path: &Path) -> std::io::Result<Vec<Row>> {
    let reader = BufReader::new(File::open(path)?);
    let mut lines = reader.lines();
    let header = lines.next().expect("CSV header line present")?;
    let columns: Vec<&str> = header.split(',').collect();
    let column = |name: &str| {
        columns
            .iter()
            .position(|c| *c == name)
            .unwrap_or_else(|| panic!("`{name}` column present in CSV header"))
    };
    let (tl_idx, tl_num_idx, poj_num_idx) = (column("tl"), column("tl_num"), column("poj_num"));
    let (tl_notone_idx, poj_notone_idx) = (column("tl_notone"), column("poj_notone"));
    let max_idx = tl_idx
        .max(tl_num_idx)
        .max(poj_num_idx)
        .max(tl_notone_idx)
        .max(poj_notone_idx);

    let mut rows = Vec::new();
    for line in lines {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() <= max_idx {
            continue;
        }
        rows.push(Row {
            tl: fields[tl_idx].to_string(),
            tl_num: fields[tl_num_idx].to_string(),
            poj_num: fields[poj_num_idx].to_string(),
            tl_notone: fields[tl_notone_idx].to_string(),
            poj_notone: fields[poj_notone_idx].to_string(),
        });
    }
    Ok(rows)
}

/// A reading the continuous path can actually reach: a `tl:`/`poj:` lookup key
/// is romanization, so a row whose `tl` column holds 漢字 (an upstream
/// build-pipeline anomaly) is unreachable and out of the gate — the same
/// carve-out `tps_notone_parity` makes for non-Bopomofo `tps_notone`.
fn is_romanization(reading: &str) -> bool {
    !reading.is_empty() && reading.chars().any(|c| c.is_ascii_alphabetic())
}

#[test]
fn runtime_tl_num_matches_build_pipeline_for_every_row() {
    assert_column_parity("tl_num", |tl| phonetics::tl_num_syllable_ends_from_tl(tl).0);
}

#[test]
fn runtime_poj_num_matches_build_pipeline_for_every_row() {
    assert_column_parity("poj_num", |tl| {
        phonetics::poj_num_syllable_ends_from_tl(tl).0
    });
}

#[test]
fn runtime_tl_notone_matches_build_pipeline_for_every_row() {
    assert_column_parity("tl_notone", |tl| {
        strip_digits(&phonetics::tl_num_syllable_ends_from_tl(tl).0)
    });
}

#[test]
fn runtime_poj_notone_matches_build_pipeline_for_every_row() {
    assert_column_parity("poj_notone", |tl| {
        strip_digits(&phonetics::poj_num_syllable_ends_from_tl(tl).0)
    });
}

fn strip_digits(face: &str) -> String {
    face.chars().filter(|c| !c.is_ascii_digit()).collect()
}

/// Every syllable end offset must land on a char boundary of the face and the
/// last one must be its full length — `SyllableReach` slices by these.
#[test]
fn syllable_ends_bound_the_face_they_describe() {
    for tl in [
        "tâi-uân",
        "kau-kuan",
        "ke-si-thâu-á",
        "hōo--guá",
        "koh",
        "thò͘-sái",
    ] {
        for (face, ends) in [
            phonetics::tl_num_syllable_ends_from_tl(tl),
            phonetics::poj_num_syllable_ends_from_tl(tl),
        ] {
            assert!(!ends.is_empty(), "{tl}: expected at least one syllable");
            assert_eq!(
                ends.last().copied().unwrap() as usize,
                face.len(),
                "{tl}: last end must be the face length ({face})"
            );
            for end in &ends {
                assert!(
                    face.is_char_boundary(*end as usize),
                    "{tl}: end {end} is not a char boundary of {face}"
                );
            }
            assert!(
                ends.windows(2).all(|w| w[0] < w[1]),
                "{tl}: ends must strictly increase, got {ends:?}"
            );
        }
    }
}

fn assert_column_parity(column: &str, derive: impl Fn(&str) -> String) {
    let path = dictionary_csv_path();
    if !path.exists() {
        eprintln!(
            "skipping {column} parity test: {} not present (lean checkout)",
            path.display()
        );
        return;
    }
    let rows = read_rows(&path).expect("read dictionary.csv rows");
    assert!(
        !rows.is_empty(),
        "expected non-empty `dictionary.csv` row stream"
    );

    let mut compared = 0usize;
    let mut skipped = 0usize;
    let mut drifted = 0usize;
    let mut drift: Vec<(String, String, String)> = Vec::new();
    for row in &rows {
        let expected = match column {
            "tl_num" => &row.tl_num,
            "poj_num" => &row.poj_num,
            "tl_notone" => &row.tl_notone,
            _ => &row.poj_notone,
        };
        if row.tl.is_empty() || expected.is_empty() {
            continue;
        }
        if !is_romanization(&row.tl) {
            skipped += 1;
            continue;
        }
        compared += 1;
        let derived = derive(&row.tl);
        if &derived != expected {
            drifted += 1;
            if drift.len() < 20 {
                drift.push((row.tl.clone(), derived, expected.clone()));
            }
        }
    }

    assert!(
        compared > 100_000,
        "expected the shipped CSV to carry >100k comparable `{column}` rows, got {compared}"
    );
    assert!(
        drifted == 0,
        "runtime `{column}` derivation drifted from the build pipeline on {drifted} of {compared} \
         rows ({skipped} non-romanization rows skipped); first divergences (tl, derived, csv): \
         {drift:#?}"
    );
}
