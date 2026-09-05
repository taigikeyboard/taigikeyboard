//! Search orchestration.
//!
//! Pipeline:
//! 1. **D-8 hard guard** — `input_type == Hanzi` short-circuits to `[]`
//!    BEFORE any reader is touched. Pinned by INVARIANT_LEX_HANZI_GUARD
//!    (Rust + iOS + Android per Codex Mod 1).
//! 2. Build trie key via `key_normalizer`.
//! 3. `prefix_index.lookup_prefix` returns insertion-ordered rowids
//!    (D-12 parity correction toward Android).
//! 4. Resolve each rowid through `dictionary_reader.record` + filter,
//!    take `limit`, return `LexiconRowOut`.
//!
//! TPS dialect er↔or recall: C-3a moved the runtime expansion into the
//! build pipeline (dual-emit `tps:` keys for the ㄜ and ㄛ glyphs at the
//! same rowid). The lexicon search path is now mode-blind for that axis.

use indexmap::IndexSet;

use crate::association_reader::{AssocFilter, AssociationReader};
use crate::dictionary_reader::{DictionaryReader, DictionaryRecord, Filter};
use crate::error::LexiconError;
use crate::key_normalizer::{self, KeyMode, KeyType};
use crate::prefix_index::PrefixIndex;

/// Public per-row output. Mirrors proto `TaigiWord` but kept Rust-native to
/// avoid coupling search internals to prost types.
#[derive(Debug, Clone)]
pub struct LexiconRowOut {
    // 1-based dictionary rowid.
    pub id: i64,
    pub roman: String,
    pub hanji: Option<String>,
    // Sort score; reuses the raw frequency value.
    pub length_score: Option<i32>,
    // Source bitmask, so the platform can tag the source.
    pub source_bitmask: Option<u32>,
}

/// Public per-bigram output. Mirrors proto `LexiconAssocEntry`.
#[derive(Debug, Clone)]
pub struct LexiconAssocOut {
    // Previous word — the key that was queried.
    pub previous_word: String,
    // Following candidate, written in hanji.
    pub candidate_word: String,
    pub candidate_tl: String,
    // Bigram occurrence count.
    pub count: u32,
}

// Search input type; dispatch maps proto InputType onto this enum.
#[derive(Debug, Clone, Copy)]
pub enum SearchInputType {
    RomanNoTone,
    // Toned roman input; diacritic or digit tones both accepted.
    RomanWithTone,
    // Hanzi input; search() short-circuits to [].
    Hanzi,
}

// Search input mode (romanization scheme); dispatch maps proto InputMode onto this enum.
#[derive(Debug, Clone, Copy)]
pub enum SearchInputMode {
    Tl,
    Poj,
    // TPS bopomofo — queries the `tps:` key family, independent since C-1.
    Tps,
}

#[derive(Debug, Clone)]
pub struct SearchParams {
    // Raw user input, before normalization.
    pub input: String,
    pub input_type: SearchInputType,
    pub input_mode: SearchInputMode,
    pub limit: u32,
    // Enabled-source bitmask, including the variant + khiin control bits.
    pub enabled_sources_bitmask: u32,
}

pub fn search(
    params: &SearchParams,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<LexiconRowOut>, LexiconError> {
    // D-8 hard guard.
    if matches!(params.input_type, SearchInputType::Hanzi) {
        return Ok(Vec::new());
    }
    if params.limit == 0 {
        return Ok(Vec::new());
    }

    let key = key_normalizer::build(
        &params.input,
        KeyType::Romanization,
        match params.input_mode {
            SearchInputMode::Tl => KeyMode::Tl,
            SearchInputMode::Poj => KeyMode::Poj,
            SearchInputMode::Tps => KeyMode::Tps,
        },
    );

    // Exact-then-prefix concatenation matches Android's
    // `(exactRowIds + prefixRowIds).distinct()` semantics. IndexSet
    // dedup preserves insertion order — D-12 parity correction toward
    // Android pinned by INVARIANT_LEX_LOOKUP_ROWIDS_ORDER.
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
        params.enabled_sources_bitmask,
        params.limit,
    ))
}

pub fn search_with_sources(
    params: &SearchParams,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<LexiconRowOut>, LexiconError> {
    // Tab3 multi-source — matches the romanization path; hanzi inputs are
    // dispatched to search_by_hanzi instead.
    search(params, prefix_index, dict)
}

// Tab3 hanzi lookup — scans the FST for `hanzi:`-prefixed keys.
pub fn search_by_hanzi(
    query: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<LexiconRowOut>, LexiconError> {
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let key = format!("hanzi:{query}");
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

/// Filter rowids through `passesFilter`, then sort by `frequency` descending,
/// then take `limit`. Mirrors iOS `DictionaryRepository.lookupRowIds` +
/// platform sort step (see audit §3 — frequency-desc sort happens INSIDE
/// the repository today; this slice consolidates both into the engine).
/// Pinned by `INVARIANT_LEX_FREQUENCY_SORT` (parity test added in commit 7
/// follow-up).
fn collect_filtered_sorted(
    rowids: IndexSet<u32>,
    dict: &DictionaryReader,
    enabled_sources_bitmask: u32,
    limit: u32,
) -> Vec<LexiconRowOut> {
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
    // Stable sort by frequency descending. Tied scores fall back to
    // insertion order (IndexSet rowid order preserved by `sort_by_key`).
    staged.sort_by_key(|entry| std::cmp::Reverse(entry.1.frequency));
    let limit_usize = limit as usize;
    staged.truncate(limit_usize);
    staged
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
        .collect()
}

// NextWord bigram lookup — up to `limit` records, filtered by the 9-bit source mask.
pub fn assoc_lookup(
    previous_word: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
    assoc: &AssociationReader,
) -> Result<Vec<LexiconAssocOut>, LexiconError> {
    if previous_word.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let filter = AssocFilter {
        all_enabled: enabled_sources_bitmask == u32::MAX,
        enabled_mask: (enabled_sources_bitmask & 0x1FF) as u16, // 9 association bits
    };
    let raw = assoc.lookup(previous_word, limit as usize);
    let mut out: Vec<LexiconAssocOut> = Vec::with_capacity(raw.len());
    for entry in raw {
        if !AssociationReader::passes_filter(entry.bitmask, &filter) {
            continue;
        }
        out.push(LexiconAssocOut {
            previous_word: previous_word.to_string(),
            candidate_word: entry.next_word,
            candidate_tl: entry.next_tl,
            count: entry.count,
        });
    }
    Ok(out)
}

fn record_to_row(rowid: u32, record: DictionaryRecord, effective_bitmask: u16) -> LexiconRowOut {
    LexiconRowOut {
        id: rowid as i64,
        roman: record.tl,
        hanji: record.hanzi,
        length_score: Some(record.frequency as i32),
        source_bitmask: Some(effective_bitmask as u32),
    }
}
