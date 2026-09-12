//! `PrefixIndex::lookup_prefix_shortest_first` /
//! `lookup_prefix_shortest_first_tps_readings` walk the FST in widening
//! entry-length bands and stop after the first band that fills `cap`
//! (`prefix_index.rs::collect_shortest_first`). These tests pin the
//! band mechanics against a full-scan oracle: the result must equal
//! "every surviving entry, ordered shortest key first (byte order within
//! a length), truncated to `cap`" no matter where the band edges fall.
//!
//! Fixture keys are deliberately NOT real readings — `skip` is supplied
//! per test, so the phonetic predicates never run here.

use std::collections::BTreeMap;

mod common;
use common::{build_wire_index, wire_entry};

/// Full-scan oracle: every fixture rowid under `prefix` whose key is not
/// skipped, bucketed by key byte length, wire byte order within a bucket,
/// truncated to `cap`.
fn oracle(keys: &[(&str, u32)], prefix: &str, cap: usize, skip: impl Fn(&str) -> bool) -> Vec<u32> {
    let mut wires: Vec<(Vec<u8>, usize, u32)> = keys
        .iter()
        .filter(|(key, _)| key.starts_with(prefix) && !skip(key))
        .map(|(key, rowid)| (wire_entry(key, *rowid), key.len(), *rowid))
        .collect();
    wires.sort();
    let mut buckets: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (_, key_len, rowid) in wires {
        buckets.entry(key_len).or_default().push(rowid);
    }
    buckets.into_values().flatten().take(cap).collect()
}

/// Within one band the stream is byte order, not length order: `tl:taa`
/// (rowid 1) streams before the shorter `tl:ta` (rowid 2). A `cap` of 1
/// must still return the shorter key — the stop check runs after the
/// band, never inside it.
#[test]
fn cap_one_returns_shortest_key_even_when_a_longer_key_streams_first() {
    let keys = [("tl:taa", 1), ("tl:ta", 2)];
    let idx = build_wire_index("byte-vs-length", &keys);
    assert_eq!(
        idx.lookup_prefix_shortest_first("tl:t", 1, |_| false),
        vec![2]
    );
    assert_eq!(
        idx.lookup_prefix_shortest_first("tl:t", 1, |_| false),
        oracle(&keys, "tl:t", 1, |_| false)
    );
}

/// The first band (typed prefix + 2 bytes) holds fewer survivors than
/// `cap`; the second band (+4) must be drained completely before the cap
/// is applied — inside it the 8-byte `tl:taang` streams before the
/// 7-byte `tl:tang`, and the shorter key must still take the slot.
#[test]
fn cap_filled_across_two_bands_matches_full_scan_oracle() {
    // Band 1 (key ≤ 6 bytes): `tl:ta`, `tl:ti` → 2 rowids.
    // Band 2 (key ≤ 8 bytes): `tl:taang`, `tl:tang` → 2 rowids.
    // Band 3 (key ≤ 12 bytes): `tl:tsiuntsai` — must never be reached.
    let keys = [
        ("tl:ta", 10),
        ("tl:ti", 12),
        ("tl:taang", 20),
        ("tl:tang", 21),
        ("tl:tsiuntsai", 30),
    ];
    let idx = build_wire_index("two-bands", &keys);
    let cap = 3;
    let got = idx.lookup_prefix_shortest_first("tl:t", cap, |_| false);
    assert_eq!(got, oracle(&keys, "tl:t", cap, |_| false));
    assert_eq!(got, vec![10, 12, 21]);
}

/// Fewer survivors than `cap` in every finite band: the unbounded final
/// band must still bring back the entries past the last finite ceiling
/// (+32 bytes), with nothing duplicated or dropped.
#[test]
fn total_below_cap_reaches_unbounded_band_without_duplicates() {
    let long_body = "a".repeat(60);
    let long_key = format!("tl:t{long_body}");
    let keys = [("tl:ta", 1), ("tl:tai", 2), (long_key.as_str(), 3)];
    let idx = build_wire_index("unbounded", &keys);
    assert_eq!(
        idx.lookup_prefix_shortest_first("tl:t", 100, |_| false),
        vec![1, 2, 3]
    );
}

/// `skip` runs once per distinct key, not per rowid, and rowids whose
/// little-endian bytes contain `0xFF` or sort differently from their
/// numeric value decode intact in byte order.
#[test]
fn skip_runs_once_per_key_and_rowid_bytes_decode_in_byte_order() {
    // Byte order of the rowid tail: 256 = `00 01 00 00` < 255 = `FF 00 00
    // 00` < u32::MAX = `FF FF FF FF`.
    let keys = [
        ("tl:ta", u32::MAX),
        ("tl:ta", 255),
        ("tl:ta", 256),
        ("tl:tb", 7),
        ("tl:tb", 8),
    ];
    let idx = build_wire_index("skip-memo", &keys);
    let mut seen: Vec<String> = Vec::new();
    let got = idx.lookup_prefix_shortest_first("tl:t", 100, |key| {
        seen.push(key.to_string());
        key == "tl:tb"
    });
    assert_eq!(got, vec![256, 255, u32::MAX]);
    assert_eq!(seen, vec!["tl:ta".to_string(), "tl:tb".to_string()]);
}

/// A skipped key's remaining rowids are not streamed (the walk re-seeks
/// to the key's lex sibling) — but its extensions in the same band, the
/// sibling key after it, and its longer extensions in a later band must
/// all still be collected.
#[test]
fn skipped_key_reseek_keeps_extensions_and_following_keys() {
    let keys = [
        ("tl:tsi", 1),
        ("tl:ts", 2),
        ("tl:ts", 3),
        ("tl:tt", 4),
        ("tl:tsiah", 5),
    ];
    let idx = build_wire_index("reseek", &keys);
    let mut seen: Vec<String> = Vec::new();
    let got = idx.lookup_prefix_shortest_first("tl:t", 100, |key| {
        seen.push(key.to_string());
        key == "tl:ts"
    });
    assert_eq!(got, oracle(&keys, "tl:t", 100, |key| key == "tl:ts"));
    assert_eq!(got, vec![4, 1, 5]);
    assert_eq!(seen, vec!["tl:tsi", "tl:ts", "tl:tt", "tl:tsiah"]);
}

/// TPS variant: the substitution-count tiebreak inside one key length
/// survives the band walk. Typed `tps:ㆬ`: the substituted `tps:ㄇㄧ`
/// (U+3107) byte-sorts BEFORE the literal `tps:ㆬㄧ` (U+31AC), yet with
/// `cap` cutting inside the 2-glyph group the literal reading must win
/// the slot.
#[test]
fn tps_substitution_tiebreak_survives_band_walk() {
    let keys = [("tps:ㄇㄧ", 1), ("tps:ㆬㄧ", 2), ("tps:ㆬㄧㄚ", 3)];
    let idx = build_wire_index("tps-subst", &keys);
    let hits = idx.lookup_prefix_shortest_first_tps_readings("tps:ㆬ", 1, |_| false);
    assert_eq!(hits, vec![("tps:ㆬㄧ".to_string(), 2)]);
    let hits = idx.lookup_prefix_shortest_first_tps_readings("tps:ㆬ", 3, |_| false);
    assert_eq!(
        hits,
        vec![
            ("tps:ㆬㄧ".to_string(), 2),
            ("tps:ㄇㄧ".to_string(), 1),
            ("tps:ㆬㄧㄚ".to_string(), 3),
        ]
    );
}
