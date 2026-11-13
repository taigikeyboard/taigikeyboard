//! How the pending buffer is read and cut, from the slot-0 walk.

use super::recase::recase_roman;
use super::walk::walk_buffer;

use crate::shadow::{build_continuous_keys, greedy_longest_syllabification, strip_tones_for_mode};
use lexicon::{ContinuousFetchCtx, CustomEntry, EngineHandle as LexiconHandle, LearnedEntry};
use ranking::FrequencyMap;

/// One piece of the typed buffer, in order. `gap` is the stretch the
/// syllable grid or the walk did not cover (a trailing hyphen, an unfinished
/// onset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BufferCut {
    pub text: String,
    pub gap: bool,
}

/// One word of the cut the engine shows: the walker's lowest-cost cover, or
/// the syllable reading when no dictionary word is on that path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WordCut {
    pub text: String,
    pub hanji: Option<String>,
    pub roman: String,
    pub kind: &'static str,
    pub user_row: bool,
    pub picks: i32,
    pub weight: f64,
}

/// How the pending buffer is read and cut. `cut_by` is `walker` (a dictionary
/// path won), `syllables` (nothing in the dictionary covers it, so the
/// reading stays split) or `none`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Segmentation {
    pub cut_by: &'static str,
    pub syllables: Vec<BufferCut>,
    pub words: Vec<WordCut>,
}

impl Default for Segmentation {
    fn default() -> Self {
        Self {
            cut_by: "none",
            syllables: Vec::new(),
            words: Vec::new(),
        }
    }
}

/// The syllable grid and the sentence cut for `raw`, from the same walk the
/// candidate list's slot 0 uses. An unfinished buffer has syllables (or
/// neither) and no word cut. The arguments are the fetch's own, the same
/// set [`assemble_candidates`] takes.
///
/// [`assemble_candidates`]: super::assemble::assemble_candidates
#[allow(clippy::too_many_arguments)]
pub(crate) fn explain_segmentation(
    raw: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
    custom: &[CustomEntry],
    learned: &[LearnedEntry],
    mode: phonetics::InputMode,
    enabled_sources_bitmask: u32,
    context: &ranking::ContextRanks,
) -> Segmentation {
    LexiconHandle::with_state(|state| {
        let Some(inv) = state.syllable_inventory.as_ref() else {
            return Ok(Segmentation::default());
        };
        let keys = build_continuous_keys(raw, inv, mode);
        let syllables = cuts_from_shadow(raw, &keys.shadow_to_raw_end, &keys.one_syllable_segments);
        let Some(prefix) = state.prefix_index.as_ref() else {
            return Ok(Segmentation {
                syllables,
                ..Segmentation::default()
            });
        };
        let Some(dict) = state.dictionary.as_ref() else {
            return Ok(Segmentation {
                syllables,
                ..Segmentation::default()
            });
        };
        let ctx = ContinuousFetchCtx {
            enabled_sources_bitmask,
            freq_map,
            now_ms,
            context,
            custom,
            learned,
            prefix_index: prefix,
            dict,
            mode,
            tone_pin: crate::shadow::whole_buffer_tone_pin(raw, mode, Some(inv)),
            whole_buffer_key: None,
        };
        let Some(path) = walk_buffer(&keys, inv, &ctx) else {
            return Ok(Segmentation {
                syllables,
                ..Segmentation::default()
            });
        };
        if path.choices.iter().any(|choice| choice.dict_hit) {
            let words = path
                .edges
                .iter()
                .zip(&path.choices)
                .filter_map(|(&(start, end), choice)| {
                    let (raw_start, raw_end) = raw_bounds(&keys.shadow_to_raw_end, start, end)?;
                    let text = raw.get(raw_start..raw_end)?.to_owned();
                    let roman = recase_roman(&choice.roman, &text, mode);
                    let display = choice
                        .hanji
                        .clone()
                        .unwrap_or_else(|| choice.canonical_tl.clone());
                    let picks = freq_map.get(&display, &choice.canonical_tl);
                    Some(WordCut {
                        text,
                        hanji: choice.hanji.clone(),
                        roman,
                        kind: if choice.is_custom {
                            "custom"
                        } else if choice.dict_hit {
                            "dictionary"
                        } else {
                            "reading"
                        },
                        user_row: choice.user_row,
                        picks: picks.count,
                        weight: choice.user_weight_delta,
                    })
                })
                .collect();
            return Ok(Segmentation {
                cut_by: "walker",
                syllables,
                words,
            });
        }
        let barriers =
            crate::shadow::oov_reading_barriers(&keys.barriers, 0, keys.shadow.len(), mode);
        let Some(segs) = greedy_longest_syllabification(&keys.shadow, inv, mode, &barriers) else {
            return Ok(Segmentation {
                syllables,
                ..Segmentation::default()
            });
        };
        let words: Vec<WordCut> = segs
            .iter()
            .filter_map(|&(start, end)| {
                let (raw_start, raw_end) = raw_bounds(&keys.shadow_to_raw_end, start, end)?;
                let text = raw.get(raw_start..raw_end)?.to_owned();
                let roman = strip_tones_for_mode(&keys.shadow[start..end], mode);
                Some(WordCut {
                    text,
                    hanji: None,
                    roman,
                    kind: "reading",
                    user_row: false,
                    picks: 0,
                    weight: 0.0,
                })
            })
            .collect();
        Ok(Segmentation {
            cut_by: if words.is_empty() {
                "none"
            } else {
                "syllables"
            },
            syllables,
            words,
        })
    })
    .unwrap_or_default()
}

fn raw_bounds(map: &[usize], start: usize, end: usize) -> Option<(usize, usize)> {
    let raw_start = *map.get(start)?;
    let raw_end = *map.get(end)?;
    (raw_start <= raw_end).then_some((raw_start, raw_end))
}

fn cuts_from_shadow(raw: &str, map: &[usize], spans: &[(usize, usize)]) -> Vec<BufferCut> {
    let mut cuts = Vec::new();
    let mut covered = 0usize;
    for &(start, end) in spans {
        let Some((raw_start, raw_end)) = raw_bounds(map, start, end) else {
            continue;
        };
        if raw_start > covered {
            if let Some(gap) = raw.get(covered..raw_start) {
                if !gap.is_empty() {
                    cuts.push(BufferCut {
                        text: gap.to_owned(),
                        gap: true,
                    });
                }
            }
        }
        if let Some(text) = raw.get(raw_start..raw_end) {
            if !text.is_empty() {
                cuts.push(BufferCut {
                    text: text.to_owned(),
                    gap: false,
                });
            }
        }
        covered = covered.max(raw_end);
    }
    if let Some(tail) = raw.get(covered..) {
        if !tail.is_empty() {
            cuts.push(BufferCut {
                text: tail.to_owned(),
                gap: true,
            });
        }
    }
    cuts
}
