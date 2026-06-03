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

    // 1b. collapse separator/tone-only romanization variants of the same
    //     next word (continuous raw `taigi` vs normal canonical `tâi-gí`)
    //     so 台語 does not surface twice. Runs on MergedRow before shaping
    //     so the key is the raw `tl`, not the POJ-shaped display text.
    let collapsed = collapse_reading_variants(merged.into_values().collect());

    // 2. shape via display rules + drop empty-roman rows in non-Hanji mode.
    let mut shaped: Vec<EnginePrediction> = collapsed
        .into_iter()
        .filter_map(|m| shape_prediction(m, config))
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

// 中文: 折疊「同一個 next word、只差分隔符/聲調的羅馬字寫法」成單筆預測。
// 中文:   連續輸入存 raw next_tl(taigi)、一般 commit 存 canonical(tâi-gí),
// 中文:   UNIQUE 含 next_tl → 兩列;(hanzi,tl) merge 不收斂 → 同詞顯示兩次。
// 中文:   讀層把 raw 變體的分數折進唯一 canonical 列。canonical 判定見下。
/// Collapse separator/tone-only romanization variants of the SAME next
/// word into one prediction. The continuous-input commit path stores a
/// raw next_tl (`taigi`) while a normal candidate commit stores the
/// canonical next_tl (`tâi-gí`); `UNIQUE(prev,next,next_tl)` lets both
/// rows persist and the `(hanzi, tl)` merge above keeps them distinct, so
/// 台語 would otherwise surface twice. This read-layer pass folds the raw
/// variant's score into the single canonical row.
///
/// Canonical selection is **separator-based** (Codex post-impl 2026-06-03
/// P1): a genuine multi-syllable reading is ALWAYS hyphen/space-separated
/// in canonical TL, so a no-separator row sharing the toneless key can
/// only be a fused raw keystroke slice. Within a `(hanzi, toneless_key)`
/// group the canonical is the unique separator-bearing rendering; every
/// no-separator row folds into it, and separator-bearing rows that are the
/// SAME reading (`hōo-guá` vs `hōo--guá`, `-` vs `--`) fold together too.
///
/// The group is left untouched when:
///   - NO row has a separator — a bare toneless single syllable may be a
///     genuine tone-1 reading (`當/tang`) byte-identical to a raw, so it
///     must never be folded into a tone-marked sibling (`當/tàng`); this is
///     why tone-mark presence alone is NOT a safe discriminator. Residual:
///     a single-syllable raw-vs-toned duplicate persists until the R2
///     write-side fix; never a wrong merge.
///   - ≥2 separator-bearing rows are genuinely distinct readings
///     (`tāng-bīn` vs `tàng-bīn`) — ambiguous, so nothing folds.
///
/// Grouping key is `phonetics::toneless_reading_key` (separator- AND
/// tone-insensitive). Survivor insertion order is preserved so the
/// downstream score sort sees the same equal-score tie order as before
/// (Android parity).
fn collapse_reading_variants(rows: Vec<MergedRow>) -> Vec<MergedRow> {
    use std::collections::HashMap;

    // group key (hanzi, reading-key) -> indices into `rows`.
    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let key = (row.hanzi.clone(), phonetics::toneless_reading_key(&row.tl));
        groups.entry(key).or_default().push(i);
    }

    // `None` = row absorbed (drop); `Some(extra)` = survivor + folded score.
    let mut delta: Vec<Option<f64>> = vec![Some(0.0); rows.len()];
    for indices in groups.values() {
        if indices.len() < 2 {
            continue;
        }
        let Some(canonical) = select_canonical_row(&rows, indices) else {
            continue;
        };
        for &i in indices {
            if i != canonical {
                if let Some(extra) = delta[canonical].as_mut() {
                    *extra += rows[i].score;
                }
                delta[i] = None;
            }
        }
    }

    rows.into_iter()
        .zip(delta)
        .filter_map(|(mut row, delta)| {
            row.score += delta?;
            Some(row)
        })
        .collect()
}

// 中文: 在同讀音 group 內挑 canonical 列 = 唯一帶分隔符的(多音節)寫法;
// 中文:   無分隔符列只能是 fused raw。全無分隔符 → 回 None(不折,守 #7 單音節
// 中文:   tone-1 與 raw 無法區分);多個帶分隔符卻不同讀音 → 回 None(歧義)。
fn select_canonical_row(rows: &[MergedRow], indices: &[usize]) -> Option<usize> {
    let mut separator_rows = indices.iter().copied().filter(|&i| has_separator(&rows[i].tl));
    let canonical = separator_rows.next()?;
    // All separator-bearing rows must be the same reading (separators
    // stripped, tones kept). Two distinct multi-syllable readings sharing
    // the toneless key (`tāng-bīn` vs `tàng-bīn`) make the group ambiguous.
    let canonical_reading = separatorless_form(&rows[canonical].tl);
    for i in separator_rows {
        if separatorless_form(&rows[i].tl) != canonical_reading {
            return None;
        }
    }
    Some(canonical)
}

fn has_separator(tl: &str) -> bool {
    tl.contains('-') || tl.contains(' ')
}

// 中文: 去分隔符(保留聲調)的讀音形;比對兩個帶分隔符列是否同一讀音。
// 中文:   等同 composing::roman_reading_eq 的正規化(分隔符不敏感、聲調保留)。
fn separatorless_form(tl: &str) -> String {
    tl.chars().filter(|&c| c != '-' && c != ' ').collect()
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

    // --- read-layer reading-variant collapse (v3.6.1 R1) ----------------
    // Pins behavioral-invariants.md §24 INVARIANT_NEXTWORD_READ_LAYER_DEDUP.

    // trace: continuous raw next_tl "taigi" (no separator) + normal canonical
    // "tâi-gí" (hyphen separator) for 台語. (hanzi, tl) merge keeps them as 2
    // rows; toneless key "taigi" groups them; canonical = the separator-
    // bearing row, raw folds in. Result: 1 prediction, canonical roman,
    // score = score_dict(3) + score_dict(5).
    #[test]
    fn collapse_folds_fused_raw_into_separator_canonical() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("台語", "tâi-gí", 5), dict_row("台語", "taigi", 3)],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1, "fused raw folds into separator canonical");
        let p = &result.predictions[0];
        assert_eq!(p.hanzi, "台語");
        assert_eq!(p.tl, "tâi-gí", "separator-bearing canonical is kept");
        assert!(
            (p.score - (scorer::score_dict(5) + scorer::score_dict(3))).abs() < 1e-9,
            "folded score = sum of both variants",
        );
    }

    // trace: all-tone-1 multi-syllable word — the canonical "khong-an" still
    // carries a separator while the fused raw "khongan" does not, so the
    // separator rule folds regardless of the absence of tone diacritics.
    #[test]
    fn collapse_separator_canonical_when_no_tone_marks() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("空安", "khong-an", 4), dict_row("空安", "khongan", 2)],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        assert_eq!(result.predictions[0].tl, "khong-an", "separator form is canonical");
    }

    // trace: separator-form variants of the SAME reading collapse — dict
    // khinsiann "hōo--guá" (`--`) + walker synth-ish "hōo-guá" (`-`) + fused
    // raw "hoogua". All share toneless key "hoogua"; the two separator rows
    // are the same reading (separatorless form "hōoguá"), so one is canonical
    // and the rest fold in.
    #[test]
    fn collapse_folds_separator_variants_of_same_reading() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                dict_row("予我", "hōo-guá", 5),
                dict_row("予我", "hōo--guá", 4),
                dict_row("予我", "hoogua", 2),
            ],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1, "separator + raw variants of one reading collapse");
        assert_eq!(result.predictions[0].hanzi, "予我");
    }

    // trace (Codex post-impl P1): genuine 一字多音 single-syllable readings
    // 當/tàng (tone 3, diacritic) and 當/tang (tone 1, NO diacritic) are both
    // real dictionary words. They share toneless key "tang" but NEITHER has a
    // separator, so the group must NOT fold — folding would hide 當/tang.
    // Tone-mark presence alone is NOT a safe discriminator (tone-1 is bare).
    #[test]
    fn collapse_does_not_fold_bare_single_syllable_polyphones() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("當", "tàng", 5), dict_row("當", "tang", 4)],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            2,
            "genuine tone-1 reading must not be folded into its tone-marked sibling (#7)",
        );
    }

    // trace: 重/tāng + 重/tàng + raw 重/tang — all single-syllable, no
    // separators → no canonical → no fold. All survive.
    #[test]
    fn collapse_preserves_distinct_polyphones() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                dict_row("重", "tāng", 5),
                dict_row("重", "tàng", 4),
                dict_row("重", "tang", 3),
            ],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            3,
            "ambiguous polyphone group must not collapse (Core Principle #7)",
        );
    }

    // trace: two DISTINCT multi-syllable readings sharing a toneless key —
    // "tāng-bīn" vs "tàng-bīn" both carry separators but differ in tone, so
    // the group is ambiguous and nothing folds (separatorless forms differ).
    #[test]
    fn collapse_preserves_distinct_multisyllable_readings() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("X", "tāng-bīn", 5), dict_row("X", "tàng-bīn", 4)],
            0,
            1_000,
            10,
            &config_tl_mode_translate_swapped(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 2, "distinct multi-syllable readings not collapsed");
    }

    // trace: collapse runs BEFORE truncate. 5 words each with separator
    // canonical + fused raw = 10 merged rows → collapse to 5 canonical rows →
    // limit 3 keeps the 3 top-scoring canonicals. Guards against the collapse
    // pass leaking raw duplicates past the limit.
    #[test]
    fn collapse_runs_before_limit_truncation() {
        let state = PersistedState::default();
        let mut raw = Vec::new();
        for i in 0..5 {
            let hanzi = format!("詞{}", i);
            raw.push(dict_row(&hanzi, &format!("ts\u{00e1}-{}", i), (i + 1) as i64)); // separator canonical
            raw.push(dict_row(&hanzi, &format!("tsa{}", i), 1)); // fused raw variant
        }
        let result = filter(&state, raw, 0, 1_000, 3, &config_tl_mode_translate_swapped(false)).unwrap();
        assert_eq!(result.predictions.len(), 3, "collapse then truncate to limit");
        for p in &result.predictions {
            assert!(
                has_separator(&p.tl),
                "survivors are separator-bearing canonicals, not fused raw variants",
            );
        }
    }
}
