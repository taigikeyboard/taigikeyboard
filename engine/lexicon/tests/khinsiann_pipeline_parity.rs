//! Parity between `phonetics::api::{align_hanji_syllables, render_hanji_khinsiann}`
//! and `dictionary/common/cleanup.py` (`tokenize_hanji_syllables`, `format_khinsiann_hanji`),
//! plus reachability checks for `--`-opening readings in the shipped dictionary.

use std::collections::HashSet;
use std::path::PathBuf;

use lexicon::{TonePin, TypedBoundary};
use phonetics::api::{align_hanji_syllables, render_hanji_khinsiann, strip_khinsiann_hanji};
use test_support::{dictionary_csv_or_skip, dictionary_csv_path};

fn repo_root() -> PathBuf {
    dictionary_csv_path()
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf()
}

fn is_kana_only(text: &str) -> bool {
    let mut saw_kana = false;
    for c in text.chars() {
        if matches!(c, ' ' | '\u{3000}' | '-') {
            continue;
        }
        let u = c as u32;
        let is_kana = (0x3040..=0x309F).contains(&u)
            || (0x30A0..=0x30FF).contains(&u)
            || (0x31F0..=0x31FF).contains(&u)
            || (0xFF65..=0xFF9F).contains(&u);
        if !is_kana {
            return false;
        }
        saw_kana = true;
    }
    saw_kana
}

#[test]
fn rust_aligner_matches_pipeline_alignment_report() {
    let Some(csv) = dictionary_csv_or_skip("khinsiann_pipeline") else {
        return;
    };
    let report_path = repo_root().join("dictionary/output/alignment_report.txt");
    if !report_path.exists() {
        eprintln!(
            "skipping alignment_report parity: {} absent",
            report_path.display()
        );
        return;
    }
    let report_text = std::fs::read_to_string(&report_path).expect("read alignment_report.txt");
    let expected: HashSet<(String, String)> = report_text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("hanzi\t"))
        .map(|l| {
            let mut parts = l.split('\t');
            (
                parts.next().unwrap_or("").to_owned(),
                parts.next().unwrap_or("").to_owned(),
            )
        })
        .collect();

    let mut actual: HashSet<(String, String)> = HashSet::new();
    for [hanzi, tl] in csv.select(["hanzi", "tl"]) {
        if hanzi.is_empty() || is_kana_only(hanzi) {
            continue;
        }
        if align_hanji_syllables(hanzi, tl).is_none() {
            actual.insert((hanzi.to_owned(), tl.to_owned()));
        }
    }
    assert_eq!(
        actual, expected,
        "Rust aligner drifted from Python alignment_report.txt"
    );
}

#[test]
fn rust_display_transform_reproduces_curated_lkk_entries() {
    let lkk_path = repo_root().join("dictionary/sources/curated/lkk/data/lkk.csv");
    if !lkk_path.exists() {
        return;
    }
    let text = std::fs::read_to_string(&lkk_path).expect("read lkk.csv");
    for line in text.lines().skip(1) {
        if line.is_empty() {
            continue;
        }
        let (hanzi, tl) = line.split_once(',').expect("hanzi,tl in lkk.csv");
        if (hanzi.contains("--") || tl.contains("--")) && phonetics::api::is_hanji(hanzi) {
            let stripped = strip_khinsiann_hanji(hanzi);
            assert_eq!(
                render_hanji_khinsiann(&stripped, tl, true),
                hanzi,
                "lkk row ({hanzi:?}, {tl:?}) failed round-trip from {stripped:?}"
            );
        }
    }
}

#[test]
fn khinsiann_opening_readings_match_with_or_without_typed_double_dash() {
    let Some(csv) = dictionary_csv_or_skip("khinsiann_reach") else {
        return;
    };
    let mut opening_rows: Vec<(String, String, String)> = Vec::new();
    for [hanzi, tl, tl_notone] in csv.select(["hanzi", "tl", "tl_notone"]) {
        if tl.starts_with("--") {
            opening_rows.push((hanzi.to_owned(), tl.to_owned(), tl_notone.to_owned()));
        }
    }
    assert!(
        !opening_rows.is_empty(),
        "expected `--`-opening rows in dictionary.csv"
    );

    // Every curated `--`-opening reading matches without typing its leading
    //    marker, and still matches after an explicit neutral boundary.
    for (hanzi, tl, tl_notone) in &opening_rows {
        let pin = TonePin::TypedTones {
            mode: phonetics::InputMode::Tl,
            typed: tl_notone.clone(),
            boundaries: vec![TypedBoundary {
                at: 0,
                khinsiann: true,
            }],
            one_syllable_stretches: Vec::new(),
        };
        assert!(
            pin.admits(Some(&format!("tl:{tl_notone}")), tl),
            "`--`-opening row ({hanzi:?}, {tl:?}) rejected after typed `--` on {tl_notone:?}"
        );
        assert!(
            TonePin::None.admits(Some(&format!("tl:{tl_notone}")), tl),
            "`--`-opening row ({hanzi:?}, {tl:?}) rejected without typed `--`"
        );
    }
}
