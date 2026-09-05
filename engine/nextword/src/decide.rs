//! Pure decide table. Every rule here is platform-neutral: what a commit
//! teaches NextWord depends on the writing system it is written in, never on
//! which OS produced it, so iOS, Android, and macOS learn the same thing from
//! the same commit. `AppConfig.platform_id` is still validated as caller
//! metadata but no longer selects behavior — see
//! `docs/architecture/behavioral-invariants.md`
//! §40 `INVARIANT_NEXTWORD_LEARNING_DECISION_CONTRACT`, which also records why
//! the three rules it used to branch on were drift rather than design.
//!
//! Generation bump rule: every state-mutating intent EXCEPT
//! `UpdateLastSelectedWord` bumps `current_generation`. Wrapping add —
//! `u64::MAX + 1 = 0` is a fresh value.

use crate::api::{Intent, NextWordError, PersistedState};
use protos::engine::{
    next_word_effect, AppConfig, AssociationPair, CancelContextTimeout, ClearPredictionsUi,
    DecideResult, NextWordEffect, Platform, QueryPredictions, RecordAssociation,
    RecordCompoundAssociations, RescheduleContextTimeout,
};

/// Strict-`<` association window (10 s).
pub(crate) const ASSOCIATION_TIMEOUT_MS: i64 = 10_000;

/// Context timeout (30 s) — wire field is u64 ms; iOS bridge converts to
/// `TimeInterval` seconds, Android uses `delay(Long ms)`.
const CONTEXT_TIMEOUT_MS: u64 = 30_000;

/// Sentence-end punctuation. Common across both platforms (iOS
/// `NextWordEngine.swift:33`; Android `NextWordEngine.kt:42`).
const SENTENCE_END_PUNCTUATION: &[char] = &['。', '！', '？', '.', '!', '?'];

/// Apply `intent` against `state`, returning the `DecideResult`.
///
/// `platform_id` is a legacy field kept for wire compatibility and possible
/// future routing; no rule below reads it (§40). The `Unspecified` rejection
/// is likewise legacy — it predates the convergence and is retained only so
/// this round changes nothing a platform can observe.
pub(crate) fn apply(
    state: &mut PersistedState,
    intent: Intent,
    config: &AppConfig,
) -> Result<DecideResult, NextWordError> {
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
        } => decide_word_selected(
            state,
            text,
            roman,
            require_roman_mode,
            trigger_prediction,
            now_ms,
            config,
        ),
        Intent::Backspace { last_char, now_ms } => decide_backspace(state, last_char, now_ms),
        Intent::ContextTimeoutFired { now_ms: _ } => reset_and_clear_predictions(state),
        Intent::ClearForNewComposing { now_ms: _ } => decide_clear_for_new_composing(state),
        Intent::ResetFull { now_ms: _ } => reset_and_clear_predictions(state),
        Intent::UpdateLastSelectedWord {
            text,
            roman,
            now_ms,
        } => decide_update_last_selected_word(state, text, roman, now_ms, config),
        Intent::SetIsShowing { is_showing } => decide_set_is_showing(state, is_showing),
    })
}

fn decide_word_selected(
    state: &mut PersistedState,
    text: String,
    roman: String,
    require_roman_mode: bool,
    trigger_prediction: bool,
    now_ms: i64,
    config: &AppConfig,
) -> DecideResult {
    // Enter commits raw romanization only; skip entirely in Hanji mode.
    if require_roman_mode && config.is_translate_swapped {
        return result_unchanged(state);
    }

    // Noise text never records or predicts. Sentence-end is a subset of
    // noise: branch on it first so the reset path fires instead of no-op.
    if is_noise_text(&text) {
        if is_sentence_end_punctuation(&text) {
            return reset_and_clear_predictions(state);
        }
        return result_unchanged(state);
    }

    // poj→tl is idempotent on TL input — safe for POJ and TPS alike.
    let text_tl = phonetics::api::poj_display_to_tl_display(&roman);
    let prev_tl = phonetics::api::poj_display_to_tl_display(
        state.last_selected_roman.as_deref().unwrap_or(""),
    );

    let mut effects: Vec<NextWordEffect> = Vec::new();

    if config.is_association_recording_enabled {
        if let Some(prev_word) = state.last_selected_word.clone() {
            if should_record_association(state, now_ms) {
                effects.push(NextWordEffect {
                    kind: Some(next_word_effect::Kind::RecordAssociation(
                        RecordAssociation {
                            pair: Some(AssociationPair {
                                prev: prev_word,
                                prev_tl: prev_tl.clone(),
                                next: text.clone(),
                                next_tl: text_tl.clone(),
                            }),
                        },
                    )),
                });
            }
        }
        let compound = compound_association_pairs(&text, &text_tl);
        if !compound.is_empty() {
            effects.push(NextWordEffect {
                kind: Some(next_word_effect::Kind::RecordCompoundAssociations(
                    RecordCompoundAssociations { pairs: compound },
                )),
            });
        }
    }

    effects.push(NextWordEffect {
        kind: Some(next_word_effect::Kind::RescheduleContextTimeout(
            RescheduleContextTimeout {
                after_ms: CONTEXT_TIMEOUT_MS,
            },
        )),
    });

    state.last_selected_word = Some(text);
    state.last_selected_roman = Some(text_tl.clone());
    state.last_selection_time_ms = now_ms;
    state.current_generation = state.current_generation.wrapping_add(1);

    if trigger_prediction {
        let word = state.last_selected_word.clone().unwrap_or_default();
        effects.push(NextWordEffect {
            kind: Some(next_word_effect::Kind::QueryPredictions(QueryPredictions {
                word,
                roman: text_tl,
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
    let was_showing = state.is_showing;
    state.is_showing = false;
    state.current_generation = state.current_generation.wrapping_add(1);

    let effects: Vec<NextWordEffect> = if was_showing {
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
/// and `ResetFull` intents.
fn reset_and_clear_predictions(state: &mut PersistedState) -> DecideResult {
    let was_showing = state.is_showing;
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
    if was_showing {
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

/// Mid-commit handshake: mutates state without bumping generation or
/// scheduling the timeout. Records compound associations only (no
/// `prev → this` bigram). Audit §5 #5 / Codex v1 P1. Android's Space path is
/// where this came from; iOS's continuous mid-commit and macOS use it too.
fn decide_update_last_selected_word(
    state: &mut PersistedState,
    text: String,
    roman: String,
    now_ms: i64,
    config: &AppConfig,
) -> DecideResult {
    if text.is_empty() {
        return result_unchanged(state);
    }

    // Noise neither records nor becomes context (§40) — same early return as
    // `decide_word_selected`, except this arm still stamps the clock so the
    // association window keeps advancing across a committed punctuation mark.
    if is_noise_text(&text) {
        state.last_selection_time_ms = now_ms;
        return result_unchanged(state);
    }

    let roman_to_convert = if roman.is_empty() { &text } else { &roman };
    let roman_tl = phonetics::api::poj_display_to_tl_display(roman_to_convert);

    let mut effects: Vec<NextWordEffect> = Vec::new();
    if config.is_association_recording_enabled {
        let compound = compound_association_pairs(&text, &roman_tl);
        if !compound.is_empty() {
            effects.push(NextWordEffect {
                kind: Some(next_word_effect::Kind::RecordCompoundAssociations(
                    RecordCompoundAssociations { pairs: compound },
                )),
            });
        }
    }

    state.last_selected_word = Some(text);
    state.last_selected_roman = Some(roman_tl);
    state.last_selection_time_ms = now_ms;
    // NO generation bump — distinguishes from WordSelected/Backspace etc.

    snapshot_into_decide_result(state, effects)
}

/// Platform-driven visibility sync. No effects, no generation bump —
/// the predict() round-trip whose render produced this update already
/// completed; subsequent intents will bump as usual. Returns the current
/// snapshot so the platform receives a consistent value echo.
fn decide_set_is_showing(state: &mut PersistedState, is_showing: bool) -> DecideResult {
    state.is_showing = is_showing;
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
/// is the only word boundary — `-` is a 連字 / 輕聲 joiner *inside* a word, in
/// engine-rendered output and in raw typed text alike
/// (`engine/composing/src/transition.rs:481-483` passes the pending tail's
/// literal keystroke buffer). See `behavioral-invariants.md` §40.
pub(crate) fn split_compound(word: &str) -> Vec<&str> {
    word.split_whitespace().collect()
}

/// Build sequential bigram pairs from a compound word; order preserved so
/// the platform executor records sequentially (parallel writes race on
/// the SQLite UNIQUE constraint).
pub(crate) fn compound_association_pairs(display_text: &str, roman: &str) -> Vec<AssociationPair> {
    // No boundary, nothing to pair — and the cheapest question to ask, which
    // matters because a single-word commit is the common case.
    if !display_text.contains(char::is_whitespace) {
        return Vec::new();
    }
    // In TPS an ASCII space is the tone-1 syllable marker, not a word break
    // (§31 `INVARIANT_TPS_SPACE_SOFT_SEPARATOR` — `ㄍㄠ` ␣ `ㄉㄞ` is 交代, one
    // word), so a TPS payload has no boundary to learn across. Detected from
    // Bopomofo content because `AppConfig.input_mode` never says `"tps"` here:
    // every platform folds TPS into `"tl"` / `"poj"` when it builds the
    // NextWord config (`ios/…/RustEngineBridge+NextWord.swift:401`,
    // `android/…/NextWordBridge.kt:300-305`). Content upgrading the mode is the
    // established shape (`composing/src/dispatch.rs:187-212`), not a workaround.
    if phonetics::contains_tps(display_text) || phonetics::contains_tps(roman) {
        return Vec::new();
    }
    let parts = split_compound(display_text);
    let roman_parts = split_compound(roman);
    // Record nothing rather than a pair whose romanization had to be guessed.
    // The two strings are split by the same rule and zipped by position, so
    // they line up only when both sides segmented identically — 漢字 `也是`
    // carries no space while its romanization `iā sī` does, and padding the
    // short side would attach an empty `next_tl` to a real word. An empty TL is
    // not a neutral value here: word identity is the `(漢字, canonical TL)`
    // pair (`CLAUDE.md` Core Principle #7), so a blank one writes a row no
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
        .map(|(words, romans)| AssociationPair {
            prev: words[0].to_owned(),
            prev_tl: romans[0].to_owned(),
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

// Whether the first char is sentence-end punctuation; triggers the shared reset path.
pub(crate) fn is_sentence_end_punctuation(text: &str) -> bool {
    text.chars()
        .next()
        .map(|c| SENTENCE_END_PUNCTUATION.contains(&c))
        .unwrap_or(false)
}

fn snapshot_into_decide_result(
    state: &PersistedState,
    effects: Vec<NextWordEffect>,
) -> DecideResult {
    DecideResult {
        effects,
        current_generation: state.current_generation,
        is_showing: state.is_showing,
        last_selected_word: state.last_selected_word.clone().unwrap_or_default(),
    }
}

fn result_unchanged(state: &PersistedState) -> DecideResult {
    snapshot_into_decide_result(state, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(platform: Platform, association_enabled: bool, translate_swapped: bool) -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: translate_swapped,
            is_association_recording_enabled: association_enabled,
            platform_id: platform as i32,
            output_both_scripts: false,
            candidate_display_mode: 0,
        }
    }

    fn ios_config(association_enabled: bool, translate_swapped: bool) -> AppConfig {
        config(Platform::Ios, association_enabled, translate_swapped)
    }

    /// Every recorded compound pair in `result`, or `None` when it recorded no
    /// compound association at all.
    fn compound_pairs(result: &DecideResult) -> Option<&[AssociationPair]> {
        result.effects.iter().find_map(|e| match &e.kind {
            Some(next_word_effect::Kind::RecordCompoundAssociations(c)) => Some(&c.pairs[..]),
            _ => None,
        })
    }

    /// The two intents that can record a compound association: the terminal
    /// commit and the mid-commit handshake.
    fn both_entry_points(text: &str) -> [Intent; 2] {
        [
            Intent::WordSelected {
                text: text.to_owned(),
                roman: text.to_owned(),
                require_roman_mode: false,
                trigger_prediction: false,
                now_ms: 1_000,
            },
            Intent::UpdateLastSelectedWord {
                text: text.to_owned(),
                roman: text.to_owned(),
                now_ms: 1_000,
            },
        ]
    }

    #[test]
    fn unspecified_platform_returns_invalid_platform() {
        let config = config(Platform::Unspecified, true, false);
        let mut state = PersistedState::default();
        let err = apply(&mut state, Intent::ResetFull { now_ms: 0 }, &config).unwrap_err();
        assert!(matches!(err, NextWordError::InvalidPlatform));
    }

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

    #[test]
    fn backspace_does_not_emit_record_effects() {
        let mut state = PersistedState {
            last_selected_word: Some("早安".to_owned()),
            last_selection_time_ms: 1_000,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::Backspace {
                last_char: "好".to_owned(),
                now_ms: 1_500,
            },
            &ios_config(true, false),
        )
        .unwrap();
        for effect in &result.effects {
            assert!(
                !matches!(
                    effect.kind,
                    Some(next_word_effect::Kind::RecordAssociation(_))
                        | Some(next_word_effect::Kind::RecordCompoundAssociations(_))
                ),
                "Backspace must not record associations"
            );
        }
    }

    #[test]
    fn sentence_end_resets_state_and_clears_predictions() {
        let mut state = PersistedState {
            last_selected_word: Some("早安".to_owned()),
            is_showing: true,
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
                now_ms: 1_000,
            },
            &ios_config(true, false),
        )
        .unwrap();
        assert_eq!(state.last_selected_word, None);
        assert!(!state.is_showing);
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

    #[test]
    fn compound_pairs_are_sequential() {
        let pairs = compound_association_pairs("a b c", "x y z");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].prev, "a");
        assert_eq!(pairs[0].next, "b");
        assert_eq!(pairs[1].prev, "b");
        assert_eq!(pairs[1].next, "c");
    }

    #[test]
    fn split_compound_breaks_on_whitespace_but_never_on_hyphen() {
        // trace: 詞組 spacing is the only word boundary a commit carries.
        assert_eq!(
            split_compound("iā sī"),
            vec!["iā", "sī"],
            "the walker's space-join is a real word boundary",
        );
        // 連字 and 輕聲 hyphens live INSIDE one word — and so does a hyphen the
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
        assert_eq!(pairs[0].prev, "iā");
        assert_eq!(pairs[0].prev_tl, "iā");
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
            // Segmentation disagreement: 漢字 `也是` has no space, `iā sī` does.
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
        ] {
            let mut state = PersistedState::default();
            let result = apply(
                &mut state,
                Intent::WordSelected {
                    text: "tâi-gí khí-puânn".to_owned(),
                    roman: "tâi-gí khí-puânn".to_owned(),
                    require_roman_mode: false,
                    trigger_prediction: false,
                    now_ms: 1_000,
                },
                &config(platform, true, false),
            )
            .unwrap();
            let pairs = compound_pairs(&result).unwrap_or_default();
            assert_eq!(pairs.len(), 1, "{platform:?}");
            assert_eq!(pairs[0].prev, "tâi-gí", "{platform:?}");
            assert_eq!(pairs[0].next, "khí-puânn", "{platform:?}");
        }
    }

    #[test]
    fn hyphenated_single_word_teaches_no_compound_on_either_entry_point() {
        // trace: 台語 is one word; neither the terminal WordSelected nor the
        // mid-commit UpdateLastSelectedWord may split it into tâi → gí.
        for intent in both_entry_points("tâi-gí") {
            let mut state = PersistedState::default();
            let result = apply(&mut state, intent, &ios_config(true, false)).unwrap();
            assert!(
                compound_pairs(&result).is_none(),
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
            for intent in both_entry_points(text) {
                let mut state = PersistedState::default();
                let result = apply(&mut state, intent, &ios_config(true, false)).unwrap();
                assert!(
                    compound_pairs(&result).is_none(),
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
                now_ms: 1_000,
            },
            &config(Platform::Macos, true, false),
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
    fn windows_platform_id_passes_validation() {
        // trace: `PLATFORM_WINDOWS = 4` decodes to `Platform::Windows`, so the
        // UNSPECIFIED gate at the top of `apply` must let it through exactly
        // like the three older platforms.
        let mut state = PersistedState::default();
        assert!(
            apply(
                &mut state,
                Intent::SetIsShowing { is_showing: false },
                &config(Platform::Windows, true, false),
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
                Intent::SetIsShowing { is_showing: false },
                &config(Platform::Macos, true, false),
            )
            .is_ok(),
            "PLATFORM_MACOS must not read as PLATFORM_UNSPECIFIED",
        );
    }

    #[test]
    fn update_last_selected_word_does_not_bump_generation() {
        let mut state = PersistedState {
            current_generation: 7,
            ..PersistedState::default()
        };
        let _ = apply(
            &mut state,
            Intent::UpdateLastSelectedWord {
                text: "早安".to_owned(),
                roman: "tsá-an".to_owned(),
                now_ms: 1_000,
            },
            &config(Platform::Android, true, false),
        )
        .unwrap();
        assert_eq!(
            state.current_generation, 7,
            "must NOT bump on UpdateLastSelectedWord"
        );
        assert_eq!(state.last_selected_word, Some("早安".to_owned()));
    }

    #[test]
    fn generation_bumps_on_invalidating_intents() {
        let invalidating = vec![
            Intent::WordSelected {
                text: "好".to_owned(),
                roman: "hó".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                now_ms: 1_000,
            },
            Intent::Backspace {
                last_char: "你".to_owned(),
                now_ms: 1_000,
            },
            Intent::ContextTimeoutFired { now_ms: 2_000 },
            Intent::ClearForNewComposing { now_ms: 3_000 },
            Intent::ResetFull { now_ms: 4_000 },
        ];
        for intent in invalidating {
            let mut state = PersistedState {
                current_generation: 1,
                is_showing: true,
                ..PersistedState::default()
            };
            apply(&mut state, intent.clone(), &ios_config(true, false)).unwrap();
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
            Intent::ResetFull { now_ms: 1_000 },
            &ios_config(true, false),
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
                now_ms: 1_000,
            },
            &ios_config(true, true), // is_translate_swapped = true
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
        let result = apply(
            &mut state,
            Intent::WordSelected {
                text: "安".to_owned(),
                roman: "an".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                now_ms: 5_000,
            },
            &ios_config(true, false),
        )
        .unwrap();
        let has_record = result
            .effects
            .iter()
            .any(|e| matches!(e.kind, Some(next_word_effect::Kind::RecordAssociation(_))));
        assert!(has_record, "should record bigram within 10 s window");
    }

    #[test]
    fn word_selected_skips_record_outside_window() {
        let mut state = PersistedState {
            last_selected_word: Some("早".to_owned()),
            last_selected_roman: Some("tsá".to_owned()),
            last_selection_time_ms: 0,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::WordSelected {
                text: "安".to_owned(),
                roman: "an".to_owned(),
                require_roman_mode: false,
                trigger_prediction: true,
                now_ms: 20_000,
            },
            &ios_config(true, false),
        )
        .unwrap();
        let has_record = result
            .effects
            .iter()
            .any(|e| matches!(e.kind, Some(next_word_effect::Kind::RecordAssociation(_))));
        assert!(!has_record, "outside 10 s window must not record");
    }

    #[test]
    fn set_is_showing_updates_state_without_bump_or_effects() {
        let mut state = PersistedState {
            current_generation: 9,
            is_showing: false,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::SetIsShowing { is_showing: true },
            &ios_config(true, false),
        )
        .unwrap();
        assert!(state.is_showing, "is_showing flipped to true");
        assert_eq!(state.current_generation, 9, "no generation bump");
        assert!(result.effects.is_empty(), "no effects emitted");
    }

    #[test]
    fn set_is_showing_then_clear_for_new_composing_emits_clear() {
        // Regression guard for the v3.5.5 bridge gap fix: pre-fix, platform
        // had no way to push is_showing=true into the engine, so the
        // ClearForNewComposing → ClearPredictionsUI gate never tripped.
        let mut state = PersistedState::default();
        apply(
            &mut state,
            Intent::SetIsShowing { is_showing: true },
            &ios_config(true, false),
        )
        .unwrap();
        let result = apply(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 1_000 },
            &ios_config(true, false),
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
            is_showing: true,
            current_generation: 3,
            ..PersistedState::default()
        };
        let result = apply(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 1_000 },
            &ios_config(true, false),
        )
        .unwrap();
        assert!(!state.is_showing);
        assert_eq!(state.current_generation, 4);
        assert!(matches!(
            result.effects[0].kind,
            Some(next_word_effect::Kind::ClearPredictionsUi(_))
        ));

        // Already not showing → no effect emitted.
        state.is_showing = false;
        let result = apply(
            &mut state,
            Intent::ClearForNewComposing { now_ms: 2_000 },
            &ios_config(true, false),
        )
        .unwrap();
        assert!(result.effects.is_empty());
    }
}
