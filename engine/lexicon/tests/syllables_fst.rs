//! Engine-side contract test for the v3.5.9 B-1 tagged-single-FST
//! syllable inventory.
//!
//! The v3.5.8 Phase 2 contract (`docs/releases/v3.5.8/plan.md` § Phase 2 — TL syllable inventory FST)
//! called for 50 valid + 50 invalid samples covering boundary cases
//! `bak (T4)`, `tai (T1/T5/T7)`, `khih (T4-h)`, `m`, `ng`, `tsh`, `oo`,
//! `uainn`. v3.5.9 B-1 extends this to two families:
//!
//! 1. **TL family** — the original 50/50 matrix, now queried via
//!    `SyllableInventory::contains_in(InputMode::Tl, …)` against keys
//!    stored with the `tl:` prefix.
//!
//! 2. **POJ family** — a smaller pin test verifying POJ-shaped
//!    syllables that diverge from TL (`chit`, `goa`, `toa`, `che`,
//!    `koe`, `peng`) land under the `poj:` prefix and route through
//!    `contains_in(InputMode::Poj, …)`, while the same canonical
//!    strings are **absent** from the TL family (and vice versa).
//!
//! Loader behaviour is intentionally minimal (membership only); the
//! syllabifier crate exercises prefix walking. See
//! `docs/reports/2026-05-20-v359-b-plan.md` §B-1.

use std::path::PathBuf;

use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::{canonicalize_poj_syllable, canonicalize_syllable, InputMode};

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
            inv.contains_in(InputMode::Tl, &numeric),
            "numeric `{}` (from {}) missing in tl: family",
            numeric,
            input
        );
        assert!(
            inv.contains_in(InputMode::Tl, canonical),
            "toneless `{}` (from {}) missing in tl: family",
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
        // Membership is byte-exact — invalid raw strings must not appear
        // in either family.
        assert!(
            !inv.contains_in(InputMode::Tl, s),
            "INVALID `{s}` unexpectedly in tl: family"
        );
        assert!(
            !inv.contains_in(InputMode::Poj, s),
            "INVALID `{s}` unexpectedly in poj: family"
        );
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
    assert!(inv.contains_in(InputMode::Tl, "tai1"));
    assert!(inv.contains_in(InputMode::Tl, "tai5"));
    assert!(inv.contains_in(InputMode::Tl, "tai7"));
    assert!(inv.contains_in(InputMode::Tl, "tai"));
    assert!(!inv.contains_in(InputMode::Tl, "tai2"));
}

#[test]
fn open_round_trips_through_mmap() {
    let inv = build_inventory_from_pairs(&[
        canonicalize_syllable("ka1").unwrap(),
        canonicalize_syllable("oo1").unwrap(),
    ]);
    assert!(inv.contains_in(InputMode::Tl, "ka1"));
    assert!(inv.contains_in(InputMode::Tl, "ka"));
    assert!(inv.contains_in(InputMode::Tl, "oo1"));
    assert!(inv.contains_in(InputMode::Tl, "oo"));
    assert_eq!(inv.entry_count(), 4);
    assert!(!inv.is_empty());
}

/// v3.5.9 B-1 — POJ family pin. POJ-shaped syllables whose TL fold
/// would land on a different canonical (`chit`/`tsit`, `goa`/`gua`,
/// `toa`/`tua`, `che`/`tse`, `koe`/`kue`, `peng`/`ping`) must stay
/// under the `poj:` prefix, queryable through
/// `contains_in(InputMode::Poj, …)`, and **must not** leak into the
/// `tl:` family (and vice versa for the TL forms).
#[test]
fn poj_family_isolates_divergent_canonicals_from_tl() {
    let divergent_pairs: &[(&str, &str)] = &[
        ("chit8", "tsit8"),
        ("goa2", "gua2"),
        ("toa7", "tua7"),
        ("che1", "tse1"),
        ("koe1", "kue1"),
        ("peng5", "ping5"),
        ("pek4", "pik4"),
    ];
    let tl_pairs: Vec<(String, String)> = divergent_pairs
        .iter()
        .map(|(_, tl)| canonicalize_syllable(tl).unwrap())
        .collect();
    let poj_pairs: Vec<(String, String)> = divergent_pairs
        .iter()
        .map(|(poj, _)| canonicalize_poj_syllable(poj).unwrap())
        .collect();
    let inv = build_dual_inventory(&tl_pairs, &poj_pairs);

    for ((poj_in, tl_in), (tl_canon, _)) in divergent_pairs.iter().zip(tl_pairs.iter()) {
        assert!(
            inv.contains_in(InputMode::Tl, tl_canon),
            "TL `{tl_canon}` (from {tl_in}) missing in tl: family"
        );
        // POJ-shape canonical absent from TL family — proves no leak.
        let poj_canon = canonicalize_poj_syllable(poj_in).unwrap().0;
        if poj_canon != *tl_canon {
            assert!(
                !inv.contains_in(InputMode::Tl, &poj_canon),
                "POJ `{poj_canon}` (from {poj_in}) leaked into tl: family"
            );
        }
    }
    for ((poj_in, tl_in), (poj_canon, _)) in divergent_pairs.iter().zip(poj_pairs.iter()) {
        assert!(
            inv.contains_in(InputMode::Poj, poj_canon),
            "POJ `{poj_canon}` (from {poj_in}) missing in poj: family"
        );
        // TL-shape canonical absent from POJ family — proves no leak.
        let tl_canon = canonicalize_syllable(tl_in).unwrap().0;
        if tl_canon != *poj_canon {
            assert!(
                !inv.contains_in(InputMode::Poj, &tl_canon),
                "TL `{tl_canon}` (from {tl_in}) leaked into poj: family"
            );
        }
    }
}

/// v3.5.9 B-1 production-artifact sentinel — open the checked-in
/// `dictionary/output/syllables.fst` and probe one `tl:*` + one
/// `poj:*` membership, plus a cross-family negative. Codex post-impl
/// B-1 finding #2: `SyllableInventory::open` only validates FST parse,
/// so loading an unprefixed pre-B asset would succeed but every
/// `contains_in` probe would silently miss. This sentinel binds the
/// checked-in binary to the tagged-single-FST format so a regression
/// (e.g. accidentally checking in an unregenerated pre-B `.fst`) fails
/// loud here instead of degrading runtime continuous input.
#[test]
fn production_syllables_fst_carries_both_tl_and_poj_families() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionary/output/syllables.fst");
    if !path.exists() {
        // Dev / CI environments that strip the data artifact run
        // every other test in this file unchanged; skip with a
        // diagnostic so the absence is visible.
        eprintln!(
            "[sentinel] {} missing — production asset not bundled here; skipping",
            path.display()
        );
        return;
    }
    let inv = SyllableInventory::open(&path).expect("open production syllables.fst");
    assert!(
        inv.contains_in(InputMode::Tl, "tsua"),
        "production fst missing TL canonical `tsua` (紙/珠仔 base); \
         the checked-in artifact may be pre-B unprefixed format"
    );
    assert!(
        inv.contains_in(InputMode::Poj, "chiah"),
        "production fst missing POJ canonical `chiah` (吃); \
         the checked-in artifact may be pre-B (TL-only) format"
    );
    // Negative pin: `chiah` (POJ shape) MUST NOT appear under the `tl:`
    // family — TL fold would map this to `tsiah`.
    assert!(
        !inv.contains_in(InputMode::Tl, "chiah"),
        "POJ-shape `chiah` unexpectedly under tl: family — family routing broken"
    );
    // Cross-direction: `tsiah` (TL shape) MUST NOT appear under poj:.
    assert!(
        !inv.contains_in(InputMode::Poj, "tsiah"),
        "TL-shape `tsiah` unexpectedly under poj: family — family routing broken"
    );
}

/// Build a `SyllableInventory` from canonical `(toneless, tone)` pairs by
/// emitting numeric + toneless keys, sort+dedup, then fst::SetBuilder →
/// temp file → `SyllableInventory::open`. Exercises the loader + mmap
/// path; does NOT cover the upstream `split_into_syllables` / residue
/// detection / invalid-skip stats — those are pinned in
/// `engine/build-helpers/fst-builder/src/syllables.rs#tests`.
fn build_inventory_from_pairs(pairs: &[(String, String)]) -> SyllableInventory {
    // v3.5.9 B-1: emit keys with the `tl:` family prefix so the
    // hermetic inventory matches the tagged-single-FST format.
    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in pairs {
        if tone.is_empty() {
            keys.push(format!("tl:{}", canonical));
        } else {
            keys.push(format!("tl:{}{}", canonical, tone));
            keys.push(format!("tl:{}", canonical));
        }
    }
    keys.sort();
    keys.dedup();

    write_set_to_temp_inventory(&keys)
}

/// v3.5.9 B-1 — build an inventory carrying BOTH `tl:` and `poj:`
/// families in one fst::Set, mirroring the production tagged-single-FST
/// layout produced by `fst-builder build-syllables --tl-input
/// --poj-input`.
fn build_dual_inventory(
    tl_pairs: &[(String, String)],
    poj_pairs: &[(String, String)],
) -> SyllableInventory {
    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in tl_pairs {
        if tone.is_empty() {
            keys.push(format!("tl:{}", canonical));
        } else {
            keys.push(format!("tl:{}{}", canonical, tone));
            keys.push(format!("tl:{}", canonical));
        }
    }
    for (canonical, tone) in poj_pairs {
        if tone.is_empty() {
            keys.push(format!("poj:{}", canonical));
        } else {
            keys.push(format!("poj:{}{}", canonical, tone));
            keys.push(format!("poj:{}", canonical));
        }
    }
    keys.sort();
    keys.dedup();
    write_set_to_temp_inventory(&keys)
}

/// Final fst::SetBuilder pass over already sorted+deduped keys.
fn write_set_to_temp_inventory(keys: &[String]) -> SyllableInventory {
    let path = unique_temp_path();
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for key in keys {
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
