//! Hanji conversion of a TPS pending tail
//! (`docs/architecture/desktop-tps-hanji-conversion-roadmap.md` H1, H2): the
//! walker's best path over the closed readings, kept in `Phase::Continuous`
//! and shown in the preedit in front of the glyphs of the reading still being
//! typed.
//!
//! A reading is closed by its tone mark, or by Space for tones 1 and 4. The
//! closed part is the longest prefix of the tail ending on such a boundary
//! that the syllabifier reaches from the start of the tail; it is walked as
//! a buffer of its own, so its trailing Space keeps the §41 tone pin and a
//! tail the whole-buffer walk cannot span still converts what is closed.
//!
//! The caret moves by word over the converted words and by glyph inside the
//! text between them (H3). A reading typed inside the tail keeps the words
//! on both sides until it closes; then the whole tail is walked again.
//!
//! The list of the word before the caret starts at [`word_list_start`];
//! its pick nails what the preedit shows before that start (H4). At the
//! tail's start that word is the last nailed segment, which the pick
//! re-opens first. A walk
//! ranks with the user's learned counts when the request supplies them
//! ([`ConversionFrequency`], H5).

use lexicon::{ContinuousFetchCtx, EngineHandle as LexiconHandle, SyllableInventory};
use phonetics::InputMode;
use protos::engine::AppConfig;

use crate::api::{CaretDirection, NailedSegment};
use crate::continuous::{
    edge_homophone_words, edge_toneless_bodies, synth_consumed_span, walk_buffer,
};
use crate::derived::{buffer_input_mode, tps_slice_display};
use crate::shadow::{
    build_continuous_keys, build_shadow_lattice_with_barriers, whole_buffer_tone_pin,
    ContinuousKeys, ShadowLattice,
};

/// The user data a conversion walk ranks with (H5): the learned counts, and
/// the custom words and learned phrases that join the walk as edges, as they
/// do in a TL / POJ walk. The engine holds neither the user-data stores nor a
/// clock; the dispatch layer, which holds both, supplies one with a
/// mutation's request.
pub trait ConversionFrequency {
    /// The custom-dictionary rows and learned phrases filed under any of the
    /// toneless TPS `edge_keys` (`ㄉㄢㄙㆩ`); `frequency` is not read.
    fn rows_for_edge_keys(&self, edge_keys: &[String]) -> crate::api::UserRows;
    /// The learned rows for `words`, keyed by the `(word, canonical TL)`
    /// pair the ranking reads.
    fn rows_for_words(&self, words: &[String]) -> ranking::FrequencyMap;
    /// The wall clock (epoch ms) the recency ranking reads.
    fn now_ms(&self) -> i64;
}

/// The converted words of a pending tail. The raw text no word covers — the
/// reading being typed, a reading typed inside the tail and not closed yet,
/// what cannot be closed — shows as its glyphs. Under a shown conversion the
/// caret sits on a word boundary or in that text, never inside a word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversion {
    /// The words in raw order, never overlapping and never empty. Typing
    /// forward they cover the closed part from the start of the tail.
    pub segments: Vec<ConvertedSegment>,
    /// The dictionary source filter the walk ran with. A request asking for
    /// another filter neither shows nor re-uses this conversion.
    pub enabled_sources_bitmask: u32,
    /// Whether the walk ran with the custom dictionary off
    /// (`AppConfig.custom_dictionary_disabled`); a request with the other
    /// setting neither shows nor re-uses this conversion.
    pub custom_dictionary_disabled: bool,
}

/// One word of the walker's path over the closed part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConvertedSegment {
    /// Byte offsets in the pending raw. The separator run that closes the
    /// word's last reading belongs to it, so the caret stop after the word
    /// is past the separator and a glyph typed there starts a new reading.
    pub raw_span: (usize, usize),
    /// The word's Hanji, or its glyphs when the dictionary has none.
    pub display_text: String,
    /// The word's Hanji; `None` for a word the dictionary has none for.
    pub hanji: Option<String>,
    /// The word's canonical TL, the other half of its `(Hanji, canonical-TL)`
    /// identity; empty for a word the dictionary has none for.
    pub canonical_tl: String,
    pub syllable_count: u8,
}

impl ConvertedSegment {
    /// Raw text no word covers, `raw[start..end]`, as the preedit shows it:
    /// its glyphs, with no word identity — no Hanji, no TL, no syllables.
    fn glyphs(raw: &str, start: usize, end: usize) -> Self {
        Self {
            raw_span: (start, end),
            display_text: tps_slice_display(&raw[start..end]),
            hanji: None,
            canonical_tl: String::new(),
            syllable_count: 0,
        }
    }

    /// This piece of the pending `raw`, nailed as shown after `nailed`: not
    /// picked, its raw text byte for byte (H4).
    pub(crate) fn nailed_as_shown(&self, raw: &str, nailed: &[NailedSegment]) -> NailedSegment {
        let (start, end) = self.raw_span;
        NailedSegment {
            display_text: self.display_text.clone(),
            canonical_text: self.display_text.clone(),
            raw_text: raw[start..end].to_string(),
            association_tl: self.canonical_tl.clone(),
            hanji: self.hanji.clone(),
            raw_span: next_raw_span(nailed, end - start),
            syllable_count: self.syllable_count,
            is_picked: false,
        }
    }
}

/// The composition span of a segment of `len` raw bytes nailed after
/// `nailed`.
pub(crate) fn next_raw_span(nailed: &[NailedSegment], len: usize) -> (usize, usize) {
    let start = nailed.last().map_or(0, |segment| segment.raw_span.1);
    (start, start + len)
}

impl Conversion {
    /// Byte offset in the pending raw where the last word ends; typing
    /// forward, what follows is the open reading.
    pub fn closed_end(&self) -> usize {
        self.segments.last().map_or(0, |segment| segment.raw_span.1)
    }

    /// Whether `config` asks for the conversion this one was walked for.
    pub(crate) fn is_for(&self, config: &AppConfig) -> bool {
        requested_sources(config) == Some(self.enabled_sources_bitmask)
            && config.custom_dictionary_disabled == self.custom_dictionary_disabled
    }

    /// The pending tail as the preedit shows it — each word as its display
    /// text, the raw text between and after them as glyphs — and the
    /// caret's UTF-16 offset inside it. `None` when `config` does not ask
    /// for this conversion.
    pub(crate) fn tail_display(
        &self,
        raw: &str,
        caret: usize,
        config: &AppConfig,
    ) -> Option<(String, usize)> {
        if !self.is_for(config) {
            return None;
        }
        let mut display = String::new();
        let mut caret_utf16 = None;
        for piece in self.pieces_before(raw, raw.len()) {
            let (start, end) = piece.raw_span;
            // The caret is never inside a word: strictly inside a piece, it
            // is among glyphs.
            if caret_utf16.is_none() && (start..end).contains(&caret) {
                let before_caret = tps_slice_display(&raw[start..caret]);
                caret_utf16 = Some(utf16_len(&display) + utf16_len(&before_caret));
            }
            display.push_str(&piece.display_text);
        }
        let caret_utf16 = caret_utf16.unwrap_or_else(|| utf16_len(&display));
        Some((display, caret_utf16))
    }

    /// Where the list of the word before `caret` starts (H4): the start of the
    /// word ending at the caret, else the start of the text no word covers
    /// that the caret is in — the reading being typed, whose own list it is.
    /// At the start of the tail, `0`: the first word, or the first reading.
    fn anchor(&self, caret: usize) -> usize {
        self.step_over_word(caret, CaretDirection::Left)
            .unwrap_or_else(|| gap_start(&self.segments, caret))
    }

    /// The word whose raw span ends at `offset`.
    pub(crate) fn word_ending_at(&self, offset: usize) -> Option<&ConvertedSegment> {
        self.segments.iter().find(|word| word.raw_span.1 == offset)
    }

    /// The pending `raw` up to `end` cut into the pieces the preedit shows,
    /// in order and covering every byte: the words, and the text between them
    /// as [`ConvertedSegment::glyphs`]. `end` is a boundary no word crosses
    /// (a list start, or the end of the tail).
    pub(crate) fn pieces_before(&self, raw: &str, end: usize) -> Vec<ConvertedSegment> {
        let mut pieces = Vec::new();
        let mut position = 0;
        for word in self.segments.iter().filter(|word| word.raw_span.1 <= end) {
            if position < word.raw_span.0 {
                pieces.push(ConvertedSegment::glyphs(raw, position, word.raw_span.0));
            }
            pieces.push(word.clone());
            position = word.raw_span.1;
        }
        if position < end {
            pieces.push(ConvertedSegment::glyphs(raw, position, end));
        }
        pieces
    }

    /// Where a step from `caret` goes when a word lies on the side it steps
    /// to: the far edge of that word. `None` when the next step is a glyph
    /// between the words, or the edge of the tail. A jump to the tail's
    /// start / end steps over no word (`move_caret` never asks).
    pub(crate) fn step_over_word(&self, caret: usize, direction: CaretDirection) -> Option<usize> {
        self.segments.iter().find_map(|segment| {
            let (start, end) = segment.raw_span;
            match direction {
                CaretDirection::Left => (end == caret).then_some(start),
                CaretDirection::Right => (start == caret).then_some(end),
                CaretDirection::Start | CaretDirection::End => None,
            }
        })
    }
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The source filter `config` asks a conversion with — its
/// `dictionary_sources`, the fetch's filter — or `None` when it asks for no
/// conversion (`hanji_conversion` absent).
fn requested_sources(config: &AppConfig) -> Option<u32> {
    config
        .hanji_conversion
        .as_ref()
        .map(|_| crate::requests::source_filter_bitmask(config))
}

/// Whether `config` asks for the conversion of the pending text `raw`: the
/// switch is present and the buffer is TPS.
pub(crate) fn is_requested(raw: &str, config: &AppConfig) -> bool {
    requested_sources(config).is_some() && buffer_input_mode(raw, config) == InputMode::Tps
}

/// Whether `config` asks for the conversion of the composition of `nailed`
/// segments and the pending `raw` — judged on the whole composition, so a
/// tail a pick left with tone marks only is still a TPS composition's.
pub(crate) fn is_requested_for_composition(
    nailed: &[NailedSegment],
    raw: &str,
    config: &AppConfig,
) -> bool {
    let composition: String = nailed
        .iter()
        .map(|segment| segment.raw_text.as_str())
        .chain([raw])
        .collect();
    is_requested(&composition, config)
}

/// Where the list of the word before the caret starts (H4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordListStart {
    /// At this byte offset of the pending tail.
    InTail(usize),
    /// At the last nailed segment: the caret is at the tail's start with
    /// segments nailed before it, so the word before the caret is the last
    /// of them. Its list reads that segment's raw text followed by the tail;
    /// a pick from it re-opens the segment and nails again from its start
    /// (`transition::reopen_last_nailed_for_pick`), so merely opening the
    /// list leaves the composition — and the pick it holds — as it was.
    LastNailed,
}

impl WordListStart {
    /// The text the list reads and the offset of its first byte in the
    /// pending `raw`, by which the list's spans shift back into the tail —
    /// `0` for the last nailed segment's list, whose pick puts that segment
    /// back in front of the tail first.
    pub(crate) fn listed(self, nailed: &[NailedSegment], raw: &str) -> (String, usize) {
        match self {
            Self::LastNailed => {
                let last = nailed
                    .last()
                    .map_or("", |segment| segment.raw_text.as_str());
                (format!("{last}{raw}"), 0)
            }
            Self::InTail(start) => (raw[start..].to_string(), start),
        }
    }
}

/// Where the list of the word before the caret starts (H4) — the one rule a
/// fetch, its context and its pick read — or `None` when `config` asks for
/// no conversion of the composition: the whole tail's list, a legacy pick.
/// At the tail's start with segments nailed before it, the last of them;
/// otherwise [`Conversion::anchor`] of the conversion the tail holds for
/// `config`, `0` when it holds none (a reading open from the start, or a
/// tail of tone marks).
pub(crate) fn word_list_start(
    nailed: &[NailedSegment],
    raw: &str,
    caret: usize,
    conversion: Option<&Conversion>,
    config: &AppConfig,
) -> Option<WordListStart> {
    is_requested_for_composition(nailed, raw, config).then(|| {
        if caret == 0 && !nailed.is_empty() {
            return WordListStart::LastNailed;
        }
        WordListStart::InTail(
            conversion
                .filter(|conversion| conversion.is_for(config))
                .map_or(0, |conversion| conversion.anchor(caret)),
        )
    })
}

/// The pending tail a mutation replaces, with the conversion it held.
pub(crate) struct PreviousTail<'a> {
    pub raw: &'a str,
    pub caret: usize,
    pub conversion: &'a Conversion,
}

/// The conversion of the pending `raw` under `config`, and the caret — moved
/// to the end of a word a new walk put it inside. `None` when the request
/// does not ask for one, the buffer is not TPS, no word is converted, or the
/// walk finds no path (no lexicon installed included). `previous` is the
/// tail being replaced, `None` to walk afresh (a re-opened nailed segment);
/// a walk runs only when a reading closes or closed text changes, so a glyph
/// of an open reading walks nothing:
///
/// - the same text (a caret step) keeps the words;
/// - an edit at the end walks the closed part again when its text changed,
///   as typing forward does;
/// - an edit inside the tail keeps the words it did not touch until the
///   reading at the caret closes, then walks the whole tail again.
///
/// A walk ranks with `frequency`'s rows for every word any edge of the
/// closed part could take; `None` walks neutral.
pub(crate) fn convert(
    previous: Option<PreviousTail<'_>>,
    raw: &str,
    caret: usize,
    config: &AppConfig,
    frequency: Option<&dyn ConversionFrequency>,
) -> (Option<Conversion>, usize) {
    let Some(enabled_sources_bitmask) = requested_sources(config) else {
        return (None, caret);
    };
    if buffer_input_mode(raw, config) != InputMode::Tps {
        return (None, caret);
    }
    let previous = previous.filter(|previous| previous.conversion.is_for(config));
    if let Some(previous) = previous.as_ref().filter(|previous| previous.raw == raw) {
        return (Some(previous.conversion.clone()), caret);
    }
    // One read scope for the boundary and the walk: both read the same
    // inventory and dictionary.
    LexiconHandle::with_state(|state| {
        let Some(inventory) = state.syllable_inventory.as_ref() else {
            return Ok((None, caret));
        };
        let walk = |closed: &str| {
            let (prefix_index, dict) = (state.prefix_index.as_ref()?, state.dictionary.as_ref()?);
            let continuous_keys = build_continuous_keys(closed, inventory, InputMode::Tps);
            // The path spans the shadow; it covers the closed part only when
            // the shadow's end maps to its end — the slot-0 synthesis's own
            // gate. Checked before the user's counts are read.
            synth_consumed_span(
                &continuous_keys.shadow_to_raw_end,
                continuous_keys.shadow.len(),
                closed.len() as u32,
            )?;
            // The custom words and learned phrases filed under an edge's
            // body join that edge's pick, as in a TL / POJ walk. No
            // previous-word context.
            let user_rows = frequency
                .map(|frequency| {
                    frequency
                        .rows_for_edge_keys(&edge_toneless_bodies(&continuous_keys, InputMode::Tps))
                })
                .unwrap_or_default();
            let mut ctx = ContinuousFetchCtx {
                enabled_sources_bitmask,
                freq_map: &ranking::FrequencyMap::new(),
                context: ranking::ContextRanks::empty(),
                now_ms: 0,
                custom: &user_rows.custom,
                learned: &user_rows.learned,
                prefix_index,
                dict,
                mode: InputMode::Tps,
                // TPS has no one-syllable `-` stretches (§52 is TL / POJ).
                tone_pin: whole_buffer_tone_pin(closed, InputMode::Tps, None),
                whole_buffer_key: None,
            };
            // The user's counts for every word an edge could take, the
            // custom and learned ones included: user weight both picks an
            // edge's word and prices its span.
            let rows = frequency
                .map(|frequency| {
                    let mut words = edge_homophone_words(&continuous_keys, &ctx);
                    let mut seen: std::collections::HashSet<String> =
                        words.iter().cloned().collect();
                    let user_words = user_rows
                        .custom
                        .iter()
                        .map(|entry| entry.word(InputMode::Tps))
                        .chain(user_rows.learned.iter().map(|entry| entry.hanji.clone()));
                    words.extend(user_words.filter(|word| seen.insert(word.clone())));
                    (frequency.rows_for_words(&words), frequency.now_ms())
                })
                .unwrap_or_default();
            ctx.freq_map = &rows.0;
            ctx.now_ms = rows.1;
            walk_closed_part(closed, &continuous_keys, inventory, &ctx)
        };
        // The closed part runs to the furthest closing end; it is walked
        // as a buffer of its own.
        let closed_end = || closing_ends(raw, inventory).into_iter().max();
        let walk_closed = |closed_end: Option<usize>| walk(&raw[..closed_end?]);
        let segments = match previous {
            None => walk_closed(closed_end()),
            Some(previous) if caret == raw.len() => {
                // Words with glyphs between them (a reading re-opened inside
                // the tail) are walked again once the edit is at the end.
                let closed_end = closed_end();
                let previous_closed = previous.raw.get(..previous.conversion.closed_end());
                if covers_from_start(&previous.conversion.segments)
                    && closed_end.is_some_and(|end| previous_closed == Some(&raw[..end]))
                {
                    Some(previous.conversion.segments.clone())
                } else {
                    walk_closed(closed_end)
                }
            }
            Some(previous) => match words_outside_the_edit(&previous, raw, caret) {
                Some(kept) => {
                    let (gap_start, gap_end) = gap_around(&kept, raw.len(), caret);
                    // An empty gap closes nothing; the lattice is built once.
                    let ends = if gap_start < gap_end {
                        closing_ends(raw, inventory)
                    } else {
                        Vec::new()
                    };
                    // The reading at the caret closes when a reading of the
                    // gap ends at or after the caret: open text may follow.
                    let closes_at_caret = caret.max(gap_start + 1)..=gap_end;
                    if ends.iter().any(|end| closes_at_caret.contains(end)) {
                        walk_closed(ends.into_iter().max())
                    } else {
                        Some(kept)
                    }
                }
                // Not an edit before the caret: no word is known to survive.
                None => walk_closed(closed_end()),
            },
        };
        let segments = segments.filter(|segments| !segments.is_empty());
        let caret = segments
            .as_deref()
            .map_or(caret, |segments| caret_out_of_words(segments, caret));
        let conversion = segments.map(|segments| Conversion {
            segments,
            enabled_sources_bitmask,
            custom_dictionary_disabled: config.custom_dictionary_disabled,
        });
        Ok((conversion, caret))
    })
    .unwrap_or((None, caret))
}

/// The raw offsets where a reading of `raw` closes: each syllable boundary
/// the lattice reaches from the start where a tone mark ends the syllable
/// before it, or the separator was typed right after it — with that
/// separator run, which belongs to the reading it closes. A separator typed
/// after a hyphen (`ㄒㄧ- `) closes nothing: the hyphen run stays pending,
/// and a closed part cut before it would lose the separator and with it the
/// §41 tone pin. Empty when no reading is closed: an open reading only, an
/// orphan tone mark, a tail the syllabifier rejects.
///
/// Every lattice edge starts at an offset reachable from 0, so its end is
/// reachable too. The lattice is the whole tail's: a boundary it rejects
/// because of what follows (a tone mark typed after the separator) is not
/// closed, even though the prefix alone would span.
fn closing_ends(raw: &str, inventory: &SyllableInventory) -> Vec<usize> {
    let ShadowLattice {
        shadow,
        shadow_to_raw_end,
        lattice,
        space_barriers,
        ..
    } = build_shadow_lattice_with_barriers(raw, inventory, InputMode::Tps);
    // The raw end of the reading ending at shadow offset `end`, its
    // separator run included. An interior run follows the mapped offset; a
    // trailing one is already inside it (`shadow_to_raw_end`'s full-span end).
    let closed_end_at = |end: usize| {
        let closed_end = with_separator_run(raw, shadow_to_raw_end[end]);
        let has_tone_mark = shadow[..end]
            .chars()
            .next_back()
            .is_some_and(phonetics::is_tps_tone_mark);
        let has_separator = space_barriers.contains(&end) && raw[..closed_end].ends_with(' ');
        (has_tone_mark || has_separator).then_some(closed_end)
    };
    lattice
        .edges()
        .iter()
        .filter_map(|&(_, end)| closed_end_at(end))
        .collect()
}

/// `offset` moved past the separator run that starts there.
fn with_separator_run(raw: &str, offset: usize) -> usize {
    offset
        + raw[offset..]
            .bytes()
            .take_while(|&byte| byte == b' ')
            .count()
}

/// The words of `previous` an edit at the caret left alone, in the new
/// `raw`: those before the text it changed, and those after the caret,
/// shifted by what it inserted or removed. A word it touched is dropped and
/// shows as glyphs. `None` when the change is not an edit before the caret —
/// every mutator keeps the text after the caret.
fn words_outside_the_edit(
    previous: &PreviousTail<'_>,
    raw: &str,
    caret: usize,
) -> Option<Vec<ConvertedSegment>> {
    if raw.get(caret..)? != previous.raw.get(previous.caret..)? {
        return None;
    }
    let unchanged_prefix = common_prefix_len(&previous.raw[..previous.caret], &raw[..caret]);
    let words = previous.conversion.segments.iter().filter_map(|segment| {
        let (start, end) = segment.raw_span;
        if end <= unchanged_prefix {
            Some(segment.clone())
        } else if start >= previous.caret {
            let shift = |offset: usize| offset - previous.caret + caret;
            Some(ConvertedSegment {
                raw_span: (shift(start), shift(end)),
                ..segment.clone()
            })
        } else {
            None
        }
    });
    Some(words.collect())
}

/// Byte length of the longest common prefix of `a` and `b`, on a char
/// boundary of both.
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .find(|&((_, a_char), b_char)| a_char != b_char)
        .map_or(a.len().min(b.len()), |((offset, _), _)| offset)
}

/// The raw text around `caret` that no word covers: from the end of the word
/// before it (or the start of the tail) to the start of the word after it (or
/// the end of the tail).
fn gap_around(words: &[ConvertedSegment], raw_len: usize, caret: usize) -> (usize, usize) {
    let start = gap_start(words, caret);
    let end = words
        .iter()
        .map(|word| word.raw_span.0)
        .filter(|&start| start >= caret)
        .min()
        .unwrap_or(raw_len);
    (start, end)
}

/// The end of the last word ending at or before `caret`, else `0`: where the
/// text no word covers around the caret starts.
fn gap_start(words: &[ConvertedSegment], caret: usize) -> usize {
    words
        .iter()
        .map(|word| word.raw_span.1)
        .filter(|&end| end <= caret)
        .max()
        .unwrap_or(0)
}

/// `caret`, or the end of the word it falls inside: after a walk the caret
/// stays on a word boundary or between the words.
fn caret_out_of_words(words: &[ConvertedSegment], caret: usize) -> usize {
    words
        .iter()
        .find(|word| word.raw_span.0 < caret && caret < word.raw_span.1)
        .map_or(caret, |word| word.raw_span.1)
}

/// Whether `words` cover the tail from its start without a gap — what a
/// conversion walked typing forward is, and one an edit inside the tail
/// left glyphs in is not.
fn covers_from_start(words: &[ConvertedSegment]) -> bool {
    let mut position = 0;
    words.iter().all(|word| {
        let is_adjacent = word.raw_span.0 == position;
        position = word.raw_span.1;
        is_adjacent
    })
}

/// The walker's best path over `closed` (its `continuous_keys`, whose shadow
/// spans it), as words in raw coordinates: they cover it from 0, and a
/// separator run belongs to the word before it. `None` when no edge chain
/// spans it.
fn walk_closed_part(
    closed: &str,
    continuous_keys: &ContinuousKeys,
    inventory: &SyllableInventory,
    ctx: &ContinuousFetchCtx<'_>,
) -> Option<Vec<ConvertedSegment>> {
    let shadow_to_raw_end = &continuous_keys.shadow_to_raw_end;
    let path = walk_buffer(continuous_keys, inventory, ctx)?;
    let mut start = 0;
    Some(
        path.edges
            .iter()
            .zip(path.choices)
            .map(|(&(_, end), choice)| {
                let raw_span = (start, with_separator_run(closed, shadow_to_raw_end[end]));
                start = raw_span.1;
                let display_text = choice
                    .hanji
                    .clone()
                    .unwrap_or_else(|| tps_slice_display(&closed[raw_span.0..raw_span.1]));
                ConvertedSegment {
                    raw_span,
                    display_text,
                    hanji: choice.hanji,
                    canonical_tl: choice.canonical_tl,
                    syllable_count: choice.syllable_count,
                }
            })
            .collect(),
    )
}
