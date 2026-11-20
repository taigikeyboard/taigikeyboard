//! Lexicon slice of the engine bridge: loading the dictionary data, the
//! user's source toggles on the wire, and the Manage Dictionaries search's
//! one engine call. Twin of iOS `EngineLexiconClient`.

use protos::engine::{
    lexicon_request, lexicon_response, request, response, DictionarySearchRequest,
    DictionarySearchRow, DictionarySourceCode,
    DictionarySourceToggles as WireDictionarySourceToggles, InstallRequest,
    KautianSubcollectionToggles, LexiconRequest, LexiconResponse,
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
    /// supplements and the custom dictionary share one, as on iOS and
    /// Android (`DictionarySearchPanel.tagKey`, `DictionarySource.tagKey`).
    pub(crate) fn badge_key(self) -> StringKey {
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
            Self::Khpoo | Self::Khiin | Self::Dev | Self::Custom => {
                StringKey::DictionarySupplementSectionTitle
            }
        }
    }

    /// An unrecognised code is dropped — a newer engine naming a source this
    /// build has never heard of is not a reason to fail a search.
    pub(crate) fn from_code(code: i32) -> Option<Self> {
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

/// The user's dictionary toggles on the wire — what `DictionarySearch` and
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
            alt_reading: subcollections.alt_reading,
        }),
    }
}

/// The Manage Dictionaries search (lexicon `DictionarySearch`): the engine
/// picks the Hanji or romanization lookup, filters by `toggles`, renders the
/// romanization for `mode` and lists the custom entries first (when
/// `custom_dictionary_enabled`), at most `limit` rows per section. Empty when
/// the engine could not answer — logged, never retried.
pub fn dictionary_search(
    query: &str,
    mode: InputMode,
    toggles: &DictionarySourceToggles,
    custom_dictionary_enabled: bool,
    limit: u32,
) -> Vec<DictionarySearchRow> {
    let op = "lexiconDictionarySearch";
    let request = DictionarySearchRequest {
        query: query.to_owned(),
        input_mode: mode.wire().to_owned(),
        toggles: Some(dictionary_toggles(toggles)),
        custom_dictionary_enabled,
        limit,
    };
    let Some(response) = lexicon_response(lexicon_request::Method::DictionarySearch(request), op)
    else {
        return Vec::new();
    };
    match response.result {
        Some(lexicon_response::Result::DictionarySearchResult(result)) => result.rows,
        _ => {
            record_failure(op, "response carried no dictionary-search result");
            Vec::new()
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
                    alt_reading: _,
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
            subcollection!(alt_reading),
        ]
    }

    /// The 25 flags are copied field by field, and a swapped or inverted
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
