//! §35 (`INVARIANT_TPS_DEFOLD_ENUMERATE`) — hermetic pins for the TPS
//! ambiguity-aware reading resolution: `lookup_exact_tps_readings` /
//! `lookup_prefix_shortest_first_tps_readings` against an in-memory
//! `dictionary.fst`-shaped wire set.
//!
//! Fixture rule (`.claude/rules/taigi-incidents.md` § Trace before
//! assert): every fixture that exercises a reading also carries the
//! rival readings sharing its key pattern, and asserts their presence /
//! ordering explicitly — reading recovery must be an ADDITION with the
//! user's literal text first, never a replacement.

mod common;
use common::build_wire_index;

#[test]
fn khokng_literal_resolves_to_the_khokng_reading() {
    // 考卷: the buffer literal ㄎㄛㆻㄫ must reach the stored ㄎㄛㄍㆭ.
    // The literal key itself is absent from the index (no word spells
    // that way), so the ONLY hit is the two-substitution reading.
    let index = build_wire_index("tps-readings", &[("tps:ㄎㄛㄍㆭ", 1), ("tps:ㄎㄚ", 9)]);
    let readings = index.lookup_exact_tps_readings("tps:ㄎㄛㆻㄫ", &[]);
    assert_eq!(
        readings,
        vec![("tps:ㄎㄛㄍㆭ".to_string(), 1, 2)],
        "kho|kng must be reachable with substitution count 2",
    );
}

#[test]
fn literal_reading_orders_before_substituted_readings() {
    // ㆬㄚ (long-pressed ㆬ then ㄚ): 毋仔 m̄-á (literal, 0 subst) must
    // order before 媽 ma (ㄇㄚ, 1 subst) — the user's own text first.
    let index = build_wire_index("tps-readings", &[("tps:ㄇㄚ", 7), ("tps:ㆬㄚ", 3)]);
    let readings = index.lookup_exact_tps_readings("tps:ㆬㄚ", &[]);
    assert_eq!(
        readings,
        vec![
            ("tps:ㆬㄚ".to_string(), 3, 0),
            ("tps:ㄇㄚ".to_string(), 7, 1),
        ],
        "substitution count ascending — literal first",
    );
}

#[test]
fn barrier_final_only_blocks_the_onset_reading() {
    // §35 contract part (b), the S23-inherited direction rule:
    // `ㄎㄛㆻ`␣`ㄫ` — the ㆻ sits immediately before the user's separator,
    // so it may not be re-read as the onset ㄍ; 考卷 must NOT surface.
    // The nasal AFTER the barrier still reads as ㆭ (the separator proves
    // the nasal cannot be an onset waiting for a vowel), so a stored
    // khok+ng word WOULD be reachable — assert both directions.
    let index = build_wire_index(
        "tps-readings",
        &[
            ("tps:ㄎㄛㄍㆭ", 1), // 考卷 kho|kng — must NOT match
            ("tps:ㄎㄛㆻㆭ", 2), // hypothetical khok|ng — may match
        ],
    );
    // Key = literal full span; the ㆻ glyph starts at byte 4 ("tps:") + 6.
    let readings = index.lookup_exact_tps_readings("tps:ㄎㄛㆻㄫ", &[10]);
    let matched: Vec<&str> = readings.iter().map(|(k, _, _)| k.as_str()).collect();
    assert!(
        !matched.contains(&"tps:ㄎㄛㄍㆭ"),
        "onset re-reading across the barrier must be blocked, got {matched:?}",
    );
    assert!(
        matched.contains(&"tps:ㄎㄛㆻㆭ"),
        "coda-preserving reading must survive, got {matched:?}",
    );
}

#[test]
fn without_the_barrier_both_readings_surface() {
    // Same index as above, no barrier (no separator typed): both the
    // khok|ng and the kho|kng readings are legitimate; literal-closest
    // (fewer substitutions) first.
    let index = build_wire_index("tps-readings", &[("tps:ㄎㄛㄍㆭ", 1), ("tps:ㄎㄛㆻㆭ", 2)]);
    let readings = index.lookup_exact_tps_readings("tps:ㄎㄛㆻㄫ", &[]);
    assert_eq!(
        readings,
        vec![
            ("tps:ㄎㄛㆻㆭ".to_string(), 2, 1),
            ("tps:ㄎㄛㄍㆭ".to_string(), 1, 2),
        ],
    );
}

#[test]
fn non_tps_wire_shapes_never_match() {
    // The pattern is namespaced: a `tl:` key with a byte-coincident tail
    // must not leak into TPS readings (range is clamped to `tps:`).
    let index = build_wire_index("tps-readings", &[("tl:kokng", 5), ("tps:ㄎㄛㄍㆭ", 1)]);
    let readings = index.lookup_exact_tps_readings("tps:ㄎㄛㆻㄫ", &[]);
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].0, "tps:ㄎㄛㄍㆭ");
}

#[test]
fn prefix_readings_surface_both_nasal_families_shortest_first() {
    // Bare ㄇ partial-prefix: both ㄇ… (名) and ㆬ… (毋是/毋通) words
    // hydrate; budget order = matched-key length asc, then substitution
    // asc, then byte order.
    let index = build_wire_index(
        "tps-readings",
        &[
            ("tps:ㄇㄧㄚ", 11), // 名 miâ — literal family
            ("tps:ㆬㄒㄧ", 21), // 毋是 — substituted family
            ("tps:ㆬㄊㄤ", 22), // 毋通
            ("tps:ㆬ", 20),     // 毋 — shortest, substituted
            ("tps:ㄎㄚ", 99),   // unrelated — must not hydrate
        ],
    );
    let hits = index.lookup_prefix_shortest_first_tps_readings("tps:ㄇ", 10, |_| false);
    // trace: lengths — ㆬ=1 glyph < the 3-glyph words; among equal length,
    // 名 ㄇㄧㄚ subst 0 first; then ㆬㄊㄤ before ㆬㄒㄧ (both subst 1,
    // byte order: ㄊ U+310A < ㄒ U+3112). Each rowid carries its MATCHED
    // key so the record guard validates the pattern's actual hit.
    assert_eq!(
        hits,
        vec![
            ("tps:ㆬ".to_string(), 20),
            ("tps:ㄇㄧㄚ".to_string(), 11),
            ("tps:ㆬㄊㄤ".to_string(), 22),
            ("tps:ㆬㄒㄧ".to_string(), 21),
        ],
        "length asc, then substitution asc, then byte order — with matched keys",
    );
}

#[test]
fn prefix_readings_respect_the_skip_closure() {
    let index = build_wire_index("tps-readings", &[("tps:ㆬ", 20), ("tps:ㄇㄧㄚ", 11)]);
    let hits =
        index.lookup_prefix_shortest_first_tps_readings("tps:ㄇ", 10, |key| key == "tps:ㆬ");
    assert_eq!(hits, vec![("tps:ㄇㄧㄚ".to_string(), 11)]);
}

#[test]
fn prefix_readings_can_reach_an_acronym_key_the_literal_range_cannot() {
    // Documented hazard behind the abbrev-face guard in
    // `fetch_partial_prefix_candidates`: expanding bare ㄇ pulls in the
    // acronym key `tps:ㆬㄒ` (毋是's per-syllable initials), which the
    // literal `tps:ㄇ` range never scans. The lookup layer surfaces it —
    // the record-level guard (matched body == the record's acronym face,
    // acronym != toneless) is what rejects it downstream.
    let index = build_wire_index("tps-readings", &[("tps:ㆬㄒ", 30), ("tps:ㆬㄒㄧ", 21)]);
    let hits = index.lookup_prefix_shortest_first_tps_readings("tps:ㄇ", 10, |_| false);
    assert_eq!(
        hits,
        vec![("tps:ㆬㄒ".to_string(), 30), ("tps:ㆬㄒㄧ".to_string(), 21),],
    );
}

#[test]
fn unambiguous_prefix_returns_the_stored_matched_key() {
    // No family glyph in the prefix: matched keys are still the STORED
    // keys, not the query prefix — the record guards depend on it.
    let index = build_wire_index("tps-readings", &[("tps:ㄚㄒㄧ", 40), ("tps:ㄚ", 41)]);
    let hits = index.lookup_prefix_shortest_first_tps_readings("tps:ㄚ", 10, |_| false);
    assert_eq!(
        hits,
        vec![("tps:ㄚ".to_string(), 41), ("tps:ㄚㄒㄧ".to_string(), 40),],
    );
}
