//! Lexicon slice of the engine bridge: loading the dictionary data and
//! resolving the user's source toggles into the engine's bitmask, and the
//! dictionary-search page's two lookups. Port of
//! `RustEngineBridge+Lexicon.swift` + `Engine/DictionarySource.swift`.

// 辭典切片 — 安裝四個辭典檔、把使用者的來源開關解析成引擎位元遮罩。

use std::collections::BTreeSet;

use protos::engine::{
    lexicon_request, lexicon_response, request, response, DictionaryFiltersRequest,
    DictionarySourceCode, DictionaryToggles, InputMode as WireInputMode, InstallRequest,
    IsHanziRequest, KautianSubcollToggles, LexiconRequest, LexiconResponse, SearchByHanziRequest,
    SearchWithSourcesRequest, TaigiWord,
};

use super::bridge::{record_failure, roundtrip};
use crate::dictionary_artifacts::DictionaryArtifacts;
use crate::settings::{DictionarySourceToggles, InputMode};
use crate::strings::StringKey;

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

/// The sources a record's `source_bitmask` names, in bit order — which is
/// the order the badges are drawn in (`LexiconBitmask.sourceBits`,
/// `DictionarySource.swift:43-56`). Bit 12 (variant) is a filter, not a
/// source a record wears a badge for.
const SOURCE_BITS: [(u32, DictionarySource); 12] = [
    (1 << 0, DictionarySource::Kautian),
    (1 << 1, DictionarySource::Taigitv),
    (1 << 2, DictionarySource::Itaigi),
    (1 << 3, DictionarySource::Sitbut),
    (1 << 4, DictionarySource::Taihoa),
    (1 << 5, DictionarySource::Taijit),
    (1 << 6, DictionarySource::Kungge),
    (1 << 7, DictionarySource::Stti),
    (1 << 8, DictionarySource::Khpoo),
    (1 << 9, DictionarySource::Khiin),
    (1 << 10, DictionarySource::Dev),
    (1 << 11, DictionarySource::Lkk),
];

impl DictionarySource {
    /// The sources a record belongs to, in bit order.
    pub fn from_bitmask(bitmask: u32) -> Vec<Self> {
        SOURCE_BITS
            .iter()
            .filter(|(bit, _)| bitmask & bit != 0)
            .map(|(_, source)| *source)
            .collect()
    }

    /// The badge a search result wears for this source
    /// (`DictionarySearchPage.swift:160-175`): the three supplements share
    /// one word, the custom dictionary its pane's name.
    pub fn badge_key(self) -> StringKey {
        match self {
            Self::Kautian => StringKey::DictionaryKautianTag,
            Self::Taigitv => StringKey::DictionaryTaigitvTag,
            Self::Itaigi => StringKey::DictionaryITaigiTag,
            Self::Sitbut => StringKey::DictionarySitbutTag,
            Self::Taihoa => StringKey::DictionaryTaihoaTag,
            Self::Taijit => StringKey::DictionaryTaijitTag,
            Self::Kungge => StringKey::DictionaryKunggeTag,
            Self::Stti => StringKey::DictionarySttiTag,
            Self::Lkk => StringKey::DictionaryLkkTag,
            Self::Khpoo | Self::Khiin | Self::Dev => StringKey::DictionarySupplementSectionTitle,
            Self::Custom => StringKey::DictionaryCustomDictionary,
        }
    }

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

/// One dictionary record as the search page lists it (`LexiconRow.swift`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconRow {
    pub id: i64,
    pub roman: String,
    pub hanzi: Option<String>,
    pub length_score: Option<i32>,
    pub source_bitmask: Option<u32>,
}

impl LexiconRow {
    fn from_wire(word: TaigiWord) -> Self {
        Self {
            id: word.id,
            roman: word.roman,
            hanzi: word.hanji,
            length_score: word.length_score,
            source_bitmask: word.source_bitmask,
        }
    }

    /// The search page's order (`DictionarySearchService.Ordering`): 教典
    /// records first, then by length score descending, then as the engine
    /// listed them.
    pub fn sorted_for_search(rows: Vec<LexiconRow>) -> Vec<LexiconRow> {
        let is_kautian = |row: &LexiconRow| {
            DictionarySource::from_bitmask(row.source_bitmask.unwrap_or(0))
                .contains(&DictionarySource::Kautian)
        };
        let mut indexed: Vec<(usize, LexiconRow)> = rows.into_iter().enumerate().collect();
        indexed.sort_by(|(first_index, first), (second_index, second)| {
            is_kautian(second)
                .cmp(&is_kautian(first))
                .then_with(|| {
                    second
                        .length_score
                        .unwrap_or(0)
                        .cmp(&first.length_score.unwrap_or(0))
                })
                .then_with(|| first_index.cmp(second_index))
        });
        indexed.into_iter().map(|(_, row)| row).collect()
    }
}

fn wire_input_mode(mode: InputMode) -> i32 {
    match mode {
        InputMode::Tl => WireInputMode::Tl as i32,
        InputMode::Poj => WireInputMode::Poj as i32,
    }
}

/// The 辭典搜尋 page's all-source romanization lookup
/// (`RustEngineBridge+Lexicon.swift:245-264`).
pub fn search_with_sources(
    input: &str,
    mode: InputMode,
    limit: u32,
    enabled_sources_bitmask: u32,
) -> Vec<LexiconRow> {
    let op = "lexiconSearchWithSources";
    let request = SearchWithSourcesRequest {
        input: input.to_owned(),
        input_mode: wire_input_mode(mode),
        limit,
        enabled_sources_bitmask,
    };
    let Some(response) = lexicon_response(lexicon_request::Method::SearchWithSources(request), op)
    else {
        return Vec::new();
    };
    match response.result {
        Some(lexicon_response::Result::SearchWithSourcesResult(result)) => {
            result.rows.into_iter().map(LexiconRow::from_wire).collect()
        }
        _ => {
            record_failure(op, "response carried no search result");
            Vec::new()
        }
    }
}

/// The 辭典搜尋 page's hanji-prefix lookup (`RustEngineBridge+Lexicon.swift:267-286`).
pub fn search_by_hanzi(
    query: &str,
    mode: InputMode,
    limit: u32,
    enabled_sources_bitmask: u32,
) -> Vec<LexiconRow> {
    let op = "lexiconSearchByHanzi";
    let request = SearchByHanziRequest {
        query: query.to_owned(),
        input_mode: wire_input_mode(mode),
        limit,
        enabled_sources_bitmask,
    };
    let Some(response) = lexicon_response(lexicon_request::Method::SearchByHanzi(request), op)
    else {
        return Vec::new();
    };
    match response.result {
        Some(lexicon_response::Result::SearchByHanziResult(result)) => {
            result.rows.into_iter().map(LexiconRow::from_wire).collect()
        }
        _ => {
            record_failure(op, "response carried no search result");
            Vec::new()
        }
    }
}

/// Whether `text` is a hanji query (`RustEngineBridge+Lexicon.swift:298-309`).
pub fn is_hanzi(text: &str) -> bool {
    let op = "isHanzi";
    let request = IsHanziRequest {
        text: text.to_owned(),
    };
    let Some(response) = lexicon_response(lexicon_request::Method::IsHanzi(request), op) else {
        return false;
    };
    match response.result {
        Some(lexicon_response::Result::IsHanziResult(result)) => result.is_hanzi,
        _ => {
            record_failure(op, "response carried no is-hanzi result");
            false
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

    fn row(index: i64, score: Option<i32>, bitmask: u32) -> LexiconRow {
        LexiconRow {
            id: index,
            roman: format!("r{index}"),
            hanzi: None,
            length_score: score,
            source_bitmask: Some(bitmask),
        }
    }

    #[test]
    fn a_bitmask_decodes_in_bit_order_and_skips_the_non_source_bits() {
        // trace: bits 0 (kautian), 9 (khiin), 11 (lkk), 12 (variant: not a
        // badge source) → [Kautian, Khiin, Lkk].
        assert_eq!(
            DictionarySource::from_bitmask((1 << 0) | (1 << 9) | (1 << 11) | (1 << 12)),
            vec![
                DictionarySource::Kautian,
                DictionarySource::Khiin,
                DictionarySource::Lkk
            ]
        );
        assert!(DictionarySource::from_bitmask(0).is_empty());
    }

    #[test]
    fn search_order_puts_kautian_first_then_length_score_then_engine_order() {
        // trace: DictionarySearchService.Ordering — kautian beats a higher
        // score; among kautian rows the higher score wins; ties keep order.
        let rows = vec![
            row(0, Some(9), 1 << 1),
            row(1, Some(2), 1 << 0),
            row(2, Some(5), 1 << 0),
            row(3, Some(5), 1 << 0),
            row(4, None, 1 << 2),
        ];
        let ids: Vec<i64> = LexiconRow::sorted_for_search(rows)
            .into_iter()
            .map(|row| row.id)
            .collect();
        assert_eq!(ids, vec![2, 3, 1, 0, 4]);
    }

    #[test]
    fn is_hanzi_answers_through_the_engine() {
        assert!(is_hanzi("台語"));
        assert!(!is_hanzi("tai"));
    }

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
