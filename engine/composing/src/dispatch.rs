//! Decode `ComposingRequest` → `Intent`, apply against `Engine`, encode the
//! `ComposingResponse`. The generation-mismatch reset path also lives here
//! (per plan §5b.2).

use crate::api::{ComposingError, Engine, Intent};
use protos::engine::{composing_request, AppConfig, ComposingRequest, ComposingResponse};

/// Decode the proto request into a typed `Intent`. Returns `MissingMethod`
/// when `oneof method` is empty.
pub(crate) fn decode_intent(req: &ComposingRequest) -> Result<Intent, ComposingError> {
    use composing_request::Method;
    let Some(method) = req.method.clone() else {
        return Err(ComposingError::MissingMethod);
    };
    Ok(match method {
        Method::Start(m) => Intent::Start { text: m.text },
        Method::Append(m) => Intent::Append { ch: m.char },
        Method::AppendHyphen(_) => Intent::AppendHyphen,
        Method::ReplaceLast(m) => Intent::ReplaceLast {
            replacement: m.replacement,
        },
        Method::DeleteBackward(_) => Intent::DeleteBackward,
        Method::CommitDerived(_) => Intent::CommitDerived,
        Method::CommitRaw(_) => Intent::CommitRaw,
        Method::SelectSuggestion(m) => Intent::SelectSuggestion { text: m.text },
        Method::CommitPreeditThenInsertExternal(m) => {
            Intent::CommitPreeditThenInsertExternal { text: m.text }
        }
        Method::Reset(_) => Intent::Reset,
        Method::SetSelectedCandidateIndex(m) => {
            Intent::SetSelectedCandidateIndex { index: m.index }
        }
        Method::QueryState(_) => Intent::QueryState,
    })
}

/// Pure dispatch entry: decode the proto request into an `Intent` and
/// apply it against `engine`. Generation-mismatch handling lives one
/// layer up in `EngineHandle::handle` (`handle.rs`); this fn is the
/// in-process Rust API also used directly by the workspace tests.
pub fn handle(
    req: &ComposingRequest,
    engine: &mut Engine,
    config: &AppConfig,
) -> Result<ComposingResponse, ComposingError> {
    let intent = decode_intent(req)?;
    if matches!(intent, Intent::QueryState) {
        return Ok(engine.snapshot(config));
    }
    Ok(engine.apply(intent, config))
}
