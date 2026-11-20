//! Dispatch: route every `LexiconRequest.method` oneof variant to the
//! per-method API — Install and DictionarySearch (its custom entries
//! answered by the caller).

use protos::engine::lexicon_request::Method;
use protos::engine::lexicon_response::Result as LexResult;
use protos::engine::{
    CustomDictionaryEntry, DictionarySearchRequest, InstallRequest, LexiconResponse,
};

use crate::api;
use crate::error::LexiconError;

pub fn handle_install(req: InstallRequest) -> Result<LexiconResponse, LexiconError> {
    let resp = api::install(req)?;
    Ok(LexiconResponse {
        result: Some(LexResult::InstallResult(resp)),
    })
}

/// `custom_entries` answers the custom-entry lookup (`api::dictionary_search`).
pub fn handle_dictionary_search(
    req: DictionarySearchRequest,
    custom_entries: impl FnOnce(&DictionarySearchRequest) -> Vec<CustomDictionaryEntry>,
) -> Result<LexiconResponse, LexiconError> {
    let resp = api::dictionary_search(req, custom_entries)?;
    Ok(LexiconResponse {
        result: Some(LexResult::DictionarySearchResult(resp)),
    })
}

/// Dispatch `LexiconRequest.method` to the matching handler.
pub fn handle(method: Method) -> Result<LexiconResponse, LexiconError> {
    match method {
        Method::Install(req) => handle_install(req),
        // Without user data: no custom entries (engine/dispatch answers this
        // method itself when the user data is built in).
        Method::DictionarySearch(req) => handle_dictionary_search(req, |_| Vec::new()),
    }
}
