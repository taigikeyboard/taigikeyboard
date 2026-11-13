// Shadow lattice build: the canonical shadow, its barriers and the BFS lattice
// over it.

use lexicon::SyllableInventory;
use phonetics::InputMode;

use crate::lattice::{build_lattice_with_barriers, Lattice};

use super::{canonicalize_poj_shadow, lookup_form, separator_layers, SeparatorLayers};

/// Cap on syllabifier BFS depth for Phase 6 fetches. Matches the
/// `max_syllables=8` budget called out in `docs/releases/v3.5.8/plan.md` § Phase 3 — Performance and
/// keeps the worst-case lookup at O(n × 3 × 8) FST hits. Owned by
/// [`build_shadow_lattice_with_barriers`] (lattice BFS budget); since v3.5.9 D / C-3b
/// the legacy per-mode `continuous::build_keys_tps` is retired and all
/// four modes (TL/POJ/English/TPS) share the unified shadow → lattice
/// path through this single cap.
pub(crate) const MAX_SYLLABLES: usize = 8;

/// The per-fetch segmentation input: the shadow, its offset map, the
/// DAG, and the barriers (all, and the `--` subset) in shadow coordinates.
pub(crate) struct ShadowLattice {
    pub shadow: String,
    pub shadow_to_raw_end: Vec<usize>,
    pub lattice: Lattice,
    /// Stripped-separator / hyphen barriers (§35).
    pub barriers: Vec<usize>,
    /// The TPS separator (Space) barriers among them — where a reading
    /// without a tone mark was closed (`crate::conversion`).
    pub space_barriers: Vec<usize>,
    /// The typed `-` runs among them, `(shadow offset, run length)` (§52 kind,
    /// §55 rendering).
    pub hyphen_runs: Vec<(usize, usize)>,
    /// The typed-`-` stretches that are one syllable
    /// ([`crate::syllabifier::one_syllable_segments`]), read by [`span_key`]
    /// so a reading may not split one (§52).
    ///
    /// [`span_key`]: super::keys::span_key
    pub one_syllable_segments: Vec<(usize, usize)>,
}

/// Run the canonicalize → hyphen-shadow → (TPS-only) space-strip
/// pipeline and build the segmentation lattice over the resulting
/// shadow. The TPS space-strip ([`build_separator_shadow`]) folds the
/// keyboard's tone-1 / syllable-separator space out of the shadow so a
/// first-tone phrase (`ㄍㄠ ㄉㄞ`) yields a cross-space edge; it is a
/// no-op for TL/POJ/English.
/// Returns a [`ShadowLattice`] — the shadow, its raw offset map, the
/// DAG, the stripped-separator / hyphen barrier set in shadow coordinates
/// (§35) and the `--` subset of it (§52). Shared by [`build_continuous_keys`] (left-anchored projection —
/// byte-identical to pre-S1, the S1 pinning tests guard this)
/// and `continuous::fetch_walker_slot0_inner` (S2 whole-sentence walker), so the
/// shadow + offset map + DAG are constructed exactly once per fetch and
/// the consumers cannot drift.
///
/// [`build_continuous_keys`]: super::keys::build_continuous_keys
pub(crate) fn build_shadow_lattice_with_barriers(
    raw: &str,
    inv: &SyllableInventory,
    mode: InputMode,
) -> ShadowLattice {
    let lower = lookup_form(raw);
    let (canonical, canonical_to_raw_end) = canonicalize_poj_shadow(&lower, mode);
    lattice_from_canonical_with_barriers(&canonical, &canonical_to_raw_end, inv, mode)
}

/// [`lattice_from_canonical`] plus the merged barrier set: every shadow
/// byte offset where a user separator (TPS space) or compound hyphen was
/// stripped. Barriers gate the §35 ambiguity expansion — a single
/// syllable may not cross one, and the glyph before one is Final-only —
/// and the TPS syllabifier receives them so an expanded probe cannot
/// fuse across the user's explicit boundary.
fn lattice_from_canonical_with_barriers(
    canonical: &str,
    canonical_to_raw_end: &[usize],
    inv: &SyllableInventory,
    mode: InputMode,
) -> ShadowLattice {
    let SeparatorLayers {
        hyphenless_to_canonical,
        shadow,
        shadow_to_hyphenless,
        barriers,
        space_barriers,
        hyphen_runs,
    } = separator_layers(canonical, mode);
    // Compose the three offset maps: shadow → hyphenless → canonical → raw.
    let shadow_to_raw_end: Vec<usize> = shadow_to_hyphenless
        .iter()
        .map(|&hyphenless_idx| canonical_to_raw_end[hyphenless_to_canonical[hyphenless_idx]])
        .collect();
    // §41 — consume a TRAILING separator marker into the full-span end.
    //
    // `strip_char_shadow_with_barriers` only records map entries for the
    // characters it KEEPS, so an interior space is swept up by the next
    // glyph's entry (`build_separator_shadow_tps_strips_space_and_maps_to_raw`
    // pins that) but a TRAILING one falls past the last entry: `ㄒㄧ␣` maps
    // its full span to raw 6 of 7 bytes. `commit_continuous` then keeps
    // `pending[consumed_bytes..]` — a lone `" "` the user never sees (the
    // display seam hides it) but that leaves the engine composing a phantom
    // buffer: the next backspace deletes the invisible marker, the
    // final-commit test misreads, and the NextWord terminal segment can get
    // an empty display.
    //
    // Fixed HERE, after the barrier merge, rather than inside the shared
    // strip primitive: that primitive also serves the hyphen layer, where a
    // trailing hyphen MUST stay pending
    // (`build_hyphen_shadow_trailing_hyphen_is_not_consumed`), and moving the
    // endpoint earlier would shift the hyphen-barrier `rposition` projection
    // in `super::separator_layers`. Barrier offsets, lattice edges and
    // `key_final_only_offsets` are all untouched — only the full-span
    // shadow→raw endpoint moves.
    let mut shadow_to_raw_end = shadow_to_raw_end;
    if matches!(mode, InputMode::Tps) {
        let kept_end = hyphenless_to_canonical[shadow_to_hyphenless[shadow.len()]];
        if kept_end < canonical.len() && canonical[kept_end..].chars().all(|c| c == ' ') {
            if let Some(full_span_end) = shadow_to_raw_end.last_mut() {
                *full_span_end = canonical_to_raw_end[canonical.len()];
            }
        }
    }
    // v3.5.9 B-2 — thread `mode` into the lattice builder; the inventory is
    // mode-aware (`SyllableInventory::contains_in(mode, …)`), so a POJ-mode
    // shadow now resolves against the `poj:` family of `syllables.fst` and
    // emits POJ-shaped syllable boundaries (`chiah`, `goa`, …) rather than
    // collapsing onto the TL forms.
    let lattice = build_lattice_with_barriers(&shadow, inv, mode, MAX_SYLLABLES, &barriers);
    // `shadow` is already lowercase (`lookup_form`).
    let one_syllable_segments =
        crate::syllabifier::one_syllable_segments(&shadow, inv, mode, &barriers);
    ShadowLattice {
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        space_barriers,
        hyphen_runs,
        one_syllable_segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Barrier metadata — the §35 contract's raw material. The pipeline
    // strips the TPS space / hyphen but records where they sat, and the
    // ambiguity-aware lookup uses those offsets for the Final-only
    // restriction (direction tests live in lexicon/tests/tps_readings.rs).
    #[test]
    fn barriers_are_recorded_where_separators_were_stripped() {
        let ShadowLattice {
            shadow, barriers, ..
        } = {
            // No inventory needed for the strip half — build a tiny one.
            let inv = test_inventory(&["tps:ㄎㄛ"]);
            build_shadow_lattice_with_barriers("ㄎㄛㆻ ㄫ", &inv, InputMode::Tps)
        };
        assert_eq!(shadow, "ㄎㄛㆻㄫ");
        assert_eq!(barriers, vec![9], "barrier at the stripped-space offset");
    }

    // §41 — a trailing separator marker is consumed by the full span, so a
    // whole-buffer commit leaves nothing pending. Interior separators were
    // already consumed via the next glyph's map entry; this pins the tail
    // case the reported bug exposed.
    #[test]
    fn trailing_separator_is_consumed_by_the_full_span() {
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let raw = "ㄒㄧ "; // 3 + 3 + 1 bytes
        assert_eq!(raw.len(), 7, "raw byte length precondition");
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            barriers,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄒㄧ");
        assert_eq!(barriers, vec![6], "barrier where the space was stripped");
        assert_eq!(
            shadow_to_raw_end[shadow.len()],
            raw.len(),
            "full-span end must reach raw len so the commit eats the marker"
        );
    }

    #[test]
    fn repeated_trailing_separators_are_all_consumed() {
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let raw = "ㄒㄧ  ";
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄒㄧ");
        assert_eq!(shadow_to_raw_end[shadow.len()], raw.len());
    }

    #[test]
    fn interior_separator_keeps_its_shorter_span_end_unchanged() {
        // The tail fix must not move a span that ends BEFORE the separator:
        // `ㄍㄠ` in `ㄍㄠ␣ㄉㄞ` still ends at raw 6, leaving the space pending
        // for the rest of the phrase (existing S18 contract).
        let inv = test_inventory(&["tps:ㄍㄠ", "tps:ㄉㄞ"]);
        let raw = "ㄍㄠ ㄉㄞ";
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄍㄠㄉㄞ");
        assert_eq!(shadow_to_raw_end[6], 6, "first-syllable end is unmoved");
        assert_eq!(shadow_to_raw_end[shadow.len()], raw.len());
    }

    // Mixed tails the endpoint rule must NOT consume: the moment a hyphen
    // appears in the tail the hyphen contract wins, whichever order the two
    // separators came in. Codex post-impl 2026-08-21 asked for these
    // explicitly — the comment claimed the `␠-` / `-␠` shapes were covered
    // when only the bare trailing hyphen was.
    #[test]
    fn mixed_separator_tails_keep_the_hyphen_contract() {
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        for raw in ["ㄒㄧ -", "ㄒㄧ- ", "ㄒㄧ - "] {
            let ShadowLattice {
                shadow,
                shadow_to_raw_end,
                ..
            } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
            assert_eq!(shadow, "ㄒㄧ", "{raw}");
            assert_eq!(
                shadow_to_raw_end[shadow.len()],
                6,
                "{raw}: a hyphen in the tail keeps the span short"
            );
        }
    }

    #[test]
    fn separator_only_and_empty_buffers_are_safe() {
        // Degenerate shapes must not panic or invent coverage: an empty
        // buffer keeps the baseline map, an all-separator buffer produces an
        // empty shadow (no lattice edge, so no candidate can claim the span).
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers("", &inv, InputMode::Tps);
        assert_eq!(shadow, "");
        assert_eq!(shadow_to_raw_end, vec![0]);

        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            barriers,
            ..
        } = build_shadow_lattice_with_barriers("  ", &inv, InputMode::Tps);
        assert_eq!(shadow, "");
        assert_eq!(barriers, vec![0], "one barrier at the collapsed offset");
        assert_eq!(shadow_to_raw_end[0], 2, "the whole buffer is consumable");
    }

    #[test]
    fn trailing_hyphen_is_still_not_consumed() {
        // The hyphen contract is untouched: a trailing hyphen stays pending
        // (the tail rule accepts ASCII spaces only).
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let raw = "ㄒㄧ-";
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄒㄧ");
        assert_eq!(
            shadow_to_raw_end[shadow.len()],
            6,
            "trailing hyphen must stay pending"
        );
    }

    #[test]
    fn mixed_separator_orders_project_to_the_same_barrier() {
        // `A␠-B` and `A-␠B` must both yield ONE barrier right after A —
        // a first-index-≥ projection put the `A␠-B` hyphen barrier after
        // B's first glyph (a phantom barrier inside the next syllable).
        let inv = test_inventory(&["tps:ㄎㄛ"]);
        for raw in ["ㄎㄛ -ㄫ", "ㄎㄛ- ㄫ"] {
            let ShadowLattice {
                shadow, barriers, ..
            } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
            assert_eq!(shadow, "ㄎㄛㄫ", "{raw}");
            assert_eq!(barriers, vec![6], "{raw}: one barrier after ㄎㄛ");
        }
    }

    fn test_inventory(keys: &[&str]) -> SyllableInventory {
        let path = test_support::write_fst_set(
            "inventory.fst",
            keys.iter().map(|key| key.as_bytes().to_vec()).collect(),
        );
        SyllableInventory::open(&path).expect("open inventory")
    }
}
