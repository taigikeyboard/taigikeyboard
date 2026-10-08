//! Search orchestration.
//!
//! Pipeline (romanization; hanji queries go through `search_by_hanji`):
//! 1. Build trie key via `phonetics::KeyFamily::search_key`.
//! 2. `prefix_index.lookup_prefix` returns insertion-ordered rowids
//!    (D-12 parity correction toward Android).
//! 3. Resolve each rowid through `dictionary_reader.record` + filter,
//!    take `limit`, return `SearchRow`.
//!
//! TPS dialect er↔or recall: C-3a moved the runtime expansion into the
//! build pipeline (dual-emit `tps:` keys for the ㄜ and ㄛ glyphs at the
//! same rowid). The lexicon search path is now mode-blind for that axis.

use indexmap::IndexSet;
use phonetics::{abbrev_family_key, KeyFamily, HANJI_KEY_PREFIX};

use crate::association_reader::{word_key, AssociationFilter, AssociationReader};
use crate::dictionary_reader::{DictionaryReader, DictionaryRecord, Filter, KAUTIAN_BIT};
use crate::error::LexiconError;
use crate::prefix_index::PrefixIndex;

/// Public per-row output. Mirrors proto `TaigiWord` but kept Rust-native to
/// avoid coupling search internals to prost types.
#[derive(Debug, Clone)]
pub struct SearchRow {
    // 1-based dictionary rowid.
    pub id: i64,
    pub roman: String,
    pub hanji: Option<String>,
    // The record's raw dictionary frequency; no platform orders on it since
    // the engine owns the search order (E1 P5b).
    pub length_score: Option<i32>,
    // Source bitmask, so the platform can tag the source.
    pub source_bitmask: Option<u32>,
}

impl SearchRow {
    // From the MOE dictionary (Kautian), by the effective source bitmask.
    fn is_kautian(&self) -> bool {
        self.source_bitmask
            .is_some_and(|bitmask| bitmask & u32::from(KAUTIAN_BIT) != 0)
    }
}

/// One bundled bigram continuation — what `api::lookup_associations` returns.
#[derive(Debug, Clone)]
pub struct AssociationHit {
    // Previous word — the key that was queried.
    pub previous_word: String,
    // Following candidate, written in hanji.
    pub candidate_word: String,
    pub candidate_tl: String,
    // Bigram occurrence count.
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct SearchParams {
    // Raw user input, before normalization.
    pub input: String,
    // Key family of the input; `api::proto_key_family` maps proto InputMode onto it.
    pub family: KeyFamily,
    pub limit: u32,
    // Enabled-source bitmask, including the variant + khiin control bits.
    pub enabled_sources_bitmask: u32,
}

pub fn search(
    params: &SearchParams,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<SearchRow>, LexiconError> {
    if params.limit == 0 {
        return Ok(Vec::new());
    }

    let key = params.family.search_key(&params.input);

    // Exact-then-prefix concatenation matches Android's
    // `(exactRowIds + prefixRowIds).distinct()` semantics. IndexSet
    // dedup preserves insertion order — D-12 parity correction toward
    // Android pinned by INVARIANT_LEX_LOOKUP_ROWIDS_ORDER.
    //
    // Acronym matching (typing `gi` finds 外夷 `guā-î`) is intentional
    // here: the abbreviation keys live in their own `*-abbrev:` family
    // since §46, so both families are unioned — main exact, abbrev exact,
    // main prefix, abbrev prefix. Only the insertion order of frequency
    // ties changed against the pre-§46 single-range byte order.
    let abbrev_key = abbrev_family_key(&key);
    let mut rowids: IndexSet<u32> = IndexSet::new();
    for id in prefix_index.lookup_exact(&key) {
        rowids.insert(id);
    }
    if let Some(abbrev_key) = &abbrev_key {
        for id in prefix_index.lookup_exact(abbrev_key) {
            rowids.insert(id);
        }
    }
    for id in prefix_index.lookup_prefix(&key) {
        rowids.insert(id);
    }
    if let Some(abbrev_key) = &abbrev_key {
        for id in prefix_index.lookup_prefix(abbrev_key) {
            rowids.insert(id);
        }
    }

    Ok(collect_filtered_sorted(
        rowids,
        dict,
        params.enabled_sources_bitmask,
        params.limit,
    ))
}

// Tab3 hanji lookup — scans the FST for `hanzi:`-prefixed keys.
pub fn search_by_hanji(
    query: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<SearchRow>, LexiconError> {
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let key = format!("{HANJI_KEY_PREFIX}{query}");
    let mut rowids: IndexSet<u32> = IndexSet::new();
    for id in prefix_index.lookup_exact(&key) {
        rowids.insert(id);
    }
    for id in prefix_index.lookup_prefix(&key) {
        rowids.insert(id);
    }
    Ok(collect_filtered_sorted(
        rowids,
        dict,
        enabled_sources_bitmask,
        limit,
    ))
}

/// The Dictionary Search page's order, owned here for every platform: filter
/// the rowids through `passes_filter`, sort by corpus `walker_cost` ascending
/// (the scale the candidate list and slot 0 use, E1 P5b) with the old
/// `frequency` descending and then rowid insertion order breaking an equal
/// cost, take `limit`, then move the MOE (Kautian) rows ahead of the rest,
/// keeping each group's order. Kautian goes first after the cut, so the
/// `limit` rows are the corpus's top rows whatever their source. Kautian is
/// read from the EFFECTIVE bitmask, so a row whose Kautian subcollection is
/// disabled does not jump the queue.
fn collect_filtered_sorted(
    rowids: IndexSet<u32>,
    dict: &DictionaryReader,
    enabled_sources_bitmask: u32,
    limit: u32,
) -> Vec<SearchRow> {
    let filter = Filter::from_enabled_bitmask(enabled_sources_bitmask);
    let mut staged: Vec<(u32, DictionaryRecord)> = Vec::with_capacity(rowids.len());
    for rowid in rowids {
        if let Some(record) = dict.record(rowid) {
            if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, &filter) {
                continue;
            }
            staged.push((rowid, record));
        }
    }
    // Stable sort: an equal (cost, frequency) keeps the IndexSet rowid order.
    staged.sort_by_key(|(_, record)| (record.walker_cost, std::cmp::Reverse(record.frequency)));
    staged.truncate(limit as usize);
    let mut rows: Vec<SearchRow> = staged
        .into_iter()
        .map(|(rowid, record)| {
            // Emit the EFFECTIVE source bitmask (kautian bit dropped when its
            // subcollection is disabled) so a multi-source survivor ranks by
            // its other source's tier, not kautian's (DD6 ranking-weight drop).
            let effective = DictionaryReader::effective_source_bitmask(
                record.bitmask,
                record.kautian_subtag,
                &filter,
            );
            record_to_row(rowid, record, effective)
        })
        .collect();
    rows.sort_by_key(|row| !row.is_kautian());
    rows
}

// NextWord bigram lookup for a committed word: its word key `hanji\u{1}tl`
// when `previous_tl` is known and the key has rows under the source mask,
// else the character key of its last character (an empty `previous_tl` goes
// straight there). The bigram model's own backoff, word → character, never a
// merge (behavioral-invariants §24 `INVARIANT_NEXTWORD_WORD_KEY_BACKOFF`).
// Up to `limit` records, filtered before the cut so disabled top entries
// never starve the list.
pub fn lookup_associations(
    previous_word: &str,
    previous_tl: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
    reader: &AssociationReader,
) -> Result<Vec<AssociationHit>, LexiconError> {
    let Some(last_character) = previous_word.chars().last() else {
        return Ok(Vec::new());
    };
    let filter = AssociationFilter::from_sources_bitmask(enabled_sources_bitmask);
    let limit = limit as usize;
    let mut entries = if previous_tl.is_empty() {
        Vec::new()
    } else {
        reader.lookup(&word_key(previous_word, previous_tl), limit, &filter)
    };
    if entries.is_empty() {
        entries = reader.lookup(&last_character.to_string(), limit, &filter);
    }
    Ok(entries
        .into_iter()
        .map(|entry| AssociationHit {
            previous_word: previous_word.to_string(),
            candidate_word: entry.next_word,
            candidate_tl: entry.next_tl,
            count: entry.count,
        })
        .collect())
}

fn record_to_row(rowid: u32, record: DictionaryRecord, effective_bitmask: u16) -> SearchRow {
    SearchRow {
        id: rowid as i64,
        roman: record.tl,
        hanji: record.hanji,
        length_score: Some(record.frequency as i32),
        source_bitmask: Some(effective_bitmask as u32),
    }
}
