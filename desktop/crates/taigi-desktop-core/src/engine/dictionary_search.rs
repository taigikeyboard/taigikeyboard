//! The Manage Dictionaries search: the settings in force sent as one engine
//! `DictionarySearch` (custom entries first, then the dictionaries under the
//! user's source toggles, in the engine's order), decoded for the pane. One
//! lookup for the Windows, Linux and macOS settings windows. Twin of iOS
//! `DictionarySearchService.swift`.

use protos::engine::{dictionary_search_row, DictionarySearchRow};

use super::lexicon::dictionary_search;
use super::{chhoe_url, moe_url, DictionarySource};
use crate::settings::{keys, DictionarySourceToggles, InputMode, SettingsDocument};
use crate::strings::StringKey;

/// `DictionarySearchService.resultLimit`.
const RESULT_LIMIT: u32 = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultId {
    System(i64),
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionarySearchResult {
    pub id: ResultId,
    /// The romanization as the current mode spells it.
    pub roman: String,
    /// What the web dictionaries are asked with (the engine's `lookup_tl`):
    /// the row's TL, or a custom entry's own romanization.
    pub lookup_tl: String,
    pub hanji: Option<String>,
    pub sources: Vec<DictionarySource>,
}

impl DictionarySearchResult {
    /// A stable identity for the row: a replaced result list must not
    /// reuse the native node — or the menu — of a different word.
    pub fn key(&self) -> String {
        match &self.id {
            ResultId::System(id) => format!("system:{id}"),
            ResultId::Custom(id) => format!("custom:{id}"),
        }
    }

    pub fn moe_url(&self) -> Option<String> {
        moe_url(&self.lookup_tl)
    }

    pub fn chhoe_url(&self) -> Option<String> {
        chhoe_url(&self.lookup_tl)
    }

    /// The badges the row wears, one per distinct word in `sources` order:
    /// the three supplements share one (`DictionarySource::badge_key`).
    pub fn badge_keys(&self) -> Vec<StringKey> {
        let mut keys = Vec::new();
        for source in &self.sources {
            let key = source.badge_key();
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys
    }

    /// A row without an id is one this build cannot key — dropped, never
    /// shown with a made-up identity.
    fn from_wire(row: DictionarySearchRow) -> Option<Self> {
        let id = match row.id? {
            dictionary_search_row::Id::SystemId(id) => ResultId::System(id),
            dictionary_search_row::Id::CustomId(id) => ResultId::Custom(id),
        };
        Some(Self {
            id,
            roman: row.roman,
            lookup_tl: row.lookup_tl,
            hanji: row.hanji,
            sources: row
                .sources
                .into_iter()
                .filter_map(DictionarySource::from_code)
                .collect(),
        })
    }
}

/// Every hit for `query` under the settings in force: custom entries (never
/// for a Hanji query, none when Enable Custom Dictionary is off), then the
/// dictionary rows.
pub fn search(query: &str, settings: &SettingsDocument) -> Vec<DictionarySearchResult> {
    lookup(
        query,
        settings.choice(&keys::INPUT_MODE),
        &settings.dictionary_sources(),
        settings.bool(&keys::IS_CUSTOM_DICT_ENABLED),
        RESULT_LIMIT,
    )
}

/// [`search`] with every input spelled out, `limit` rows per section.
pub fn lookup(
    query: &str,
    mode: InputMode,
    toggles: &DictionarySourceToggles,
    custom_dictionary_enabled: bool,
    limit: u32,
) -> Vec<DictionarySearchResult> {
    dictionary_search(query, mode, toggles, custom_dictionary_enabled, limit)
        .into_iter()
        .filter_map(DictionarySearchResult::from_wire)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_rows_keep_the_engine_order_and_drop_unknown_codes() {
        // trace: codes 1 (kautian), 10 (khiin), 12 (dev), 11 (lkk) in the
        // engine's bit order; 0 (unspecified) and 999 (a newer engine's
        // source) are dropped by `from_code`.
        let row = DictionarySearchRow {
            id: Some(dictionary_search_row::Id::SystemId(7)),
            sources: vec![1, 10, 12, 11, 0, 999],
            ..DictionarySearchRow::default()
        };
        let result = DictionarySearchResult::from_wire(row).expect("keyed");
        assert_eq!(result.key(), "system:7");
        assert_eq!(
            result.sources,
            [
                DictionarySource::Kautian,
                DictionarySource::Khiin,
                DictionarySource::Dev,
                DictionarySource::Lkk
            ]
        );
        let custom = DictionarySearchRow {
            id: Some(dictionary_search_row::Id::CustomId("c1".to_owned())),
            ..DictionarySearchRow::default()
        };
        assert_eq!(
            DictionarySearchResult::from_wire(custom).map(|result| result.key()),
            Some("custom:c1".to_owned())
        );
        assert_eq!(
            DictionarySearchResult::from_wire(DictionarySearchRow::default()),
            None,
            "no id"
        );
    }

    #[test]
    fn badge_keys_dedup_by_word_in_source_order() {
        // trace: Khpoo / Khiin / Dev / Custom all badge as
        // DictionarySupplementSectionTitle (`DictionarySource::badge_key`, as
        // mobile's `tagKey`), so they read once, where the first stood.
        let result = DictionarySearchResult {
            id: ResultId::System(1),
            roman: "tsia̍h".to_owned(),
            lookup_tl: "tsia̍h".to_owned(),
            hanji: Some("食".to_owned()),
            sources: vec![
                DictionarySource::Khpoo,
                DictionarySource::Kautian,
                DictionarySource::Khiin,
                DictionarySource::Custom,
            ],
        };
        assert_eq!(
            result.badge_keys(),
            [
                StringKey::DictionarySupplementSectionTitle,
                StringKey::DictionaryKautianTag
            ]
        );
    }
}
