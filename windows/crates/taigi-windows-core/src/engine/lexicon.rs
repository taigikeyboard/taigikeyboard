//! Lexicon slice of the engine bridge: loading the dictionary data and
//! resolving the user's source toggles into the engine's bitmask. Port of
//! `RustEngineBridge+Lexicon.swift:1-237`. The search ops arrive with the
//! dictionary-search page (PR8).

// 中文: 辭典切片 — 安裝四個辭典檔、把使用者的來源開關解析成引擎位元遮罩。

use std::collections::BTreeSet;

use protos::engine::{
    lexicon_request, lexicon_response, request, response, DictionaryFiltersRequest,
    DictionarySourceCode, DictionaryToggles, InstallRequest, KautianSubcollToggles, LexiconRequest,
    LexiconResponse,
};

use super::bridge::{record_failure, roundtrip};
use crate::dictionary_artifacts::DictionaryArtifacts;
use crate::settings::DictionarySourceToggles;

/// Record counts the engine reports after loading. Their only job is to make
/// a successful install self-evident in the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexiconInstallStats {
    pub dictionary_record_count: u64,
    pub prefix_index_entry_count: u64,
}

/// A dictionary source, in the platform's own vocabulary. Decoded from the
/// wire's `DictionarySourceCode` through an explicit match — the wire codes
/// are stable numbers and this enum has no numeric contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DictionarySource {
    Kautian,
    Taigitv,
    Itaigi,
    Sitbut,
    Taihoa,
    Taijit,
    Kungge,
    Stti,
    Khpoo,
    Khiin,
    Lkk,
    Dev,
    Custom,
}

impl DictionarySource {
    /// An unrecognised code is dropped — a newer engine naming a source this
    /// build has never heard of is not a reason to fail a search.
    fn from_code(code: i32) -> Option<Self> {
        Some(match DictionarySourceCode::try_from(code).ok()? {
            DictionarySourceCode::DictSourceKautian => Self::Kautian,
            DictionarySourceCode::DictSourceTaigitv => Self::Taigitv,
            DictionarySourceCode::DictSourceItaigi => Self::Itaigi,
            DictionarySourceCode::DictSourceSitbut => Self::Sitbut,
            DictionarySourceCode::DictSourceTaihoa => Self::Taihoa,
            DictionarySourceCode::DictSourceTaijit => Self::Taijit,
            DictionarySourceCode::DictSourceKungge => Self::Kungge,
            DictionarySourceCode::DictSourceStti => Self::Stti,
            DictionarySourceCode::DictSourceKhpoo => Self::Khpoo,
            DictionarySourceCode::DictSourceKhiin => Self::Khiin,
            DictionarySourceCode::DictSourceLkk => Self::Lkk,
            DictionarySourceCode::DictSourceDev => Self::Dev,
            DictionarySourceCode::DictSourceCustom => Self::Custom,
            DictionarySourceCode::DictSourceUnspecified => return None,
        })
    }
}

/// What one resolve of the user's dictionary toggles answered. Both halves
/// come from the same round-trip on purpose: the mask decides which rows the
/// engine returns and the source set decides which badges those rows carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryFilters {
    /// The engine's own answer, verbatim.
    pub dictionary_filter_bitmask: u32,
    /// The sources the user has switched on, for labelling results.
    pub enabled_sources: BTreeSet<DictionarySource>,
}

/// A mask carrying no source bits, for the user who switched every
/// dictionary off. The wire cannot say that with `0`: the composing path
/// reads `0` as "platform did not wire this" and turns everything back on
/// (`engine/composing/src/dispatch.rs:247`). Bit 13 is the kautian
/// subcollection gate's "active" flag, which makes the mask non-zero while
/// leaving the source region (bits 0-12) empty.
///
/// NAMED CROSS-PLATFORM DIVERGENCE (deferred): iOS and Android send the
/// engine's `0` straight through and still search every dictionary in this
/// state; macOS and Windows do not.
pub const NO_SOURCES_ENABLED_BITMASK: u32 = 1 << 13;

/// What a SEARCH sends when the toggles could not be resolved. The search
/// path takes `u32::MAX` as its "filter disabled" sentinel
/// (`engine/lexicon/src/dictionary_reader.rs:147`); sending `0` there would
/// be fail-CLOSED.
pub const ALL_SOURCES_ENABLED_SEARCH_BITMASK: u32 = u32::MAX;

impl DictionaryFilters {
    /// The value to put on the composing wire (`FetchAtPos.enabled_sources_bitmask`).
    pub fn wire_mask(&self) -> u32 {
        if self.dictionary_filter_bitmask == 0 {
            NO_SOURCES_ENABLED_BITMASK
        } else {
            self.dictionary_filter_bitmask
        }
    }
}

/// Points the engine at the dictionary data. Sent once per process; the
/// files are read-only and outlive every composing session.
///
/// `None` means the engine has no lexicon installed. Nothing retries or falls
/// back: a search against an uninstalled engine returns no candidates, the
/// same graceful degradation any other empty result produces.
pub fn install(
    artifacts: &DictionaryArtifacts,
    dictionary_version: u32,
) -> Option<LexiconInstallStats> {
    let op = "lexiconInstall";
    let path = |path: &std::path::Path| {
        DictionaryArtifacts::wire(path).or_else(|| {
            record_failure(op, &format!("non-UTF-8 artefact path {}", path.display()));
            None
        })
    };
    let install = InstallRequest {
        trie_path: path(&artifacts.trie_path)?,
        dictionary_bin_path: path(&artifacts.dictionary_bin_path)?,
        association_bin_path: path(&artifacts.association_bin_path)?,
        dictionary_version,
        syllable_inventory_path: path(&artifacts.syllable_inventory_path)?,
    };
    let response = lexicon_response(lexicon_request::Method::Install(install), op)?;
    match response.result {
        Some(lexicon_response::Result::InstallResult(result)) => Some(LexiconInstallStats {
            dictionary_record_count: result.dictionary_record_count,
            prefix_index_entry_count: result.prefix_index_entry_count,
        }),
        _ => {
            record_failure(op, "response carried no install result");
            None
        }
    }
}

/// Resolves the user's dictionary toggles into the bitmask the engine filters
/// candidates by. The bit layout — including the kautian subcollection region
/// in bits 13-25 — belongs to Rust (`engine/lexicon/src/dictionary_filters.rs`);
/// this asks for it rather than reproducing it (`planning.md` § No redundant
/// fallback).
///
/// `None` means the round-trip failed. Callers resolve ONCE per query and pass
/// the answer down, so mask and badge set describe one instant.
pub fn dictionary_filters(toggles: &DictionarySourceToggles) -> Option<DictionaryFilters> {
    let subcollections = &toggles.kautian_subcollections;
    let toggles_proto = DictionaryToggles {
        kautian: toggles.kautian,
        taigitv: toggles.taigitv,
        itaigi: toggles.itaigi,
        sitbut: toggles.sitbut,
        taihoa: toggles.taihoa,
        taijit: toggles.taijit,
        kungge: toggles.kungge,
        stti: toggles.stti,
        khpoo: toggles.khpoo,
        variant: toggles.variant,
        khiin: toggles.khiin,
        lkk: toggles.lkk,
        dev: toggles.dev,
        // Always sent: an absent subcollection message tells the engine to
        // skip the gate and treat every subcollection as on.
        kautian_subcoll: Some(KautianSubcollToggles {
            accent_lukang: subcollections.accent_lukang,
            accent_sansia: subcollections.accent_sansia,
            accent_taipak: subcollections.accent_taipak,
            accent_gilan: subcollections.accent_gilan,
            accent_tainan: subcollections.accent_tainan,
            accent_kaohsiung: subcollections.accent_kaohsiung,
            accent_kinmen: subcollections.accent_kinmen,
            accent_makung: subcollections.accent_makung,
            accent_sintik: subcollections.accent_sintik,
            accent_taichung: subcollections.accent_taichung,
            name_appendix: subcollections.name_appendix,
        }),
    };
    let op = "lexiconDictionaryFilters";
    let response = lexicon_response(
        lexicon_request::Method::DictionaryFilters(DictionaryFiltersRequest {
            toggles: Some(toggles_proto),
        }),
        op,
    )?;
    match response.result {
        Some(lexicon_response::Result::DictionaryFiltersResult(result)) => {
            Some(DictionaryFilters {
                dictionary_filter_bitmask: result.dictionary_filter_bitmask,
                enabled_sources: result
                    .enabled_source_codes
                    .iter()
                    .filter_map(|code| DictionarySource::from_code(*code))
                    .collect(),
            })
        }
        _ => {
            record_failure(op, "response carried no dictionary-filters result");
            None
        }
    }
}

/// The value to put in `FetchAtPos.enabled_sources_bitmask` for `toggles`.
/// Three answers: a resolved mask goes out as it is; a resolved `0` (the
/// user turned everything off) goes out as `NO_SOURCES_ENABLED_BITMASK`; a
/// FAILED resolve goes out as `0` so the engine searches everything — a
/// failure is not a preference, and degrading to a wider list is recoverable
/// where degrading to none looks like a broken keyboard.
pub fn enabled_sources_bitmask(toggles: &DictionarySourceToggles) -> u32 {
    dictionary_filters(toggles).map_or(0, |filters| filters.wire_mask())
}

fn lexicon_response(method: lexicon_request::Method, op: &str) -> Option<LexiconResponse> {
    let payload = request::Payload::Lexicon(LexiconRequest {
        method: Some(method),
    });
    match roundtrip(payload, op, 0, None)? {
        response::Payload::Lexicon(response) => Some(response),
        other => {
            record_failure(op, &format!("expected a lexicon payload, got {other:?}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_mask_turns_all_off_into_the_sentinel() {
        let all_off = DictionaryFilters {
            dictionary_filter_bitmask: 0,
            enabled_sources: BTreeSet::new(),
        };
        assert_eq!(all_off.wire_mask(), NO_SOURCES_ENABLED_BITMASK);
        let some = DictionaryFilters {
            dictionary_filter_bitmask: 0b101,
            enabled_sources: BTreeSet::new(),
        };
        assert_eq!(some.wire_mask(), 0b101);
    }

    #[test]
    fn source_codes_decode_by_explicit_match() {
        assert_eq!(
            DictionarySource::from_code(1),
            Some(DictionarySource::Kautian)
        );
        assert_eq!(
            DictionarySource::from_code(13),
            Some(DictionarySource::Custom)
        );
        assert_eq!(DictionarySource::from_code(0), None);
        assert_eq!(DictionarySource::from_code(999), None);
    }
}
