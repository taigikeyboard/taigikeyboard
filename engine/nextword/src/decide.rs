//! Pure decide table — port of iOS `NextWordEngine.decide` + Android
//! `NextWordEngine.decide`. Branches on `AppConfig.platform_id` for two
//! known divergences (audit §5 #1 compound-split separator, #2 noise-punct
//! superset). All other behavior identical across platforms.
//!
//! Generation bump rule: every state-mutating intent EXCEPT
//! `UpdateLastSelectedWord` bumps `current_generation`. Wrapping add —
//! `u64::MAX + 1 = 0` is a fresh value.

// 中文: 純決策表;移植自兩平台 NextWordEngine.decide,以 platform_id 處理兩處已知差異(複合詞拆字、噪音標點)。
// 中文: 世代規則:除 UpdateLastSelectedWord 外的狀態變更 intent 都會 +1 current_generation(wrapping_add)。

use crate::api::{Intent, NextWordError, PersistedState};
use protos::engine::{
    next_word_effect, AppConfig, AssociationPair, CancelContextTimeout, ClearPredictionsUi,
    DecideResult, NextWordEffect, Platform, QueryPredictions, RecordAssociation,
    RecordCompoundAssociations, RescheduleContextTimeout,
};

/// Strict-`<` association window (10 s).
// 中文: association 紀錄時間窗,10 秒嚴格小於。
pub(crate) const ASSOCIATION_TIMEOUT_MS: i64 = 10_000;

/// Context timeout (30 s) — wire field is u64 ms; iOS bridge converts to
/// `TimeInterval` seconds, Android uses `delay(Long ms)`.
// 中文: 上下文逾時 (30 秒),平台側依此重設預測排程。
const CONTEXT_TIMEOUT_MS: u64 = 30_000;

/// Sentence-end punctuation. Common across both platforms (iOS
/// `NextWordEngine.swift:33`; Android `NextWordEngine.kt:42`).
// 中文: 句尾標點集合,iOS / Android 共用。
const SENTENCE_END_PUNCTUATION: &[char] = &['。', '！', '？', '.', '!', '?'];

/// iOS noise-punctuation set (`NextWordEngine.swift:38`). Used by
/// iOS-only branch of `is_noise_text`.
// 中文: iOS 雜訊標點集合,用於 iOS 特有的文字過濾分支。
const IOS_NOISE_PUNCTUATION: &[char] = &[
    '。', '！', '？', '.', '!', '?', '，', ',', '、', '；', ';', '：', ':', '「', '」', '『', '』',
    '"', '“', '”', '\u{2018}', '\u{2019}', '（', '）', '(', ')', '【', '】', '[', ']', '{', '}',
    '—', '–', '-', '～', '~', '…', '·',
];

/// Android noise-punctuation superset (`NextWordEngine.kt:53-93`). Adds
/// ASCII space + full-width space (audit §5 #2).
// 中文: Android 雜訊標點超集,額外納入 ASCII 空白與全形空白。
const ANDROID_NOISE_PUNCTUATION: &[char] = &[
    '。', '！', '？', '.', '!', '?', '，', ',', '、', '；', ';', '：', ':', '「', '」', '『', '』',
    '"', '“', '”', '\'', '（', '）', '(', ')', '【', '】', '[', ']', '{', '}', '—', '–', '-', '～',
    '~', '…', '·', ' ', '\u{3000}',
];

/// Apply `intent` against `state`, returning the platform-neutral
/// `DecideResult`. Validates `Platform::Unspecified` upfront.
// 中文: 決策表入口;先擋掉未指定平台,再依 intent 分派到對應的 decide_* 子函式。
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
            DecideContext { config, platform },
        ),
        Intent::Backspace { last_char, now_ms } => decide_backspace(state, last_char, now_ms),
        Intent::ContextTimeoutFired { now_ms: _ } => reset_and_clear_predictions(state),
        Intent::ClearForNewComposing { now_ms: _ } => decide_clear_for_new_composing(state),
        Intent::ResetFull { now_ms: _ } => reset_and_clear_predictions(state),
        Intent::UpdateLastSelectedWord {
            text,
            roman,
            now_ms,
        } => decide_update_last_selected_word(state, text, roman, now_ms, config, platform),
        Intent::SetIsShowing { is_showing } => decide_set_is_showing(state, is_showing),
    })
}

struct DecideContext<'a> {
    config: &'a AppConfig,
    platform: Platform,
}

fn decide_word_selected(
    state: &mut PersistedState,
    text: String,
    roman: String,
    require_roman_mode: bool,
    trigger_prediction: bool,
    now_ms: i64,
    ctx: DecideContext<'_>,
) -> DecideResult {
    let DecideContext { config, platform } = ctx;
    // Enter commits raw romanization only; skip entirely in Hanji mode.
    if require_roman_mode && config.is_translate_swapped {
        return result_unchanged(state);
    }

    // Noise text never records or predicts. Sentence-end is a subset of
    // noise: branch on it first so the reset path fires instead of no-op.
    if text.is_empty() || is_noise_text(&text, platform) {
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
        let compound = compound_association_pairs(&text, &text_tl, platform);
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
// 中文: 共用重置路徑;句尾標點、上下文逾時、ResetFull 都走這裡。
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

/// Android-only Space-path: mutates state without bumping generation
/// or scheduling the timeout. Records compound associations only (no
/// `prev → this` bigram). Audit §5 #5 / Codex v1 P1.
// 中文: Android 限定 Space 路徑;只更新狀態,不增世代、不重排 timer,只記錄複合詞 association。
fn decide_update_last_selected_word(
    state: &mut PersistedState,
    text: String,
    roman: String,
    now_ms: i64,
    config: &AppConfig,
    platform: Platform,
) -> DecideResult {
    if text.is_empty() {
        return result_unchanged(state);
    }

    let roman_to_convert = if roman.is_empty() { &text } else { &roman };
    let roman_tl = phonetics::api::poj_display_to_tl_display(roman_to_convert);

    let mut effects: Vec<NextWordEffect> = Vec::new();
    if config.is_association_recording_enabled {
        let compound = compound_association_pairs(&text, &roman_tl, platform);
        if !compound.is_empty() {
            effects.push(NextWordEffect {
                kind: Some(next_word_effect::Kind::RecordCompoundAssociations(
                    RecordCompoundAssociations { pairs: compound },
                )),
            });
        }
    }

    if is_noise_text(&text, platform) {
        state.last_selection_time_ms = now_ms;
    } else {
        state.last_selected_word = Some(text);
        state.last_selected_roman = Some(roman_tl);
        state.last_selection_time_ms = now_ms;
    }
    // NO generation bump — distinguishes from WordSelected/Backspace etc.

    snapshot_into_decide_result(state, effects)
}

/// Platform-driven visibility sync. No effects, no generation bump —
/// the predict() round-trip whose render produced this update already
/// completed; subsequent intents will bump as usual. Returns the current
/// snapshot so the platform receives a consistent value echo.
// 中文: 平台告知候選詞顯示狀態;只同步 is_showing,不發 effect 也不增世代,僅回傳最新快照。
fn decide_set_is_showing(state: &mut PersistedState, is_showing: bool) -> DecideResult {
    state.is_showing = is_showing;
    snapshot_into_decide_result(state, Vec::new())
}

/// Strict-`<` window check; non-negative lower bound rejects clock-skew /
/// wrapping. Mirrors iOS `shouldRecordAssociation` /
/// Android `shouldRecordAssociation`.
// 中文: 判斷是否落在 association 窗內;以嚴格小於 + 非負下限阻擋時鐘倒退或溢位。
pub(crate) fn should_record_association(state: &PersistedState, now_ms: i64) -> bool {
    if state.last_selected_word.is_none() {
        return false;
    }
    let delta = now_ms - state.last_selection_time_ms;
    (0..ASSOCIATION_TIMEOUT_MS).contains(&delta)
}

/// Split a compound word. iOS: `-` only. Android: `-` and whitespace.
// 中文: 拆解複合詞;iOS 只切連字符,Android 連字符與空白都當分隔。
pub(crate) fn split_compound(word: &str, platform: Platform) -> Vec<String> {
    if word.is_empty() {
        return Vec::new();
    }
    let split: Vec<&str> = match platform {
        Platform::Ios => word.split('-').collect(),
        Platform::Android => word
            .split(|c: char| c == '-' || c.is_whitespace())
            .collect(),
        Platform::Unspecified => return Vec::new(), // unreachable — apply() validates
    };
    split
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Build sequential bigram pairs from a compound word; order preserved so
/// the platform executor records sequentially (parallel writes race on
/// the SQLite UNIQUE constraint).
// 中文: 把複合詞拆成前後連續 bigram;保留順序避免平台側並行寫入撞 SQLite UNIQUE。
pub(crate) fn compound_association_pairs(
    display_text: &str,
    roman: &str,
    platform: Platform,
) -> Vec<AssociationPair> {
    let parts = split_compound(display_text, platform);
    let roman_parts = split_compound(roman, platform);
    if parts.len() <= 1 {
        return Vec::new();
    }
    let mut pairs = Vec::with_capacity(parts.len() - 1);
    for i in 0..(parts.len() - 1) {
        pairs.push(AssociationPair {
            prev: parts[i].clone(),
            prev_tl: roman_parts.get(i).cloned().unwrap_or_default(),
            next: parts[i + 1].clone(),
            next_tl: roman_parts.get(i + 1).cloned().unwrap_or_default(),
        });
    }
    pairs
}

/// iOS: punctuation / whitespace / pure-ASCII-digit text never triggers.
/// Android: ALL chars are noise-punct or ASCII digit.
// 中文: 噪音文字判斷;iOS 看首字 + 純 ASCII 數字,Android 則整串檢查每個字元。
pub(crate) fn is_noise_text(text: &str, platform: Platform) -> bool {
    if text.is_empty() {
        return true;
    }
    match platform {
        Platform::Ios => {
            let first = match text.chars().next() {
                Some(c) => c,
                None => return true,
            };
            if IOS_NOISE_PUNCTUATION.contains(&first) {
                return true;
            }
            if first.is_whitespace() {
                return true;
            }
            text.chars().all(|c| c.is_ascii() && c.is_ascii_digit())
        }
        Platform::Android => text
            .chars()
            .all(|c| ANDROID_NOISE_PUNCTUATION.contains(&c) || c.is_ascii_digit()),
        Platform::Unspecified => false, // unreachable — apply() validates
    }
}

// 中文: 判斷首字是否為句尾標點(。!?.!?),用來觸發共用重置路徑。
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

    fn ios_config(association_enabled: bool, translate_swapped: bool) -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: translate_swapped,
            is_association_recording_enabled: association_enabled,
            platform_id: Platform::Ios as i32,
        }
    }

    fn android_config(association_enabled: bool, translate_swapped: bool) -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: translate_swapped,
            is_association_recording_enabled: association_enabled,
            platform_id: Platform::Android as i32,
        }
    }

    #[test]
    fn unspecified_platform_returns_invalid_platform() {
        let config = AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: true,
            platform_id: Platform::Unspecified as i32,
        };
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
        let pairs = compound_association_pairs("a-b-c", "x-y-z", Platform::Ios);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].prev, "a");
        assert_eq!(pairs[0].next, "b");
        assert_eq!(pairs[1].prev, "b");
        assert_eq!(pairs[1].next, "c");
    }

    #[test]
    fn compound_split_ios_dash_only_android_includes_whitespace() {
        let ios = split_compound("a b-c", Platform::Ios);
        assert_eq!(ios, vec!["a b", "c"]);
        let android = split_compound("a b-c", Platform::Android);
        assert_eq!(android, vec!["a", "b", "c"]);
    }

    #[test]
    fn ios_noise_first_char_only_android_all_chars() {
        // "a." starts with 'a' (not noise) on iOS, so iOS says NOT noise.
        // Android scans every char — 'a' is not in punct set, so NOT noise either.
        // So that case isn't a divergence example. Use "  ab" instead.
        // iOS: first char ' ' (whitespace) → NOISE.
        // Android: ' ' is in punct set, but 'a'+'b' are not digit/punct → NOT noise.
        assert!(is_noise_text("  ab", Platform::Ios));
        assert!(!is_noise_text("  ab", Platform::Android));
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
            &android_config(true, false),
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
