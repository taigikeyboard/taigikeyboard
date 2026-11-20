//! Public API for the bridge layer. Each method takes proto-shaped inputs
//! and returns proto-shaped outputs; the dispatch module wraps these into
//! envelope responses.

use phonetics::{InputMode, KeyFamily};
use protos::engine::{
    dictionary_search_row, CustomDictionaryEntry, DictionarySearchRequest,
    DictionarySearchResponse, DictionarySearchRow, DictionarySourceCode, InstallRequest,
    InstallResponse,
};

use crate::classification;
use crate::dictionary_filters::source_codes;
// The source filter alone, for an engine composing request that carries the
// toggles in its config (`AppConfig.dictionary_sources`).
pub use crate::dictionary_filters::{association_bitmask, dictionary_filter_bitmask};
use crate::dictionary_reader::DictionaryReader;
use crate::error::LexiconError;
use crate::handle::EngineHandle;
use crate::paths::LexiconPaths;
use crate::prefix_index::PrefixIndex;
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

// The Manage Dictionaries search. A Hanji query takes the Hanji lookup,
// anything else the romanization lookup in the input mode's family, and
// also lists the custom entries `custom_entries` finds for it (asked only
// when Enable Custom Dictionary is on — engine/dispatch answers from the
// user data, an answer without it passes `|_| Vec::new()`). Rows: the
// custom entries, then the dictionary rows, each romanization rendered
// (POJ under POJ, TL otherwise — TPS shows TL, as every platform did) and
// badged with the sources the user has on. Before the dictionary is
// installed there are no dictionary rows; the custom entries still show.
pub fn dictionary_search(
    req: DictionarySearchRequest,
    custom_entries: impl FnOnce(&DictionarySearchRequest) -> Vec<CustomDictionaryEntry>,
) -> Result<DictionarySearchResponse, LexiconError> {
    if req.query.is_empty() {
        return Ok(DictionarySearchResponse::default());
    }
    let enabled_sources_bitmask = dictionary_filter_bitmask(&req.toggles.unwrap_or_default());
    let mode = phonetics::api::parse_input_mode(&req.input_mode);
    let is_hanji_query = classification::is_hanji(&req.query);
    let found = with_search_readers(|prefix_index, dict| {
        if is_hanji_query {
            search::search_by_hanji(
                &req.query,
                req.limit,
                enabled_sources_bitmask,
                prefix_index,
                dict,
            )
        } else {
            let params = SearchParams {
                input: req.query.clone(),
                family: KeyFamily::for_input_mode(mode),
                limit: req.limit,
                enabled_sources_bitmask,
            };
            search::search(&params, prefix_index, dict)
        }
    });
    let system_rows = match found {
        Ok(rows) => rows,
        Err(LexiconError::NotInitialized) => Vec::new(),
        Err(err) => return Err(err),
    };
    // Asked after the lexicon lock is released: the user-data read may wait
    // for a background open.
    let custom_rows = if req.custom_dictionary_enabled && !is_hanji_query {
        custom_entries(&req)
    } else {
        Vec::new()
    };
    Ok(DictionarySearchResponse {
        rows: custom_rows
            .into_iter()
            .map(custom_search_row)
            .chain(
                system_rows
                    .into_iter()
                    .map(|row| system_search_row(row, mode, enabled_sources_bitmask)),
            )
            .collect(),
    })
}

fn system_search_row(
    row: SearchRow,
    mode: InputMode,
    enabled_sources_bitmask: u32,
) -> DictionarySearchRow {
    let lookup_tl = row.roman;
    let roman = if mode == InputMode::Poj {
        phonetics::api::tl_display_to_poj_display(&lookup_tl)
    } else {
        lookup_tl.clone()
    };
    DictionarySearchRow {
        id: Some(dictionary_search_row::Id::SystemId(row.id)),
        roman,
        lookup_tl,
        hanji: row.hanji,
        // `source_codes` reads the source bits only, so the filter's variant
        // and kautian-subcollection bits drop out.
        sources: source_codes(row.source_bitmask.unwrap_or(0) & enabled_sources_bitmask)
            .into_iter()
            .map(|code| code as i32)
            .collect(),
    }
}

// A custom entry as shown: its romanization as saved, asked of the web
// dictionaries as is; an empty stored Hanji is a romanization-only entry.
fn custom_search_row(entry: CustomDictionaryEntry) -> DictionarySearchRow {
    DictionarySearchRow {
        id: Some(dictionary_search_row::Id::CustomId(entry.id)),
        lookup_tl: entry.roman.clone(),
        roman: entry.roman,
        hanji: (!entry.hanji.is_empty()).then_some(entry.hanji),
        sources: vec![DictionarySourceCode::DictSourceCustom as i32],
    }
}

// The installed index and dictionary reader, for a search.
fn with_search_readers<R>(
    read: impl FnOnce(&PrefixIndex, &DictionaryReader) -> Result<R, LexiconError>,
) -> Result<R, LexiconError> {
    EngineHandle::with_state(|state| {
        let prefix_index = state
            .prefix_index
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("prefix_index unavailable".into()))?;
        let dict = state
            .dictionary
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("dictionary reader unavailable".into()))?;
        read(prefix_index, dict)
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
