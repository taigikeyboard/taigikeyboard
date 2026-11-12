//! What a shell draws to show why the buffer was cut this way and why the
//! candidate list is in this order. Built from the same walk and the same
//! [`ranking::CandidateSortKey`] the list uses. The page renders it; it
//! does not re-rank.

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
    /// `literal`, `lead`, `picked_over_sentence`, or a
    /// [`ranking::CandidateSortKey`] dimension name.
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
