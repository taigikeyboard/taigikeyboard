//! Lexicon slice of the engine bridge: loading the dictionary data and
//! resolving the user's source toggles into the engine's bitmask, and the
//! dictionary-search page's two lookups. Twin of iOS
//! `RustEngineBridge+Lexicon.swift`.

use std::collections::BTreeSet;

use protos::engine::{
    lexicon_request, lexicon_response, request, response, DictionaryFiltersRequest,
    DictionarySourceCode, DictionarySourceToggles as WireDictionarySourceToggles,
    InputMode as WireInputMode, InstallRequest, IsHanjiRequest, KautianSubcollectionToggles,
    LexiconRequest, LexiconResponse, SearchByHanjiRequest, SearchWithSourcesRequest, TaigiWord,
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

impl DictionarySource {
    /// The badge a search result wears for this source: the three
    /// supplements share one word, the custom dictionary its pane's name.
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

/// What a SEARCH sends when the toggles could not be resolved. The search
/// path takes `u32::MAX` as its "filter disabled" sentinel
/// (`engine/lexicon/src/dictionary_reader.rs:147`); sending `0` there would
/// be fail-CLOSED.
pub const ALL_SOURCES_ENABLED_SEARCH_BITMASK: u32 = u32::MAX;

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
/// this asks for it rather than reproducing it (no redundant fallback,
/// `AGENTS.md` § Design principles).
///
/// `None` means the round-trip failed. Callers resolve ONCE per query and pass
/// the answer down, so mask and badge set describe one instant.
pub fn dictionary_filters(toggles: &DictionarySourceToggles) -> Option<DictionaryFilters> {
    let op = "lexiconDictionaryFilters";
    let response = lexicon_response(
        lexicon_request::Method::DictionaryFilters(DictionaryFiltersRequest {
            toggles: Some(dictionary_toggles(toggles)),
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

/// The user's dictionary toggles on the wire — what `DictionaryFilters` and
/// `FetchAtPos` carry; the engine resolves them into its source filter.
pub(crate) fn dictionary_toggles(toggles: &DictionarySourceToggles) -> WireDictionarySourceToggles {
    let subcollections = &toggles.kautian_subcollections;
    WireDictionarySourceToggles {
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
        kautian_subcollections: Some(KautianSubcollectionToggles {
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
    }
}

/// One dictionary record as the Dictionary Search page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconRow {
    pub id: i64,
    pub roman: String,
    pub hanji: Option<String>,
    /// The dictionaries the record belongs to, in the engine's (source-bit)
    /// order — the order the badges are drawn in.
    pub sources: Vec<DictionarySource>,
}

impl LexiconRow {
    fn from_wire(word: TaigiWord) -> Self {
        Self {
            id: word.id,
            roman: word.roman,
            hanji: word.hanji,
            sources: word
                .sources
                .into_iter()
                .filter_map(DictionarySource::from_code)
                .collect(),
        }
    }
}

fn wire_input_mode(mode: InputMode) -> i32 {
    match mode {
        InputMode::Tl => WireInputMode::Tl as i32,
        InputMode::Poj => WireInputMode::Poj as i32,
        InputMode::Tps => WireInputMode::Tps as i32,
    }
}

/// The Dictionary Search page's all-source romanization lookup.
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

/// The Dictionary Search page's hanji-prefix lookup.
pub fn search_by_hanji(
    query: &str,
    mode: InputMode,
    limit: u32,
    enabled_sources_bitmask: u32,
) -> Vec<LexiconRow> {
    let op = "lexiconSearchByHanji";
    let request = SearchByHanjiRequest {
        query: query.to_owned(),
        input_mode: wire_input_mode(mode),
        limit,
        enabled_sources_bitmask,
    };
    let Some(response) = lexicon_response(lexicon_request::Method::SearchByHanji(request), op)
    else {
        return Vec::new();
    };
    match response.result {
        Some(lexicon_response::Result::SearchByHanjiResult(result)) => {
            result.rows.into_iter().map(LexiconRow::from_wire).collect()
        }
        _ => {
            record_failure(op, "response carried no search result");
            Vec::new()
        }
    }
}

/// Whether `text` is a hanji query.
pub fn is_hanji(text: &str) -> bool {
    let op = "isHanji";
    let request = IsHanjiRequest {
        text: text.to_owned(),
    };
    let Some(response) = lexicon_response(lexicon_request::Method::IsHanji(request), op) else {
        return false;
    };
    match response.result {
        Some(lexicon_response::Result::IsHanjiResult(result)) => result.is_hanji,
        _ => {
            record_failure(op, "response carried no is-hanji result");
            false
        }
    }
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

    /// One toggle: its name, reading it, flipping it, and reading its wire
    /// field.
    type ToggleField = (
        &'static str,
        fn(&DictionarySourceToggles) -> bool,
        fn(&mut DictionarySourceToggles),
        fn(&WireDictionarySourceToggles) -> bool,
    );

    /// Every toggle `dictionary_toggles` copies. A field added to the
    /// settings fails to compile in the destructure below — give it a row
    /// there too.
    fn toggle_fields() -> Vec<ToggleField> {
        macro_rules! source {
            ($field:ident) => {
                (
                    stringify!($field),
                    |toggles: &DictionarySourceToggles| toggles.$field,
                    |toggles: &mut DictionarySourceToggles| toggles.$field = !toggles.$field,
                    |wire: &WireDictionarySourceToggles| wire.$field,
                )
            };
        }
        macro_rules! subcollection {
            ($field:ident) => {
                (
                    stringify!($field),
                    |toggles: &DictionarySourceToggles| toggles.kautian_subcollections.$field,
                    |toggles: &mut DictionarySourceToggles| {
                        toggles.kautian_subcollections.$field =
                            !toggles.kautian_subcollections.$field
                    },
                    |wire: &WireDictionarySourceToggles| {
                        wire.kautian_subcollections
                            .as_ref()
                            .is_some_and(|subcollections| subcollections.$field)
                    },
                )
            };
        }
        let DictionarySourceToggles {
            kautian: _,
            taigitv: _,
            itaigi: _,
            sitbut: _,
            taihoa: _,
            taijit: _,
            kungge: _,
            stti: _,
            khpoo: _,
            variant: _,
            khiin: _,
            lkk: _,
            dev: _,
            kautian_subcollections:
                crate::settings::KautianSubcollections {
                    accent_lukang: _,
                    accent_sansia: _,
                    accent_taipak: _,
                    accent_gilan: _,
                    accent_tainan: _,
                    accent_kaohsiung: _,
                    accent_kinmen: _,
                    accent_makung: _,
                    accent_sintik: _,
                    accent_taichung: _,
                    name_appendix: _,
                },
        } = DictionarySourceToggles::DEFAULT;
        vec![
            source!(kautian),
            source!(taigitv),
            source!(itaigi),
            source!(sitbut),
            source!(taihoa),
            source!(taijit),
            source!(kungge),
            source!(stti),
            source!(khpoo),
            source!(variant),
            source!(khiin),
            source!(lkk),
            source!(dev),
            subcollection!(accent_lukang),
            subcollection!(accent_sansia),
            subcollection!(accent_taipak),
            subcollection!(accent_gilan),
            subcollection!(accent_tainan),
            subcollection!(accent_kaohsiung),
            subcollection!(accent_kinmen),
            subcollection!(accent_makung),
            subcollection!(accent_sintik),
            subcollection!(accent_taichung),
            subcollection!(name_appendix),
        ]
    }

    /// The 24 flags are copied field by field, and a swapped or inverted
    /// pair compiles and filters the wrong dictionary: every wire field
    /// carries its own toggle's value, at the defaults and with any one
    /// toggle flipped. A subcollection read from an absent message is off,
    /// so this also pins that the message is always sent (absent, the engine
    /// treats every subcollection as on). Ported from the Swift encoder's tests
    /// (`RustEngineBridgeDictionaryTogglesTests`) when macOS stopped
    /// encoding the toggles itself (roadmap P13).
    #[test]
    fn each_toggle_lands_on_its_own_wire_field() {
        let fields = toggle_fields();
        let defaults = DictionarySourceToggles::DEFAULT;
        let one_flipped = fields.iter().map(|(_, _, flip, _)| {
            let mut toggles = defaults.clone();
            flip(&mut toggles);
            toggles
        });
        for toggles in std::iter::once(defaults.clone()).chain(one_flipped) {
            let wire = dictionary_toggles(&toggles);
            for (name, read, _, wire_field) in &fields {
                assert_eq!(wire_field(&wire), read(&toggles), "{name} in {toggles:?}");
            }
        }
    }

    #[test]
    fn wire_sources_keep_the_engine_order_and_drop_unknown_codes() {
        // trace: codes 1 (kautian), 10 (khiin), 12 (dev), 11 (lkk) in the
        // engine's bit order; 0 (unspecified) and 999 (a newer engine's
        // source) are dropped by `from_code`.
        let word = TaigiWord {
            sources: vec![1, 10, 12, 11, 0, 999],
            ..TaigiWord::default()
        };
        assert_eq!(
            LexiconRow::from_wire(word).sources,
            vec![
                DictionarySource::Kautian,
                DictionarySource::Khiin,
                DictionarySource::Dev,
                DictionarySource::Lkk
            ]
        );
    }

    #[test]
    fn is_hanji_answers_through_the_engine() {
        assert!(is_hanji("台語"));
        assert!(!is_hanji("tai"));
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
