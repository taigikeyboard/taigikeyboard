//! v3.5.8 Phase 9 Item 9 — POJ-display canonicalization integration matrix.
//!
//! Pins `build_keys_tl_with_inventory` against a hermetic
//! `SyllableInventory` for the shapes Codex's pre-impl + post-impl
//! consults (2026-05-15) called out as the user-visible coverage gap
//! and the offset-map atomic-fold regression hot-spot:
//!
//! - `pe̍h-ōe-jī` (NFC) — POJ display with combining tone marks and the
//!   `oe→ue` substitution, the showcase scenario for 白話字 recovery.
//! - `pe̍h-ōe-jī` (NFD canary) — one NFD form to lock parity per
//!   F5C; full NFC/NFD duplication is over-built per YAGNI.
//! - `chóa` — POJ initial `ch→ts` plus `oa→ua` plus combining tone-2
//!   acute on the base vowel.
//! - `pe\u{207f}` — POJ nasal superscript `ⁿ` → `nn`.
//! - `so\u{0358}` — POJ `o + U+0358 (combining dot above right)` → `oo`.
//! - `so\u{0358}` with `so` + `soo` sibling inventory — Codex post-impl
//!   P1 regression guard: the partial-prefix `tl:so` candidate must
//!   consume the whole source spelling, not just `so`, so the
//!   combining dot never dangles in the pending buffer.
//! - `tâi-ōe` — mixed: combining circumflex on first syllable, ASCII
//!   hyphen, combining macron on second syllable.
//! - `tâi5-ban3` — combining circumflex AND trailing ASCII tone digit
//!   on the same first syllable (regression for the dual-marker edge
//!   case Codex highlighted).
//! - `taibak` (regression) — pure-ASCII input must remain byte-for-byte
//!   identical to the Item 8 pipeline; the F3C identity-fast-path is
//!   what keeps `tó-uī` (`dictionary/output/dictionary.csv:1984`,
//!   `tl_notone=toui`) from being garbled by `ou→oo`.
//! - `oe-ji` — ASCII-only F3C sanity: `oe→ue` substitution must NOT
//!   fire on pure ASCII input.
//!
//! The hermetic inventory builder mirrors
//! `engine/composing/tests/build_keys_tl_hyphen.rs:177-214`.

// 中文: Phase 9 Item 9 — POJ-display canonicalize + offset map 的 end-to-end 測試。
// 中文: 九組案例:NFC 白話字 / NFD canary / chóa / peⁿ / so͘ / so͘ + so/soo 兄弟 / tâi-ōe / 混合 combining + digit / ASCII regression / ASCII-only 防呆。

use std::path::PathBuf;

use composing::dispatch::build_keys_tl_with_inventory;
use fst::SetBuilder;
use lexicon::SyllableInventory;
use phonetics::canonicalize_syllable;
use unicode_normalization::UnicodeNormalization;

#[test]
fn nfc_peh_oe_ji_surfaces_full_fused_key() {
    // `pe̍h-ōe-jī` (NFC, 13 bytes: 1+1+2+1+1+2+1+1+1+2 — `e̍h` is
    // not precomposed, `ō` and `ī` are precomposed). The candidate
    // ending at the whole buffer must produce `tl:pehueji`, which is
    // the live dictionary's `tl_notone` for 白話字
    // (`dictionary/output/dictionary.csv:3181`).
    let inv = build_inventory(&["peh8", "ue7", "ji7"]);
    let keys = build_keys_tl_with_inventory("pe\u{030d}h-\u{014d}e-j\u{012b}", &inv);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert!(
        mapped.contains(&((0, 13), "tl:pehueji")),
        "expected full-buffer `tl:pehueji` candidate, got {mapped:?}",
    );
    // The single-syllable `tl:peh` candidate must consume the whole
    // `pe̍h` (5 bytes: `p` + `e` + `\u{030d}` (2 bytes) + `h`). The
    // dropped `\u{030d}` raw bytes have to fold into the preceding
    // `e`'s raw_end so commit does not leave a dangling combining
    // mark in the pending buffer — Codex pre-impl risk #2 (2026-05-15).
    assert!(
        mapped.contains(&((0, 5), "tl:peh")),
        "expected `tl:peh` candidate at raw_end=5 (consumes whole `pe̍h`), got {mapped:?}",
    );
}

#[test]
fn nfd_peh_oe_ji_matches_nfc_canary() {
    // F5C single NFD canary. `pe̍h-ōe-jī` re-expressed as NFD doubles
    // the combining marks (each precomposed `ō` / `ī` decomposes into
    // base + macron). The canonical output must be byte-identical to
    // the NFC case; raw byte offsets shift because NFD itself is
    // longer in bytes.
    let nfc = "pe\u{030d}h-\u{014d}e-j\u{012b}";
    let nfd: String = nfc.nfd().collect();
    assert!(nfd.len() > nfc.len(), "NFD canary assumption violated");
    let inv = build_inventory(&["peh8", "ue7", "ji7"]);
    let keys = build_keys_tl_with_inventory(&nfd, &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:pehueji"),
        "NFD canary failed to surface `tl:pehueji`, got {key_strs:?}",
    );
    // The full-buffer candidate's raw_end must equal the NFD byte count
    // (not the NFC byte count) — proving the offset map tracked NFD
    // bytes faithfully.
    let full = keys
        .iter()
        .find(|(_, k)| k == "tl:pehueji")
        .expect("full-buffer candidate present");
    assert_eq!(full.0 .1 as usize, nfd.len(), "raw_end != NFD len");
}

#[test]
fn poj_initial_ch_substitution_canonicalizes_to_ts() {
    // `chóa` → NFD `cho + U+0301 + a` → drop combining → `choa` →
    // normalize_to_tl (`ch→ts`, then `oa→ua`) → `tsua`. The dictionary
    // already keys `紙` etc. as `tl_notone=tsua`.
    let inv = build_inventory(&["tsua7"]);
    let keys = build_keys_tl_with_inventory("ch\u{00f3}a", &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:tsua"),
        "expected `tl:tsua` after POJ `chóa` canonicalize, got {key_strs:?}",
    );
}

#[test]
fn poj_superscript_nasal_marker_becomes_nn() {
    // `pe\u{207f}` (5 bytes) → Phase 1 NFD walk emits `pe\u{207f}` →
    // Phase 2 `\u{207f}→nn` → `penn` → matches dict canonical for
    // 平 (`pee` + nasal) etc.
    let inv = build_inventory(&["penn1"]);
    let keys = build_keys_tl_with_inventory("pe\u{207f}", &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:penn"),
        "expected `tl:penn` after POJ superscript ⁿ canonicalize, got {key_strs:?}",
    );
    // Raw end for the full-buffer match equals the raw byte length (5)
    // — both ASCII bytes emitted by `\u{207f}→nn` collapse onto the
    // post-superscript raw_end.
    let full = keys
        .iter()
        .find(|(_, k)| k == "tl:penn")
        .expect("`tl:penn` present");
    assert_eq!(full.0 .1, 5, "raw_end after `pe\\u207f` must be 5");
}

#[test]
fn poj_o_with_dot_above_right_becomes_oo() {
    // `so\u{0358}` (4 bytes) → Phase 1 keeps `\u{0358}` → Phase 2
    // `o\u{0358}→oo` → `soo` → matches dict canonical for 數 etc.
    let inv = build_inventory(&["soo3"]);
    let keys = build_keys_tl_with_inventory("so\u{0358}", &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:soo"),
        "expected `tl:soo` after POJ `o\\u0358` canonicalize, got {key_strs:?}",
    );
}

#[test]
fn poj_o_dot_atomic_substitution_no_prefix_dangling_mark() {
    // Codex post-impl P1 (2026-05-15) regression: against the live
    // `dictionary.csv` sibling pair where both `so` (no tone, e.g.
    // 蓑) and `soo` (no tone, e.g. 數) exist, the syllabifier emits
    // BOTH endings — and the shorter `tl:so` candidate must NOT
    // commit raw_end=2 (which would leave the combining dot `\u{0358}`
    // dangling in the pending buffer). Both candidates have to
    // consume the whole `so\u{0358}` source spelling so the user
    // never sees a stranded combining mark.
    let inv = build_inventory(&["soo3", "so7"]);
    let keys = build_keys_tl_with_inventory("so\u{0358}", &inv);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    let so_candidate = mapped.iter().find(|(_, k)| *k == "tl:so");
    assert!(
        so_candidate.is_some(),
        "test fixture mismatch: expected `tl:so` candidate to be emitted, got {mapped:?}",
    );
    assert_eq!(
        so_candidate.unwrap().0,
        (0, 4),
        "`tl:so` must consume the whole `so\\u0358` (raw_end=4) to keep the combining dot from dangling, got {mapped:?}",
    );
    let soo_candidate = mapped.iter().find(|(_, k)| *k == "tl:soo");
    assert!(
        soo_candidate.is_some(),
        "expected `tl:soo` candidate to be emitted, got {mapped:?}",
    );
    assert_eq!(soo_candidate.unwrap().0, (0, 4),);
}

#[test]
fn mixed_combining_with_hyphen_chains_to_full_fused_key() {
    // `tâi-ōe` — combining circumflex on first syllable, ASCII hyphen,
    // combining macron on second syllable. Both transforms (Item 9
    // canonicalize + Item 8 hyphen-shadow) must compose without
    // shifting the offset map.
    let inv = build_inventory(&["tai5", "ue7"]);
    let keys = build_keys_tl_with_inventory("t\u{00e2}i-\u{014d}e", &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:tai"),
        "expected `tl:tai` single-syllable hit, got {key_strs:?}",
    );
    assert!(
        key_strs.contains(&"tl:taiue"),
        "expected fused `tl:taiue`, got {key_strs:?}",
    );
}

#[test]
fn dual_marker_combining_and_trailing_digit_canonicalizes() {
    // `tâi5-ban3` — combining circumflex on `â` AND trailing ASCII
    // tone digit `5` on the same first syllable. Codex pre-impl
    // 2026-05-15 expected key set must include `tl:taiban` (combining
    // is dropped during Phase 1 → `tai5-ban3` → hyphen-strip →
    // `tai5ban3` → digit-strip → `taiban`).
    let inv = build_inventory(&["tai5", "ban3"]);
    let keys = build_keys_tl_with_inventory("t\u{00e2}i5-ban3", &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        key_strs.contains(&"tl:taiban"),
        "expected `tl:taiban` after dual-marker canonicalize, got {key_strs:?}",
    );
}

#[test]
fn pure_ascii_input_takes_identity_fast_path() {
    // `taibak` — F3C pure-ASCII identity guard. Output must be
    // byte-identical to the Item 8 pipeline (consumed_span values
    // unchanged). Any drift here means `canonicalize_poj_shadow` is
    // running on ASCII input, which would risk garbling the `tó-uī`
    // class of real dictionary entries.
    let inv = build_inventory(&["tai5", "bak4"]);
    let keys = build_keys_tl_with_inventory("taibak", &inv);
    let mapped: Vec<((u32, u32), &str)> = keys
        .iter()
        .map(|(span, key)| (*span, key.as_str()))
        .collect();
    assert_eq!(
        mapped,
        vec![((0, 3), "tl:tai"), ((0, 6), "tl:taibak")],
        "pure-ASCII input must take the F3C identity fast path",
    );
}

#[test]
fn ascii_only_poj_spellings_skip_canonicalize() {
    // F3C guard end-to-end: `oe-ji` is pure ASCII so canonicalize is a
    // no-op; the `oe→ue` substitution does NOT fire, and the FST
    // lookup against `oeji` misses. Without this guard, ASCII-only
    // dictionary entries whose `tl_notone` legitimately contain `oa`,
    // `oe`, `ou`, etc. (e.g. `toui` for 刀位) would be mis-rewritten
    // into `uai`, `uei`, `oo`. Test asserts the negative: no
    // `tl:ueji` or `tl:tooi` candidate emerges from pure ASCII.
    let inv = build_inventory(&["ue7", "ji7"]);
    let keys = build_keys_tl_with_inventory("oe-ji", &inv);
    let key_strs: Vec<&str> = keys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(
        !key_strs.iter().any(|k| k.contains("ue")),
        "ASCII-only `oe-ji` must NOT trigger `oe→ue` substitution, got {key_strs:?}",
    );
}

// ---- Hermetic SyllableInventory builder -----------------------------
// Pattern mirrors `engine/composing/tests/build_keys_tl_hyphen.rs:177-214`.

fn build_inventory(samples: &[&str]) -> SyllableInventory {
    let pairs: Vec<(String, String)> = samples
        .iter()
        .map(|s| {
            canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"))
        })
        .collect();

    let mut keys: Vec<String> = Vec::new();
    for (canonical, tone) in &pairs {
        if tone.is_empty() {
            keys.push(canonical.clone());
        } else {
            keys.push(format!("{canonical}{tone}"));
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
    std::env::temp_dir().join(format!("composing-build-keys-tl-poj-{pid}-{n}.fst"))
}
