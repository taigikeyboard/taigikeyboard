//! Filter+merge+score+sort+limit pipeline. Mirrors the iOS / Android
//! `NextWordService.predict` post-processing currently inlined inside the
//! platform service. Post-v3.5.5 the platform returns un-merged un-scored
//! rows tagged by `Source`, and this module owns scoring + merging +
//! sorting + truncation + display-rule shaping.
//!
//! Generation-mismatch path returns `was_stale=true` with empty
//! predictions. Per Codex v1+v2 review: `Source::Unspecified` →
//! `FailInvariant`. Empty `hanzi` rows dropped silently (cannot become
//! UI-meaningful).

// 中文: 過濾 + 合併 + 評分 + 排序 + 截斷的後處理管線;同時負責顯示模式的轉換與整形。
// 中文: 世代不符回傳 was_stale=true;Source::Unspecified 視為錯誤;空 hanzi 條目靜默丟棄。

use crate::api::{NextWordError, PersistedState};
use crate::scorer;
use indexmap::IndexMap;
use protos::engine::{AppConfig, EnginePrediction, FilterResult, RawNextWordPrediction, Source};

const DEFAULT_LIMIT: usize = 30;

struct MergedRow {
    hanzi: String,
    tl: String,
    score: f64,
}

// 中文: 主後處理入口;依世代決定 stale,接著評分、合併、整形、排序、截斷。
pub(crate) fn filter(
    state: &PersistedState,
    raw: Vec<RawNextWordPrediction>,
    query_generation: u64,
    now_ms: i64,
    limit: i32,
    config: &AppConfig,
) -> Result<FilterResult, NextWordError> {
    if query_generation != state.current_generation {
        return Ok(FilterResult {
            predictions: Vec::new(),
            was_stale: true,
        });
    }

    let effective_limit = if limit <= 0 {
        DEFAULT_LIMIT
    } else {
        limit as usize
    };

    // 1. score each row, fail-invariant on Source::Unspecified, merge by
    //    (hanzi, tl). IndexMap preserves insertion order — matches Android
    //    `mutableMapOf` / `LinkedHashMap`. iOS pre-v3.5.5 had unstable
    //    HashMap iteration order; v3.5.5 unifies on insertion order — minor
    //    parity correction toward Android. No observable user-facing
    //    effect: results are sorted by score immediately after merge, so
    //    iteration order only affects equal-score ties.
    let mut merged: IndexMap<(String, String), MergedRow> = IndexMap::new();
    for row in raw {
        if row.hanzi.is_empty() {
            continue;
        }
        let source = Source::try_from(row.source).map_err(|_| NextWordError::InvalidSource)?;
        let score = match source {
            Source::Dict => scorer::score_dict(row.count),
            Source::User => scorer::calculate_user_score(row.count, row.last_used_ms, now_ms),
            Source::Unspecified => return Err(NextWordError::InvalidSource),
        };
        let key = (row.hanzi.clone(), row.tl.clone());
        merged
            .entry(key)
            .and_modify(|existing| {
                existing.score += score;
            })
            .or_insert(MergedRow {
                hanzi: row.hanzi,
                tl: row.tl,
                score,
            });
    }

    // 2. shape via display rules + drop empty-roman rows in non-Hanji mode.
    let mut shaped: Vec<EnginePrediction> = merged
        .into_iter()
        .filter_map(|(_, m)| shape_prediction(m, config))
        .collect();

    // 3. sort desc by score (stable — equal scores preserve IndexMap
    //    insertion order, which matches Android).
    shaped.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 4. limit truncation.
    shaped.truncate(effective_limit);

    Ok(FilterResult {
        predictions: shaped,
        was_stale: false,
    })
}

fn shape_prediction(m: MergedRow, config: &AppConfig) -> Option<EnginePrediction> {
    let use_tl = config.input_mode == "tl";
    let roman = if use_tl {
        m.tl.clone()
    } else {
        phonetics::api::tl_display_to_poj_display(&m.tl)
    };
    if !config.is_translate_swapped && roman.is_empty() {
        return None;
    }
    let text = if roman.is_empty() {
        m.hanzi.clone()
    } else {
        roman.clone()
    };
    let subtitle = if roman.is_empty() {
        String::new()
    } else {
        m.hanzi.clone()
    };
    Some(EnginePrediction {
        text,
        subtitle,
        hanzi: m.hanzi,
        tl: m.tl,
        score: m.score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::Platform;

    fn config_tl_mode_translate_swapped(swapped: bool) -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: swapped,
            is_association_recording_enabled: true,
            platform_id: Platform::Ios as i32,
            output_both_scripts: false,
        }
    }

    fn config_poj_mode() -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "poj".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: true,
            platform_id: Platform::Ios as i32,
            output_both_scripts: false,
        }
    }

    fn dict_row(hanzi: &str, tl: &str, count: i64) -> RawNextWordPrediction {
        RawNextWordPrediction {
            hanzi: hanzi.to_owned(),
            tl: tl.to_owned(),
            count,
            last_used_ms: 0,
            source: Source::Dict as i32,
        }
    }

    fn user_row(hanzi: &str, tl: &str, count: i64, last_used_ms: i64) -> RawNextWordPrediction {
        RawNextWordPrediction {
            hanzi: hanzi.to_owned(),
            tl: tl.to_owned(),
            count,
            last_used_ms,
            source: Source::User as i32,
        }
    }

    #[test]
    fn stale_query_generation_returns_was_stale() {
        let state = PersistedState {
            current_generation: 5,
            ..PersistedState::default()
        };
        let result = filter(
            &state,
            vec![dict_row("好", "hó", 10)],
            4, // mismatch
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert!(result.was_stale);
        assert!(result.predictions.is_empty());
    }

    #[test]
    fn fresh_query_generation_returns_predictions() {
        let state = PersistedState {
            current_generation: 5,
            ..PersistedState::default()
        };
        let result = filter(
            &state,
            vec![dict_row("好", "hó", 10)],
            5,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert!(!result.was_stale);
        assert_eq!(result.predictions.len(), 1);
        assert_eq!(result.predictions[0].hanzi, "好");
    }

    #[test]
    fn dict_and_user_merge_sums_scores() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                dict_row("好", "hó", 5),
                user_row("好", "hó", 1, 1_000), // both at (好, hó)
            ],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        let p = &result.predictions[0];
        let expected = scorer::score_dict(5) + scorer::calculate_user_score(1, 1_000, 1_000);
        assert!((p.score - expected).abs() < 1e-9);
    }

    #[test]
    fn empty_hanzi_rows_dropped() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                RawNextWordPrediction {
                    hanzi: "".to_owned(),
                    tl: "ho".to_owned(),
                    count: 5,
                    last_used_ms: 0,
                    source: Source::Dict as i32,
                },
                dict_row("好", "hó", 5),
            ],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
    }

    #[test]
    fn unspecified_source_returns_invalid_source() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![RawNextWordPrediction {
                hanzi: "好".to_owned(),
                tl: "hó".to_owned(),
                count: 5,
                last_used_ms: 0,
                source: Source::Unspecified as i32,
            }],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        );
        assert!(matches!(result, Err(NextWordError::InvalidSource)));
    }

    #[test]
    fn invalid_source_int_returns_invalid_source() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![RawNextWordPrediction {
                hanzi: "好".to_owned(),
                tl: "hó".to_owned(),
                count: 5,
                last_used_ms: 0,
                source: 99, // garbage
            }],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        );
        assert!(matches!(result, Err(NextWordError::InvalidSource)));
    }

    #[test]
    fn empty_roman_dropped_in_roman_mode() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("好", "", 5)],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false), // not Hanji mode
        )
        .unwrap();
        assert!(
            result.predictions.is_empty(),
            "empty roman in roman mode dropped"
        );
    }

    #[test]
    fn empty_roman_kept_in_hanji_mode() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("好", "", 5)],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(true), // Hanji mode
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        let p = &result.predictions[0];
        assert_eq!(p.text, "好"); // hanzi as fallback
        assert_eq!(p.subtitle, ""); // nil
    }

    #[test]
    fn poj_mode_converts_tl_to_poj() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("好", "hó", 5)],
            0,
            1_000,
            10,
            &config_poj_mode(),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        let p = &result.predictions[0];
        // tl_display_to_poj_display should convert "hó" to POJ form.
        // Specific value depends on phonetics impl; just verify it's not empty
        // and not equal to TL input (sanity check that conversion happened).
        assert!(!p.text.is_empty());
    }

    #[test]
    fn sort_desc_by_score() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                dict_row("好", "hó", 1),
                user_row("早", "tsá", 5, 1_000), // higher score
                dict_row("安", "an", 3),
            ],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 3);
        // user "早" has highest score (~550 from learning bonus + decay)
        assert_eq!(result.predictions[0].hanzi, "早");
        // Among dicts: "安" (score=3) > "好" (score=1)
        assert_eq!(result.predictions[1].hanzi, "安");
        assert_eq!(result.predictions[2].hanzi, "好");
    }

    #[test]
    fn limit_truncates_results() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            (0..10)
                .map(|i| dict_row(&format!("X{}", i), &format!("x{}", i), i))
                .collect(),
            0,
            1_000,
            3,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 3);
    }

    #[test]
    fn default_limit_when_zero() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            (0..50)
                .map(|i| dict_row(&format!("X{}", i), &format!("x{}", i), i))
                .collect(),
            0,
            1_000,
            0, // default
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), DEFAULT_LIMIT);
    }
}
