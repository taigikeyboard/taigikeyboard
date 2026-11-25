//! Filter+merge+score+sort+limit pipeline over un-merged, un-scored rows
//! tagged by `Source` (the bundled ones and the engine's own
//! `user_association.db` rows, `PredictNext` expansion in `dispatch`); this
//! module owns scoring + merging +
//! sorting + truncation + display-rule shaping.
//!
//! Generation-mismatch path returns `was_stale=true` with empty
//! predictions. Per Codex v1+v2 review: `Source::Unspecified` →
//! `FailInvariant`. Empty `hanji` rows dropped silently (cannot become
//! UI-meaningful).

use crate::api::{NextWordError, PersistedState};
use crate::scorer;
use indexmap::IndexMap;
use protos::engine::{
    AppConfig, CellScript, EnginePrediction, FilterResult, RawNextWordPrediction, Source,
};

struct MergedRow {
    hanji: String,
    tl: String,
    /// Summed `Source::Dict` scores; capped only when ranked
    /// (`scorer::combined_score`), so merges never lift a bundled-only
    /// prediction past a learned one (§8).
    dict_score: f64,
    user_score: f64,
    /// Whether a `Source::User` row has already contributed to `score` — see
    /// the merge loop for why a second one must not.
    has_user_score: bool,
}

// Post-processing entry: stale check by generation, then score, merge, shape, sort, truncate.
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

    let effective_limit = crate::api::effective_prediction_limit(limit);

    // 1. score each row, fail-invariant on Source::Unspecified, merge by
    //    (hanji, tl). IndexMap preserves insertion order — matches Android
    //    `mutableMapOf` / `LinkedHashMap`. iOS pre-v3.5.5 had unstable
    //    HashMap iteration order; v3.5.5 unifies on insertion order — minor
    //    parity correction toward Android. No observable user-facing
    //    effect: results are sorted by score immediately after merge, so
    //    iteration order only affects equal-score ties.
    let mut merged: IndexMap<(String, String), MergedRow> = IndexMap::new();
    for row in raw {
        if row.hanji.is_empty() {
            continue;
        }
        let source = Source::try_from(row.source).map_err(|_| NextWordError::InvalidSource)?;
        let score = match source {
            Source::Dict => scorer::score_dict(row.count),
            Source::User => scorer::calculate_user_score(row.count, row.last_used_ms, now_ms),
            Source::Unspecified => return Err(NextWordError::InvalidSource),
        };
        let is_user = source == Source::User;
        let (dict_score, user_score) = if is_user { (0.0, score) } else { (score, 0.0) };
        let key = (row.hanji.clone(), row.tl.clone());
        merged
            .entry(key)
            .and_modify(|existing| {
                // At most ONE user contribution per exact `(hanji, tl)` in a
                // request. Summing is right for dict + user — that IS the
                // design, a learned word outranking the same word from the
                // dictionary — and wrong for user + user: under the v6 storage
                // key (`behavioral-invariants.md` §24) one predicted word can be
                // backed by several rows differing in the PREVIOUS word's
                // reading (重/tîng → 複 and 重/tāng → 複, both recalled by the
                // Hanji-only lookup), and adding them would give it several
                // `LEARNING_BONUS` terms for being polyphonic rather than for
                // being well-learned.
                //
                // The engine does not — and cannot — verify that two colliding
                // user rows differ only in `prev_tl`; `RawNextWordPrediction`
                // does not carry it. The rule it enforces is the broader one
                // stated above, which is why `raw` is a PRIORITY-ORDERED list
                // rather than a set: the first user row wins, and the store's
                // query puts the best evidence first
                // (`userdata::UserAssociationStore::rows_following`, §24; see
                // `nextword.proto::FilterPredictions`).
                if is_user && existing.has_user_score {
                    return;
                }
                existing.dict_score += dict_score;
                existing.user_score += user_score;
                existing.has_user_score |= is_user;
            })
            .or_insert(MergedRow {
                hanji: row.hanji,
                tl: row.tl,
                dict_score,
                user_score,
                has_user_score: is_user,
            });
    }

    // 1b. collapse separator/tone-only romanization variants of the same
    //     next word (continuous raw `taigi` vs normal canonical `tâi-gí`)
    //     so 台語 does not surface twice. Runs on MergedRow before shaping
    //     so the key is the raw `tl`, not the POJ-shaped display text.
    let mut collapsed =
        collapse_reading_variants(merged.into_values().collect(), config.khinsiann_in_hanji);

    // 1c. Hàn-Lô lām (§42): each reading's romanization as a prediction of
    //     its own, after the collapse so a reading is twinned once.
    let roman_twins = config.lists_roman_twins();
    if roman_twins {
        collapsed = add_roman_twins(collapsed);
    }

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

    // 3b. Candidate Display = Romanization Only (§44): the prediction cell hides `subtitle`
    //     (the hanji), so homophone predictions (`食/tsia̍h` + `𤆬/tsia̍h`,
    //     distinct after the `(hanji, tl)` merge) read as duplicates.
    //     Collapse by the rendered `text` — under Romanization Only the platform sends
    //     `is_hanji_first = false`, so `shape_prediction` already
    //     dropped roman-empty rows and `text` is always the romanization.
    //     After the score sort so first-seen = best-scored; before the
    //     truncation so the limit is filled with distinct cells.
    if config.is_roman_only_display() {
        let mut seen = std::collections::HashSet::with_capacity(shaped.len());
        shaped.retain(|p| seen.insert(p.text.clone()));
    }

    // 4. limit truncation. Hanji with Romanization (§42) counts the limit in
    //    Hanji predictions: a romanization-only one (Hàn-Lô lām) is a
    //    romanization cell, as a Hanji prediction's own romanization would
    //    be, so it rides along when ranked inside the cut and 30 predictions
    //    still give up to 60 cells.
    let splits = config.splits_single_script_cells();
    let is_romanization_only = |p: &EnginePrediction| p.hanji.is_empty() && !p.text.is_empty();
    let cut = if splits {
        shaped
            .iter()
            .enumerate()
            .filter(|(_, p)| !is_romanization_only(p))
            .nth(effective_limit)
            .map_or(shaped.len(), |(at, _)| at)
    } else {
        effective_limit
    };
    shaped.truncate(cut);

    // 5. Hanji with Romanization (§42): a prediction with romanization becomes a
    //    Hanji cell then a romanization cell, deduped per script; a Hanji-only
    //    prediction lists its Hanji cell alone. After the truncation, so the
    //    limit counts predictions, not cells. A Hanji prediction whose
    //    reading's romanization prediction made the list is a Hanji cell
    //    only: that prediction is the romanization cell.
    if splits {
        let romanized: std::collections::HashSet<String> = shaped
            .iter()
            .filter(|p| is_romanization_only(p))
            .map(|p| p.tl.clone())
            .collect();
        shaped = protos::split_single_script_cells(
            shaped,
            |p| &p.hanji,
            |p| {
                if p.hanji.is_empty() {
                    &p.text
                } else if p.subtitle.is_empty() || romanized.contains(&p.tl) {
                    ""
                } else {
                    &p.text
                }
            },
            |p, script| p.set_cell_script(script),
        );
    }

    Ok(FilterResult {
        predictions: shaped,
        was_stale: false,
    })
}

/// Collapse separator/tone-only romanization variants of the SAME next
/// word into one prediction. The continuous-input commit path stores a
/// raw next_tl (`taigi`) while a normal candidate commit stores the
/// canonical next_tl (`tâi-gí`); `UNIQUE(prev,next,next_tl)` lets both
/// rows persist and the `(hanji, tl)` merge above keeps them distinct, so
/// 台語 would otherwise surface twice. This read-layer pass folds the raw
/// variant's score into the single canonical row.
///
/// Canonical selection is **separator-based** (Codex post-impl 2026-06-03
/// P1): a genuine multi-syllable reading is ALWAYS hyphen/space-separated
/// in canonical TL, so a no-separator row sharing the toneless key can
/// only be a fused raw keystroke slice. Within a `(hanji, toneless_key)`
/// group the canonical is the unique separator-bearing rendering; every
/// no-separator row folds into it, and separator-bearing rows that are the
/// SAME reading (`hōo-guá` vs `hōo--guá`, `-` vs `--`) fold together too
/// (except when `khinsiann_in_hanji` is enabled and their displayed Hanji
/// forms differ, such as `轉--去` vs `轉去`).
///
/// The group is left untouched when:
///   - NO row has a separator — a bare toneless single syllable may be a
///     genuine tone-1 reading (`當/tang`) byte-identical to a raw, so it
///     must never be folded into a tone-marked sibling (`當/tàng`); this is
///     why tone-mark presence alone is NOT a safe discriminator. Residual:
///     a single-syllable raw-vs-toned duplicate persists until the R2
///     write-side fix; never a wrong merge.
///   - ≥2 separator-bearing rows are genuinely distinct readings
///     (`tāng-bīn` vs `tàng-bīn`, or `轉--去` vs `轉去` when
///     `khinsiann_in_hanji` is enabled) — ambiguous, so nothing folds.
///
/// Grouping key is `phonetics::toneless_reading_key` (separator- AND
/// tone-insensitive). Survivor insertion order is preserved so the
/// downstream score sort sees the same equal-score tie order as before
/// (Android parity).
fn collapse_reading_variants(rows: Vec<MergedRow>, khinsiann_in_hanji: bool) -> Vec<MergedRow> {
    use std::collections::HashMap;

    // group key (hanji, reading-key) -> indices into `rows`.
    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let key = (row.hanji.clone(), phonetics::toneless_reading_key(&row.tl));
        groups.entry(key).or_default().push(i);
    }

    // `None` = row absorbed (drop); `Some((dict, user))` = survivor + folded scores.
    let mut delta: Vec<Option<(f64, f64)>> = vec![Some((0.0, 0.0)); rows.len()];
    for indices in groups.values() {
        if indices.len() < 2 {
            continue;
        }
        let Some(canonical) = select_canonical_row(&rows, indices, khinsiann_in_hanji) else {
            continue;
        };
        for &i in indices {
            if i != canonical {
                if let Some((dict, user)) = delta[canonical].as_mut() {
                    *dict += rows[i].dict_score;
                    *user += rows[i].user_score;
                }
                delta[i] = None;
            }
        }
    }

    rows.into_iter()
        .zip(delta)
        .filter_map(|(mut row, delta)| {
            let (dict, user) = delta?;
            row.dict_score += dict;
            row.user_score += user;
            Some(row)
        })
        .collect()
}

// Canonical row = the unique separator-bearing (multi-syllable) rendering; a no-separator row can
// only be a fused raw. None when no row has a separator (a bare single syllable may be a genuine
// tone-1 reading) or when the separator-bearing rows are different readings.
fn select_canonical_row(
    rows: &[MergedRow],
    indices: &[usize],
    khinsiann_in_hanji: bool,
) -> Option<usize> {
    let mut separator_rows = indices
        .iter()
        .copied()
        .filter(|&i| has_separator(&rows[i].tl));
    let canonical = separator_rows.next()?;
    // All separator-bearing rows must be the same reading (separators
    // stripped, tones kept). Two distinct multi-syllable readings sharing
    // the toneless key (`tāng-bīn` vs `tàng-bīn`) make the group ambiguous.
    // When `khinsiann_in_hanji` is on, `--` vs `-` also produces distinct
    // Hanji outputs (`轉--去` vs `轉去`), so treat differing displayed
    // Hanji as distinct.
    let canonical_tl = &rows[canonical].tl;
    let canonical_hanji = phonetics::api::render_hanji_khinsiann(
        &rows[canonical].hanji,
        canonical_tl,
        khinsiann_in_hanji,
    );
    if separator_rows.any(|i| {
        !phonetics::api::roman_reading_eq(&rows[i].tl, canonical_tl)
            || (khinsiann_in_hanji
                && phonetics::api::render_hanji_khinsiann(&rows[i].hanji, &rows[i].tl, true)
                    != canonical_hanji)
    }) {
        return None;
    }
    Some(canonical)
}

/// How much less likely a reading's romanization is written than its most
/// written Hanji: `e⁻¹`, the next-word image of the candidate list's
/// `lexicon` `ROMAN_TWIN_COST_PENALTY` (1000 milli-nats of `−ln p`).
const ROMAN_TWIN_SCORE_FACTOR: f64 = 0.367_879_441_171_442_3;

/// A row whose stored spelling is its own reading in romanization (`kah`, a
/// learned `tsia̍h`): a word written in romanization, identity `(TL, TL)`. A
/// Latin-spelled word with another reading (`KTV` / `khe-thi-bui`, `OK`) is
/// a word of its own, not a romanization.
fn is_romanization_word(row: &MergedRow) -> bool {
    phonetics::api::is_roman_only_spelling(&row.hanji)
        && phonetics::api::roman_reading_eq(&row.hanji, &row.tl)
}

/// Hàn-Lô lām (§42, `AppConfig::lists_roman_twins`): every reading among the
/// Hanji predictions also offers its romanization, a prediction whose
/// identity is `(TL, TL)` — the pair a romanization pick is counted and
/// learned under, so its learned score is its own. Its bundled prior is the
/// reading's best Hanji prediction's capped dictionary score ×
/// [`ROMAN_TWIN_SCORE_FACTOR`]; a romanization row already present (learned,
/// or a bundled romanization word) is that reading's row, lifted to the prior
/// when below it. Each new row follows its reading's first Hanji row, so an
/// equal-score tie keeps it there.
fn add_roman_twins(rows: Vec<MergedRow>) -> Vec<MergedRow> {
    use std::collections::{HashMap, HashSet};

    let mut prior: HashMap<String, f64> = HashMap::new();
    for row in rows
        .iter()
        .filter(|row| !row.tl.is_empty() && !is_romanization_word(row))
    {
        let twin = row.dict_score.min(scorer::DICT_SCORE_CAP) * ROMAN_TWIN_SCORE_FACTOR;
        prior
            .entry(row.tl.clone())
            .and_modify(|best| *best = best.max(twin))
            .or_insert(twin);
    }
    let mut twinned: HashSet<String> = rows
        .iter()
        .filter(|row| is_romanization_word(row))
        .map(|row| row.tl.clone())
        .collect();
    let mut out = Vec::with_capacity(rows.len() * 2);
    for mut row in rows {
        let Some(&twin) = prior.get(&row.tl) else {
            out.push(row);
            continue;
        };
        if is_romanization_word(&row) {
            row.dict_score = row.dict_score.max(twin);
            out.push(row);
            continue;
        }
        let tl = row.tl.clone();
        out.push(row);
        if twinned.insert(tl.clone()) {
            out.push(MergedRow {
                hanji: tl.clone(),
                tl,
                dict_score: twin,
                user_score: 0.0,
                has_user_score: false,
            });
        }
    }
    out
}

fn has_separator(tl: &str) -> bool {
    tl.contains('-') || tl.contains(' ')
}

fn shape_prediction(m: MergedRow, config: &AppConfig) -> Option<EnginePrediction> {
    // The TPS layout renders predictions from TL, as it did while TPS arrived as `"tl"`.
    let use_tl = config.input_mode == "tl" || config.is_tps_layout();
    let mut roman = if use_tl {
        m.tl.clone()
    } else {
        phonetics::api::tl_display_to_poj_display(&m.tl)
    };
    // Syllable Separator — presentation only; `tl` below stays the association key.
    if let Some(joiner) = config.rendered_syllable_joiner() {
        roman = phonetics::api::syllable_joiner_display(&roman, joiner);
    }
    // The stored swap, unfolded: a TPS prediction reads it as it always has.
    if !config.is_hanji_first && roman.is_empty() {
        return None;
    }
    // A word written in romanization (a Hàn-Lô lām pick learned as
    // `(TL, TL)`) is a romanization-only prediction (no Hanji, no subtitle),
    // like a romanization-only candidate, whether the switch is still on or
    // not: the tap writes `text`. The TPS layout, which never shows a
    // romanization, drops it. Otherwise the shown Hanji, its romanization runs
    // in POJ wherever `roman` is. `display_text` keeps the stored spelling,
    // the key a pick is counted under and NextWord learns.
    let romanization_word = !roman.is_empty() && is_romanization_word(&m);
    if romanization_word && config.is_tps_layout() {
        return None;
    }
    let hanji = if romanization_word {
        String::new()
    } else {
        phonetics::api::render_hanji_display(&m.hanji, &m.tl, config.khinsiann_in_hanji, !use_tl)
            .into_owned()
    };
    let text = if roman.is_empty() {
        hanji.clone()
    } else {
        roman.clone()
    };
    let subtitle = if roman.is_empty() || hanji.is_empty() {
        String::new()
    } else {
        hanji.clone()
    };
    Some(EnginePrediction {
        text,
        subtitle,
        hanji,
        tl: m.tl,
        score: scorer::combined_score(m.dict_score, m.user_score),
        cell_script: CellScript::Paired as i32,
        display_text: m.hanji,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{CandidateDisplayMode, SyllableSeparator};

    fn config_tl_mode_hanji_first(swapped: bool) -> AppConfig {
        AppConfig {
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_hanji_first: swapped,
            output_both_scripts: false,
            candidate_display_mode: 0,
            syllable_separator: 0,
            force_lowercase_nasal_marker: false,
            tps_or_maps_to_er: false,
            hanji_conversion: None,
            dictionary_sources: None,
            custom_dictionary_disabled: false,
            c_types_affricate: false,
            f_types_hyphen: false,
            khinsiann_in_hanji: false,
            han_lo_mixed: false,
        }
    }

    fn config_poj_mode() -> AppConfig {
        AppConfig {
            input_mode: "poj".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_hanji_first: false,
            output_both_scripts: false,
            candidate_display_mode: 0,
            syllable_separator: 0,
            force_lowercase_nasal_marker: false,
            tps_or_maps_to_er: false,
            hanji_conversion: None,
            dictionary_sources: None,
            custom_dictionary_disabled: false,
            c_types_affricate: false,
            f_types_hyphen: false,
            khinsiann_in_hanji: false,
            han_lo_mixed: false,
        }
    }

    fn config_roman_only_display() -> AppConfig {
        AppConfig {
            candidate_display_mode: CandidateDisplayMode::RomanOnly as i32,
            ..config_tl_mode_hanji_first(false)
        }
    }

    fn dict_row(hanji: &str, tl: &str, count: i64) -> RawNextWordPrediction {
        RawNextWordPrediction {
            hanji: hanji.to_owned(),
            tl: tl.to_owned(),
            count,
            last_used_ms: 0,
            source: Source::Dict as i32,
        }
    }

    fn user_row(hanji: &str, tl: &str, count: i64, last_used_ms: i64) -> RawNextWordPrediction {
        RawNextWordPrediction {
            hanji: hanji.to_owned(),
            tl: tl.to_owned(),
            count,
            last_used_ms,
            source: Source::User as i32,
        }
    }

    /// §24 prediction identity — under the v6 storage key the two readings of
    /// a polyphonic previous word are two rows, and the Hanji-only lookup returns
    /// BOTH. Their scores must not stack: one predicted word, one learning
    /// bonus. Two count-5 user rows would otherwise score ~2× a single one.
    #[test]
    fn second_user_row_for_one_prediction_does_not_stack_its_bonus() {
        let state = PersistedState::default();
        let config = config_tl_mode_hanji_first(false);
        let one = filter(
            &state,
            vec![user_row("複", "ho̍k", 5, 1_000)],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();
        let two = filter(
            &state,
            vec![
                user_row("複", "ho̍k", 5, 1_000),
                user_row("複", "ho̍k", 9, 1_000),
            ],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();

        assert_eq!(two.predictions.len(), 1, "still one prediction");
        assert_eq!(
            two.predictions[0].score, one.predictions[0].score,
            "the second reading's row adds nothing — not its count, not a bonus",
        );
    }

    /// The row kept is the FIRST, because the platform SQL orders user rows by
    /// how well `prev_tl` matches the reading being typed. Here the first row
    /// has the LOWER count, so keeping it proves order beats magnitude.
    #[test]
    fn first_user_row_wins_regardless_of_the_later_row_count() {
        let state = PersistedState::default();
        let config = config_tl_mode_hanji_first(false);
        let expected = filter(
            &state,
            vec![user_row("複", "ho̍k", 2, 1_000)],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();
        let result = filter(
            &state,
            vec![
                user_row("複", "ho̍k", 2, 1_000),
                user_row("複", "ho̍k", 99, 1_000),
            ],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();

        assert_eq!(result.predictions[0].score, expected.predictions[0].score);
    }

    /// A dict row between the two user rows must still be added, and must not
    /// let the second user row through. Pins that the cap tracks "has a user
    /// contribution", not "the previous row was a user row".
    #[test]
    fn a_dict_row_between_two_user_rows_changes_nothing_about_the_cap() {
        let state = PersistedState::default();
        let config = config_tl_mode_hanji_first(false);
        let expected = filter(
            &state,
            vec![user_row("好", "hó", 2, 1_000), dict_row("好", "hó", 7)],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();
        let result = filter(
            &state,
            vec![
                user_row("好", "hó", 2, 1_000),
                dict_row("好", "hó", 7),
                user_row("好", "hó", 99, 1_000),
            ],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();

        assert_eq!(result.predictions[0].score, expected.predictions[0].score);
    }

    /// A row that already has a user contribution must not short-circuit source
    /// validation: an invalid source is still an error, never a silent skip.
    #[test]
    fn invalid_source_after_a_user_collision_still_errors() {
        let state = PersistedState::default();
        let bad = RawNextWordPrediction {
            hanji: "好".to_owned(),
            tl: "hó".to_owned(),
            count: 1,
            last_used_ms: 0,
            source: Source::Unspecified as i32,
        };
        let err = filter(
            &state,
            vec![
                user_row("好", "hó", 2, 1_000),
                user_row("好", "hó", 3, 1_000),
                bad,
            ],
            0,
            1_000,
            10,
            &config_tl_mode_hanji_first(false),
        )
        .unwrap_err();
        assert!(matches!(err, NextWordError::InvalidSource));
    }

    /// Documents a KNOWN LIMIT, unchanged by the per-key cap: two user rows
    /// that are separator/tone-only variants of one word (`tâi-gí` vs the raw
    /// `taigi` a pre-v3.6.1 continuous commit stored) are two exact keys, so
    /// each keeps its own user contribution, and `collapse_reading_variants`
    /// then folds both scores into the surviving row. The cap is per exact
    /// `(hanji, tl)`, not per final displayed prediction. The old SQL subquery
    /// did not prevent this either — it deduped the same `(next_word, next_tl)`
    /// — so this is not a regression, and #383's canonical-TL write fix stops
    /// NEW fragmentation from appearing.
    #[test]
    fn collapse_still_folds_two_user_variants_of_one_word() {
        let state = PersistedState::default();
        let config = config_tl_mode_hanji_first(false);
        let single = filter(
            &state,
            vec![user_row("台語", "tâi-gí", 3, 1_000)],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();
        let variants = filter(
            &state,
            vec![
                user_row("台語", "tâi-gí", 3, 1_000),
                user_row("台語", "taigi", 3, 1_000),
            ],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();

        assert_eq!(variants.predictions.len(), 1, "still one displayed word");
        assert!(
            variants.predictions[0].score > single.predictions[0].score,
            "the folded variant's score is still added — known limit, see doc",
        );
    }

    /// The cap is user+user only. A dict row and a user row for the same word
    /// still sum — that IS the design, a learned word outranking the same word
    /// from the dictionary.
    #[test]
    fn dict_and_user_scores_still_sum_for_one_prediction() {
        let state = PersistedState::default();
        let config = config_tl_mode_hanji_first(false);
        let user_only = filter(
            &state,
            vec![user_row("好", "hó", 5, 1_000)],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();
        let both = filter(
            &state,
            vec![dict_row("好", "hó", 7), user_row("好", "hó", 5, 1_000)],
            0,
            1_000,
            10,
            &config,
        )
        .unwrap();

        assert_eq!(both.predictions.len(), 1);
        assert!(
            both.predictions[0].score > user_only.predictions[0].score,
            "the dict row still contributes on top of the user row",
        );
    }

    /// §8: every learned prediction outranks every bundled-only one, however
    /// large the corpus count — even when two bundled reading variants of one
    /// word fold together (`tâi-gí` + raw `taigi`, 2 × 2137 before the cap).
    /// Capped bundled ties keep their incoming (corpus) order.
    #[test]
    fn learned_prediction_outranks_any_bundled_prediction() {
        let state = PersistedState::default();
        let config = config_tl_mode_hanji_first(true);
        let now = 1_000_000_000_000;
        let a_year_ago = now - 365 * 24 * 3_600_000;
        let result = filter(
            &state,
            vec![
                dict_row("台語", "tâi-gí", 2137),
                dict_row("台語", "taigi", 2137),
                dict_row("灣", "uân", 900),
                dict_row("北", "pak", 400),
                user_row("南", "lâm", 1, a_year_ago),
            ],
            0,
            now,
            10,
            &config,
        )
        .unwrap();

        let hanji: Vec<&str> = result
            .predictions
            .iter()
            .map(|p| p.hanji.as_str())
            .collect();
        assert_eq!(hanji, vec!["南", "台語", "灣", "北"]);
        assert_eq!(result.predictions[1].score, scorer::DICT_SCORE_CAP);
    }

    /// Two readings of the NEXT word are different predictions, not duplicates
    /// — the cap keys on `(hanji, tl)` and must not collapse them.
    #[test]
    fn different_next_tl_stays_two_predictions() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                user_row("重", "tāng", 4, 1_000),
                user_row("重", "tîng", 3, 1_000),
            ],
            0,
            1_000,
            10,
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();

        assert_eq!(result.predictions.len(), 2);
    }

    // INVARIANT_NEXTWORD_LATE_PREDICTION_IS_DISCARDED (nextword-engine-boundary.md §10)
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
            &config_tl_mode_hanji_first(false),
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert!(!result.was_stale);
        assert_eq!(result.predictions.len(), 1);
        assert_eq!(result.predictions[0].hanji, "好");
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        let p = &result.predictions[0];
        let expected = scorer::score_dict(5) + scorer::calculate_user_score(1, 1_000, 1_000);
        assert!((p.score - expected).abs() < 1e-9);
    }

    #[test]
    fn empty_hanji_rows_dropped() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![
                RawNextWordPrediction {
                    hanji: "".to_owned(),
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
            &config_tl_mode_hanji_first(false),
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
                hanji: "好".to_owned(),
                tl: "hó".to_owned(),
                count: 5,
                last_used_ms: 0,
                source: Source::Unspecified as i32,
            }],
            0,
            1_000,
            10,
            &config_tl_mode_hanji_first(false),
        );
        assert!(matches!(result, Err(NextWordError::InvalidSource)));
    }

    #[test]
    fn invalid_source_int_returns_invalid_source() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![RawNextWordPrediction {
                hanji: "好".to_owned(),
                tl: "hó".to_owned(),
                count: 5,
                last_used_ms: 0,
                source: 99, // garbage
            }],
            0,
            1_000,
            10,
            &config_tl_mode_hanji_first(false),
        );
        assert!(matches!(result, Err(NextWordError::InvalidSource)));
    }

    // INVARIANT_NEXTWORD_PREDICTION_FILTER_HIDES_EMPTY_TL_IN_ROMAN_MODE (nextword-engine-boundary.md §10)
    #[test]
    fn empty_roman_dropped_in_roman_mode() {
        let state = PersistedState::default();
        let result = filter(
            &state,
            vec![dict_row("好", "", 5)],
            0,
            1_000,
            10,
            &config_tl_mode_hanji_first(false), // not Hanji mode
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
            &config_tl_mode_hanji_first(true), // Hanji mode
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        let p = &result.predictions[0];
        assert_eq!(p.text, "好"); // hanji as fallback
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
    fn poj_mode_shows_a_han_lo_romanization_in_poj_keyed_by_the_stored_spelling() {
        // trace: `tsiânn` → POJ `chiâⁿ` (`tl_roman_in_hanji_to_poj`); the
        // production row `親tsiânn` reads `chhin-chiâⁿ` in its `poj` column.
        let shaped = |config: &AppConfig| {
            let result = filter(
                &PersistedState::default(),
                vec![dict_row("親tsiânn", "tshin-tsiânn", 5)],
                0,
                1_000,
                10,
                config,
            )
            .unwrap();
            let p = result.predictions[0].clone();
            (p.hanji, p.display_text)
        };
        assert_eq!(
            shaped(&config_poj_mode()),
            ("親chi\u{e2}\u{207f}".to_owned(), "親tsiânn".to_owned())
        );
        assert_eq!(
            shaped(&config_tl_mode_hanji_first(false)),
            ("親tsiânn".to_owned(), "親tsiânn".to_owned())
        );
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 3);
        // user "早" has highest score (~550 from learning bonus + decay)
        assert_eq!(result.predictions[0].hanji, "早");
        // Among dicts: "安" (score=3) > "好" (score=1)
        assert_eq!(result.predictions[1].hanji, "安");
        assert_eq!(result.predictions[2].hanji, "好");
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
            &config_tl_mode_hanji_first(false),
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            crate::api::DEFAULT_PREDICTION_LIMIT
        );
    }

    // Pins behavioral-invariants.md §24 INVARIANT_NEXTWORD_READ_LAYER_DEDUP.

    // trace: continuous raw next_tl "taigi" (no separator) + normal canonical
    // "tâi-gí" (hyphen separator) for 台語. (hanji, tl) merge keeps them as 2
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            1,
            "fused raw folds into separator canonical"
        );
        let p = &result.predictions[0];
        assert_eq!(p.hanji, "台語");
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
            vec![
                dict_row("空安", "khong-an", 4),
                dict_row("空安", "khongan", 2),
            ],
            0,
            1_000,
            10,
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(result.predictions.len(), 1);
        assert_eq!(
            result.predictions[0].tl, "khong-an",
            "separator form is canonical"
        );
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            1,
            "separator + raw variants of one reading collapse"
        );
        assert_eq!(result.predictions[0].hanji, "予我");
    }

    // trace (Codex post-impl P1): genuine polyphonic single-syllable readings
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
            &config_tl_mode_hanji_first(false),
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            3,
            "ambiguous polyphone group must not collapse (Core Principle #6)",
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
            &config_tl_mode_hanji_first(false),
        )
        .unwrap();
        assert_eq!(
            result.predictions.len(),
            2,
            "distinct multi-syllable readings not collapsed"
        );
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
            let hanji = format!("詞{}", i);
            raw.push(dict_row(
                &hanji,
                &format!("ts\u{00e1}-{}", i),
                (i + 1) as i64,
            )); // separator canonical
            raw.push(dict_row(&hanji, &format!("tsa{}", i), 1)); // fused raw variant
        }
        let result = filter(&state, raw, 0, 1_000, 3, &config_tl_mode_hanji_first(false)).unwrap();
        assert_eq!(
            result.predictions.len(),
            3,
            "collapse then truncate to limit"
        );
        for p in &result.predictions {
            assert!(
                has_separator(&p.tl),
                "survivors are separator-bearing canonicals, not fused raw variants",
            );
        }
    }

    /// §44 — Romanization Only cells hide the hanji, so homophone predictions (distinct
    /// after the `(hanji, tl)` merge) collapse to the best-scored one;
    /// side-by-side keeps both.
    #[test]
    fn roman_only_display_collapses_same_roman_predictions_keeping_the_best() {
        let state = PersistedState::default();
        let rows = || vec![dict_row("食", "tsia̍h", 9), dict_row("𤆬", "tsia̍h", 3)];
        let collapsed = filter(&state, rows(), 0, 0, 10, &config_roman_only_display()).unwrap();
        assert_eq!(collapsed.predictions.len(), 1);
        assert_eq!(
            collapsed.predictions[0].hanji, "食",
            "first-seen after the score sort"
        );
        assert_eq!(collapsed.predictions[0].text, "tsia̍h");

        let kept = filter(&state, rows(), 0, 0, 10, &config_tl_mode_hanji_first(false)).unwrap();
        assert_eq!(
            kept.predictions.len(),
            2,
            "side-by-side keeps 同音異字 apart"
        );
    }

    /// §42 — under Hanji with Romanization a prediction is a Hanji cell then a
    /// romanization cell, deduped per script; the limit counts predictions,
    /// not cells; the TPS layout and the other displays list paired rows.
    #[test]
    fn combined_display_splits_predictions_into_single_script_cells() {
        let state = PersistedState::default();
        let combined = |input_mode: &str| AppConfig {
            input_mode: input_mode.to_owned(),
            candidate_display_mode: CandidateDisplayMode::Combined as i32,
            ..config_tl_mode_hanji_first(true)
        };
        // trace: scores 食 9 > 𤆬 3 > 台灣 2 → 食 Hanji, tsia̍h roman, 𤆬 Hanji
        // (tsia̍h already shown); 台灣 truncated by limit 2.
        let rows = || {
            vec![
                dict_row("食", "tsia̍h", 9),
                dict_row("𤆬", "tsia̍h", 3),
                dict_row("台灣", "tâi-uân", 2),
            ]
        };
        let split = filter(&state, rows(), 0, 0, 2, &combined("tl")).unwrap();
        let cells: Vec<(&str, &str, CellScript)> = split
            .predictions
            .iter()
            .map(|p| (p.hanji.as_str(), p.text.as_str(), p.cell_script()))
            .collect();
        assert_eq!(
            cells,
            vec![
                ("食", "tsia̍h", CellScript::Hanji),
                ("食", "tsia̍h", CellScript::Roman),
                ("𤆬", "tsia̍h", CellScript::Hanji),
            ]
        );

        for config in [combined("tps"), config_tl_mode_hanji_first(true)] {
            let paired = filter(&state, rows(), 0, 0, 2, &config).unwrap();
            assert_eq!(paired.predictions.len(), 2, "{config:?}");
            assert!(paired
                .predictions
                .iter()
                .all(|p| p.cell_script() == CellScript::Paired));
        }
    }

    fn separated(separator: SyllableSeparator, config: AppConfig) -> AppConfig {
        AppConfig {
            syllable_separator: separator as i32,
            ..config
        }
    }

    /// The Syllable Separator shapes only the cell text; `tl` stays the
    /// association key the platform hands back on tap, in TL and POJ mode
    /// alike.
    #[test]
    fn syllable_separator_shapes_prediction_text_but_not_its_tl_key() {
        let state = PersistedState::default();
        let rows = || {
            vec![
                dict_row("台灣", "tâi-uân", 9),
                dict_row("予我", "hōo--guá", 3),
            ]
        };
        let tl = separated(SyllableSeparator::None, config_tl_mode_hanji_first(false));
        let shaped = filter(&state, rows(), 0, 0, 10, &tl).unwrap();
        assert_eq!(shaped.predictions[0].text, "tâiuân");
        assert_eq!(shaped.predictions[0].tl, "tâi-uân");
        assert_eq!(shaped.predictions[1].text, "hōo\u{00b7}guá");
        assert_eq!(shaped.predictions[1].tl, "hōo--guá");

        let spaced = separated(SyllableSeparator::Space, config_tl_mode_hanji_first(false));
        let shaped = filter(&state, rows(), 0, 0, 10, &spaced).unwrap();
        assert_eq!(shaped.predictions[0].text, "tâi uân");
        assert_eq!(shaped.predictions[0].tl, "tâi-uân");
        assert_eq!(shaped.predictions[1].text, "hōo \u{00b7}guá");
        assert_eq!(shaped.predictions[1].tl, "hōo--guá");

        let poj = separated(SyllableSeparator::None, config_poj_mode());
        let shaped = filter(&state, rows(), 0, 0, 10, &poj).unwrap();
        assert_eq!(shaped.predictions[0].text, "tâioân");
        assert_eq!(shaped.predictions[0].tl, "tâi-uân");
    }

    /// The TPS layout as `"tps"` with the stored flags shapes exactly what
    /// `"tl"` with the separator folded to Hyphen and the stored swap shapes:
    /// TL text, hyphens kept, and the empty-roman drop still reading the
    /// stored swap.
    #[test]
    fn tps_layout_shapes_predictions_like_the_legacy_tl_wire() {
        let state = PersistedState::default();
        let rows = || {
            vec![
                dict_row("台灣", "tâi-uân", 9),
                dict_row("予我", "hōo--guá", 3),
                dict_row("彼", "", 1),
            ]
        };
        for swapped in [false, true] {
            let legacy = config_tl_mode_hanji_first(swapped);
            let expected = filter(&state, rows(), 0, 0, 10, &legacy).unwrap();
            for separator in [
                SyllableSeparator::Hyphen,
                SyllableSeparator::None,
                SyllableSeparator::Space,
            ] {
                let tps = AppConfig {
                    input_mode: "tps".to_owned(),
                    syllable_separator: separator as i32,
                    ..config_tl_mode_hanji_first(swapped)
                };
                let shaped = filter(&state, rows(), 0, 0, 10, &tps).unwrap();
                assert_eq!(
                    shaped, expected,
                    "swapped={swapped} separator={separator:?}"
                );
                assert_eq!(shaped.predictions[0].text, "tâi-uân");
                assert_eq!(shaped.predictions[1].text, "hōo--guá");
                // trace: `彼` has no roman — kept (as its hanji) only when swapped.
                let kept = shaped.predictions.iter().any(|p| p.text == "彼");
                assert_eq!(kept, swapped, "swapped={swapped}");
            }
        }
    }

    /// The collapse runs before the limit so the strip is filled with
    /// distinct cells, not with a duplicate that then hides the next word.
    #[test]
    fn roman_only_display_dedupe_runs_before_limit_truncation() {
        let state = PersistedState::default();
        let rows = vec![
            dict_row("食", "tsia̍h", 9),
            dict_row("𤆬", "tsia̍h", 8),
            dict_row("好", "hó", 1),
        ];
        let result = filter(&state, rows, 0, 0, 2, &config_roman_only_display()).unwrap();
        let texts: Vec<&str> = result.predictions.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["tsia̍h", "hó"]);
    }

    #[test]
    fn khinsiann_in_hanji_transforms_prediction_hanji_and_subtitle() {
        let state = PersistedState::default();
        let rows = vec![
            dict_row("會記得", "ē-kì--tit", 9),
            dict_row("轉去", "--tńg-khì", 5),
            dict_row("邊--a", "pinn--a", 3),
        ];
        let mut cfg = config_tl_mode_hanji_first(true);
        cfg.khinsiann_in_hanji = true;
        let res = filter(&state, rows, 0, 0, 10, &cfg).unwrap();
        let got: Vec<(&str, &str, &str)> = res
            .predictions
            .iter()
            .map(|p| (p.hanji.as_str(), p.subtitle.as_str(), p.tl.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("會記--得", "會記--得", "ē-kì--tit"),
                ("--轉-去", "--轉-去", "--tńg-khì"),
                ("邊--a", "邊--a", "pinn--a"),
            ]
        );
    }

    #[test]
    fn khinsiann_in_hanji_controls_reading_variant_collapse_without_merging_homographs() {
        let state = PersistedState::default();
        let rows = || {
            vec![
                dict_row("轉去", "tńg--khì", 9),
                dict_row("轉去", "tńg-khì", 5),
                dict_row("重", "tîng", 4),
                dict_row("重", "tāng", 3),
                dict_row("好", "hó", 1),
            ]
        };

        // khinsiann_in_hanji = false: `轉去/tńg--khì` and `轉去/tńg-khì` share the
        // same reading and rendered Hanji so `collapse_reading_variants` folds them,
        // while distinct `(漢字, TL)` homographs `重/tîng` and `重/tāng` stay distinct.
        let plain = filter(&state, rows(), 0, 0, 10, &config_tl_mode_hanji_first(true)).unwrap();
        let pairs_plain: Vec<(&str, &str)> = plain
            .predictions
            .iter()
            .map(|p| (p.hanji.as_str(), p.tl.as_str()))
            .collect();
        assert_eq!(
            pairs_plain,
            vec![
                ("轉去", "tńg--khì"),
                ("重", "tîng"),
                ("重", "tāng"),
                ("好", "hó"),
            ]
        );

        // khinsiann_in_hanji = true: `轉--去` vs `轉去` render distinct Hanji, so both survive.
        let mut khinsiann = config_tl_mode_hanji_first(true);
        khinsiann.khinsiann_in_hanji = true;
        let shown = filter(&state, rows(), 0, 0, 10, &khinsiann).unwrap();
        let pairs_shown: Vec<(&str, &str)> = shown
            .predictions
            .iter()
            .map(|p| (p.hanji.as_str(), p.tl.as_str()))
            .collect();
        assert_eq!(
            pairs_shown,
            vec![
                ("轉--去", "tńg--khì"),
                ("轉去", "tńg-khì"),
                ("重", "tîng"),
                ("重", "tāng"),
                ("好", "hó"),
            ]
        );
    }

    fn han_lo(config: AppConfig) -> AppConfig {
        AppConfig {
            han_lo_mixed: true,
            ..config
        }
    }

    /// `(hanji, text, subtitle, display_text)` per prediction, list order.
    fn shown(rows: Vec<RawNextWordPrediction>, config: &AppConfig) -> Vec<[String; 4]> {
        filter(&PersistedState::default(), rows, 0, 1_000, 10, config)
            .unwrap()
            .predictions
            .into_iter()
            .map(|p| [p.hanji, p.text, p.subtitle, p.display_text])
            .collect()
    }

    fn cell(hanji: &str, text: &str, subtitle: &str, display_text: &str) -> [String; 4] {
        [hanji, text, subtitle, display_text].map(str::to_owned)
    }

    // Hàn-Lô lām: each reading's romanization is a prediction of its own,
    // priced at e⁻¹ of its best Hanji's capped score — 食 100 → 36.8 sorts
    // between 隻 40 and 𤆬 30. Off, nothing changes.
    #[test]
    fn han_lo_lists_each_readings_romanization_as_a_prediction() {
        let rows = || {
            vec![
                dict_row("食", "tsia̍h", 100),
                dict_row("隻", "tsiah", 40),
                dict_row("𤆬", "tsia̍h", 30),
            ]
        };
        let on = han_lo(config_tl_mode_hanji_first(true));
        assert_eq!(
            shown(rows(), &on),
            vec![
                cell("食", "tsia̍h", "食", "食"),
                cell("隻", "tsiah", "隻", "隻"),
                cell("", "tsia̍h", "", "tsia̍h"),
                cell("𤆬", "tsia̍h", "𤆬", "𤆬"),
                cell("", "tsiah", "", "tsiah"),
            ]
        );
        assert_eq!(shown(rows(), &config_tl_mode_hanji_first(true)).len(), 3);
        // Romanization Only already lists every word as its romanization.
        let roman_only = han_lo(config_roman_only_display());
        assert!(shown(rows(), &roman_only)
            .iter()
            .all(|[hanji, ..]| !hanji.is_empty()));
    }

    // A learned romanization (the `(TL, TL)` pair a romanization tap stores)
    // is that reading's prediction with its own score, ahead of the Hanji it
    // was learned instead of, and the reading gets no second one.
    #[test]
    fn han_lo_ranks_a_learned_romanization_on_its_own_score() {
        let on = han_lo(config_tl_mode_hanji_first(true));
        assert_eq!(
            shown(
                vec![
                    dict_row("食", "tsia̍h", 100),
                    user_row("tsia̍h", "tsia̍h", 1, 1_000),
                ],
                &on
            ),
            vec![
                cell("", "tsia̍h", "", "tsia̍h"),
                cell("食", "tsia̍h", "食", "食"),
            ]
        );
    }

    // POJ shows the romanization in POJ; the identity stays TL.
    #[test]
    fn han_lo_shows_the_romanization_in_poj_under_poj() {
        let on = han_lo(config_poj_mode());
        assert_eq!(
            shown(vec![dict_row("食", "tsia̍h", 100)], &on),
            vec![
                cell("食", "chia̍h", "食", "食"),
                cell("", "chia̍h", "", "tsia̍h"),
            ]
        );
    }

    // Hanji with Romanization: a Hanji prediction is a Hanji cell only, its
    // reading's romanization prediction the romanization cell.
    #[test]
    fn han_lo_splits_hanji_predictions_into_hanji_cells_only() {
        let on = AppConfig {
            candidate_display_mode: CandidateDisplayMode::Combined as i32,
            ..han_lo(config_tl_mode_hanji_first(true))
        };
        let cells: Vec<(String, CellScript)> = filter(
            &PersistedState::default(),
            vec![dict_row("食", "tsia̍h", 100)],
            0,
            0,
            10,
            &on,
        )
        .unwrap()
        .predictions
        .into_iter()
        .map(|p| {
            let script = p.cell_script();
            (p.text, script)
        })
        .collect();
        assert_eq!(
            cells,
            vec![
                ("tsia̍h".to_owned(), CellScript::Hanji),
                ("tsia̍h".to_owned(), CellScript::Roman),
            ]
        );
    }

    fn cells(
        rows: Vec<RawNextWordPrediction>,
        limit: i32,
        config: &AppConfig,
    ) -> Vec<(String, CellScript)> {
        filter(&PersistedState::default(), rows, 0, 0, limit, config)
            .unwrap()
            .predictions
            .into_iter()
            .map(|p| {
                let script = p.cell_script();
                let shown = if script == CellScript::Hanji {
                    p.hanji
                } else {
                    p.text
                };
                (shown, script)
            })
            .collect()
    }

    // A Latin-spelled word with a reading of its own (`KTV` / `khe-thi-bui`)
    // is a word, not a romanization: it keeps its spelling, and its reading
    // gets its own romanization prediction.
    #[test]
    fn han_lo_keeps_a_latin_spelled_word_as_a_word() {
        let on = han_lo(config_tl_mode_hanji_first(true));
        assert_eq!(
            shown(vec![user_row("KTV", "khe-thi-bui", 1, 1_000)], &on),
            vec![
                cell("KTV", "khe-thi-bui", "KTV", "KTV"),
                cell("", "khe-thi-bui", "", "khe-thi-bui"),
            ]
        );
    }

    // A romanization learned while the switch was on stays a
    // romanization-only prediction once it is off — never `tsia̍h / tsia̍h`
    // — and the TPS layout, which shows no romanization, drops it.
    #[test]
    fn a_learned_romanization_stays_romanization_only_with_the_switch_off() {
        let rows = || {
            vec![
                dict_row("食", "tsia̍h", 100),
                user_row("tsia̍h", "tsia̍h", 1, 1_000),
            ]
        };
        assert_eq!(
            shown(rows(), &config_tl_mode_hanji_first(true)),
            vec![
                cell("", "tsia̍h", "", "tsia̍h"),
                cell("食", "tsia̍h", "食", "食"),
            ]
        );
        let tps = AppConfig {
            input_mode: "tps".to_owned(),
            ..config_tl_mode_hanji_first(true)
        };
        assert_eq!(
            shown(rows(), &tps)
                .into_iter()
                .map(|[hanji, ..]| hanji)
                .collect::<Vec<_>>(),
            vec!["食".to_owned()]
        );
        // Hanji with Romanization: one romanization cell, one Hanji cell.
        let combined = AppConfig {
            candidate_display_mode: CandidateDisplayMode::Combined as i32,
            ..config_tl_mode_hanji_first(true)
        };
        assert_eq!(
            cells(rows(), 10, &combined),
            vec![
                ("tsia̍h".to_owned(), CellScript::Roman),
                ("食".to_owned(), CellScript::Hanji),
            ]
        );
    }

    // Hanji with Romanization counts the limit in Hanji predictions, and a
    // Hanji prediction whose reading's romanization did not make the list
    // keeps its own romanization cell.
    #[test]
    fn han_lo_combined_counts_the_limit_in_hanji_predictions() {
        let on = AppConfig {
            candidate_display_mode: CandidateDisplayMode::Combined as i32,
            ..han_lo(config_tl_mode_hanji_first(true))
        };
        let rows = vec![
            dict_row("食", "tsia̍h", 100),
            dict_row("隻", "tsiah", 40),
            dict_row("甲", "kah", 30),
        ];
        // Limit 2: 食, 隻 and the `tsia̍h` romanization (36.8, ahead of 甲)
        // are kept; 隻's romanization (14.7) is not, so 隻 keeps its cell.
        assert_eq!(
            cells(rows, 2, &on),
            vec![
                ("食".to_owned(), CellScript::Hanji),
                ("隻".to_owned(), CellScript::Hanji),
                ("tsiah".to_owned(), CellScript::Roman),
                ("tsia̍h".to_owned(), CellScript::Roman),
            ]
        );
    }
}
