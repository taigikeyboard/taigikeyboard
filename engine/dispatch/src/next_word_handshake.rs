//! Applies a composing response's next-word handshakes in the same call
//! (roadmap E2: one FFI hop per commit).
//!
//! The composing engine emits `NextWord*` effects and keeps no clock; the
//! shell sends its next-word envelope on the request
//! (`ComposingRequest.next_word_context`). Each handshake becomes the
//! `NextWordRequest` the shells used to send, and its `DecideResult` takes the
//! handshake's place in the effect list, so the executor runs it in the same
//! order as before (`nextword-engine-boundary.md` §2). Without the envelope
//! the handshakes are dropped (`drop_unapplied`).

use protos::engine::{
    effect, next_word_request, next_word_response, AppConfig, ClearForNewComposing,
    ComposingResponse, DecideResult, DecisionInput, Effect, NextWordContext, NextWordRequest,
    UpdateLastSelectedWord, WordSelected,
};

/// Replaces each handshake in `response` with the decision the next-word
/// engine answered. A handshake the engine refused is dropped and logged — a
/// shell whose own next-word call failed did nothing either.
pub(crate) fn apply(
    response: &mut ComposingResponse,
    context: &NextWordContext,
    config: &AppConfig,
) {
    // Most answers (every keystroke) carry none: leave the list untouched.
    if !response.effect.iter().any(is_handshake) {
        return;
    }
    let effects = std::mem::take(&mut response.effect);
    response.effect = effects
        .into_iter()
        .filter_map(|effect| match next_word_request(effect, context.now_ms) {
            Ok(request) => decide(request, config, context.generation).map(|decided| Effect {
                kind: Some(effect::Kind::NextWordDecided(decided)),
            }),
            Err(other) => Some(other),
        })
        .collect();
}

/// Drops the handshakes of an answer whose request carried no next-word
/// envelope: there is no generation or clock to apply them under, and a
/// handshake never reaches a shell. The commit then taught nothing.
pub(crate) fn drop_unapplied(response: &mut ComposingResponse) {
    if !response.effect.iter().any(is_handshake) {
        return;
    }
    log::warn!(
        "composing request without a next-word context; its next-word handshakes are dropped"
    );
    response.effect.retain(|effect| !is_handshake(effect));
    if let Some(commit) = response.commit.as_mut() {
        commit.taught_next_word = false;
    }
}

fn is_handshake(effect: &Effect) -> bool {
    matches!(
        effect.kind,
        Some(
            effect::Kind::NextWordWordSelected(_)
                | effect::Kind::NextWordUpdateLastSelectedWord(_)
                | effect::Kind::NextWordClearForNewComposing(_)
        )
    )
}

/// The next-word request a handshake stands for; every other effect back as
/// `Err`, untouched.
fn next_word_request(effect: Effect, now_ms: i64) -> Result<NextWordRequest, Effect> {
    let input = Some(DecisionInput { now_ms });
    let method = match effect.kind {
        Some(effect::Kind::NextWordWordSelected(selected)) => {
            next_word_request::Method::WordSelected(WordSelected {
                text: selected.text,
                roman: selected.roman,
                // Raw-romanization Enter paths are platform keys, never a
                // composing handshake.
                require_roman_mode: false,
                trigger_prediction: selected.trigger_prediction,
                input,
                preceding: selected.preceding,
            })
        }
        Some(effect::Kind::NextWordUpdateLastSelectedWord(nailed)) => {
            next_word_request::Method::UpdateLastSelectedWord(UpdateLastSelectedWord {
                text: nailed.text,
                roman: nailed.roman,
                input,
            })
        }
        Some(effect::Kind::NextWordClearForNewComposing(_)) => {
            next_word_request::Method::ClearForNewComposing(ClearForNewComposing { input })
        }
        kind => return Err(Effect { kind }),
    };
    Ok(NextWordRequest {
        method: Some(method),
    })
}

fn decide(request: NextWordRequest, config: &AppConfig, generation: u64) -> Option<DecideResult> {
    // The composing engine has already moved on (a commit is written), so a
    // next-word panic must cost the learning only, never the composing answer
    // — as when the shell sent this request on its own.
    let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::user_data::handle_nextword(request, config, generation)
    }));
    let Ok(handled) = handled else {
        log::error!("next-word handshake panicked; the composing answer stands");
        return None;
    };
    match handled {
        Ok(response) => match response.result {
            Some(next_word_response::Result::Decide(decided)) => Some(decided),
            _ => {
                log::warn!("next-word handshake answered without a decision");
                None
            }
        },
        Err(err) => {
            log::warn!("next-word handshake failed: {err}");
            None
        }
    }
}
