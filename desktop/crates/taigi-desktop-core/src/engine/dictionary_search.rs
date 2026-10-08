//! The Dictionary Search page's lookup: the custom dictionary first, then the
//! engine's dictionaries under the user's source toggles, in the page's
//! order. One lookup for the Windows and Linux settings windows. Twin of
//! iOS `DictionarySearchService.swift`.

use super::user_data::search_custom_entries;
use super::{
    chhoe_url, dictionary_filters, is_hanji, moe_url, search_by_hanji, search_with_sources,
    tl_to_poj, DictionarySource, ALL_SOURCES_ENABLED_SEARCH_BITMASK,
};
use crate::settings::{keys, InputMode, SettingsDocument};

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
    /// The TL the web dictionaries are asked with; `None` for a custom
    /// entry (no lookup menu).
    pub lookup_tl: Option<String>,
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
        self.lookup_tl.as_deref().and_then(moe_url)
    }

    pub fn chhoe_url(&self) -> Option<String> {
        self.lookup_tl.as_deref().and_then(chhoe_url)
    }
}

/// Every hit for `query`: custom entries (never for a hanji query), then
/// the system rows sorted the page's way.
pub fn search(query: &str, settings: &SettingsDocument) -> Vec<DictionarySearchResult> {
    if query.is_empty() {
        return Vec::new();
    }
    let mode: InputMode = settings.choice(&keys::INPUT_MODE);
    let toggles = settings.dictionary_sources();
    let filters = dictionary_filters(&toggles);
    let is_hanji_query = is_hanji(query);
    let sources_bitmask = filters
        .as_ref()
        .map_or(ALL_SOURCES_ENABLED_SEARCH_BITMASK, |filters| {
            filters.dictionary_filter_bitmask
        });
    let rows = if is_hanji_query {
        search_by_hanji(query, mode, RESULT_LIMIT, sources_bitmask)
    } else {
        search_with_sources(query, mode, RESULT_LIMIT, sources_bitmask)
    };
    // The engine owns the order (corpus order, MOE rows first — `lexicon::search`).
    let system: Vec<DictionarySearchResult> = rows
        .into_iter()
        .map(|row| {
            let mut sources = row.sources;
            // Badges only for sources the user has on.
            if let Some(filters) = &filters {
                sources.retain(|source| filters.enabled_sources.contains(source));
            }
            DictionarySearchResult {
                id: ResultId::System(row.id),
                roman: display_roman(&row.roman, mode),
                lookup_tl: Some(row.roman),
                hanji: row.hanji,
                sources,
            }
        })
        .collect();
    let mut results = if is_hanji_query {
        Vec::new()
    } else {
        custom_results(query, settings, mode)
    };
    results.extend(system);
    results
}

fn display_roman(tl: &str, mode: InputMode) -> String {
    match mode {
        InputMode::Poj => tl_to_poj(tl).unwrap_or_else(|| tl.to_owned()),
        // TL under TPS, as iOS shows (`DictionarySearchService.swift`); the
        // search itself still reads the `tps:` family (`wire_input_mode`).
        InputMode::Tl | InputMode::Tps => tl.to_owned(),
    }
}

/// The custom entries the query finds the way the keyboard does (the
/// engine derives the key and prefix-matches it); none when the custom
/// dictionary is off or the engine could not answer.
fn custom_results(
    query: &str,
    settings: &SettingsDocument,
    mode: InputMode,
) -> Vec<DictionarySearchResult> {
    if !settings.bool(&keys::IS_CUSTOM_DICT_ENABLED) {
        return Vec::new();
    }
    search_custom_entries(query, mode, RESULT_LIMIT as usize)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| DictionarySearchResult {
            id: ResultId::Custom(entry.id),
            roman: entry.roman,
            lookup_tl: None,
            hanji: (!entry.hanji.is_empty()).then_some(entry.hanji),
            sources: vec![DictionarySource::Custom],
        })
        .collect()
}
