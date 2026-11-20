//! The user-data request, and the composing / next-word / dictionary-search
//! requests the engine's own user data joins. Built into every adapter: with
//! the `user-data` feature and the stores open, `with_stores` answers; before
//! the open, or in a build without the feature, a fetch ranks by the bundled
//! context only (`fetch`), a prediction has no user rows, a dictionary search
//! has no custom entries, and nothing taught or recorded is kept — nowhere to
//! keep it. Plan:
//! `docs/architecture/user-data-engine-roadmap.md`.

#[cfg(feature = "user-data")]
mod with_stores;

use composing::api::ComposingError;
use lexicon::LexiconError;
use nextword::NextWordError;
use protos::engine::{
    composing_request, lexicon_request, AppConfig, ComposingRequest, ComposingResponse,
    LexiconResponse, NextWordRequest, NextWordResponse,
};

#[cfg(feature = "user-data")]
pub(crate) use with_stores::respond;

/// Answers one user-data request — the `UserData` arm of `crate::run`: a
/// build without the stores refuses it (user-data-engine-roadmap U11).
#[cfg(not(feature = "user-data"))]
pub(crate) fn respond(
    id: u32,
    _request: &protos::engine::UserDataRequest,
) -> protos::engine::Response {
    log::warn!("user-data request on a build without the user-data feature (id={id})");
    crate::error_response(id, protos::engine::ErrorCode::FailInvariant)
}

/// The open composition's cut and candidate order, or `None` when nothing
/// is being composed — ranked as a fetch ranks it now.
pub(crate) fn explain_composition(
    config: &AppConfig,
    generation: u64,
    now_ms: i64,
    literal_roman_candidate_disabled: bool,
) -> Option<composing::explain::Mechanism> {
    let sent = protos::engine::FetchAtPos {
        now_ms,
        literal_roman_candidate_disabled,
    };
    #[cfg(feature = "user-data")]
    if let Some(stores) = userdata::UserDataHandle::instance().stores() {
        let rows = with_stores::StoredRows { stores, config };
        return crate::fetch::explain(Some(&rows), &sent, config, generation);
    }
    crate::fetch::explain(None, &sent, config, generation)
}

/// A composing request — the `Composing` arm of `crate::run`. A
/// `FetchAtPos` takes the one fetch path, with or without the stores; every
/// other request goes to composing — with the stores, ranked by and
/// teaching the user's data. The answer's next-word handshakes never leave
/// here: applied with the shell's next-word envelope on the request
/// (`next_word_handshake`, after the composing engine's lock is released),
/// dropped without it.
pub(crate) fn handle_composing(
    request: &ComposingRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<ComposingResponse, ComposingError> {
    let mut response = composing_response(request, config, generation)?;
    match request.next_word_context.as_ref() {
        Some(context) => crate::next_word_handshake::apply(&mut response, context, config),
        None => crate::next_word_handshake::drop_unapplied(&mut response),
    }
    Ok(response)
}

fn composing_response(
    request: &ComposingRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<ComposingResponse, ComposingError> {
    #[cfg(feature = "user-data")]
    if let Some(stores) = userdata::UserDataHandle::instance().stores() {
        return with_stores::handle_composing(stores, request, config, generation);
    }
    match request.method.as_ref() {
        Some(composing_request::Method::FetchAtPos(sent)) => {
            Ok(crate::fetch::fetch_at_pos(None, sent, config, generation))
        }
        _ => composing::EngineHandle::instance().handle(request, config, generation),
    }
}

/// A next-word request — the `Nextword` arm of `crate::run`.
pub(crate) fn handle_nextword(
    request: NextWordRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<NextWordResponse, NextWordError> {
    #[cfg(feature = "user-data")]
    if let Some(stores) = userdata::UserDataHandle::instance().stores() {
        return with_stores::handle_nextword(stores, request, config, generation);
    }
    nextword::EngineHandle::instance().handle(
        &crate::predict::expand_predict_next(request, Vec::new()),
        config,
        generation,
    )
}

/// A lexicon request — the `Lexicon` arm of `crate::run`. With the user
/// data built in, a `DictionarySearch` lists the user's custom entries too.
pub(crate) fn handle_lexicon(
    method: lexicon_request::Method,
) -> Result<LexiconResponse, LexiconError> {
    match method {
        #[cfg(feature = "user-data")]
        lexicon_request::Method::DictionarySearch(search) => {
            lexicon::requests::handle_dictionary_search(
                search,
                with_stores::custom_entries_matching,
            )
        }
        method => lexicon::requests::handle(method),
    }
}
