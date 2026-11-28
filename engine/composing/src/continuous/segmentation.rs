//! How the pending buffer is read and cut, from the slot-0 walk.

use super::recase::{recase_hanji, recase_roman, shows_poj_roman_in_hanji};
use super::walk::{edge_romans, sentence_hanji, SynthCut, SynthWords};

use crate::shadow::{strip_tones_for_mode, ContinuousKeys};
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
    pub khinsiann_before: bool,
}

/// How the pending buffer is read and cut. `cut_by` is `walker` (a dictionary
/// path won), `syllables` (nothing in the dictionary covers it, so the
/// reading stays split) or `none` (the list carries no slot-0 synth: an
/// unfinished buffer, or a synth the walker suppressed).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Segmentation {
    pub cut_by: &'static str,
    pub syllables: Vec<BufferCut>,
    pub words: Vec<WordCut>,
    /// The whole-sentence row's shown Hanji, built as slot 0 builds it
    /// ([`sentence_hanji`]) — a Hàn-Lô lām sentence's romanized words
    /// included — or `None` when the cut spells no Hanji.
    pub sentence_hanji: Option<String>,
}

impl Default for Segmentation {
    fn default() -> Self {
        Self {
            cut_by: "none",
            syllables: Vec::new(),
            words: Vec::new(),
            sentence_hanji: None,
        }
    }
}

/// The syllable grid and the sentence cut for `raw`, read off the fetch that
/// listed it (`Assembled::keys` / `Assembled::synth`): the view shows the cut
/// slot 0 was built from, or none when the list carries no synth (an
/// unfinished buffer, a suppressed synth, a several-word path in a word
/// list). It never walks the buffer itself.
#[allow(clippy::too_many_arguments)]
pub(crate) fn explain_segmentation(
    raw: &str,
    keys: Option<&ContinuousKeys>,
    synth: Option<&SynthCut>,
    freq_map: &FrequencyMap,
    mode: phonetics::InputMode,
    khinsiann_in_hanji: bool,
    prev_in_neutral: bool,
    at_buffer_start: bool,
) -> Segmentation {
    let Some(keys) = keys else {
        return Segmentation::default();
    };
    let syllables = cuts_from_shadow(raw, &keys.shadow_to_raw_end, &keys.one_syllable_segments);
    let Some(SynthCut { path, words, .. }) = synth else {
        return Segmentation {
            syllables,
            ..Segmentation::default()
        };
    };
    let edges = matches!(words, SynthWords::Edges);
    let sentence_hanji = edges
        .then(|| {
            let parts = edge_romans(raw, &keys.shadow_to_raw_end, path);
            sentence_hanji(
                path,
                &parts,
                raw,
                keys,
                mode,
                khinsiann_in_hanji,
                edges,
                prev_in_neutral,
                at_buffer_start,
            )
        })
        .flatten();
    let raw_leading_run = if at_buffer_start {
        crate::nailed::typed_separator_run(raw)
    } else {
        ""
    };
    let (cut_by, words): (&'static str, Vec<WordCut>) = match words {
        SynthWords::Edges => {
            let mut in_neutral = khinsiann_in_hanji && prev_in_neutral;
            let words = path
                .edges
                .iter()
                .zip(&path.choices)
                .enumerate()
                .filter_map(|(edge_idx, (&(start, end), choice))| {
                    let typed_run = keys
                        .hyphen_runs
                        .iter()
                        .find(|&&(b, _)| b == start)
                        .map_or(0, |&(_, run)| run);
                    let leading = if !khinsiann_in_hanji {
                        phonetics::api::LeadingKhinsiann::None
                    } else if typed_run >= 2 {
                        phonetics::api::LeadingKhinsiann::DoubleDash
                    } else if typed_run == 1 && in_neutral {
                        phonetics::api::LeadingKhinsiann::ContinueHyphen
                    } else {
                        phonetics::api::LeadingKhinsiann::None
                    };
                    in_neutral = khinsiann_in_hanji
                        && phonetics::api::ends_in_khinsiann(&choice.canonical_tl, leading);
                    let (raw_start, raw_end) = raw_bounds(&keys.shadow_to_raw_end, start, end)?;
                    let text = raw.get(raw_start..raw_end)?.to_owned();
                    let roman = recase_roman(&choice.roman, &text);
                    let display = choice
                        .hanji
                        .clone()
                        .unwrap_or_else(|| choice.canonical_tl.clone());
                    let lookup_key = phonetics::api::learning_hanji_key(&display);
                    let picks = freq_map.get(&lookup_key, &choice.canonical_tl);
                    // As the sentence row shows the word (`sentence_hanji`).
                    let hanji = choice.hanji.as_deref().map(|h| {
                        let rendered = phonetics::api::render_hanji_display_with_leading(
                            h,
                            &choice.canonical_tl,
                            khinsiann_in_hanji,
                            leading,
                            shows_poj_roman_in_hanji(mode, choice.is_custom),
                        );
                        let rendered =
                            if edge_idx == 0 && !raw_leading_run.is_empty() && typed_run > 0 {
                                match phonetics::api::apply_leading_literal_hyphens(
                                    &rendered,
                                    raw_leading_run,
                                ) {
                                    std::borrow::Cow::Owned(prefixed) => {
                                        std::borrow::Cow::Owned(prefixed)
                                    }
                                    std::borrow::Cow::Borrowed(_) => rendered,
                                }
                            } else {
                                rendered
                            };
                        recase_hanji(&rendered, &text, choice.is_custom).into_owned()
                    });
                    let khinsiann_before = typed_run >= 2;
                    Some(WordCut {
                        text,
                        hanji,
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
                        khinsiann_before,
                    })
                })
                .collect();
            ("walker", words)
        }
        SynthWords::Syllables(segs) => {
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
                        khinsiann_before: false,
                    })
                })
                .collect();
            (
                if words.is_empty() {
                    "none"
                } else {
                    "syllables"
                },
                words,
            )
        }
    };
    Segmentation {
        cut_by,
        syllables,
        words,
        sentence_hanji,
    }
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
