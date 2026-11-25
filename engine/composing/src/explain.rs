//! What a shell draws to show why the buffer was cut this way and why the
//! candidate list is in this order. Built from the same walk and the same
//! [`ranking::CandidateSortKey`] the list uses. The page renders it; it
//! does not re-rank.

use crate::api::{Engine, UserRows};
use crate::continuous::is_one_syllable_prefix_pick;
use crate::requests::Listed;
use lexicon::{RawCandidate, COVERAGE_KIND_ABBREV, COVERAGE_KIND_PARTIAL_PREFIX};
use protos::engine::AppConfig;
use ranking::{
    source_tier_rank, CandidateSortKey, CONTEXT_RANK_BUNDLED, CONTEXT_RANK_USER,
    WALKER_COST_UNPRICED,
};

/// One ordered piece of the typed buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferPiece {
    pub text: String,
    /// A stretch the cut did not take (an unfinished onset, a trailing hyphen).
    pub gap: bool,
}

/// One word of the sentence cut.
#[derive(Debug, Clone, PartialEq)]
pub struct WordPiece {
    /// The letters the user typed for this word.
    pub text: String,
    pub hanji: Option<String>,
    pub roman: String,
    /// `dictionary`, `custom`, `reading` or `gap`.
    pub kind: &'static str,
    /// A word the user saved or learned sits on this stretch, so the walk
    /// prices it as one word.
    pub user_row: bool,
    pub picks: i32,
    /// Decayed selection weight, `0.0` when this word was never picked.
    pub weight: f64,
}

/// One candidate row and the reason it sits where it does.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidatePiece {
    pub hanji: Option<String>,
    pub roman: String,
    /// Typed text before / inside / after the span this row would commit.
    pub before: String,
    pub head: String,
    pub rest: String,
    /// `sentence`, `literal` or `word`.
    pub role: &'static str,
    /// Why this row is ahead of the previous scored row: `sentence`,
    /// `sentence_after` (the walker's path, behind rows that stay ahead of
    /// it), `path_over_prefix_pick` (a one-word path the Step 4d post-pass
    /// moved ahead of one-syllable prefix picks), `behind_path` (a pick it
    /// moved behind that path), `literal`, `lead`, `picked_over_sentence`,
    /// or a [`ranking::CandidateSortKey`] dimension name.
    pub placed_by: &'static str,
    /// `picked`, `custom` or `rest`.
    pub preference: &'static str,
    /// `full`, `prefix` or `abbreviation`.
    pub coverage: &'static str,
    /// `learned`, `bundled` or `none` — whether it continues the previous word.
    pub context: &'static str,
    pub picks: i32,
    /// Syllables in this row's reading. The front slot can be taken only by a
    /// prefix of at least two.
    pub syllables: u8,
    pub weight: f64,
    /// `0` when the word was never picked.
    pub last_used_ms: i64,
    /// `whole` when the span reaches the end of the buffer the list orders,
    /// otherwise `front`. The same test as [`ranking::CandidateSortKey`]'s tier.
    pub tier: &'static str,
    pub score: f32,
    pub frequency: u32,
    /// `None` when the corpus does not price this row.
    pub walker_cost: Option<u16>,
    pub coverage_bytes: u32,
    /// [`ranking::source_tier_rank`].
    pub source_rank: u8,
    /// Every dictionary bit on the row, in bit order, or `custom`. Empty when
    /// the row has no dictionary record.
    pub sources: Vec<&'static str>,
    /// The source `source_rank` actually read: `custom`, one ranked source,
    /// or `other`.
    pub ranking_source: &'static str,
}

/// Dictionary bits in bitmask order. A custom row is `custom` alone.
pub(crate) fn source_ids(bitmask: u16, is_custom: bool) -> Vec<&'static str> {
    if is_custom {
        return vec!["custom"];
    }
    // Bit positions: `dictionary/common/source_bits.py` `SOURCE_BITS`.
    const BITS: &[(u16, &str)] = &[
        (1 << 0, "kautian"),
        (1 << 1, "taigitv"),
        (1 << 2, "itaigi"),
        (1 << 3, "sitbut"),
        (1 << 4, "taihoa"),
        (1 << 5, "taijit"),
        (1 << 6, "kungge"),
        (1 << 7, "stti"),
        (1 << 8, "khpoo"),
        (1 << 9, "khiin"),
        (1 << 10, "dev"),
        (1 << 11, "lkk"),
    ];
    BITS.iter()
        .filter(|(bit, _)| bitmask & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}

/// The name [`ranking::source_tier_rank`] selected. Ranks `1`–`4` are the
/// priority sources; every other dictionary bit shares `other`.
pub(crate) fn ranking_source(bitmask: u16, is_custom: bool) -> &'static str {
    match ranking::source_tier_rank(bitmask, is_custom) {
        0 => "custom",
        1 => "kautian",
        2 => "taigitv",
        3 => "stti",
        4 => "kungge",
        _ => "other",
    }
}

/// The open composition, as the inspector draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Mechanism {
    /// The buffer the cuts and the coverage bars index.
    pub raw: String,
    /// `walker`, `syllables` or `none`.
    pub cut_by: &'static str,
    pub syllables: Vec<BufferPiece>,
    pub words: Vec<WordPiece>,
    pub candidates: Vec<CandidatePiece>,
}

/// The open composition's cut and candidate order, for a shell that shows
/// the mechanism. `None` when nothing is being composed. The candidate rows
/// are `requests::listed_candidates`: the same list the fetch sends.
pub fn mechanism(
    engine: &Engine,
    now_ms: i64,
    user_rows: &UserRows,
    context: &ranking::ContextRanks,
    literal_roman_candidate_disabled: bool,
    config: &AppConfig,
) -> Option<Mechanism> {
    let listed = crate::requests::listed_candidates(
        engine,
        now_ms,
        user_rows,
        context,
        literal_roman_candidate_disabled,
        config,
    )?;
    let mut segmentation = if listed.blocked {
        crate::continuous::Segmentation::default()
    } else {
        crate::continuous::explain_segmentation(
            &listed.segment_raw,
            listed.keys.as_ref(),
            listed.synth.as_ref(),
            &user_rows.frequency,
            listed.mode,
            config.khinsiann_in_hanji,
        )
    };
    if listed.shift > 0 {
        if let Some(prefix) = listed.raw.get(..listed.shift as usize) {
            if !prefix.is_empty() {
                segmentation.syllables.insert(
                    0,
                    crate::continuous::BufferCut {
                        text: prefix.to_owned(),
                        gap: true,
                    },
                );
                segmentation.words.insert(
                    0,
                    crate::continuous::WordCut {
                        text: prefix.to_owned(),
                        hanji: None,
                        roman: String::new(),
                        kind: "gap",
                        user_row: false,
                        picks: 0,
                        weight: 0.0,
                        khinsiann_before: false,
                    },
                );
            }
        }
    }
    Some(explain_mechanism(
        &listed,
        &segmentation,
        &user_rows.frequency,
        config,
    ))
}

fn explain_mechanism(
    listed: &Listed,
    segmentation: &crate::continuous::Segmentation,
    frequency: &ranking::FrequencyMap,
    config: &AppConfig,
) -> Mechanism {
    let sentence = segmentation.sentence_hanji.as_deref().map(|hanji| {
        // The row as listed: Step 5 renders the Syllable Separator (§49) on
        // its romanized words and recases its nasal markers (§53).
        let joined = match config.rendered_syllable_joiner() {
            Some(joiner) => phonetics::api::syllable_joiner_in_hanji(hanji, joiner),
            None => std::borrow::Cow::Borrowed(hanji),
        };
        phonetics::case_transform::apply_nasal_marker_case(
            &joined,
            config.force_lowercase_nasal_marker,
        )
        .into_owned()
    });
    // One word is still the walker's path, but it is ranked with the rest
    // (`continuous::assemble` Step 4), so it is an ordinary row: tagging it
    // `sentence` would pull it back out of the twelve stages. A path of
    // several words stays a sentence, and so does a one-word path the Step 4d
    // post-pass moved ahead of one-syllable prefix picks — no sort field
    // placed it, so it reads `path_over_prefix_pick` and the picks it passed
    // `behind_path`.
    let one_word = segmentation
        .words
        .iter()
        .filter(|word| word.kind != "gap")
        .count()
        <= 1;
    let raw_len = listed.raw.len() as u32;
    let is_sentence = |candidate: &RawCandidate| {
        sentence
            .as_ref()
            .is_some_and(|hanji| candidate.hanji.as_deref() == Some(hanji))
    };
    let path_moved = one_word && listed.walker_moved;
    let sentence_at = if path_moved {
        listed
            .candidates
            .iter()
            .position(|candidate| is_sentence(candidate) && candidate.consumed_span.1 == raw_len)
    } else if one_word {
        None
    } else {
        listed.candidates.iter().position(is_sentence)
    };
    let mut previous: Option<CandidateSortKey> = None;
    let mut behind_path = false;
    let pieces: Vec<CandidatePiece> = listed
        .candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let role = if listed.literal_at == Some(index) {
                "literal"
            } else if sentence_at == Some(index) {
                "sentence"
            } else {
                "word"
            };
            behind_path = if role == "sentence" {
                path_moved
            } else {
                behind_path && is_one_syllable_prefix_pick(candidate)
            };
            let key = CandidateSortKey::new(&candidate.rank_facts(), raw_len, index as u32);
            let placed_by = match role {
                "literal" => "literal",
                "sentence" if path_moved => "path_over_prefix_pick",
                "sentence" if previous.is_some() => "sentence_after",
                "sentence" => "sentence",
                _ if behind_path => "behind_path",
                _ if !path_moved
                    && sentence_at.is_some_and(|at| at > index)
                    && previous.is_none() =>
                {
                    "picked_over_sentence"
                }
                _ => previous
                    .as_ref()
                    .map_or("lead", |ahead| ahead.deciding_field(&key)),
            };
            if role == "word" {
                previous = Some(key);
            }
            let (start, end) = candidate.consumed_span;
            let lookup_key = phonetics::api::learning_hanji_key(&candidate.display_text);
            let picks = frequency.get(&lookup_key, &candidate.canonical_tl);
            CandidatePiece {
                hanji: candidate.hanji.clone(),
                roman: candidate.roman.clone(),
                before: slice_raw(&listed.raw, 0, start),
                head: slice_raw(&listed.raw, start, end),
                rest: slice_raw(&listed.raw, end, raw_len),
                role,
                placed_by,
                preference: if candidate.user_weight > 0.0 {
                    "picked"
                } else if candidate.is_custom {
                    "custom"
                } else {
                    "rest"
                },
                coverage: match candidate.coverage_kind {
                    COVERAGE_KIND_PARTIAL_PREFIX => "prefix",
                    COVERAGE_KIND_ABBREV => "abbreviation",
                    _ => "full",
                },
                context: match candidate.context_rank {
                    CONTEXT_RANK_USER => "learned",
                    CONTEXT_RANK_BUNDLED => "bundled",
                    _ => "none",
                },
                picks: picks.count,
                syllables: candidate.syllable_count,
                weight: candidate.user_weight,
                last_used_ms: picks.last_used_ms,
                tier: if end == raw_len { "whole" } else { "front" },
                score: if candidate.score.is_finite() {
                    candidate.score
                } else {
                    0.0
                },
                frequency: candidate.frequency,
                walker_cost: (candidate.walker_cost != WALKER_COST_UNPRICED)
                    .then_some(candidate.walker_cost),
                coverage_bytes: end.saturating_sub(start),
                source_rank: source_tier_rank(candidate.bitmask, candidate.is_custom),
                sources: source_ids(candidate.bitmask, candidate.is_custom),
                ranking_source: ranking_source(candidate.bitmask, candidate.is_custom),
            }
        })
        .collect();
    Mechanism {
        raw: listed.raw.clone(),
        cut_by: if listed.blocked {
            "none"
        } else {
            segmentation.cut_by
        },
        syllables: segmentation
            .syllables
            .iter()
            .map(|cut| BufferPiece {
                text: cut.text.clone(),
                gap: cut.gap,
            })
            .collect(),
        words: segmentation
            .words
            .iter()
            .map(|word| WordPiece {
                text: word.text.clone(),
                hanji: word.hanji.clone(),
                roman: word.roman.clone(),
                kind: word.kind,
                user_row: word.user_row,
                picks: word.picks,
                weight: word.weight,
            })
            .collect(),
        candidates: pieces,
    }
}

fn slice_raw(raw: &str, start: u32, end: u32) -> String {
    raw.get(start as usize..end as usize)
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{ranking_source, source_ids};

    #[test]
    fn source_ids_list_every_set_bit_and_custom_stands_alone() {
        assert_eq!(source_ids(0, true).as_slice(), ["custom"]);
        assert_eq!(
            source_ids((1 << 0) | (1 << 2), false).as_slice(),
            ["kautian", "itaigi"]
        );
        assert_eq!(source_ids(1 << 2, true).as_slice(), ["custom"]);
    }

    #[test]
    fn ranking_source_follows_the_sort_rank() {
        assert_eq!(ranking_source(0, true), "custom");
        assert_eq!(ranking_source((1 << 1) | (1 << 7), false), "taigitv");
        assert_eq!(ranking_source(1 << 2, false), "other");
    }
}
