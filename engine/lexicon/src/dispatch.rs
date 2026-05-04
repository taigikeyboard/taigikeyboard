//! Dispatch: route `LexiconRequest.method` oneof variants 11-18 to the
//! per-method API. Tag 10 (process_candidates) stays routed to `ranking`
//! by `engine/dispatch::lib.rs`; this crate owns the read-path variants
//! (Install/Search/SearchWithSources/SearchByHanzi/AssocLookup), the
//! classification variants (ClassifyInput/IsHanzi), and the v3.5.8
//! DictionaryFilters variant.

use protos::engine::lexicon_request::Method;
use protos::engine::lexicon_response::Result as LexResult;
use protos::engine::{
    AssocLookupRequest, ClassifyInputRequest, DictionaryFiltersRequest, InstallRequest,
    IsHanziRequest, LexiconResponse, SearchByHanziRequest, SearchRequest, SearchWithSourcesRequest,
};

use crate::api;
use crate::error::LexiconError;

pub fn handle_install(req: InstallRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::install(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::InstallResult(resp)),
    })
}

pub fn handle_search(req: SearchRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::search(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::SearchResult(resp)),
    })
}

pub fn handle_search_with_sources(
    req: SearchWithSourcesRequest,
) -> Result<LexiconResponse, LexiconError> {
    let resp = api::search_with_sources(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::SearchWithSourcesResult(resp)),
    })
}

pub fn handle_search_by_hanzi(req: SearchByHanziRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::search_by_hanzi(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::SearchByHanziResult(resp)),
    })
}

pub fn handle_assoc_lookup(req: AssocLookupRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::assoc_lookup(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::AssocLookupResult(resp)),
    })
}

pub fn handle_classify_input(req: ClassifyInputRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::classify_input(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::ClassifyInputResult(resp)),
    })
}

pub fn handle_is_hanzi(req: IsHanziRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::is_hanzi(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::IsHanziResult(resp)),
    })
}

pub fn handle_dictionary_filters(
    req: DictionaryFiltersRequest,
) -> Result<LexiconResponse, LexiconError> {
    let resp = api::dictionary_filters(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::DictionaryFiltersResult(resp)),
    })
}

/// Convenience: dispatch `LexiconRequest.method` directly to the matching
/// handler. Returns `None` for tag 10 (process_candidates) — callers must
/// route that through the `ranking` crate per plan §6.
pub fn handle(method: Method) -> Result<LexiconResponse, LexiconError> {
    match method {
        Method::ProcessCandidates(_) => Err(LexiconError::Internal(
            "process_candidates routes to ranking crate; not handled here".into(),
        )),
        Method::Install(req) => handle_install(req),
        Method::Search(req) => handle_search(req),
        Method::SearchWithSources(req) => handle_search_with_sources(req),
        Method::SearchByHanzi(req) => handle_search_by_hanzi(req),
        Method::AssocLookup(req) => handle_assoc_lookup(req),
        Method::ClassifyInput(req) => handle_classify_input(req),
        Method::IsHanzi(req) => handle_is_hanzi(req),
        Method::DictionaryFilters(req) => handle_dictionary_filters(req),
    }
}
