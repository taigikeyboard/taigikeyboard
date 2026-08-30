//! The 辭典搜尋 page's lookup: the custom dictionary first, then the
//! engine's dictionaries under the user's source toggles, in the page's
//! order. Port of `DictionarySearchService.swift`.

// 中文: 辭典搜尋 — 先自訂詞庫,再引擎辭典(依來源開關),照頁面順序排。

use std::sync::Arc;
use taigi_windows_core::engine::{
    chhoe_url, derive_custom_query_key, dictionary_filters, is_hanzi, moe_url, search_by_hanzi,
    search_with_sources, tl_to_poj, DictionarySource, LexiconRow,
    ALL_SOURCES_ENABLED_SEARCH_BITMASK,
};
use taigi_windows_core::settings::{keys, InputMode, SettingsDocument};
use taigi_windows_storage::CustomDictionaryStore;

/// `DictionarySearchService.resultLimit`.
pub const RESULT_LIMIT: u32 = 20;

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
    pub hanzi: Option<String>,
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
pub fn search(
    query: &str,
    settings: &SettingsDocument,
    custom_dictionary: &Arc<CustomDictionaryStore>,
) -> Vec<DictionarySearchResult> {
    if query.is_empty() {
        return Vec::new();
    }
    let mode: InputMode = settings.choice(&keys::INPUT_MODE);
    let toggles = settings.dictionary_sources();
    let filters = dictionary_filters(&toggles);
    let is_hanzi_query = is_hanzi(query);
    let wire_mask = filters
        .as_ref()
        .map_or(ALL_SOURCES_ENABLED_SEARCH_BITMASK, |filters| {
            filters.wire_mask()
        });
    let rows = if is_hanzi_query {
        search_by_hanzi(query, mode, RESULT_LIMIT, wire_mask)
    } else {
        search_with_sources(query, mode, RESULT_LIMIT, wire_mask)
    };
    let system: Vec<DictionarySearchResult> = LexiconRow::sorted_for_search(rows)
        .into_iter()
        .map(|row| {
            let mut sources = DictionarySource::from_bitmask(row.source_bitmask.unwrap_or(0));
            // Badges only for sources the user has on.
            if let Some(filters) = &filters {
                sources.retain(|source| filters.enabled_sources.contains(source));
            }
            DictionarySearchResult {
                id: ResultId::System(row.id),
                roman: display_roman(&row.roman, mode),
                lookup_tl: Some(row.roman),
                hanzi: row.hanzi,
                sources,
            }
        })
        .collect();
    let mut results = if is_hanzi_query {
        Vec::new()
    } else {
        custom_results(query, settings, mode, custom_dictionary)
    };
    results.extend(system);
    results
}

fn display_roman(tl: &str, mode: InputMode) -> String {
    match mode {
        InputMode::Poj => tl_to_poj(tl).unwrap_or_else(|| tl.to_owned()),
        InputMode::Tl => tl.to_owned(),
    }
}

fn custom_results(
    query: &str,
    settings: &SettingsDocument,
    mode: InputMode,
    custom_dictionary: &Arc<CustomDictionaryStore>,
) -> Vec<DictionarySearchResult> {
    if !settings.bool(&keys::IS_CUSTOM_DICT_ENABLED) {
        return Vec::new();
    }
    let Some(query_key) = derive_custom_query_key(query, mode) else {
        return Vec::new();
    };
    custom_dictionary
        .rows_matching(&query_key, RESULT_LIMIT as usize)
        .into_iter()
        .map(|row| DictionarySearchResult {
            id: ResultId::Custom(row.id),
            roman: row.roman,
            lookup_tl: None,
            hanzi: (!row.hanzi.is_empty()).then_some(row.hanzi),
            sources: vec![DictionarySource::Custom],
        })
        .collect()
}
