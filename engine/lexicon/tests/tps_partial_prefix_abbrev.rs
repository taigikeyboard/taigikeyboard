//! Regression: TPS continuous partial-prefix must surface single-char
//! readings for a single-initial input (e.g. `ㄍ`), not only multi-
//! syllable phrases (user-reported 2026-06-15: typing `ㄍ` showed only
//! multi-syllable words, starting at 姑不將).
//!
//! Root cause (now fixed): the FST wire separator `0xFF` is greater than
//! any UTF-8 byte, so a short exact key (`tps:ㄍㄚ` = 家/ka) byte-sorts
//! AFTER all its longer extensions (`tps:ㄍㄚㄅㄧ`…). A plain byte-ordered
//! `lookup_prefix(..).take(cap)` front-loaded the longest (rarest) words
//! and buried the short high-frequency single-syllable readings past the
//! `PARTIAL_PREFIX_HYDRATE_CAP`. The fix spends the hydrate budget on the
//! SHORTEST matched keys first via `PrefixIndex::lookup_prefix_shortest_first`
//! (all modes); the downstream `SortKey` then ranks the pool by frequency.
//! TPS additionally drops initial-only `tps_abbrev` acronym key surfaces
//! (Bopomofo orders all initials ahead of all vowels, so those short keys
//! would otherwise win the shortest-first budget) via
//! `phonetics::is_tps_initial_only`.

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

/// One FST entry: `tps:<body>` keyed to `rowid`.
struct FstKey {
    body: &'static str,
    rowid: u32,
}

/// Build a `dictionary.fst` from `ns`-namespaced keys (wire format
/// `key || 0xFF || rowid_le_4`, the same shape `create_fst.py` emits).
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
        "lex-tps-abbrev-{name}-{}-{n}.fst",
        std::process::id()
    ))
}

fn tps_ctx<'a>(
    prefix_index: &'a PrefixIndex,
    dict: &'a DictionaryReader,
    freq_map: &'a FrequencyMap,
) -> ContinuousFetchCtx<'a> {
    ContinuousFetchCtx {
        enabled_sources_bitmask: u32::MAX,
        freq_map,
        now_ms: 0,
        custom: &[],
        prefix_index,
        dict,
        mode: InputMode::Tps,
        tps_space_pinned_body: None,
    }
}

/// The exact predicate the production TPS partial-prefix path passes.
fn tps_abbrev_skip(key: &str) -> bool {
    phonetics::is_tps_initial_only(key.strip_prefix("tps:").unwrap_or(key))
}

/// `lookup_prefix_shortest_first` (a) drops the abbrev (initial-only) key
/// surface and (b) orders survivors shortest-matched-key first — which,
/// for the `0xFF`-separated wire, is the OPPOSITE of plain byte order
/// (where the short exact key sorts LAST, behind its longer extensions).
#[test]
fn lookup_prefix_shortest_first_drops_abbrev_and_orders_short_first() {
    let idx = build_index(
        "mech",
        "tps:",
        &[
            FstKey {
                body: "ㄍㄅ",
                rowid: 1,
            }, // abbrev surface (dropped)
            FstKey {
                body: "ㄍㄚ",
                rowid: 2,
            }, // short full reading 家/ka
            FstKey {
                body: "ㄍㄚㄅㄧ",
                rowid: 3,
            }, // long full reading ka-pi
        ],
    );
    // Plain byte order: abbrev first, then the LONG key, then the short
    // exact key last (the `0xFF` separator > any UTF-8 byte).
    assert_eq!(idx.lookup_prefix("tps:ㄍ"), vec![1, 3, 2]);
    // Shortest-first + abbrev skip: abbrev gone, short key before long key.
    assert_eq!(
        idx.lookup_prefix_shortest_first("tps:ㄍ", 100, tps_abbrev_skip),
        vec![2, 3],
        "abbrev dropped; short ㄍㄚ ordered before long ㄍㄚㄅㄧ"
    );
    // The cap is honoured against the shortest-first order.
    assert_eq!(
        idx.lookup_prefix_shortest_first("tps:ㄍ", 1, tps_abbrev_skip),
        vec![2],
        "cap=1 keeps only the shortest survivor"
    );
}

/// Length-bucketing primitive lock: `lookup_prefix_shortest_first` orders a
/// short exact key ahead of its longer extension even though the `0xFF`
/// separator byte-sorts the short key LAST — exercised here with a no-op
/// skip (`|_| false`) to isolate the bucketing from any family filter.
/// (Production TL/POJ now ALSO pass an acronym skip predicate, pinned in
/// `tlpoj_partial_prefix_abbrev.rs`.)
#[test]
fn lookup_prefix_shortest_first_orders_short_before_long_no_skip_tl() {
    let idx = build_index(
        "tl-allmode",
        "tl:",
        &[
            FstKey {
                body: "kapi",
                rowid: 1,
            }, // long extension (ka-pi)
            FstKey {
                body: "ka",
                rowid: 2,
            }, // short exact reading
        ],
    );
    assert_eq!(
        idx.lookup_prefix("tl:k"),
        vec![1, 2],
        "byte order: long `kapi` before short exact `ka` (0xFF sorts short last)"
    );
    assert_eq!(
        idx.lookup_prefix_shortest_first("tl:k", 100, |_| false),
        vec![2, 1],
        "all-mode length bucketing: short `ka` before long `kapi`, no skip"
    );
}

/// The matched key is reconstructed by fixed offset (`entry.len() - 5`),
/// NOT by searching for `0xFF` — a rowid whose little-endian bytes contain
/// `0xFF` (255 → `FF 00 00 00`) must not corrupt key parsing.
#[test]
fn lookup_prefix_shortest_first_parses_key_with_0xff_in_rowid() {
    let idx = build_index(
        "rowid-ff",
        "tps:",
        &[FstKey {
            body: "ㄍㄚ",
            rowid: 255,
        }],
    );
    assert_eq!(
        idx.lookup_prefix_shortest_first("tps:ㄍ", 100, |_| false),
        vec![255],
        "rowid 255 (0xFF byte) decoded correctly"
    );
    let mut seen: Vec<String> = Vec::new();
    let got = idx.lookup_prefix_shortest_first("tps:ㄍ", 100, |key| {
        seen.push(key.to_string());
        false
    });
    assert_eq!(got, vec![255]);
    assert_eq!(
        seen,
        vec!["tps:ㄍㄚ".to_string()],
        "predicate sees the exact key despite 0xFF in the rowid bytes"
    );
}

/// End-to-end cap regression: a flood of LONGER `ka-pi` keys (plus their
/// abbrev surfaces) byte-sorted ahead of the short single-char `ㄍㄚ` keys
/// must NOT starve the single chars out of the `PARTIAL_PREFIX_HYDRATE_CAP`
/// budget. Shortest-first hydration puts the short `ㄍㄚ` keys in the first
/// bucket; the abbrev `ㄍㄅ` surfaces are dropped. Pre-fix, `家/加/交` were
/// absent (only long phrases survived the cap).
#[test]
fn tps_partial_prefix_surfaces_single_chars_past_abbrev_flood() {
    // > PARTIAL_PREFIX_HYDRATE_CAP (500): in plain byte order these long
    // `ㄍㄚㄅㄧ` rows (and the abbrev `ㄍㄅ` surfaces) fill the budget before
    // any short `ㄍㄚ` row; shortest-first + abbrev skip rescues the singles.
    const FLOOD: usize = 600;

    let phrase_hanji: Vec<String> = (0..FLOOD).map(|i| format!("詞{i}")).collect();
    let mut dict_rows: Vec<(u16, u32, u8, &str, &str)> = Vec::with_capacity(FLOOD + 3);
    for hanji in &phrase_hanji {
        // `ka-pi` two-syllable phrase, low freq — abbrev key `ㄍㄅ`.
        dict_rows.push((1u16 << 11, 10, 2, hanji.as_str(), "ka-pi"));
    }
    // Single-char `ka` readings, high freq — should rank at the top once
    // they are no longer starved.
    dict_rows.push((1u16 << 11, 9000, 1, "家", "ka"));
    dict_rows.push((1u16 << 11, 8000, 1, "加", "ka"));
    dict_rows.push((1u16 << 11, 7000, 1, "交", "ka"));

    let dict_bytes = build_tkdb_v3(b"TKDB", &dict_rows);
    let dict_path = write_temp("tps-abbrev-flood.dict.bin", &dict_bytes);
    let dict = DictionaryReader::open(&dict_path).expect("dict.bin opens");

    // Each phrase emits abbrev `ㄍㄅ` + full notone `ㄍㄚㄅㄧ`; each single
    // char emits full notone `ㄍㄚ`. rowid = 1-based dict position.
    let mut keys: Vec<FstKey> = Vec::with_capacity(FLOOD * 2 + 3);
    for i in 0..FLOOD {
        let rowid = (i + 1) as u32;
        keys.push(FstKey {
            body: "ㄍㄅ",
            rowid,
        }); // abbrev surface (dropped)
        keys.push(FstKey {
            body: "ㄍㄚㄅㄧ",
            rowid,
        }); // legit ka-pi prefix hit
    }
    for off in 0..3 {
        keys.push(FstKey {
            body: "ㄍㄚ",
            rowid: (FLOOD + off + 1) as u32,
        });
    }
    let idx = build_index("flood", "tps:", &keys);

    let freq = FrequencyMap::new();
    let ctx = tps_ctx(&idx, &dict, &freq);
    let key = ((0u32, "ㄍ".len() as u32), "tps:ㄍ".to_string());
    let out = fetch_partial_prefix_candidates(&key, "ㄍ".len() as u32, &ctx);

    let hanji: Vec<&str> = out.iter().filter_map(|c| c.hanji.as_deref()).collect();
    for want in ["家", "加", "交"] {
        assert!(
            hanji.contains(&want),
            "single-char {want} must surface past the abbrev flood (got {hanji:?})"
        );
    }
}
