//! `dictionary_filters` — single source of truth for the user's dictionary
//! toggle → bitmask conversion. Replaces `EnabledDictionaries.swift` /
//! `.kt` (~80 LOC verbatim-mirrored bit math) per audit residue
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
//!  1 = taigitv   4 = taihoa   7 = stti    10 = dev (always set)
//!  2 = itaigi    5 = taijit   8 = khpoo   11 = lkk
//! ```

use protos::engine::{DictionaryFiltersResponse, DictionarySourceCode, DictionaryToggles};

/// Sentinel value for `assoc_lookup_bitmask` — preserves the documented
/// shortcut at `lexicon.proto:166-173`. When the platform receives this
/// value it forwards to `AssocLookupRequest.enabled_sources_bitmask`
/// without further processing; the engine treats it as "filter disabled".
const ASSOC_ALL_ENABLED_SENTINEL: u32 = u32::MAX;

/// Compute filter bitmasks + enabled-source codes from the user's
/// 12-toggle preference snapshot.
pub(crate) fn compute_filters(toggles: &DictionaryToggles) -> DictionaryFiltersResponse {
    DictionaryFiltersResponse {
        dictionary_filter_bitmask: dictionary_filter_bitmask(toggles),
        assoc_lookup_bitmask: assoc_lookup_bitmask(toggles),
        enabled_source_codes: enabled_source_codes(toggles)
            .into_iter()
            .map(|c| c as i32)
            .collect(),
    }
}

/// Full `dictionary.bin` filter bitmask (bits 0-12).
fn dictionary_filter_bitmask(t: &DictionaryToggles) -> u32 {
    let mut mask: u32 = 0;
    if t.kautian {
        mask |= 1 << 0;
    }
    if t.taigitv {
        mask |= 1 << 1;
    }
    if t.itaigi {
        mask |= 1 << 2;
    }
    if t.sitbut {
        mask |= 1 << 3;
    }
    if t.taihoa {
        mask |= 1 << 4;
    }
    if t.taijit {
        mask |= 1 << 5;
    }
    if t.kungge {
        mask |= 1 << 6;
    }
    if t.stti {
        mask |= 1 << 7;
    }
    if t.khpoo {
        mask |= 1 << 8;
    }
    if t.khiin {
        mask |= 1 << 9;
    }
    mask |= 1 << 10; // dev always included
    if t.lkk {
        mask |= 1 << 11;
    }
    if t.variant {
        mask |= 1 << 12;
    }
    mask
}

/// Association-bin filter bitmask: bits 0-8 mask of toggled sources.
/// Returns `u32::MAX` sentinel when ALL 9 association sources are on,
/// preserving the documented shortcut consumed by
/// `AssocLookupRequest.enabled_sources_bitmask`.
fn assoc_lookup_bitmask(t: &DictionaryToggles) -> u32 {
    if all_association_sources_enabled(t) {
        return ASSOC_ALL_ENABLED_SENTINEL;
    }
    let mut mask: u32 = 0;
    if t.kautian {
        mask |= 1 << 0;
    }
    if t.taigitv {
        mask |= 1 << 1;
    }
    if t.itaigi {
        mask |= 1 << 2;
    }
    if t.sitbut {
        mask |= 1 << 3;
    }
    if t.taihoa {
        mask |= 1 << 4;
    }
    if t.taijit {
        mask |= 1 << 5;
    }
    if t.kungge {
        mask |= 1 << 6;
    }
    if t.stti {
        mask |= 1 << 7;
    }
    if t.khpoo {
        mask |= 1 << 8;
    }
    mask
}

fn all_association_sources_enabled(t: &DictionaryToggles) -> bool {
    t.kautian
        && t.taigitv
        && t.itaigi
        && t.sitbut
        && t.taihoa
        && t.taijit
        && t.kungge
        && t.stti
        && t.khpoo
}

/// `DictionarySourceCode` set the platform should mark as enabled when
/// retagging Tab3 result badges. `DEV` + `CUSTOM` are non-toggleable and
/// always present; `variant` is a filter bit, not a source code.
fn enabled_source_codes(t: &DictionaryToggles) -> Vec<DictionarySourceCode> {
    use DictionarySourceCode as C;
    let mut codes = vec![C::DictSourceDev, C::DictSourceCustom];
    if t.kautian {
        codes.push(C::DictSourceKautian);
    }
    if t.taigitv {
        codes.push(C::DictSourceTaigitv);
    }
    if t.itaigi {
        codes.push(C::DictSourceItaigi);
    }
    if t.sitbut {
        codes.push(C::DictSourceSitbut);
    }
    if t.taihoa {
        codes.push(C::DictSourceTaihoa);
    }
    if t.taijit {
        codes.push(C::DictSourceTaijit);
    }
    if t.kungge {
        codes.push(C::DictSourceKungge);
    }
    if t.stti {
        codes.push(C::DictSourceStti);
    }
    if t.khpoo {
        codes.push(C::DictSourceKhpoo);
    }
    if t.khiin {
        codes.push(C::DictSourceKhiin);
    }
    if t.lkk {
        codes.push(C::DictSourceLkk);
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_off() -> DictionaryToggles {
        DictionaryToggles::default()
    }

    fn all_on() -> DictionaryToggles {
        DictionaryToggles {
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
        }
    }

    /// INVARIANT: when every toggle is off, `dictionary_filter_bitmask`
    /// still has bit 10 (`dev`) set — verified pre-v3.5.8 in
    /// `EnabledDictionaries.swift:50-83` / `.kt:34-49`.
    #[test]
    fn all_toggles_off_keeps_dev_only() {
        let r = compute_filters(&all_off());
        assert_eq!(r.dictionary_filter_bitmask, 1 << 10);
        assert_eq!(r.assoc_lookup_bitmask, 0);
        let codes: Vec<i32> = vec![
            DictionarySourceCode::DictSourceDev as i32,
            DictionarySourceCode::DictSourceCustom as i32,
        ];
        assert_eq!(r.enabled_source_codes, codes);
    }

    /// INVARIANT: when every association source is on,
    /// `assoc_lookup_bitmask` is the `u32::MAX` sentinel — preserves the
    /// pre-v3.5.8 `allAssociationSourcesEnabled ? UInt32.max :
    /// associationBitmask()` branch documented at `lexicon.proto:166-173`.
    #[test]
    fn all_assoc_sources_returns_sentinel() {
        let r = compute_filters(&all_on());
        assert_eq!(r.assoc_lookup_bitmask, u32::MAX);
        // dictionary_filter_bitmask: bits 0-12 all set
        assert_eq!(r.dictionary_filter_bitmask, 0x1FFF);
    }

    /// 8-of-9 association sources on → NOT sentinel; equals exact mask.
    #[test]
    fn one_assoc_source_off_is_exact_mask() {
        let mut t = all_on();
        t.khpoo = false; // bit 8 off
        let r = compute_filters(&t);
        assert_eq!(r.assoc_lookup_bitmask, 0xFF); // bits 0-7
        assert_eq!(r.dictionary_filter_bitmask, 0x1FFF & !(1 << 8));
    }

    /// Bit position truth table — each toggle drives the documented bit.
    #[test]
    fn bit_positions_match_layout() {
        let cases: [(fn(&mut DictionaryToggles), u32); 12] = [
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
            (|t| t.lkk = true, 1 << 11),
            (|t| t.variant = true, 1 << 12),
        ];
        for (set, expected_extra_bit) in cases {
            let mut t = all_off();
            set(&mut t);
            let mask = compute_filters(&t).dictionary_filter_bitmask;
            // dev bit 10 always set
            assert_eq!(
                mask,
                (1 << 10) | expected_extra_bit,
                "toggle bit {expected_extra_bit:#x} should add to dev-only mask",
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
        let codes: Vec<i32> = vec![
            DictionarySourceCode::DictSourceDev as i32,
            DictionarySourceCode::DictSourceCustom as i32,
        ];
        assert_eq!(r.enabled_source_codes, codes);
        // But dictionary filter has bit 12 set
        assert_eq!(r.dictionary_filter_bitmask, (1 << 10) | (1 << 12));
    }

    /// Source codes order is stable (DEV/CUSTOM first, then toggle-on
    /// in declaration order). Pinned because Tab3 retag iterates the
    /// list and order divergence between platforms would be a bug.
    #[test]
    fn source_codes_order_is_stable() {
        let mut t = all_off();
        t.lkk = true;
        t.kautian = true;
        let r = compute_filters(&t);
        let expected: Vec<i32> = vec![
            DictionarySourceCode::DictSourceDev as i32,
            DictionarySourceCode::DictSourceCustom as i32,
            DictionarySourceCode::DictSourceKautian as i32,
            DictionarySourceCode::DictSourceLkk as i32,
        ];
        assert_eq!(r.enabled_source_codes, expected);
    }
}
