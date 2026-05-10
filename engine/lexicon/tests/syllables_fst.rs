//! Engine-side contract test for the v3.5.8 Phase 2 TL syllable inventory.
//!
//! The roadmap (`docs/roadmap.md` §Phase 2 line 195) calls for 50 valid +
//! 50 invalid samples covering boundary cases `bak (T4)`, `tai (T1/T5/T7)`,
//! `khih (T4-h)`, `m`, `ng`, `tsh`, `oo`, `uainn`. We pin two contracts
//! here:
//!
//! 1. The fst::Set produced by `fst-builder build-syllables` (mirrored
//!    inline by `build_synthetic_inventory` so the test stays hermetic)
//!    contains BOTH numeric and toneless canonical forms for every valid
//!    syllable, and rejects every invalid sample.
//!
//! 2. The `SyllableInventory` loader round-trips through mmap correctly —
//!    `contains` returns true for inserted keys and false for absent keys.
//!
//! Loader behaviour is intentionally minimal in Phase 2 (membership only).
//! Phase 3 syllabifier work will extend the loader with prefix walking
//! and any metadata it needs.

// 中文: Phase 2 TL 音節合法集合 FST 的契約測試 — 50 合法 / 50 不合法樣本。
// 中文: 建合成 fst::Set,確認 SyllableInventory loader 透過 mmap 取出正確結果。

use std::path::PathBuf;

use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::canonicalize_syllable;

/// 50 phonotactically valid TL/POJ-shaped syllables. Boundary coverage
/// per roadmap §Phase 2 line 195 plus sampled common syllables. Note —
/// some entries (the `chh*` / `oa*` / `eng*` rows) are POJ-shaped on
/// purpose to verify the canonicalization pass before insertion.
const VALID_SAMPLES: &[&str] = &[
    // bak (T4) — stop coda T4
    "bak4", "bak8", // tai (T1, T5, T7)
    "tai1", "tai5", "tai7", "tai2", "tai3", // khih (T4-h)
    "khih4", "khih8", // syllabic consonants m / ng
    "m7", "m2", "ng5", "ng2", // oo final
    "oo1", "oo2", "oo7", "ooh4", // uainn — 4-letter nasal final
    "uainn3", "uainn1", // common high-frequency syllables (sampled from dictionary.csv)
    "ka1", "kang1", "tan5", "lang5", "lai5", "u7", "kong2", "tio8", "tioh8", "tin7", "guan2",
    "ti7",
    // POJ-shaped — exercises normalize_to_tl in canonicalize
    "chiau2", // → tsiau
    "chha1",  // → tsha
    "choa7",  // → tsua
    "eng1",   // → ing
    "pek4",   // → pik
    "koe1",   // → kue
    "peng5",  // → ping
    "chhin3", // → tshin
    "kha1",   // → kha
    // non-ASCII forms
    "peⁿ5",   // → penn
    "so͘3",    // → soo
    "tsiuⁿ7", // → tsiunn
    "pho͘5",   // → phoo
    "tho͘3",   // → thoo
    // region variants
    "er1", "er5", "ir3", "ee2", "ior1",
];

/// 50 phonotactic INVALID samples that must NOT be in the inventory.
const INVALID_SAMPLES: &[&str] = &[
    // initial-only (no final)
    "tsh", "kh", "ph", "th", "ts", // garbage
    "xyz", "qq", "tj", "bx", "zk", "abc1", "qwe2", "asdf3",
    // unknown finals (initial valid, final not in TL_FINALS)
    "tang2x", "lai9k", "kham4z", // empty / digit-only
    "", "1", "2", "9",    // dual-marked malformed (combining mark + trailing digit)
    "tn̄g6", // mixed garbage
    "fuzz1", "wxyz", "qrst", "vbnm1", "rtyu5", "iopl3", "hjkl2", "cvbn7", "ertyu1", "asdfg2",
    "qwerty5", "zxcvbn3", "aei", "uoy", "ttt", "ppp", "kkk", "ggg", "bbb", "ddd", "wq", "yu", "yp",
    // unknown initials / non-TL letters
    "fai1", "ren3", "wo5", "voo7", "fang2", "zai1",
];

#[test]
fn valid_samples_canonicalize_and_appear_in_both_forms() {
    let valid_canonical: Vec<(String, String)> = VALID_SAMPLES
        .iter()
        .map(|s| {
            canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("VALID sample {s:?} failed canonicalize"))
        })
        .collect();
    assert_eq!(
        VALID_SAMPLES.len(),
        50,
        "VALID_SAMPLES must have 50 entries (got {})",
        VALID_SAMPLES.len()
    );

    let inv = build_inventory_from_pairs(&valid_canonical);

    for (input, (canonical, tone)) in VALID_SAMPLES.iter().zip(valid_canonical.iter()) {
        let numeric = if tone.is_empty() {
            canonical.clone()
        } else {
            format!("{}{}", canonical, tone)
        };
        assert!(
            inv.contains(&numeric),
            "numeric `{}` (from {}) missing",
            numeric,
            input
        );
        assert!(
            inv.contains(canonical),
            "toneless `{}` (from {}) missing",
            canonical,
            input
        );
    }
}

#[test]
fn invalid_samples_canonicalize_to_none_and_stay_out_of_inventory() {
    assert_eq!(
        INVALID_SAMPLES.len(),
        50,
        "INVALID_SAMPLES must have 50 entries (got {})",
        INVALID_SAMPLES.len()
    );

    // Every invalid sample must reject canonicalization.
    for s in INVALID_SAMPLES {
        assert!(
            canonicalize_syllable(s).is_none(),
            "INVALID sample `{s}` unexpectedly canonicalized — fix samples or canonicalize_syllable"
        );
    }

    // Build inventory from VALID samples only; check INVALID samples are absent.
    let valid_canonical: Vec<(String, String)> = VALID_SAMPLES
        .iter()
        .filter_map(|s| canonicalize_syllable(s))
        .collect();
    let inv = build_inventory_from_pairs(&valid_canonical);

    for s in INVALID_SAMPLES {
        // Membership is byte-exact — invalid raw strings must not appear.
        assert!(!inv.contains(s), "INVALID `{s}` unexpectedly in inventory");
    }
}

#[test]
fn entry_count_matches_dedup_invariant() {
    // Multiple tones share one toneless key. e.g. tai1, tai5, tai7 → 4
    // FST keys: {tai1, tai5, tai7, tai}, NOT 6. Compute the expected
    // distinct-key set explicitly so the invariant is precise.
    let pairs = vec![
        canonicalize_syllable("tai1").unwrap(),
        canonicalize_syllable("tai5").unwrap(),
        canonicalize_syllable("tai7").unwrap(),
    ];
    let inv = build_inventory_from_pairs(&pairs);
    assert_eq!(inv.entry_count(), 4, "expected {{tai1, tai5, tai7, tai}}");
    assert!(inv.contains("tai1"));
    assert!(inv.contains("tai5"));
    assert!(inv.contains("tai7"));
    assert!(inv.contains("tai"));
    assert!(!inv.contains("tai2"));
}

#[test]
fn open_round_trips_through_mmap() {
    let inv = build_inventory_from_pairs(&[
        canonicalize_syllable("ka1").unwrap(),
        canonicalize_syllable("oo1").unwrap(),
    ]);
    assert!(inv.contains("ka1"));
    assert!(inv.contains("ka"));
    assert!(inv.contains("oo1"));
    assert!(inv.contains("oo"));
    assert_eq!(inv.entry_count(), 4);
    assert!(!inv.is_empty());
}

/// Build a `SyllableInventory` from canonical `(toneless, tone)` pairs by
/// emitting numeric + toneless keys, sort+dedup, then fst::SetBuilder →
/// temp file → `SyllableInventory::open`. Exercises the loader + mmap
/// path; does NOT cover the upstream `split_into_syllables` / residue
/// detection / invalid-skip stats — those are pinned in
/// `engine/build-helpers/fst-builder/src/syllables.rs#tests`.
fn build_inventory_from_pairs(pairs: &[(String, String)]) -> SyllableInventory {
    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in pairs {
        if tone.is_empty() {
            keys.push(canonical.clone());
        } else {
            keys.push(format!("{}{}", canonical, tone));
            keys.push(canonical.clone());
        }
    }
    keys.sort();
    keys.dedup();

    let path = unique_temp_path();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for key in &keys {
        builder.insert(key.as_bytes()).expect("insert");
    }
    builder.finish().expect("finish");
    SyllableInventory::open(&path).expect("open inventory")
}

fn unique_temp_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("lexicon-test-syllables-{pid}-{n}.fst"))
}
