//! Public API for the bridge layer. Each method takes proto-shaped inputs
//! and returns proto-shaped outputs; the dispatch module wraps these into
//! envelope responses.

use phonetics::KeyFamily;
use protos::engine::{
    DictionaryFiltersRequest, DictionaryFiltersResponse, InstallRequest, InstallResponse,
    IsHanjiRequest, IsHanjiResponse, SearchByHanjiRequest, SearchByHanjiResponse,
    SearchWithSourcesRequest, SearchWithSourcesResponse, TaigiWord,
};

use crate::classification;
// The source filter alone, for an engine fetch that carries the toggles
// itself (`FetchAtPos.toggles`).
pub use crate::dictionary_filters::{association_bitmask, dictionary_filter_bitmask};
use crate::dictionary_filters::{compute_filters, source_codes};
use crate::error::LexiconError;
use crate::handle::EngineHandle;
use crate::paths::LexiconPaths;
use crate::search::{self, AssociationHit, SearchParams, SearchRow};

// Validates paths, opens FST/TKDB/TKWA (+ optional syllables.fst), atomically swaps the handle.
pub fn install(req: InstallRequest) -> Result<InstallResponse, LexiconError> {
    let paths = LexiconPaths::validated(
        &req.trie_path,
        &req.dictionary_bin_path,
        &req.association_bin_path,
        &req.syllable_inventory_path,
        req.dictionary_version,
    )?;
    let stats = EngineHandle::install(paths)?;
    Ok(InstallResponse {
        dictionary_record_count: stats.dictionary_record_count,
        prefix_index_entry_count: stats.prefix_index_entry_count,
    })
}

// Tab3 romanization lookup, filtered by the enabled-source bitmask.
pub fn search_with_sources(
    req: SearchWithSourcesRequest,
) -> Result<SearchWithSourcesResponse, LexiconError> {
    let params = SearchParams {
        input: req.input,
        family: proto_key_family(req.input_mode),
        limit: req.limit,
        enabled_sources_bitmask: req.enabled_sources_bitmask,
    };
    EngineHandle::with_state(|state| {
        let prefix_index = state
            .prefix_index
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("prefix_index unavailable".into()))?;
        let dict = state
            .dictionary
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("dictionary reader unavailable".into()))?;
        let rows = search::search(&params, prefix_index, dict)?;
        Ok(SearchWithSourcesResponse {
            rows: rows.into_iter().map(search_row_to_taigi_word).collect(),
        })
    })
}

// Tab3 Hanji lookup: scans the index under the `hanzi:` prefix, filtered by the source bitmask.
pub fn search_by_hanji(req: SearchByHanjiRequest) -> Result<SearchByHanjiResponse, LexiconError> {
    EngineHandle::with_state(|state| {
        let prefix_index = state
            .prefix_index
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("prefix_index unavailable".into()))?;
        let dict = state
            .dictionary
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("dictionary reader unavailable".into()))?;
        let rows = search::search_by_hanji(
            &req.query,
            req.limit,
            req.enabled_sources_bitmask,
            prefix_index,
            dict,
        )?;
        Ok(SearchByHanjiResponse {
            rows: rows.into_iter().map(search_row_to_taigi_word).collect(),
        })
    })
}

// NextWord bigram lookup for the committed word (word key, character-key backoff), source-filtered.
// Not a wire method: only engine/dispatch calls it (nextword `PredictNext`, the fetch context).
// `previous_tl` empty → character key only; `enabled_sources_bitmask` `u32::MAX` → no filter.
pub fn lookup_associations(
    previous_word: &str,
    previous_tl: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
) -> Result<Vec<AssociationHit>, LexiconError> {
    EngineHandle::with_state(|state| {
        let reader = state
            .association
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("association reader unavailable".into()))?;
        search::lookup_associations(
            previous_word,
            previous_tl,
            limit,
            enabled_sources_bitmask,
            reader,
        )
    })
}

// True when the text contains any CJK ideograph in `classification::CJK_RANGES`.
pub fn is_hanji(req: IsHanjiRequest) -> Result<IsHanjiResponse, LexiconError> {
    Ok(IsHanjiResponse {
        is_hanji: classification::is_hanji(&req.text),
    })
}

// Maps the user's dictionary toggles to a source bitmask plus the enabled source-code list.
pub fn dictionary_filters(
    req: DictionaryFiltersRequest,
) -> Result<DictionaryFiltersResponse, LexiconError> {
    let toggles = req.toggles.unwrap_or_default();
    Ok(compute_filters(&toggles))
}

fn proto_key_family(value: i32) -> KeyFamily {
    use protos::engine::InputMode;
    match InputMode::try_from(value).unwrap_or(InputMode::Unspecified) {
        InputMode::Poj => KeyFamily::Poj,
        InputMode::Tps => KeyFamily::Tps,
        // Unspecified + Tl default to TL.
        _ => KeyFamily::Tl,
    }
}

fn search_row_to_taigi_word(row: SearchRow) -> TaigiWord {
    TaigiWord {
        id: row.id,
        roman: row.roman,
        hanji: row.hanji,
        sources: source_codes(row.source_bitmask.unwrap_or(0))
            .into_iter()
            .map(|code| code as i32)
            .collect(),
    }
}
