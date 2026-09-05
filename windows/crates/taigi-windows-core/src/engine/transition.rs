//! Value types for one composing round-trip: the engine snapshot, the ordered
//! effects it wants the platform to run, and the continuous candidates.
//! Port of `macos/Sources/TaigiInputMethodCore/Engine/ComposingTransition.swift`.

use protos::engine::{effect, CandidateMessage, ComposingResponse, Effect as WireEffect};

/// The complete effect vocabulary of `engine/protos/proto/composing.proto`
/// `Effect`. All ten kinds are decoded even though the desktop acts on only
/// some: an effect dropped at decode time is invisible when the slice that
/// needs it lands, whereas an explicitly ignored variant shows up in every
/// `match` the compiler checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    UpdatePreedit(String),
    ClearPreeditWithoutCommit,
    CommitTextReplacingPreedit(String),
    /// Emitted only by the `Phase::Composing` backspace-to-empty branch
    /// (`engine/composing/src/transition.rs:266-276`). Ignored on desktop:
    /// the preedit only ever lived in the composition, so deleting a document
    /// character would eat a real host character (named divergence, macOS
    /// `ClientEffectExecutor.swift:56-63`).
    DeleteBackwardFromDocument,
    ResetAutocomplete,
    PerformAutocomplete,
    ResetAutocompleteContext,
    /// Continuous-input mid-commit handshake for the next-word learner.
    NextWordUpdateLastSelectedWord {
        text: String,
        roman: String,
    },
    /// Continuous-input final-commit handshake. `trigger_prediction` is the
    /// engine's decision — forwarded, never hardcoded.
    NextWordWordSelected {
        text: String,
        roman: String,
        trigger_prediction: bool,
    },
    /// Continuous-input abort handshake. Distinct from a full next-word reset.
    NextWordClearForNewComposing,
}

/// One engine response, decoded. `effects` is the whole reason this type
/// exists: the engine decides WHAT happens to the host document and in what
/// order; the platform only decides HOW to perform each step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposingTransition {
    /// The keystrokes as typed, with numeric tones (the engine's search key).
    pub raw_input: String,
    /// The rendered composition, with tone diacritics (what the user reads).
    pub display_text: String,
    pub effects: Vec<Effect>,
    pub selected_candidate_index: i32,
    pub is_composing: bool,
}

impl ComposingTransition {
    pub(super) fn decode(response: &ComposingResponse) -> Self {
        let preedit = response.preedit.clone().unwrap_or_default();
        Self {
            raw_input: preedit.raw_input,
            display_text: preedit.display_text,
            effects: response.effect.iter().filter_map(Effect::decode).collect(),
            selected_candidate_index: response.selected_candidate_index,
            is_composing: response.is_composing,
        }
    }
}

impl Effect {
    /// `None` only for an effect whose `kind` the wire left unset, which the
    /// current engine never emits. Every kind it does emit has an arm here —
    /// the exhaustive `match` makes a newly added engine effect a compile
    /// error rather than a silently dropped instruction.
    pub(super) fn decode(effect: &WireEffect) -> Option<Self> {
        Some(match effect.kind.as_ref()? {
            effect::Kind::UpdatePreedit(payload) => Effect::UpdatePreedit(payload.display.clone()),
            effect::Kind::ClearPreeditWithoutCommit(_) => Effect::ClearPreeditWithoutCommit,
            effect::Kind::CommitTextReplacingPreedit(payload) => {
                Effect::CommitTextReplacingPreedit(payload.text.clone())
            }
            effect::Kind::DeleteBackwardFromDocument(_) => Effect::DeleteBackwardFromDocument,
            effect::Kind::ResetAutocomplete(_) => Effect::ResetAutocomplete,
            effect::Kind::PerformAutocomplete(_) => Effect::PerformAutocomplete,
            effect::Kind::ResetAutocompleteContext(_) => Effect::ResetAutocompleteContext,
            effect::Kind::NextWordUpdateLastSelectedWord(payload) => {
                Effect::NextWordUpdateLastSelectedWord {
                    text: payload.text.clone(),
                    roman: payload.roman.clone(),
                }
            }
            effect::Kind::NextWordWordSelected(payload) => Effect::NextWordWordSelected {
                text: payload.text.clone(),
                roman: payload.roman.clone(),
                trigger_prediction: payload.trigger_prediction,
            },
            effect::Kind::NextWordClearForNewComposing(_) => Effect::NextWordClearForNewComposing,
        })
    }
}

/// MOE-aligned candidate-type discriminator, derived in Rust
/// (`engine/lexicon/src/continuous.rs::derive_mode`). Read, never recomputed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateMode {
    /// The wire carried no mode, or one this build does not know. Means
    /// "ignore mode", never "guess the mode locally".
    Unspecified,
    Hant,
    Tailo,
    Mixed,
}

impl CandidateMode {
    pub fn decode(wire: i32) -> Self {
        match wire {
            1 => Self::Hant,
            2 => Self::Tailo,
            3 => Self::Mixed,
            _ => Self::Unspecified,
        }
    }
}

/// One span-local continuous-input candidate.
///
/// The span offsets are byte offsets into the raw buffer the engine holds,
/// and `canonical_tl` is the identity romanization (Core Principle #7 keys a
/// word on the `(漢字, canonical TL)` pair). Both must be round-tripped back
/// to the engine verbatim on commit.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousCandidate {
    pub consumed_span_start: u32,
    pub consumed_span_end: u32,
    pub syllable_count: u32,
    pub display_text: String,
    pub score: f32,
    pub form: u32,
    pub mode: CandidateMode,
    /// Display romanization for the candidate cell — POJ-rendered in POJ
    /// mode. Not an identity key; that is `canonical_tl`.
    pub roman: String,
    /// `None` when the wire omitted it, which marks a romanization-only
    /// candidate. Distinct from an empty string.
    pub hanji: Option<String>,
    /// Canonical TL, snapshotted before any display recasing. Empty only when
    /// no canonical TL is recoverable.
    pub canonical_tl: String,
}

impl ContinuousCandidate {
    pub(super) fn decode(message: &CandidateMessage) -> Self {
        Self {
            consumed_span_start: message.consumed_span_start,
            consumed_span_end: message.consumed_span_end,
            syllable_count: message.syllable_count,
            display_text: message.display_text.clone(),
            score: message.score,
            form: message.form,
            mode: CandidateMode::decode(message.mode),
            roman: message.roman.clone(),
            hanji: message.hanji.clone(),
            canonical_tl: message.canonical_tl.clone(),
        }
    }

    /// `display_text` is `hanji.unwrap_or(roman)` by engine contract; a
    /// candidate with no hanji is romanization-only.
    pub fn is_roman_only(&self) -> bool {
        self.hanji.is_none()
    }

    /// The hanji with a producer's empty string treated as absent — a
    /// romanization-only candidate either way.
    pub fn nonempty_hanji(&self) -> Option<&str> {
        self.hanji.as_deref().filter(|hanji| !hanji.is_empty())
    }
}

/// Result of the read-only candidate query.
///
/// The two "nothing here" answers are deliberately different: a `None` from
/// `fetch_at_pos` means the round-trip never reached the engine, whose state
/// is whatever it already was, while `candidates == None` means the engine
/// answered and reported that it is not in the continuous phase. Merging
/// them corrupts state (`ComposingTransition.swift:99-113`).
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousFetchResult {
    pub transition: ComposingTransition,
    pub candidates: Option<Vec<ContinuousCandidate>>,
}

/// Builders shared by sibling modules' tests (the `metrics.rs` precedent).
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A candidate whose scripts and consumed span the test chooses; every
    /// other field defaulted.
    pub fn candidate(roman: &str, hanji: Option<&str>, span_end: u32) -> ContinuousCandidate {
        ContinuousCandidate {
            consumed_span_start: 0,
            consumed_span_end: span_end,
            syllable_count: 1,
            display_text: hanji.unwrap_or(roman).to_owned(),
            score: 0.0,
            form: 1,
            mode: CandidateMode::Unspecified,
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            canonical_tl: roman.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{
        ClearPreeditWithoutCommit, ComposingResponse, NextWordWordSelected, UpdatePreedit,
    };

    #[test]
    fn decodes_effects_in_wire_order() {
        let response = ComposingResponse {
            preedit: Some(protos::engine::composing_response::Preedit {
                raw_input: "tai5".into(),
                display_text: "tâi".into(),
            }),
            effect: vec![
                WireEffect {
                    kind: Some(effect::Kind::UpdatePreedit(UpdatePreedit {
                        display: "tâi".into(),
                    })),
                },
                WireEffect {
                    kind: Some(effect::Kind::NextWordWordSelected(NextWordWordSelected {
                        text: "台".into(),
                        roman: "tâi".into(),
                        trigger_prediction: true,
                    })),
                },
                WireEffect {
                    kind: Some(effect::Kind::ClearPreeditWithoutCommit(
                        ClearPreeditWithoutCommit {},
                    )),
                },
                WireEffect { kind: None },
            ],
            selected_candidate_index: 0,
            is_composing: true,
            continuous: None,
        };
        let transition = ComposingTransition::decode(&response);
        assert_eq!(transition.raw_input, "tai5");
        assert_eq!(transition.display_text, "tâi");
        assert_eq!(
            transition.effects,
            vec![
                Effect::UpdatePreedit("tâi".into()),
                Effect::NextWordWordSelected {
                    text: "台".into(),
                    roman: "tâi".into(),
                    trigger_prediction: true
                },
                Effect::ClearPreeditWithoutCommit,
            ],
            "an unset kind is dropped, everything else keeps its order"
        );
        assert!(transition.is_composing);
    }

    #[test]
    fn absent_hanji_is_roman_only_and_unknown_mode_is_unspecified() {
        let message = CandidateMessage {
            display_text: "tâi".into(),
            roman: "tâi".into(),
            hanji: None,
            mode: 99,
            ..Default::default()
        };
        let candidate = ContinuousCandidate::decode(&message);
        assert!(candidate.is_roman_only());
        assert_eq!(candidate.mode, CandidateMode::Unspecified);
        let with_hanji = ContinuousCandidate::decode(&CandidateMessage {
            hanji: Some(String::new()),
            mode: 1,
            ..message
        });
        assert!(
            !with_hanji.is_roman_only(),
            "an empty hanji is present, not absent"
        );
        assert_eq!(with_hanji.mode, CandidateMode::Hant);
    }
}
