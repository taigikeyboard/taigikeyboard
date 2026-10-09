//! `dictionary_filters` — single source of truth for the user's dictionary
//! toggle → bitmask conversion. Replaces the former iOS / Android
//! `EnabledDictionaries` copies (~80 LOC verbatim-mirrored bit math) per audit residue
//! 2026-05-04 § A.1 P2.
//!
//! Pure logic, no I/O. The dispatch layer wraps `compute_filters` into a
//! `LexiconResponse::DictionaryFiltersResult` envelope.
//!
//! Bit layout (must match `dictionary.bin` filter and
//! `engine/lexicon/src/search.rs::build_filter` consumer):
//!
//! ```text
//!  0 = kautian   3 = sitbut   6 = kungge   9 = khiin   12 = variant
//!  1 = taigitv   4 = taihoa   7 = stti    10 = dev (dictionary-supplement-file toggle)
//!  2 = itaigi    5 = taijit   8 = khpoo   11 = lkk
//! ```

use crate::dictionary_reader::{
    KAUTIAN_SUBTAG_ACCENT_COUNT, KAUTIAN_SUBTAG_ACCENT_SHIFT, KAUTIAN_SUBTAG_ALT_READING_BIT,
    KAUTIAN_SUBTAG_MAIN_BIT, KAUTIAN_SUBTAG_NAME_BIT, WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT,
    WIRE_KAUTIAN_SUBCOLL_MASK, WIRE_KAUTIAN_SUBCOLL_SHIFT,
};
use protos::engine::{DictionaryFiltersResponse, DictionarySourceCode, DictionarySourceToggles};
use DictionarySourceCode as C;

/// `association_bitmask` when every association source is on: the
/// association reader treats it as "filter disabled".
const ASSOC_ALL_ENABLED_SENTINEL: u32 = u32::MAX;

/// Each source's record-bitmask bit, wire code and user toggle, in bit order
/// (layout in the module header) — the one table the filter mask, the
/// association mask, the enabled codes and the per-row sources all read.
/// Bit 12 (variant) is a filter, not a source.
type SourceToggle = fn(&DictionarySourceToggles) -> bool;
const SOURCES: [(u32, DictionarySourceCode, SourceToggle); 12] = [
    (1 << 0, C::DictSourceKautian, |t| t.kautian),
    (1 << 1, C::DictSourceTaigitv, |t| t.taigitv),
    (1 << 2, C::DictSourceItaigi, |t| t.itaigi),
    (1 << 3, C::DictSourceSitbut, |t| t.sitbut),
    (1 << 4, C::DictSourceTaihoa, |t| t.taihoa),
    (1 << 5, C::DictSourceTaijit, |t| t.taijit),
    (1 << 6, C::DictSourceKungge, |t| t.kungge),
    (1 << 7, C::DictSourceStti, |t| t.stti),
    (1 << 8, C::DictSourceKhpoo, |t| t.khpoo),
    (1 << 9, C::DictSourceKhiin, |t| t.khiin),
    (1 << 10, C::DictSourceDev, |t| t.dev),
    (1 << 11, C::DictSourceLkk, |t| t.lkk),
];

/// The association bin carries the first nine sources (kautian … khpoo, bits
/// 0-8).
const ASSOCIATION_SOURCES: usize = 9;

/// Compute filter bitmasks + enabled-source codes from the user's
/// 12-toggle preference snapshot.
pub(crate) fn compute_filters(toggles: &DictionarySourceToggles) -> DictionaryFiltersResponse {
    DictionaryFiltersResponse {
        dictionary_filter_bitmask: dictionary_filter_bitmask(toggles),
        enabled_source_codes: enabled_source_codes(toggles)
            .into_iter()
            .map(|c| c as i32)
            .collect(),
    }
}

/// Full `dictionary.bin` filter bitmask: source/variant bits 0-12 plus the
/// kautian subcollection wire high region (bit 13 active + bits 14..=26 enable
/// mask) when the subcollection toggles are present.
pub fn dictionary_filter_bitmask(t: &DictionarySourceToggles) -> u32 {
    let mut mask = toggled_bits(&SOURCES, t);
    if t.variant {
        mask |= 1 << 12;
    }
    mask |= encode_kautian_subcoll_wire(t);
    mask
}

/// Encode the user's kautian subcollection enable state into the wire high
/// region: bit 13 (active sentinel) + bits 14..=26 (13-bit enable mask, same
/// layout as the record subtag). Returns 0 — leaving the engine in legacy
/// all-on mode (zero behaviour change, DD5) — when EITHER the subcollection
/// sub-message is absent (a platform whose UI is not wired yet, NextWord) OR
/// the kautian master toggle is off (kautian rows are dropped by the source-OR
/// regardless, so gating them is moot). The `main` subcollection bit is set
/// unconditionally when present: main is not a user toggle — it is
/// always on whenever the kautian master is on.
fn encode_kautian_subcoll_wire(t: &DictionarySourceToggles) -> u32 {
    let Some(sub) = t.kautian_subcollections.as_ref() else {
        return 0;
    };
    if !t.kautian {
        return 0;
    }
    let mut subtag: u16 = 1 << KAUTIAN_SUBTAG_MAIN_BIT;
    // Accent order MUST match config.yaml `dialect_columns` (subtag bit = 1 + index).
    let accents = [
        sub.accent_lukang,
        sub.accent_sansia,
        sub.accent_taipak,
        sub.accent_gilan,
        sub.accent_tainan,
        sub.accent_kaohsiung,
        sub.accent_kinmen,
        sub.accent_makung,
        sub.accent_sintik,
        sub.accent_taichung,
    ];
    // Tripwire: if a dialect column is added, the array + proto + subtag layout
    // must grow together. Guards the exact drift axis the mirror const exists for.
    debug_assert_eq!(accents.len(), KAUTIAN_SUBTAG_ACCENT_COUNT);
    for (index, enabled) in accents.iter().enumerate() {
        if *enabled {
            subtag |= 1 << (KAUTIAN_SUBTAG_ACCENT_SHIFT + index as u16);
        }
    }
    if sub.name_appendix {
        subtag |= 1 << KAUTIAN_SUBTAG_NAME_BIT;
    }
    if sub.alt_reading {
        subtag |= 1 << KAUTIAN_SUBTAG_ALT_READING_BIT;
    }
    WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT
        | ((u32::from(subtag) & WIRE_KAUTIAN_SUBCOLL_MASK) << WIRE_KAUTIAN_SUBCOLL_SHIFT)
}

/// Association-bin filter bitmask: bits 0-8 mask of toggled sources.
/// Returns the `u32::MAX` sentinel when ALL 9 association sources are on,
/// the shortcut `api::lookup_associations` reads as "no filter".
pub fn association_bitmask(t: &DictionarySourceToggles) -> u32 {
    let sources = &SOURCES[..ASSOCIATION_SOURCES];
    if sources.iter().all(|(_, _, is_on)| is_on(t)) {
        return ASSOC_ALL_ENABLED_SENTINEL;
    }
    toggled_bits(sources, t)
}

/// The bits of the `sources` the user has on.
fn toggled_bits(
    sources: &[(u32, DictionarySourceCode, SourceToggle)],
    t: &DictionarySourceToggles,
) -> u32 {
    sources
        .iter()
        .filter(|(_, _, is_on)| is_on(t))
        .fold(0, |mask, (bit, _, _)| mask | bit)
}

/// The sources a record's (effective) bitmask names, in bit order — the
/// order every platform draws the result badges in (`TaigiWord.sources`).
pub(crate) fn source_codes(bitmask: u32) -> Vec<DictionarySourceCode> {
    SOURCES
        .iter()
        .filter(|(bit, _, _)| bitmask & bit != 0)
        .map(|(_, code, _)| *code)
        .collect()
}

/// `DictionarySourceCode` set the platform should mark as enabled when
/// retagging Tab3 result badges. `CUSTOM` is non-toggleable and always
/// present; `DEV` (dictionary-supplement-file source) is gated by the dev toggle and pushed
/// first-when-present so the order stays `[DEV?, CUSTOM, …]`; `variant` is
/// a filter bit, not a source code.
fn enabled_source_codes(t: &DictionarySourceToggles) -> Vec<DictionarySourceCode> {
    let (dev, rest): (Vec<_>, Vec<_>) = SOURCES
        .iter()
        .filter(|(_, _, is_on)| is_on(t))
        .map(|(_, code, _)| *code)
        .partition(|code| *code == C::DictSourceDev);
    dev.into_iter()
        .chain([C::DictSourceCustom])
        .chain(rest)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary_reader::KAUTIAN_SUBTAG_USED_MASK;
    use protos::engine::KautianSubcollectionToggles;

    /// Bits 13..=26 of the wire mask (active sentinel + 13-bit enable mask).
    /// Low bits 0-12 are the source/variant region, asserted separately.
    const WIRE_HIGH_REGION: u32 = !0x1FFF;

    fn all_off() -> DictionarySourceToggles {
        DictionarySourceToggles::default()
    }

    fn all_subcoll_on() -> KautianSubcollectionToggles {
        KautianSubcollectionToggles {
            accent_lukang: true,
            accent_sansia: true,
            accent_taipak: true,
            accent_gilan: true,
            accent_tainan: true,
            accent_kaohsiung: true,
            accent_kinmen: true,
            accent_makung: true,
            accent_sintik: true,
            accent_taichung: true,
            name_appendix: true,
            alt_reading: true,
        }
    }

    fn all_on() -> DictionarySourceToggles {
        DictionarySourceToggles {
            kautian: true,
            taigitv: true,
            itaigi: true,
            sitbut: true,
            taihoa: true,
            taijit: true,
            kungge: true,
            stti: true,
            khpoo: true,
            variant: true,
            khiin: true,
            lkk: true,
            dev: true,
            kautian_subcollections: None,
        }
    }

    /// INVARIANT: every source is toggleable — all-off ⇒ empty source mask.
    /// dev (bit 10, dictionary-supplement-file toggle) is no longer an unconditional floor;
    /// `CUSTOM` stays in the source-code list (separate user-dict path, not a
    /// `dictionary.bin` source bit).
    #[test]
    fn all_toggles_off_keeps_nothing() {
        let r = compute_filters(&all_off());
        assert_eq!(r.dictionary_filter_bitmask, 0);
        assert_eq!(association_bitmask(&all_off()), 0);
        let codes: Vec<i32> = vec![DictionarySourceCode::DictSourceCustom as i32];
        assert_eq!(r.enabled_source_codes, codes);
    }

    /// INVARIANT: when every association source is on,
    /// `association_bitmask` is the `u32::MAX` sentinel — preserves the
    /// pre-v3.5.8 `allAssociationSourcesEnabled ? UInt32.max :
    /// associationBitmask()` branch.
    #[test]
    fn all_assoc_sources_returns_sentinel() {
        let r = compute_filters(&all_on());
        assert_eq!(association_bitmask(&all_on()), u32::MAX);
        // dictionary_filter_bitmask: bits 0-12 all set
        assert_eq!(r.dictionary_filter_bitmask, 0x1FFF);
    }

    /// 8-of-9 association sources on → NOT sentinel; equals exact mask.
    #[test]
    fn one_assoc_source_off_is_exact_mask() {
        let mut t = all_on();
        t.khpoo = false; // bit 8 off
        let r = compute_filters(&t);
        assert_eq!(association_bitmask(&t), 0xFF); // bits 0-7
        assert_eq!(r.dictionary_filter_bitmask, 0x1FFF & !(1 << 8));
    }

    /// Bit position truth table — each toggle drives the documented bit.
    #[test]
    fn bit_positions_match_layout() {
        type ToggleCase = (fn(&mut DictionarySourceToggles), u32);
        let cases: [ToggleCase; 13] = [
            (|t| t.kautian = true, 1 << 0),
            (|t| t.taigitv = true, 1 << 1),
            (|t| t.itaigi = true, 1 << 2),
            (|t| t.sitbut = true, 1 << 3),
            (|t| t.taihoa = true, 1 << 4),
            (|t| t.taijit = true, 1 << 5),
            (|t| t.kungge = true, 1 << 6),
            (|t| t.stti = true, 1 << 7),
            (|t| t.khpoo = true, 1 << 8),
            (|t| t.khiin = true, 1 << 9),
            (|t| t.dev = true, 1 << 10),
            (|t| t.lkk = true, 1 << 11),
            (|t| t.variant = true, 1 << 12),
        ];
        for (set, expected_bit) in cases {
            let mut t = all_off();
            set(&mut t);
            let mask = compute_filters(&t).dictionary_filter_bitmask;
            // Each toggle drives exactly its bit — no unconditional dev floor.
            assert_eq!(
                mask, expected_bit,
                "toggle bit {expected_bit:#x} should be the only bit set",
            );
        }
    }

    /// `variant` toggle never appears in `enabled_source_codes` —
    /// it's a filter bit, not a source.
    #[test]
    fn variant_excluded_from_source_codes() {
        let mut t = all_off();
        t.variant = true;
        let r = compute_filters(&t);
        // dev off (all_off) ⇒ only CUSTOM in source codes.
        let codes: Vec<i32> = vec![DictionarySourceCode::DictSourceCustom as i32];
        assert_eq!(r.enabled_source_codes, codes);
        // Dictionary filter has only bit 12 (variant) — no dev floor.
        assert_eq!(r.dictionary_filter_bitmask, 1 << 12);
    }

    /// Source codes order is stable (DEV/CUSTOM first, then toggle-on
    /// in declaration order). Pinned because Tab3 retag iterates the
    /// list and order divergence between platforms would be a bug.
    #[test]
    fn source_codes_order_is_stable() {
        let mut t = all_off();
        t.dev = true;
        t.lkk = true;
        t.kautian = true;
        let r = compute_filters(&t);
        // DEV (when on) first, then CUSTOM, then toggle-on in declaration order.
        let expected: Vec<i32> = vec![
            DictionarySourceCode::DictSourceDev as i32,
            DictionarySourceCode::DictSourceCustom as i32,
            DictionarySourceCode::DictSourceKautian as i32,
            DictionarySourceCode::DictSourceLkk as i32,
        ];
        assert_eq!(r.enabled_source_codes, expected);
    }

    // --- kautian subcollection wire ENCODE (Phase 3) ---

    /// DD5 zero-behaviour: absent sub-message ⇒ no high bits, engine skips the
    /// gate (legacy all-on). Mirrors a platform whose UI is not wired yet.
    #[test]
    fn subcoll_absent_sets_no_high_bits() {
        let mut t = all_on(); // kautian master on, but kautian_subcollections left None
        t.kautian_subcollections = None;
        let mask = compute_filters(&t).dictionary_filter_bitmask;
        assert_eq!(
            mask & WIRE_HIGH_REGION,
            0,
            "absent ⇒ no subcollection gating"
        );
        assert_eq!(mask & 0x1FFF, 0x1FFF, "low source/variant region unchanged");
    }

    /// Present + every subcollection on ⇒ active bit + full 13-bit enable mask.
    #[test]
    fn subcoll_present_all_on_sets_full_wire() {
        let mut t = all_off();
        t.kautian = true;
        t.kautian_subcollections = Some(all_subcoll_on());
        let mask = compute_filters(&t).dictionary_filter_bitmask;
        let expected_high = WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT
            | (KAUTIAN_SUBTAG_USED_MASK as u32) << WIRE_KAUTIAN_SUBCOLL_SHIFT;
        assert_eq!(mask & WIRE_HIGH_REGION, expected_high);
        // Never collides with the all-enabled sentinel.
        assert_ne!(mask, u32::MAX);
    }

    /// Present + every nested toggle off ⇒ main still on (not a user
    /// toggle), accent + name + alt-reading bits clear.
    #[test]
    fn subcoll_present_all_off_keeps_main() {
        let mut t = all_off();
        t.kautian = true;
        t.kautian_subcollections = Some(KautianSubcollectionToggles::default());
        let mask = compute_filters(&t).dictionary_filter_bitmask;
        let main_only = 1u32 << KAUTIAN_SUBTAG_MAIN_BIT;
        let expected_high =
            WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT | (main_only << WIRE_KAUTIAN_SUBCOLL_SHIFT);
        assert_eq!(mask & WIRE_HIGH_REGION, expected_high);
    }

    /// kautian master off ⇒ subcollection state ignored (kautian rows are
    /// dropped by the source-OR anyway), no high bits emitted.
    #[test]
    fn subcoll_ignored_when_master_off() {
        let mut t = all_off();
        t.kautian = false;
        t.kautian_subcollections = Some(all_subcoll_on());
        let mask = compute_filters(&t).dictionary_filter_bitmask;
        assert_eq!(mask & WIRE_HIGH_REGION, 0);
        assert_eq!(mask & (1 << 0), 0, "kautian source bit stays off");
    }

    /// Each accent toggle maps to subtag bit (1 + config.yaml index); name to
    /// bit 11, alt-reading to bit 12. Verifies the ENCODE order matches
    /// `dialect_columns`.
    #[test]
    fn subcoll_accent_bit_positions_match_dialect_order() {
        type AccentCase = (fn(&mut KautianSubcollectionToggles), u16);
        let cases: [AccentCase; 12] = [
            (|s| s.accent_lukang = true, 1),
            (|s| s.accent_sansia = true, 2),
            (|s| s.accent_taipak = true, 3),
            (|s| s.accent_gilan = true, 4),
            (|s| s.accent_tainan = true, 5),
            (|s| s.accent_kaohsiung = true, 6),
            (|s| s.accent_kinmen = true, 7),
            (|s| s.accent_makung = true, 8),
            (|s| s.accent_sintik = true, 9),
            (|s| s.accent_taichung = true, 10),
            (|s| s.name_appendix = true, KAUTIAN_SUBTAG_NAME_BIT),
            (|s| s.alt_reading = true, KAUTIAN_SUBTAG_ALT_READING_BIT),
        ];
        for (set, subtag_bit) in cases {
            let mut sub = KautianSubcollectionToggles::default();
            set(&mut sub);
            let mut t = all_off();
            t.kautian = true;
            t.kautian_subcollections = Some(sub);
            let mask = compute_filters(&t).dictionary_filter_bitmask;
            // main (bit 0) is always on, plus the one toggled bit.
            let subtag = (1u16 << KAUTIAN_SUBTAG_MAIN_BIT) | (1u16 << subtag_bit);
            let expected_high =
                WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT | (u32::from(subtag) << WIRE_KAUTIAN_SUBCOLL_SHIFT);
            assert_eq!(
                mask & WIRE_HIGH_REGION,
                expected_high,
                "subtag bit {subtag_bit} should be set",
            );
        }
    }

    #[test]
    fn a_record_bitmask_decodes_to_sources_in_bit_order() {
        // trace: bits 0 (kautian), 9 (khiin), 10 (dev), 11 (lkk), 12
        // (variant: a filter, no source) → [KAUTIAN, KHIIN, DEV, LKK]; the
        // old platform decoders (`LexiconBitmask` ×2, desktop
        // `SOURCE_BITS`) answered the same list.
        let mask = (1 << 0) | (1 << 9) | (1 << 10) | (1 << 11) | (1 << 12);
        assert_eq!(
            source_codes(mask),
            vec![
                C::DictSourceKautian,
                C::DictSourceKhiin,
                C::DictSourceDev,
                C::DictSourceLkk
            ]
        );
        assert!(source_codes(0).is_empty());
        assert!(source_codes(1 << 12).is_empty());
    }
}
