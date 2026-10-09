//! The user-data request, and the composing / next-word requests the
//! engine's own user data joins. Built into every adapter: with the
//! `user-data` feature and the stores open, `with_stores` answers; before
//! the open, or in a build without the feature, a composing request ranks by
//! the bundled context only, a prediction has no user rows, and nothing
//! taught or recorded is kept — nowhere to keep it. Plan:
//! `docs/architecture/user-data-engine-roadmap.md`.

#[cfg(feature = "user-data")]
mod with_stores;

use composing::api::ComposingError;
use nextword::NextWordError;
use protos::engine::{
    AppConfig, ComposingRequest, ComposingResponse, NextWordRequest, NextWordResponse,
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

/// A composing request — the `Composing` arm of `crate::run`.
pub(crate) fn handle_composing(
    request: &ComposingRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<ComposingResponse, ComposingError> {
    #[cfg(feature = "user-data")]
    if let Some(stores) = userdata::UserDataHandle::instance().stores() {
        return with_stores::handle_composing(stores, request, config, generation);
    }
    crate::context::handle_composing_without_stores(request, config, generation)
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
