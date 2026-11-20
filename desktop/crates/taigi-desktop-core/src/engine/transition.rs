//! Value types for one composing round-trip: the engine snapshot, the ordered
//! effects it wants the platform to run, and the continuous candidates.

use protos::engine::{
    effect, CandidateMessage, CellScript, CommitResolution, ComposingResponse, Effect as WireEffect,
};

/// The complete effect vocabulary of `engine/protos/proto/composing.proto`
/// `Effect` the desktop receives. Every such kind is decoded even though the
/// desktop acts on only some: an effect dropped at decode time is invisible when the slice that
/// needs it lands, whereas an explicitly ignored variant shows up in every
/// `match` the compiler checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// The composition as rendered, and where the caret sits inside it as a
    /// UTF-16 offset — the end unless the user moved it (`MoveCaret`).
    UpdatePreedit {
        text: String,
        caret_utf16: u32,
    },
    ClearPreeditWithoutCommit,
    CommitTextReplacingPreedit(String),
    ClearCandidates,
    RefreshCandidates,
    ResetCandidateContext,
    /// A next-word handshake the engine applied in the same call (every
    /// desktop request carries `NextWordContext`). Its decision is about a
    /// prediction UI the desktop does not have, so it carries nothing.
    NextWordDecided,
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
    pub is_composing: bool,
    /// Where the caret sits in `display_text`, as a UTF-16 offset — the end
    /// unless the user moved it (`MoveCaret`).
    pub caret_utf16: u32,
}

impl ComposingTransition {
    pub(super) fn decode(response: &ComposingResponse) -> Self {
        let preedit = response.preedit.clone().unwrap_or_default();
        Self {
            raw_input: preedit.raw_input,
            display_text: preedit.display_text,
            effects: response.effect.iter().filter_map(Effect::decode).collect(),
            is_composing: response.is_composing,
            caret_utf16: preedit.caret_utf16,
        }
    }
}

impl Effect {
    /// `None` for an effect whose `kind` the wire left unset, which the
    /// current engine never emits, and for the unapplied next-word handshakes
    /// a desktop request never gets back. Every kind it does emit has an arm here —
    /// the exhaustive `match` makes a newly added engine effect a compile
    /// error rather than a silently dropped instruction.
    pub(super) fn decode(effect: &WireEffect) -> Option<Self> {
        Some(match effect.kind.as_ref()? {
            effect::Kind::UpdatePreedit(payload) => Effect::UpdatePreedit {
                text: payload.display.clone(),
                caret_utf16: payload.caret_utf16,
            },
            effect::Kind::ClearPreeditWithoutCommit(_) => Effect::ClearPreeditWithoutCommit,
            effect::Kind::CommitTextReplacingPreedit(payload) => {
                Effect::CommitTextReplacingPreedit(payload.text.clone())
            }
            effect::Kind::ClearCandidates(_) => Effect::ClearCandidates,
            effect::Kind::RefreshCandidates(_) => Effect::RefreshCandidates,
            effect::Kind::ResetCandidateContext(_) => Effect::ResetCandidateContext,
            effect::Kind::NextWordDecided(_) => Effect::NextWordDecided,
            // A request carrying `NextWordContext` gets these back applied,
            // as `NextWordDecided`, and the desktop always sends it: one here
            // means its learning was skipped.
            effect::Kind::NextWordUpdateLastSelectedWord(_)
            | effect::Kind::NextWordWordSelected(_)
            | effect::Kind::NextWordClearForNewComposing(_) => {
                log::error!("composing answer carried an unapplied next-word handshake");
                return None;
            }
        })
    }
}

/// One span-local continuous-input candidate.
///
/// `consumed_span_end` is a byte offset into the raw buffer the engine holds,
/// and `canonical_tl` is the identity romanization (Core Principle #7 keys a
/// word on the `(Hanji, canonical TL)` pair). Both must be round-tripped back
/// to the engine verbatim on commit.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousCandidate {
    pub consumed_span_end: u32,
    pub syllable_count: u32,
    pub display_text: String,
    /// Display romanization for the candidate cell — POJ-rendered in POJ
    /// mode. Not an identity key; that is `canonical_tl`.
    pub roman: String,
    /// `None` when the wire omitted it, which marks a romanization-only
    /// candidate. Distinct from an empty string.
    pub hanji: Option<String>,
    /// Canonical TL, snapshotted before any display recasing. Empty only when
    /// no canonical TL is recoverable.
    pub canonical_tl: String,
    /// What the cell shows under the TPS layout — the Hanji, or the Bopomofo
    /// reading of a Hanji-less candidate; empty off TPS (`CandidateMessage.tps_text`).
    pub tps_text: String,
    /// The §42 split cell this row is (`CandidateMessage.cell_script`): under
    /// Hanji with Romanization a candidate arrives as a Hanji row then a
    /// romanization row; PAIRED under every other display.
    pub cell_script: CellScript,
}

impl ContinuousCandidate {
    pub(super) fn decode(message: &CandidateMessage) -> Self {
        Self {
            consumed_span_end: message.consumed_span_end,
            syllable_count: message.syllable_count,
            display_text: message.display_text.clone(),
            roman: message.roman.clone(),
            hanji: message.hanji.clone(),
            canonical_tl: message.canonical_tl.clone(),
            tps_text: message.tps_text.clone(),
            cell_script: message.cell_script(),
        }
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
/// them corrupts state.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousFetchResult {
    pub transition: ComposingTransition,
    pub candidates: Option<Vec<ContinuousCandidate>>,
}

/// Result of a commit whose document text the engine resolved — a candidate
/// pick, a literal commit, a confirm-then-insert: the transition to replay,
/// and what the commit did (`ComposingResponse.commit` — outcome and
/// auto-space verdict).
#[derive(Clone, Debug, PartialEq)]
pub struct CommitResult {
    pub transition: ComposingTransition,
    pub commit: CommitResolution,
}

/// Builders shared by sibling modules' tests (the `metrics.rs` precedent).
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A candidate whose scripts and consumed span the test chooses; every
    /// other field defaulted.
    pub fn candidate(roman: &str, hanji: Option<&str>, span_end: u32) -> ContinuousCandidate {
        ContinuousCandidate {
            consumed_span_end: span_end,
            syllable_count: 1,
            display_text: hanji.unwrap_or(roman).to_owned(),
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            canonical_tl: roman.to_owned(),
            tps_text: String::new(),
            cell_script: CellScript::Paired,
        }
    }

    /// [`candidate`] as the engine sends it under the TPS layout, with its `tps_text`.
    pub fn tps_candidate(
        roman: &str,
        hanji: Option<&str>,
        span_end: u32,
        tps_text: &str,
    ) -> ContinuousCandidate {
        ContinuousCandidate {
            tps_text: tps_text.to_owned(),
            ..candidate(roman, hanji, span_end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{
        ClearCandidates, ClearPreeditWithoutCommit, CommitTextReplacingPreedit, ComposingResponse,
        DecideResult, NextWordClearForNewComposing, NextWordUpdateLastSelectedWord,
        NextWordWordSelected, RefreshCandidates, ResetCandidateContext, UpdatePreedit,
    };

    #[test]
    fn decodes_effects_in_wire_order() {
        let response = ComposingResponse {
            preedit: Some(protos::engine::composing_response::Preedit {
                raw_input: "tai5".into(),
                display_text: "tâi".into(),
                ..Default::default()
            }),
            effect: vec![
                WireEffect {
                    kind: Some(effect::Kind::UpdatePreedit(UpdatePreedit {
                        display: "tâi".into(),
                        ..Default::default()
                    })),
                },
                WireEffect {
                    kind: Some(effect::Kind::NextWordDecided(DecideResult::default())),
                },
                WireEffect {
                    kind: Some(effect::Kind::ClearPreeditWithoutCommit(
                        ClearPreeditWithoutCommit {},
                    )),
                },
                WireEffect { kind: None },
            ],
            is_composing: true,
            continuous: None,
            commit: None,
        };
        let transition = ComposingTransition::decode(&response);
        assert_eq!(transition.raw_input, "tai5");
        assert_eq!(transition.display_text, "tâi");
        assert_eq!(
            transition.effects,
            vec![
                Effect::UpdatePreedit {
                    text: "tâi".into(),
                    caret_utf16: 0,
                },
                Effect::NextWordDecided,
                Effect::ClearPreeditWithoutCommit,
            ],
            "an unset kind is dropped, everything else keeps its order"
        );
        assert!(transition.is_composing);
    }

    #[test]
    fn decodes_every_effect_kind_with_its_payload() {
        // trace: every wire `effect::Kind` in one response, payloads
        // carried, wire order kept; the three unapplied handshakes (never
        // sent to a desktop, which always sends `NextWordContext`) decode to
        // nothing.
        let wire = |kind| WireEffect { kind: Some(kind) };
        let response = ComposingResponse {
            effect: vec![
                wire(effect::Kind::UpdatePreedit(UpdatePreedit {
                    display: "tâi".into(),
                    caret_utf16: 2,
                })),
                wire(effect::Kind::ClearPreeditWithoutCommit(
                    ClearPreeditWithoutCommit {},
                )),
                wire(effect::Kind::CommitTextReplacingPreedit(
                    CommitTextReplacingPreedit {
                        text: "台語".into(),
                    },
                )),
                wire(effect::Kind::ClearCandidates(ClearCandidates {})),
                wire(effect::Kind::RefreshCandidates(RefreshCandidates {})),
                wire(effect::Kind::ResetCandidateContext(
                    ResetCandidateContext {},
                )),
                wire(effect::Kind::NextWordUpdateLastSelectedWord(
                    NextWordUpdateLastSelectedWord {
                        text: "台".into(),
                        roman: "tâi".into(),
                    },
                )),
                wire(effect::Kind::NextWordWordSelected(NextWordWordSelected {
                    text: "語".into(),
                    roman: "gí".into(),
                    trigger_prediction: false,
                    preceding: vec![],
                })),
                wire(effect::Kind::NextWordClearForNewComposing(
                    NextWordClearForNewComposing {},
                )),
                wire(effect::Kind::NextWordDecided(DecideResult::default())),
            ],
            ..Default::default()
        };
        assert_eq!(
            ComposingTransition::decode(&response).effects,
            vec![
                Effect::UpdatePreedit {
                    text: "tâi".into(),
                    caret_utf16: 2,
                },
                Effect::ClearPreeditWithoutCommit,
                Effect::CommitTextReplacingPreedit("台語".into()),
                Effect::ClearCandidates,
                Effect::RefreshCandidates,
                Effect::ResetCandidateContext,
                Effect::NextWordDecided,
            ]
        );
    }

    #[test]
    fn absent_hanji_is_roman_only() {
        let message = CandidateMessage {
            display_text: "tâi".into(),
            roman: "tâi".into(),
            hanji: None,
            ..Default::default()
        };
        let candidate = ContinuousCandidate::decode(&message);
        assert!(candidate.hanji.is_none());
        let with_hanji = ContinuousCandidate::decode(&CandidateMessage {
            hanji: Some(String::new()),
            ..message
        });
        assert!(
            with_hanji.hanji.is_some(),
            "an empty hanji is present, not absent"
        );
    }
}
