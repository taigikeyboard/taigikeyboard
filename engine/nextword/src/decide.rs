//! Pure decide table. Every rule here is platform-neutral: what a commit
//! teaches NextWord depends on the writing system it is written in, never on
//! which OS produced it, so iOS, Android, and macOS learn the same thing from
//! the same commit. `AppConfig.platform_id` is still validated as caller
//! metadata but no longer selects behavior — see
//! `docs/architecture/behavioral-invariants.md`
//! §40 `INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`, which also records why
//! the three rules it used to branch on were drift rather than design.
//!
//! Generation bump rule: every state-mutating intent bumps
//! `current_generation` (`UpdateLastSelectedWord` mutates nothing, §40).
//! Wrapping add — `u64::MAX + 1 = 0` is a fresh value.

use crate::api::{Association, Decided, Intent, NextWordError, PersistedState};
use protos::engine::{
    next_word_effect, AppConfig, CancelContextTimeout, ClearPredictionsUi, CommittedWord,
    DecideResult, NextWordEffect, Platform, QueryPredictions, RescheduleContextTimeout,
};

/// Strict-`<` association window (10 s).
pub(crate) const ASSOCIATION_TIMEOUT_MS: i64 = 10_000;

/// Context timeout (30 s) — wire field is u64 ms; iOS bridge converts to
/// `TimeInterval` seconds, Android uses `delay(Long ms)`.
const CONTEXT_TIMEOUT_MS: u64 = 30_000;

/// Punctuation that ends the context: sentence ends `。！？.!?` and clause
/// marks `，、；：,;:` (USER 2026-10-02). No pair is learned across either —
/// the bundled corpus breaks its chain on the same marks
/// (`dictionary/build/corpus_bigrams.py`).
const CONTEXT_BREAK_PUNCTUATION: &[char] = &[
    '。', '！', '？', '.', '!', '?', '，', '、', '；', '：', ',', ';', ':',
];

/// Apply `intent` against `state`, returning the `DecideResult` and the
/// bigrams it recorded.
///
/// `platform_id` is a legacy field kept for wire compatibility and possible
/// future routing; no rule below reads it (§40). The `Unspecified` rejection
/// is likewise legacy — it predates the convergence and is retained only so
/// this round changes nothing a platform can observe.
pub(crate) fn decide(
    state: &mut PersistedState,
    intent: Intent,
    config: &AppConfig,
) -> Result<Decided, NextWordError> {
    let platform = Platform::try_from(config.platform_id).unwrap_or(Platform::Unspecified);
    if platform == Platform::Unspecified {
        return Err(NextWordError::InvalidPlatform);
    }
    Ok(match intent {
        Intent::WordSelected {
            text,
            roman,
            require_roman_mode,
            trigger_prediction,
            now_ms,
            preceding,
        } => decide_word_selected(
            state,
            WordSelection {
                text,
                roman,
                require_roman_mode,
                trigger_prediction,
                preceding,
            },
            now_ms,
            config,
        ),
        Intent::Backspace { last_char, now_ms } => {
            decide_backspace(state, last_char, now_ms).into()
        }
        Intent::ContextTimeoutFired { now_ms: _ } => reset_and_clear_predictions(state).into(),
        Intent::ClearForNewComposing { now_ms: _ } => decide_clear_for_new_composing(state).into(),
        Intent::ResetAll { now_ms: _ } => reset_and_clear_predictions(state).into(),
        // Nail / unnail: nothing is committed yet, so nothing is learned and
        // the committed context stays (§40); the final commit's `preceding`
        // carries the nailed segments.
        Intent::UpdateLastSelectedWord { .. } => result_unchanged(state).into(),
        Intent::SetPredictionsVisible { visible } => {
            decide_set_predictions_visible(state, visible).into()
        }
    })
}

/// A `WordSelected` intent's payload.
struct WordSelection {
    text: String,
    roman: String,
    require_roman_mode: bool,
    trigger_prediction: bool,
    preceding: Vec<CommittedWord>,
}

/// A committed word as a bigram side: display text + canonical TL.
pub(crate) type ContextWord = (String, String);

fn decide_word_selected(
    state: &mut PersistedState,
    selection: WordSelection,
    now_ms: i64,
    config: &AppConfig,
) -> Decided {
    let WordSelection {
        text,
        roman,
        require_roman_mode,
        trigger_prediction,
        preceding,
    } = selection;
    // Enter commits raw romanization only; skip entirely in Hanji mode. The
    // stored swap, unfolded: a TPS commit reads it as it always has.
    if require_roman_mode && config.is_hanji_first {
        return result_unchanged(state).into();
    }

    // One commit is one sequence (§40): the last committed word leads it
    // only inside the association window; inside the commit every adjacent
    // pair is learned, whatever time the nails took. A sentence end or a
    // clause mark inside it breaks the chain.
    let mut associations = Vec::new();
    let mut previous = committed_context(state, now_ms);
    let mut preceding_moved_context = false;
    for word in preceding {
        if is_noise_text(&word.text) {
            if breaks_context(&word.text) {
                previous = None;
                preceding_moved_context = true;
            }
            continue;
        }
        previous = Some(learn_word(
            previous,
            word.text,
            &word.roman,
            &mut associations,
        ));
        preceding_moved_context = true;
    }

    // Noise text never records or predicts. A sentence end or a clause mark
    // resets; other noise (a symbol) keeps the context — the commit's last
    // word when it had preceding words, else the context as it was.
    if is_noise_text(&text) {
        let breaks = breaks_context(&text);
        if !breaks && !preceding_moved_context {
            return result_unchanged(state).into();
        }
        let result = match previous.filter(|_| !breaks) {
            Some(context) => commit_context(state, context, now_ms, false),
            None => reset_and_clear_predictions(state),
        };
        return Decided {
            result,
            associations,
        };
    }

    let context = learn_word(previous, text, &roman, &mut associations);
    Decided {
        result: commit_context(state, context, now_ms, trigger_prediction),
        associations,
    }
}

/// The last committed word, when a word committed at `now_ms` may follow it.
pub(crate) fn committed_context(state: &PersistedState, now_ms: i64) -> Option<ContextWord> {
    if !should_record_association(state, now_ms) {
        return None;
    }
    let word = state.last_selected_word.clone()?;
    let word_tl = state.last_selected_roman.clone().unwrap_or_default();
    Some((word, word_tl))
}

/// One committed word of a sequence: records `previous → word` and the
/// word's compound pairs; returns the word as the next word's context.
///
/// `roman` is learned as sent: the senders already hand over the canonical
/// TL reading (`WordSelected.roman`, nextword.proto). A POJ → TL fold here
/// would rewrite real TL finals — `eng` / `ek` read as POJ become `ing` /
/// `ik` (蔣經國 `tsiúnn-keng-kok` → `tsiúnn-king-kok`).
fn learn_word(
    previous: Option<ContextWord>,
    text: String,
    roman: &str,
    associations: &mut Vec<Association>,
) -> ContextWord {
    let text_tl = roman.to_owned();
    if let Some((previous, previous_tl)) = previous {
        associations.push(Association {
            previous,
            previous_tl,
            next: text.clone(),
            next_tl: text_tl.clone(),
        });
    }
    associations.extend(compound_association_pairs(&text, &text_tl));
    (text, text_tl)
}

/// Makes `context` the committed context as of `now_ms`: restarts the
/// context timeout, bumps the generation (in-flight predictions for the old
/// context go stale) and, when asked, queries predictions for it.
fn commit_context(
    state: &mut PersistedState,
    (word, word_tl): ContextWord,
    now_ms: i64,
    trigger_prediction: bool,
) -> DecideResult {
    let mut effects = vec![NextWordEffect {
        kind: Some(next_word_effect::Kind::RescheduleContextTimeout(
            RescheduleContextTimeout {
                after_ms: CONTEXT_TIMEOUT_MS,
            },
        )),
    }];
    state.last_selected_word = Some(word.clone());
    state.last_selected_roman = Some(word_tl.clone());
    state.last_selection_time_ms = now_ms;
    state.current_generation = state.current_generation.wrapping_add(1);
    if trigger_prediction {
        effects.push(NextWordEffect {
            kind: Some(next_word_effect::Kind::QueryPredictions(QueryPredictions {
                word,
                roman: word_tl,
                generation: state.current_generation,
                now_ms,
            })),
        });
    }
    snapshot_into_decide_result(state, effects)
}

fn decide_backspace(state: &mut PersistedState, last_char: String, now_ms: i64) -> DecideResult {
    state.last_selected_word = Some(last_char.clone());
    state.last_selected_roman = None;
    state.last_selection_time_ms = now_ms;
    state.current_generation = state.current_generation.wrapping_add(1);

    let effects = vec![NextWordEffect {
        kind: Some(next_word_effect::Kind::QueryPredictions(QueryPredictions {
            word: last_char,
            roman: String::new(),
            generation: state.current_generation,
            now_ms,
        })),
    }];
    snapshot_into_decide_result(state, effects)
}

fn decide_clear_for_new_composing(state: &mut PersistedState) -> DecideResult {
    let was_visible = state.predictions_visible;
    state.predictions_visible = false;
    state.current_generation = state.current_generation.wrapping_add(1);

    let effects: Vec<NextWordEffect> = if was_visible {
        vec![NextWordEffect {
            kind: Some(next_word_effect::Kind::ClearPredictionsUi(
                ClearPredictionsUi {
                    generation: state.current_generation,
                },
            )),
        }]
    } else {
        Vec::new()
    };
    snapshot_into_decide_result(state, effects)
}

/// Shared reset path used by sentence-end punctuation, context timeout,
/// and `ResetAll` intents.
fn reset_and_clear_predictions(state: &mut PersistedState) -> DecideResult {
    let was_visible = state.predictions_visible;
    let new_generation = state.current_generation.wrapping_add(1);
    *state = PersistedState {
        current_generation: new_generation,
        ..PersistedState::default()
    };

    let mut effects: Vec<NextWordEffect> = vec![NextWordEffect {
        kind: Some(next_word_effect::Kind::CancelContextTimeout(
            CancelContextTimeout {},
        )),
    }];
    if was_visible {
        effects.push(NextWordEffect {
            kind: Some(next_word_effect::Kind::ClearPredictionsUi(
                ClearPredictionsUi {
                    generation: new_generation,
                },
            )),
        });
    }
    snapshot_into_decide_result(state, effects)
}

/// Platform-driven visibility sync. No effects, no generation bump —
/// the predict() round-trip whose render produced this update already
/// completed; subsequent intents will bump as usual. Returns the current
/// snapshot so the platform receives a consistent value echo.
fn decide_set_predictions_visible(state: &mut PersistedState, visible: bool) -> DecideResult {
    state.predictions_visible = visible;
    snapshot_into_decide_result(state, Vec::new())
}

/// Strict-`<` window check; non-negative lower bound rejects clock-skew /
/// wrapping. Mirrors iOS `shouldRecordAssociation` /
/// Android `shouldRecordAssociation`.
pub(crate) fn should_record_association(state: &PersistedState, now_ms: i64) -> bool {
    if state.last_selected_word.is_none() {
        return false;
    }
    let delta = now_ms - state.last_selection_time_ms;
    (0..ASSOCIATION_TIMEOUT_MS).contains(&delta)
}

/// Split a commit into the words a bigram may be learned between. Whitespace
/// is the only word boundary — `-` is a compound / neutral-tone joiner *inside* a word, in
/// engine-rendered output and in raw typed text alike
/// (`engine/composing/src/transition.rs:481-483` passes the pending tail's
/// literal keystroke buffer). See `behavioral-invariants.md` §40.
pub(crate) fn split_compound(word: &str) -> Vec<&str> {
    word.split_whitespace().collect()
}

/// Build sequential bigram pairs from a compound word; order preserved so
/// the store records them in order.
pub(crate) fn compound_association_pairs(display_text: &str, roman: &str) -> Vec<Association> {
    // No boundary, nothing to pair — and the cheapest question to ask, which
    // matters because a single-word commit is the common case.
    if !display_text.contains(char::is_whitespace) {
        return Vec::new();
    }
    // In TPS an ASCII space is the tone-1 syllable marker, not a word break
    // (§31 `INVARIANT_TPS_SPACE_SOFT_SEPARATOR` — `ㄍㄠ` ␣ `ㄉㄞ` is 交代, one
    // word), so a TPS payload has no boundary to learn across. Detected from
    // Bopomofo content, not `AppConfig.input_mode`: the payload, not the
    // layout, is what carries the §31 space. Content upgrading the mode is
    // the established shape (`composing::requests` `handle_fetch_at_pos`),
    // not a workaround.
    if phonetics::contains_tps(display_text) || phonetics::contains_tps(roman) {
        return Vec::new();
    }
    let parts = split_compound(display_text);
    let roman_parts = split_compound(roman);
    // Record nothing rather than a pair whose romanization had to be guessed.
    // The two strings are split by the same rule and zipped by position, so
    // they line up only when both sides segmented identically — Hanji `也是`
    // carries no space while its romanization `iā sī` does, and padding the
    // short side would attach an empty `next_tl` to a real word. An empty TL is
    // not a neutral value here: word identity is the `(Hanji, canonical TL)`
    // pair (`AGENTS.md` Core Principle #6), so a blank one writes a row no
    // correctly-keyed lookup will ever match again.
    if parts.len() <= 1 || roman_parts.len() != parts.len() {
        return Vec::new();
    }
    // Same reason one step down: the whole-string noise gate passes `台語 ˆ`
    // because 台語 IS a lexical base, but the split then hands `ˆ` to a pair as
    // if it were a word. A part that could not be a word disqualifies the
    // commit rather than being dropped — dropping it would splice its
    // neighbours into an adjacency the user never typed.
    if !parts.iter().chain(&roman_parts).all(is_word) {
        return Vec::new();
    }
    parts
        .windows(2)
        .zip(roman_parts.windows(2))
        .map(|(words, romans)| Association {
            previous: words[0].to_owned(),
            previous_tl: romans[0].to_owned(),
            next: words[1].to_owned(),
            next_tl: romans[1].to_owned(),
        })
        .collect()
}

/// A commit is noise — it neither records nor becomes context — unless some
/// character in it could be part of a word.
///
/// A Unicode property rather than a punctuation table on purpose: a table
/// lists what the engine has been *told* is punctuation, so the day a commit
/// carries a mark nobody added to it, the string reads as a word and gets
/// learned. The property is the invariant such a table only approximates, and
/// needs no maintenance to stay true.
pub(crate) fn is_noise_text(text: &str) -> bool {
    !text.chars().any(phonetics::is_word_material)
}

/// Whether a split part is a word rather than a run of marks the user
/// committed alongside one.
fn is_word(part: &&str) -> bool {
    part.chars().any(phonetics::is_word_material)
}

// Whether the first char ends the context (sentence end or clause mark);
// triggers the shared reset path.
pub(crate) fn breaks_context(text: &str) -> bool {
    text.chars()
        .next()
        .map(|c| CONTEXT_BREAK_PUNCTUATION.contains(&c))
        .unwrap_or(false)
}

fn snapshot_into_decide_result(
    state: &PersistedState,
    effects: Vec<NextWordEffect>,
) -> DecideResult {
    DecideResult {
        effects,
        current_generation: state.current_generation,
        predictions_visible: state.predictions_visible,
        last_selected_word: state.last_selected_word.clone().unwrap_or_default(),
    }
}

fn result_unchanged(state: &PersistedState) -> DecideResult {
    snapshot_into_decide_result(state, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(platform: Platform, hanji_first: bool) -> AppConfig {
        AppConfig {
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_hanji_first: hanji_first,
            platform_id: platform as i32,
            output_both_scripts: false,
            candidate_display_mode: 0,
            syllable_separator: 0,
            force_lowercase_nasal_marker: false,
            tps_or_maps_to_er: false,
            hanji_conversion: None,
        }
    }

    fn ios_config(hanji_first: bool) -> AppConfig {
        config(Platform::Ios, hanji_first)
    }

    /// [`decide`] without the recorded bigrams.
    fn apply(
        state: &mut PersistedState,
        intent: Intent,
        config: &AppConfig,
    ) -> Result<DecideResult, NextWordError> {
        decide(state, intent, config).map(|decided| decided.result)
    }

    /// The two places a commit can record a compound association: its
    /// terminal word, and a `preceding` word of the same commit (a nailed
    /// continuous segment — here closed by a clause mark that learns nothing).
    fn both_word_positions(text: &str) -> [Intent; 2] {
        [
            Intent::WordSelected {
                text: text.to_owned(),
                roman: text.to_owned(),
                require_roman_mode: false,
                trigger_prediction: false,
                preceding: Vec::new(),
                now_ms: 1_000,
            },
            Intent::WordSelected {
                text: "，".to_owned(),
                roman: "，".to_owned(),
                require_roman_mode: false,
                trigger_prediction: false,
                preceding: vec![committed(text, text)],
                now_ms: 1_000,
            },
        ]
    }

    fn committed(text: &str, roman: &str) -> CommittedWord {
        CommittedWord {
            text: text.to_owned(),
            roman: roman.to_owned(),
        }
    }

    /// A commit of `preceding` + `text` at `now_ms`.
    fn commit(text: &str, roman: &str, preceding: Vec<CommittedWord>, now_ms: i64) -> Intent {
        Intent::WordSelected {
            text: text.to_owned(),
            roman: roman.to_owned(),
            require_roman_mode: false,
            trigger_prediction: true,
            preceding,
            now_ms,
        }
    }

    /// State whose last committed word is 我/guá, committed at t = 0.
    fn after_gua() -> PersistedState {
        PersistedState {
            last_selected_word: Some("我".to_owned()),
            last_selected_roman: Some("guá".to_owned()),
            last_selection_time_ms: 0,
            ..PersistedState::default()
        }
    }

    fn pairs(decided: &Decided) -> Vec<String> {
        decided
            .associations
            .iter()
            .map(|a| format!("{}/{}→{}/{}", a.previous, a.previous_tl, a.next, a.next_tl))
            .collect()
    }

    fn effect_kinds(decided: &Decided) -> Vec<Option<next_word_effect::Kind>> {
        decided
            .result
            .effects
            .iter()
            .map(|e| e.kind.clone())
            .collect()
    }

    // INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING (behavioral-invariants §40):
    // one commit is one sequence — the last committed word leads it inside
    // the 10 s window, every adjacent pair inside it is learned.
    #[test]
    fn commit_learns_every_adjacent_pair_of_its_sequence() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit(
                "飯",
                "pn̄g",
                vec![committed("欲", "beh"), committed("食", "tsia̍h")],
                9_999,
            ),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(
            pairs(&decided),
            vec!["我/guá→欲/beh", "欲/beh→食/tsia̍h", "食/tsia̍h→飯/pn̄g"]
        );
        assert_eq!(state.last_selected_word.as_deref(), Some("飯"));
        assert_eq!(state.last_selection_time_ms, 9_999);
    }

    // INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING: the window gates only the
    // link to the previous commit; however long the nails took, the commit's
    // own pairs are learned.
    #[test]
    fn commit_outside_window_still_learns_its_own_pairs() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit(
                "飯",
                "pn̄g",
                vec![committed("欲", "beh"), committed("食", "tsia̍h")],
                10_000,
            ),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(pairs(&decided), vec!["欲/beh→食/tsia̍h", "食/tsia̍h→飯/pn̄g"]);
    }

    // A repeated word is learned once per occurrence, not deduplicated.
    #[test]
    fn commit_repeated_word_learns_each_occurrence() {
        let mut state = PersistedState::default();
        let decided = decide(
            &mut state,
            commit(
                "好",
                "hó",
                vec![committed("好", "hó"), committed("好", "hó")],
                1_000,
            ),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(pairs(&decided), vec!["好/hó→好/hó", "好/hó→好/hó"]);
    }

    // A sentence end or a clause mark inside the commit breaks the chain
    // there (same rule as single-word commits).
    #[test]
    fn commit_sentence_end_and_clause_mark_both_break_chain() {
        // trace: 我 (t=0) leads at 1 000 ms → 我→欲; `。` drops it; 食 has no
        // predecessor; `，` drops it; 飯 has none — one pair.
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit(
                "飯",
                "pn̄g",
                vec![
                    committed("欲", "beh"),
                    committed("。", "。"),
                    committed("食", "tsia̍h"),
                    committed("，", "，"),
                ],
                1_000,
            ),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(pairs(&decided), vec!["我/guá→欲/beh"]);
        assert_eq!(state.last_selected_word.as_deref(), Some("飯"));
    }

    // A commit ending in a clause mark keeps the pairs before it and clears
    // the context, exactly like one ending in a sentence end.
    #[test]
    fn commit_ending_in_clause_mark_clears_context() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit(
                "，",
                "，",
                vec![committed("欲", "beh"), committed("食", "tsia̍h")],
                1_000,
            ),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(pairs(&decided), vec!["我/guá→欲/beh", "欲/beh→食/tsia̍h"]);
        assert_eq!(state.last_selected_word, None);
        assert_eq!(state.last_selected_roman, None);
        // One reset: in-flight predictions go stale, the timeout is cancelled,
        // nothing is queried (predictions were not visible).
        assert_eq!(state.current_generation, 1);
        let kinds = effect_kinds(&decided);
        assert!(matches!(
            kinds.as_slice(),
            [Some(next_word_effect::Kind::CancelContextTimeout(_))]
        ));
    }

    // INVARIANT_NEXTWORD_CLAUSE_MARK_BREAKS_CONTEXT (behavioral-invariants.md §40)
    // Every clause mark, full- and half-width, breaks the context in both
    // places a commit carries one: as the commit itself, and inside
    // `preceding`.
    #[test]
    fn every_clause_mark_breaks_context_alone_and_inside_a_commit() {
        for mark in ["，", "、", "；", "：", ",", ";", ":"] {
            // Alone: reset, timeout cancelled, visible strip cleared, no query.
            let mut state = PersistedState {
                predictions_visible: true,
                ..after_gua()
            };
            let decided = decide(
                &mut state,
                commit(mark, "", Vec::new(), 1_000),
                &ios_config(false),
            )
            .unwrap();
            assert!(decided.associations.is_empty(), "{mark:?}");
            assert_eq!(state.last_selected_word, None, "{mark:?}");
            assert_eq!(state.last_selected_roman, None, "{mark:?}");
            assert_eq!(state.current_generation, 1, "{mark:?}");
            let kinds = effect_kinds(&decided);
            assert!(
                matches!(
                    kinds.as_slice(),
                    [
                        Some(next_word_effect::Kind::CancelContextTimeout(_)),
                        Some(next_word_effect::Kind::ClearPredictionsUi(_)),
                    ]
                ),
                "{mark:?}"
            );
            // trace: 我 was dropped, so 語 500 ms later pairs with nothing.
            let next = decide(
                &mut state,
                commit("語", "gí", Vec::new(), 1_500),
                &ios_config(false),
            )
            .unwrap();
            assert!(next.associations.is_empty(), "{mark:?}");

            // Inside a commit: 我→欲, the mark drops 欲, 食 → 飯.
            let mut state = after_gua();
            let decided = decide(
                &mut state,
                commit(
                    "飯",
                    "pn̄g",
                    vec![
                        committed("欲", "beh"),
                        committed(mark, ""),
                        committed("食", "tsia̍h"),
                    ],
                    1_000,
                ),
                &ios_config(true),
            )
            .unwrap();
            assert_eq!(
                pairs(&decided),
                vec!["我/guá→欲/beh", "食/tsia̍h→飯/pn̄g"],
                "{mark:?}"
            );
        }
    }

    // Control: noise that is not punctuation (a symbol) still keeps the
    // context — the commit's last word.
    #[test]
    fn commit_ending_in_symbol_keeps_last_word_as_context() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit("★", "", vec![committed("欲", "beh")], 1_000),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(pairs(&decided), vec!["我/guá→欲/beh"]);
        assert_eq!(state.last_selected_word.as_deref(), Some("欲"));
        assert_eq!(state.last_selected_roman.as_deref(), Some("beh"));
        // A new context: the old timeout must not clear it, in-flight
        // predictions for 我 go stale, and a symbol predicts nothing.
        assert_eq!(state.current_generation, 1);
        let kinds = effect_kinds(&decided);
        assert!(matches!(
            kinds.as_slice(),
            [Some(next_word_effect::Kind::RescheduleContextTimeout(_))]
        ));
    }

    // A sentence end inside the commit resets the context even when no word
    // follows it — 我 must not lead the next sentence.
    #[test]
    fn commit_of_sentence_end_then_clause_mark_resets_context() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit("，", "，", vec![committed("。", "。")], 1_000),
            &ios_config(true),
        )
        .unwrap();
        assert!(decided.associations.is_empty());
        assert_eq!(state.last_selected_word, None);
    }

    #[test]
    fn unspecified_platform_returns_invalid_platform() {
        let config = config(Platform::Unspecified, false);
        let mut state = PersistedState::default();
        let err = apply(&mut state, Intent::ResetAll { now_ms: 0 }, &config).unwrap_err();
        assert!(matches!(err, NextWordError::InvalidPlatform));
    }

    // INVARIANT_NEXTWORD_ASSOCIATION_WINDOW_STRICT_LT_10S (nextword-engine-boundary.md §10)
    #[test]
    fn association_window_strict_lt_10s() {
        let mut state = PersistedState {
            last_selected_word: Some("早安".to_owned()),
            last_selection_time_ms: 0,
            ..PersistedState::default()
        };
        assert!(should_record_association(&state, 9_999));
        assert!(!should_record_association(&state, 10_000));
        // Negative delta: clock skew or wrapped — drop.
        state.last_selection_time_ms = 500_000;
        assert!(!should_record_association(&state, 1_000));
        // No prior word — drop.
        state.last_selected_word = None;
        assert!(!should_record_association(&state, 1_000));
    }

    // INVARIANT_NEXTWORD_BACKSPACE_DOES_NOT_RECORD (nextword-engine-boundary.md §10)
    #[test]
    fn backspace_records_no_association() {
        let mut state = PersistedState {
            last_selected_word: Some("早安".to_owned()),
            last_selection_time_ms: 1_000,
            ..PersistedState::default()
        };
        let decided = decide(
            &mut state,
            Intent::Backspace {
                last_char: "好".to_owned(),
                now_ms: 1_500,
            },
            &ios_config(false),
        )
        .unwrap();
        assert!(
            decided.associations.is_empty(),
            "Backspace must not record associations"
        );
    }

    // INVARIANT_NEXTWORD_SENTENCE_END_RESETS_CONTEXT (nextword-engine-boundary.md §10)
    #[test]
    fn sentence_end_resets_state_and_clears_predictions() {
        let mut state = PersistedState {
            last_selected_word: Some("早安".to_owned()),
            predictions_visible: true,
            current_generation: 5,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::WordSelected {
                text: "。".to_owned(),
                roman: "".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                preceding: Vec::new(),
                now_ms: 1_000,
            },
            &ios_config(false),
        )
        .unwrap();
        assert_eq!(state.last_selected_word, None);
        assert!(!state.predictions_visible);
        assert_eq!(state.current_generation, 6);
        let kinds: Vec<_> = result
            .effects
            .iter()
            .filter_map(|e| e.kind.as_ref())
            .collect();
        assert!(matches!(
            kinds[0],
            next_word_effect::Kind::CancelContextTimeout(_)
        ));
        assert!(matches!(
            kinds[1],
            next_word_effect::Kind::ClearPredictionsUi(_)
        ));
    }

    // INVARIANT_NEXTWORD_COMPOUND_PAIRS_ARE_SEQUENTIAL (nextword-engine-boundary.md §10)
    #[test]
    fn compound_pairs_are_sequential() {
        let pairs = compound_association_pairs("a b c", "x y z");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].previous, "a");
        assert_eq!(pairs[0].next, "b");
        assert_eq!(pairs[1].previous, "b");
        assert_eq!(pairs[1].next, "c");
    }

    #[test]
    fn split_compound_breaks_on_whitespace_but_never_on_hyphen() {
        // trace: multi-word spacing is the only word boundary a commit carries.
        assert_eq!(
            split_compound("iā sī"),
            vec!["iā", "sī"],
            "the walker's space-join is a real word boundary",
        );
        // Compound and neutral-tone hyphens live INSIDE one word — and so does a hyphen the
        // user typed, so a raw pending tail is one unit as well.
        assert_eq!(split_compound("tâi-gí"), vec!["tâi-gí"]);
        assert_eq!(split_compound("hōo--guá"), vec!["hōo--guá"]);
        assert_eq!(split_compound("tai-bak"), vec!["tai-bak"]);
        // The whole point of dropping the old iOS arm: splitting on `-` made a
        // "part" that still contained a space.
        assert_eq!(
            split_compound("tâi-gí khí-puânn"),
            vec!["tâi-gí", "khí-puânn"],
            "台語齒盤 is two words, each internally hyphenated",
        );
        assert_eq!(
            split_compound("  tâi\u{3000}gí "),
            vec!["tâi", "gí"],
            "U+3000 is whitespace and repeated separators collapse",
        );
    }

    #[test]
    fn compound_pairs_record_only_a_trustworthy_boundary() {
        // The one shape that earns a pair: both sides segment identically and
        // every part is a word.
        let pairs = compound_association_pairs("iā sī", "iā sī");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].previous, "iā");
        assert_eq!(pairs[0].previous_tl, "iā");
        assert_eq!(pairs[0].next, "sī");
        assert_eq!(pairs[0].next_tl, "sī");

        for (display, roman, why) in [
            // TPS: the space is the tone-1 syllable marker, so `ㄍㄠ ㄉㄞ` is
            // 交代, ONE word (§31). Bopomofo on EITHER side marks the payload —
            // the part-count guard cannot catch it, since both sides do
            // segment alike.
            ("ㄍㄠ ㄉㄞ", "ㄍㄠ ㄉㄞ", "TPS on both sides"),
            ("ㄍㄠ ㄉㄞ", "kau tài", "Bopomofo in display_text alone"),
            ("交 代", "ㄍㄠ ㄉㄞ", "Bopomofo in roman alone"),
            // Segmentation disagreement: Hanji `也是` has no space, `iā sī` does.
            ("也是", "iā sī", "1 vs 2 parts — never pair a guessed TL"),
            ("a b", "x", "a missing romanization is never padded"),
            // No boundary at all.
            ("tâi-gí", "tâi-gí", "連字 compound is one word"),
            ("hōo--guá", "hōo--guá", "輕聲 compound is one word"),
            // A part that is not a word. The whole-string noise gate passes
            // this because 台語 IS word material.
            (
                "台語 \u{02c6}",
                "tâi-gí \u{02c6}",
                "a bare tone mark is not a word",
            ),
            ("台語 ,", "tâi-gí ,", "a bare comma is not a word"),
        ] {
            assert!(
                compound_association_pairs(display, roman).is_empty(),
                "{why}: {display:?} / {roman:?}",
            );
        }
    }

    #[test]
    fn noise_is_the_absence_of_word_material() {
        for text in ["", "。", "!?", "123", "  ", "😀"] {
            assert!(is_noise_text(text), "{text:?} carries no word material");
        }
        // Marks Unicode calls alphabetic that still cannot be a word alone —
        // the four `Lm` TPS tone marks and the POJ nasal in both cases. The
        // remaining three TPS tone marks are `Sk` and never qualified.
        for mark in [
            '\u{02c6}', '\u{02c7}', '\u{02ca}', '\u{02cb}', '\u{02d9}', '\u{02ea}', '\u{02eb}',
            '\u{207f}', '\u{1d3a}',
        ] {
            assert!(
                is_noise_text(&mark.to_string()),
                "standalone {mark:?} is a diacritic, not a word",
            );
        }
        for text in [
            "iā sī",
            "台語",
            "ㄍㄠㄉㄞ",    // Bopomofo is word material
            "😀台語",      // one real word is enough
            "ji\u{030d}t", // decomposed forms keep a Latin base
            "m\u{0304}",
            "o\u{0358}",
            "ho\u{207f}", // a nasal ON a syllable is fine
            "  ab",       // the old iOS first-char rule discarded
            "(彼)",       // both-scripts output, likewise
        ] {
            assert!(!is_noise_text(text), "{text:?} is a word");
        }
    }

    // INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT (behavioral-invariants.md §40)
    #[test]
    fn learning_decisions_do_not_depend_on_the_platform() {
        // trace: §40 — the same commit must teach the same thing everywhere.
        // `tâi-gí khí-puânn` is the case the three platforms used to answer
        // three different ways: iOS split on `-` into `tâi` / `gí khí` /
        // `puânn` (a "part" with a space in it), Android into four syllables,
        // macOS into the two words. Now all three give the two words.
        for platform in [
            Platform::Ios,
            Platform::Android,
            Platform::Macos,
            Platform::Windows,
            Platform::Linux,
        ] {
            let mut state = PersistedState::default();
            let result = decide(
                &mut state,
                Intent::WordSelected {
                    text: "tâi-gí khí-puânn".to_owned(),
                    roman: "tâi-gí khí-puânn".to_owned(),
                    require_roman_mode: false,
                    trigger_prediction: false,
                    preceding: Vec::new(),
                    now_ms: 1_000,
                },
                &config(platform, false),
            )
            .unwrap();
            // A fresh state has no predecessor, so every pair is the compound's.
            let pairs = &result.associations;
            assert_eq!(pairs.len(), 1, "{platform:?}");
            assert_eq!(pairs[0].previous, "tâi-gí", "{platform:?}");
            assert_eq!(pairs[0].next, "khí-puânn", "{platform:?}");
        }
    }

    #[test]
    fn hyphenated_single_word_teaches_no_compound_on_either_entry_point() {
        // trace: 台語 is one word; neither the terminal WordSelected nor the
        // mid-commit UpdateLastSelectedWord may split it into tâi → gí.
        for intent in both_word_positions("tâi-gí") {
            let mut state = PersistedState::default();
            let result = decide(&mut state, intent, &ios_config(false)).unwrap();
            assert!(
                result.associations.is_empty(),
                "a 連字 compound is one word",
            );
        }
    }

    #[test]
    fn noise_records_nothing_and_becomes_no_context_on_either_entry_point() {
        // trace: §40 — noise neither records nor becomes context. `ˆ ˇ` carries
        // no Bopomofo, so it is not a TPS payload, and the whitespace splitter
        // turns it into two "words" that segment alike on both sides — the
        // part-count guard cannot catch it. Only the noise gate can.
        for text in ["\u{02c6} \u{02c7}", ", ;"] {
            for intent in both_word_positions(text) {
                let mut state = PersistedState::default();
                let result = decide(&mut state, intent, &ios_config(false)).unwrap();
                assert!(
                    result.associations.is_empty(),
                    "{text:?} is marks only — nothing to learn",
                );
                assert_eq!(
                    state.last_selected_word, None,
                    "{text:?} must not become context either",
                );
            }
        }
    }

    #[test]
    fn macos_sentence_end_punctuation_resets_context() {
        // trace: `。` is noise, and the sentence-end check runs first, so the
        // reset path fires rather than the no-op path — which is what stops the
        // last word of one sentence being learned as the predecessor of the
        // first word of the next.
        let mut state = PersistedState {
            last_selected_word: Some("台語".to_owned()),
            current_generation: 3,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::WordSelected {
                text: "。".to_owned(),
                roman: String::new(),
                require_roman_mode: false,
                trigger_prediction: false,
                preceding: Vec::new(),
                now_ms: 1_000,
            },
            &config(Platform::Macos, false),
        )
        .unwrap();
        assert_eq!(state.last_selected_word, None);
        assert_eq!(state.current_generation, 4);
        assert!(result.effects.iter().any(|e| matches!(
            e.kind,
            Some(next_word_effect::Kind::CancelContextTimeout(_))
        )));
    }

    #[test]
    fn a_hanji_only_commit_pairs_with_an_empty_tl() {
        // trace: the hanji-only suggestion path sends roman "" — poj→tl("")
        // = "", so the pair carries `next_tl: ""` rather than a guessed one.
        let mut state = PersistedState {
            last_selected_word: Some("早".to_owned()),
            last_selected_roman: Some("tsá".to_owned()),
            last_selection_time_ms: 0,
            ..PersistedState::default()
        };
        let decided = decide(
            &mut state,
            Intent::WordSelected {
                text: "安".to_owned(),
                roman: String::new(),
                require_roman_mode: false,
                trigger_prediction: false,
                preceding: Vec::new(),
                now_ms: 5_000,
            },
            &ios_config(false),
        )
        .unwrap();
        assert_eq!(
            decided.associations,
            vec![Association {
                previous: "早".to_owned(),
                previous_tl: "tsá".to_owned(),
                next: "安".to_owned(),
                next_tl: String::new(),
            }]
        );
    }

    #[test]
    fn linux_platform_id_passes_validation() {
        // trace: `PLATFORM_LINUX = 5` decodes to `Platform::Linux`; same gate
        // as the Windows case below.
        let mut state = PersistedState::default();
        assert!(apply(
            &mut state,
            Intent::SetPredictionsVisible { visible: false },
            &config(Platform::Linux, false),
        )
        .is_ok());
    }

    #[test]
    fn windows_platform_id_passes_validation() {
        // trace: `PLATFORM_WINDOWS = 4` decodes to `Platform::Windows`, so the
        // UNSPECIFIED gate at the top of `apply` must let it through exactly
        // like the three older platforms.
        let mut state = PersistedState::default();
        assert!(
            apply(
                &mut state,
                Intent::SetPredictionsVisible { visible: false },
                &config(Platform::Windows, false),
            )
            .is_ok(),
            "PLATFORM_WINDOWS must not read as PLATFORM_UNSPECIFIED",
        );
    }

    #[test]
    fn macos_platform_id_passes_validation() {
        let mut state = PersistedState::default();
        assert!(
            apply(
                &mut state,
                Intent::SetPredictionsVisible { visible: false },
                &config(Platform::Macos, false),
            )
            .is_ok(),
            "PLATFORM_MACOS must not read as PLATFORM_UNSPECIFIED",
        );
    }

    // INVARIANT_NEXTWORD_COMMIT_SEQUENCE_LEARNING: a nail / unnail learns
    // nothing and leaves the committed context, its time and the generation
    // alone — so an abandoned composition teaches nothing and the next commit
    // still follows 我 inside 我's own window.
    #[test]
    fn update_last_selected_word_learns_nothing_and_keeps_context() {
        let mut state = PersistedState {
            current_generation: 7,
            ..after_gua()
        };
        let decided = decide(
            &mut state,
            Intent::UpdateLastSelectedWord {
                text: "早安 台灣".to_owned(),
                roman: "tsá-an tâi-uân".to_owned(),
                now_ms: 5_000,
            },
            &config(Platform::Android, false),
        )
        .unwrap();
        assert!(decided.associations.is_empty());
        assert_eq!(state.current_generation, 7);
        assert_eq!(state.last_selected_word.as_deref(), Some("我"));
        assert_eq!(state.last_selection_time_ms, 0);
        let _ = decide(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 6_000 },
            &config(Platform::Android, false),
        )
        .unwrap();
        let next = decide(
            &mut state,
            commit("好", "hó", Vec::new(), 10_000),
            &ios_config(true),
        )
        .unwrap();
        assert!(next.associations.is_empty(), "我's window closed at 10 s");
    }

    // INVARIANT_NEXTWORD_GENERATION_BUMPS_ON_INVALIDATING_INTENTS (nextword-engine-boundary.md §10)
    #[test]
    fn generation_bumps_on_invalidating_intents() {
        let invalidating = vec![
            Intent::WordSelected {
                text: "好".to_owned(),
                roman: "hó".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                preceding: Vec::new(),
                now_ms: 1_000,
            },
            Intent::Backspace {
                last_char: "你".to_owned(),
                now_ms: 1_000,
            },
            Intent::ContextTimeoutFired { now_ms: 2_000 },
            Intent::ClearForNewComposing { now_ms: 3_000 },
            Intent::ResetAll { now_ms: 4_000 },
        ];
        for intent in invalidating {
            let mut state = PersistedState {
                current_generation: 1,
                predictions_visible: true,
                ..PersistedState::default()
            };
            apply(&mut state, intent.clone(), &ios_config(false)).unwrap();
            assert!(
                state.current_generation > 1,
                "intent {:?} should bump generation",
                intent
            );
        }
    }

    #[test]
    fn generation_wraps_at_u64_max() {
        let mut state = PersistedState {
            current_generation: u64::MAX,
            ..PersistedState::default()
        };
        let _ = apply(
            &mut state,
            Intent::ResetAll { now_ms: 1_000 },
            &ios_config(false),
        )
        .unwrap();
        assert_eq!(state.current_generation, 0, "wrapping_add expected");
    }

    #[test]
    fn require_roman_mode_in_hanji_mode_is_noop() {
        let mut state = PersistedState {
            last_selected_word: Some("好".to_owned()),
            current_generation: 5,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::WordSelected {
                text: "anything".to_owned(),
                roman: "anything".to_owned(),
                require_roman_mode: true,
                trigger_prediction: true,
                preceding: Vec::new(),
                now_ms: 1_000,
            },
            &ios_config(true), // is_hanji_first = true
        )
        .unwrap();
        assert_eq!(state.current_generation, 5, "no-op must not bump");
        assert!(result.effects.is_empty());
    }

    #[test]
    fn word_selected_records_association_within_window() {
        let mut state = PersistedState {
            last_selected_word: Some("早".to_owned()),
            last_selected_roman: Some("tsá".to_owned()),
            last_selection_time_ms: 1_000,
            ..PersistedState::default()
        };
        let result = decide(
            &mut state,
            Intent::WordSelected {
                text: "安".to_owned(),
                roman: "an".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                preceding: Vec::new(),
                now_ms: 5_000,
            },
            &ios_config(false),
        )
        .unwrap();
        assert_eq!(
            result.associations,
            vec![Association {
                previous: "早".to_owned(),
                previous_tl: "tsá".to_owned(),
                next: "安".to_owned(),
                next_tl: "an".to_owned(),
            }],
            "should record bigram within 10 s window"
        );
    }

    // INVARIANT_NEXTWORD_READING_LEARNED_VERBATIM (behavioral-invariants.md §40)
    // TL special finals `eng` / `ek` (taigi-phonetics-reference §3.2.6) are
    // learned as sent; a POJ → TL fold reads them as POJ `ing` / `ik`.
    // trace: dictionary.csv rows 蔣經國/tsiúnn-keng-kok, 德國簫/tek-kok-siau;
    // no whitespace in either text → no compound pairs.
    #[test]
    fn word_selected_learns_tl_special_finals_verbatim() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit(
                "德國簫",
                "tek-kok-siau",
                vec![committed("蔣經國", "tsiúnn-keng-kok")],
                1_000,
            ),
            &ios_config(false),
        )
        .unwrap();
        assert_eq!(
            pairs(&decided),
            vec![
                "我/guá→蔣經國/tsiúnn-keng-kok",
                "蔣經國/tsiúnn-keng-kok→德國簫/tek-kok-siau",
            ]
        );
        assert_eq!(state.last_selected_roman.as_deref(), Some("tek-kok-siau"));

        // The context reading survives into the next commit's pair.
        let next = decide(
            &mut state,
            commit("人", "lâng", Vec::new(), 2_000),
            &ios_config(false),
        )
        .unwrap();
        assert_eq!(pairs(&next), vec!["德國簫/tek-kok-siau→人/lâng"]);
    }

    // A selected word that ENDS in sentence punctuation (a custom entry such
    // as 多謝！) is still a word: the pair is learned and it becomes the
    // context. Only a selection with no word material that starts with
    // sentence punctuation ends the sentence (`is_noise_text` +
    // `breaks_context`). Android used to reset before
    // sending the selection when the committed text ended this way, dropping
    // the pair (parity fix 2026-09-30); every platform now sends WordSelected
    // alone.
    // INVARIANT_NEXTWORD_TRAILING_SENTENCE_PUNCTUATION_IS_STILL_LEARNED (behavioral-invariants.md §40)
    #[test]
    fn word_selected_ending_in_sentence_punctuation_is_still_learned() {
        let mut state = after_gua();
        let decided = decide(
            &mut state,
            commit("多謝！", "to-siā!", Vec::new(), 1_000),
            &ios_config(true),
        )
        .unwrap();
        assert_eq!(pairs(&decided), vec!["我/guá→多謝！/to-siā!"]);
        assert_eq!(state.last_selected_word.as_deref(), Some("多謝！"));
    }

    #[test]
    fn word_selected_skips_record_outside_window() {
        let mut state = PersistedState {
            last_selected_word: Some("早".to_owned()),
            last_selected_roman: Some("tsá".to_owned()),
            last_selection_time_ms: 0,
            ..PersistedState::default()
        };
        let result = decide(
            &mut state,
            Intent::WordSelected {
                text: "安".to_owned(),
                roman: "an".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                preceding: Vec::new(),
                now_ms: 20_000,
            },
            &ios_config(false),
        )
        .unwrap();
        assert!(
            result.associations.is_empty(),
            "outside 10 s window must not record"
        );
    }

    #[test]
    fn set_predictions_visible_updates_state_without_bump_or_effects() {
        let mut state = PersistedState {
            current_generation: 9,
            predictions_visible: false,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::SetPredictionsVisible { visible: true },
            &ios_config(false),
        )
        .unwrap();
        assert!(
            state.predictions_visible,
            "predictions_visible flipped to true"
        );
        assert_eq!(state.current_generation, 9, "no generation bump");
        assert!(result.effects.is_empty(), "no effects emitted");
    }

    #[test]
    fn set_predictions_visible_then_clear_for_new_composing_emits_clear() {
        // Regression guard for the v3.5.5 bridge gap fix: pre-fix, platform
        // had no way to push predictions_visible=true into the engine, so the
        // ClearForNewComposing → ClearPredictionsUI gate never tripped.
        let mut state = PersistedState::default();
        apply(
            &mut state,
            Intent::SetPredictionsVisible { visible: true },
            &ios_config(false),
        )
        .unwrap();
        let result = apply(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 1_000 },
            &ios_config(false),
        )
        .unwrap();
        assert!(matches!(
            result.effects[0].kind,
            Some(next_word_effect::Kind::ClearPredictionsUi(_))
        ));
    }

    #[test]
    fn clear_for_new_composing_emits_clear_only_when_showing() {
        let mut state = PersistedState {
            predictions_visible: true,
            current_generation: 3,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 1_000 },
            &ios_config(false),
        )
        .unwrap();
        assert!(!state.predictions_visible);
        assert_eq!(state.current_generation, 4);
        assert!(matches!(
            result.effects[0].kind,
            Some(next_word_effect::Kind::ClearPredictionsUi(_))
        ));

        // Already not showing → no effect emitted.
        state.predictions_visible = false;
        let result = apply(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 2_000 },
            &ios_config(false),
        )
        .unwrap();
        assert!(result.effects.is_empty());
    }
}
