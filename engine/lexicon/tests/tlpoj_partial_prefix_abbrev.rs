//! Regression: TL/POJ continuous partial-prefix must surface single-char
//! readings for a single-initial input (e.g. `s`), not only the `sa`-family
//! + multi-syllable phrases (surfaced 2026-06-16 while building the
//! cross-mode parity test: typing `s` in TL/POJ buried 是/sī, the highest-
//! freq `s` word, behind a wall of `tl:sb`-keyed 2-syllable phrases).
//!
//! Root cause (now fixed): the `lookup_prefix_shortest_first` length
//! bucketing landed every `tl:`+2-char key in one bucket, byte-ordered.
//! There the full single-syllable keys (`tl:sa`/`tl:si`, 2nd char a VOWEL)
//! interleave with 2-syllable acronym `tl_abbrev` keys (`tl:sb`/`tl:sh`,
//! 2nd char a CONSONANT). Byte order puts `sa`(97) `sb`(98) … `sh`(104)
//! `si`(105), so the abbrev floods between `sa` and `si` consumed the
//! `PARTIAL_PREFIX_HYDRATE_CAP` before `tl:si`(是) was reached. The skip
//! predicate that already dropped TPS acronyms was TPS-only; this widens it
//! to TL/POJ via `phonetics::is_roman_acronym_key` (all-ASCII-consonant
//! body that is not a valid standalone syllable — keeps full single-
//! syllable keys, fused multi-syllable notone keys, and syllabic nasals).

// TL/POJ 連續 partial-prefix 回歸 — 單一聲母 (s) 必須撈得到單字讀音 (是/sī),
//   而非只有 sa 家族 + 雙字詞。根因:同長度桶內,完整單音節 key (tl:sa/tl:si,
//   第 2 字母為母音) 與雙音節縮寫 tl_abbrev key (tl:sb/tl:sh,第 2 字母為子音)
//   交錯,byte 序 sa(97) sb(98)…si(105) → sa 與 si 之間的縮寫洪流吃光 cap,
//   tl:si(是) 進不了 pool。修法 = 把原本只對 TPS 的縮寫 skip 推廣到 TL/POJ
//   (phonetics::is_roman_acronym_key)。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use fst::SetBuilder;
use lexicon::dictionary_reader::DictionaryReader;
use lexicon::prefix_index::PrefixIndex;
use lexicon::{fetch_partial_prefix_candidates, ContinuousFetchCtx};
use phonetics::InputMode;
use ranking::FrequencyMap;

mod common;
use common::{build_tkdb_v3, write_temp};

/// One FST entry: `<ns><body>` keyed to `rowid`.
struct FstKey {
    body: &'static str,
    rowid: u32,
}

/// Build a `dictionary.fst` from `ns`-namespaced keys (wire format
/// `key || 0xFF || rowid_le_4`, the shape `create_fst.py` emits).
fn build_index(name: &str, ns: &str, keys: &[FstKey]) -> PrefixIndex {
    let mut entries: Vec<Vec<u8>> = Vec::with_capacity(keys.len());
    for k in keys {
        let mut entry = Vec::new();
        entry.extend_from_slice(ns.as_bytes());
        entry.extend_from_slice(k.body.as_bytes());
        entry.push(0xFF);
        entry.extend_from_slice(&k.rowid.to_le_bytes());
        entries.push(entry);
    }
    entries.sort();

    let path = unique_temp(name);
    let file = std::fs::File::create(&path).expect("create fst tmp");
    let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("fst builder");
    for entry in &entries {
        builder.insert(entry).expect("fst insert");
    }
    builder.finish().expect("fst finish");
    PrefixIndex::open(&path).expect("dictionary.fst opens")
}

fn unique_temp(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "lex-tlpoj-abbrev-{name}-{}-{n}.fst",
        std::process::id()
    ))
}

fn roman_ctx<'a>(
    prefix_index: &'a PrefixIndex,
    dict: &'a DictionaryReader,
    freq_map: &'a FrequencyMap,
    mode: InputMode,
) -> ContinuousFetchCtx<'a> {
    ContinuousFetchCtx {
        enabled_sources_bitmask: u32::MAX,
        freq_map,
        now_ms: 0,
        custom: &[],
        prefix_index,
        dict,
        mode,
        tps_space_pinned_body: None,
    }
}

/// The exact predicate the production TL partial-prefix path passes.
fn tl_acronym_skip(key: &str) -> bool {
    phonetics::is_roman_acronym_key(key.strip_prefix("tl:").unwrap_or(key))
}

/// `lookup_prefix_shortest_first` drops the TL acronym (`tl:sb`) key surface
/// while keeping the full single-syllable `tl:si` and ordering it ahead of
/// its longer extension `tl:sigi` (the `0xFF` separator byte-sorts the short
/// key LAST without bucketing).
#[test]
fn lookup_prefix_shortest_first_drops_tl_acronym_and_orders_short_first() {
    let idx = build_index(
        "mech",
        "tl:",
        &[
            FstKey {
                body: "sb",
                rowid: 1,
            }, // 2-syllable acronym surface (dropped)
            FstKey {
                body: "si",
                rowid: 2,
            }, // short full reading 是/sī
            FstKey {
                body: "sigi",
                rowid: 3,
            }, // long full reading (si-gi)
        ],
    );
    // Plain byte order: acronym `sb` first, then the LONG key, then the
    // short exact key last (`0xFF` > any UTF-8 byte).
    assert_eq!(idx.lookup_prefix("tl:s"), vec![1, 3, 2]);
    // Shortest-first + acronym skip: `sb` gone, short `si` before long `sigi`.
    assert_eq!(
        idx.lookup_prefix_shortest_first("tl:s", 100, tl_acronym_skip),
        vec![2, 3],
        "acronym `tl:sb` dropped; short `si` ordered before long `sigi`"
    );
}

/// End-to-end cap regression: a flood of `tl:sb` 2-syllable acronym rows
/// byte-sorted ahead of the short single-char `tl:si` keys must NOT starve
/// the single chars out of the `PARTIAL_PREFIX_HYDRATE_CAP` budget. The
/// acronym skip + shortest-first hydration rescues the singles. Pre-fix,
/// 是/時/四 were absent (only `sa` + 2-syllable phrases survived the cap).
/// Parametrised over TL and POJ — both modes pass the acronym skip.
#[test]
fn tlpoj_partial_prefix_surfaces_single_chars_past_acronym_flood() {
    for (ns, mode) in [("tl:", InputMode::Tl), ("poj:", InputMode::Poj)] {
        // > PARTIAL_PREFIX_HYDRATE_CAP (500): in byte order these `sb`
        // acronym rows fill the budget (sorting between `sa` and `si`)
        // before any short `si` row; the acronym skip rescues the singles.
        const FLOOD: usize = 600;

        let phrase_hanji: Vec<String> = (0..FLOOD).map(|i| format!("詞{i}")).collect();
        let mut dict_rows: Vec<(u16, u32, u8, &str, &str)> = Vec::with_capacity(FLOOD + 3);
        for hanji in &phrase_hanji {
            // 2-syllable `s_-b_` phrase, low freq — acronym key `sb`.
            dict_rows.push((1u16 << 11, 10, 2, hanji.as_str(), "sa-bu"));
        }
        // Single-char `si` readings, high freq — should rank at the top once
        // they are no longer starved.
        dict_rows.push((1u16 << 11, 9000, 7, "是", "si"));
        dict_rows.push((1u16 << 11, 8000, 5, "時", "si"));
        dict_rows.push((1u16 << 11, 7000, 2, "四", "si"));

        let dict_bytes = build_tkdb_v3(b"TKDB", &dict_rows);
        let dict_path = write_temp("tlpoj-abbrev-flood.dict.bin", &dict_bytes);
        let dict = DictionaryReader::open(&dict_path).expect("dict.bin opens");

        // Each phrase emits acronym `sb` + full notone `sabu`; each single
        // char emits full notone `si`. rowid = 1-based dict position.
        let mut keys: Vec<FstKey> = Vec::with_capacity(FLOOD * 2 + 3);
        for i in 0..FLOOD {
            let rowid = (i + 1) as u32;
            keys.push(FstKey { body: "sb", rowid }); // acronym surface (dropped)
            keys.push(FstKey {
                body: "sabu",
                rowid,
            }); // legit `sa-bu` prefix hit
        }
        for off in 0..3 {
            keys.push(FstKey {
                body: "si",
                rowid: (FLOOD + off + 1) as u32,
            });
        }
        let idx = build_index("flood", ns, &keys);

        let freq = FrequencyMap::new();
        let ctx = roman_ctx(&idx, &dict, &freq, mode);
        let key = ((0u32, 1u32), format!("{ns}s"));
        let out = fetch_partial_prefix_candidates(&key, 1, &ctx);

        let hanji: Vec<&str> = out.iter().filter_map(|c| c.hanji.as_deref()).collect();
        for want in ["是", "時", "四"] {
            assert!(
                hanji.contains(&want),
                "{mode:?}: single-char {want} must surface past the acronym flood (got {hanji:?})"
            );
        }
    }
}
