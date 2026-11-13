//! Lattice-edge lookups: the key an edge is fetched with, and the words /
//! toneless bodies every edge offers.

use crate::shadow::{custom_toneless_key, strip_tones_for_mode};
use lexicon::{homophone_words_for_key, ConsumedSpan, ContinuousFetchCtx};

/// The rows `map` holds under an edge's `key` that `admits` lets through,
/// in `map` order.
pub(super) fn rows_for_edge<'a, T>(
    map: &std::collections::HashMap<String, Vec<&'a T>>,
    key: &str,
    admits: impl Fn(&T) -> bool,
) -> Vec<&'a T> {
    map.get(key)
        .into_iter()
        .flatten()
        .copied()
        .filter(|row| admits(row))
        .collect()
}

/// v3.5.8 S6 — per-fetch map from a user row's walker edge key to the rows
/// under it, in `rows` order (the edge pick's dedupe insertion order,
/// Codex Q6). `key_of` MUST run the same shadow pipeline the
/// lattice edge keys run for this fetch's `mode` so a hit is
/// byte-identical to a `tl:` / `poj:` edge key. Registers the nasal-`oo`
/// alias beside the canonical key: the dictionary build indexes the `o͘ⁿ`
/// rendering of the nasal final, but it cannot reach a user row, so typing
/// 好玄 as `ho͘nnhian` (edge key `tl:hoonnhian`) must still find a row
/// stored canonically — safe on a fused body because an alias key is
/// disjoint from every canonical key (`phonetics::nasal_oo_alias_spelling`).
/// A TPS key with ㄜ also registers its ㄛ variant, the second toneless key
/// the build files a dictionary row under (`phonetics::tps_notone_or_variant`):
/// that alias is not disjoint — TL `to` is canonically ㄉㄛ — and need not be,
/// since the dictionary itself answers both spellings with the row.
pub(super) fn edge_key_map<'a, T>(
    rows: &'a [T],
    key_of: impl Fn(&T) -> Option<String>,
) -> std::collections::HashMap<String, Vec<&'a T>> {
    let mut map: std::collections::HashMap<String, Vec<&'a T>> =
        std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        if let Some(key) = key_of(row) {
            if let Some(alias) = phonetics::nasal_oo_alias_spelling(&key) {
                map.entry(alias).or_default().push(row);
            }
            let or_variant = phonetics::tps_notone_or_variant(&key);
            if !or_variant.is_empty() {
                map.entry(or_variant).or_default().push(row);
            }
            map.entry(key).or_default().push(row);
        }
    }
    map
}

/// Learned phrases (§50) — the walker edge key for a learned row's canonical
/// TL: under POJ input the TL is rendered to POJ first (the same face the
/// lattice edges of a typed POJ buffer carry), then the same
/// [`custom_toneless_key`] pipeline a custom roman goes through, which also
/// folds a multi-word dictionary TL's space (`iā sī`) into the hyphen it is
/// between syllables. Under TPS input the same pipeline keys it by its
/// Bopomofo face, through the derivation `learned_phrases.db` files its
/// `tps:notone` key with (`custom_search_reading`), so the row the store
/// returns for an edge lands on that edge.
pub(super) fn learned_edge_key(canonical_tl: &str, mode: phonetics::InputMode) -> Option<String> {
    match mode {
        phonetics::InputMode::Poj => custom_toneless_key(
            &phonetics::api::tl_display_to_poj_display(canonical_tl),
            mode,
        ),
        _ => custom_toneless_key(canonical_tl, mode),
    }
}

/// What one lattice edge `shadow[start..end]` is looked up with: its toneless
/// body, its raw span, and the span key (key, §35 Final-only restriction,
/// tone pin). `None` for an edge with nothing to look up.
pub(super) struct EdgeLookup {
    pub(super) toneless: String,
    pub(super) raw_span: ConsumedSpan,
    pub(super) span_key: crate::shadow::SpanKey,
}

/// [`EdgeLookup`] of the edge `start..end` of `continuous_keys`' lattice — the
/// one derivation the walker resolves an edge with and
/// [`edge_homophone_words`] enumerates the edge's words with.
pub(super) fn edge_lookup(
    continuous_keys: &crate::shadow::ContinuousKeys,
    start: usize,
    end: usize,
    mode: phonetics::InputMode,
) -> Option<EdgeLookup> {
    let crate::shadow::ContinuousKeys {
        shadow,
        shadow_to_raw_end,
        barriers,
        hyphen_runs,
        one_syllable_segments,
        ..
    } = continuous_keys;
    // Edges come from the syllabifier-built lattice so they are
    // well-formed by construction; the guard is defensive
    // (mirrors the `left_anchored_keys_and_restrictions` projection
    // guard) and also drops a digit-only / empty toneless span.
    if start >= end
        || end > shadow.len()
        || !shadow.is_char_boundary(start)
        || !shadow.is_char_boundary(end)
    {
        return None;
    }
    // v3.5.9 D / C-3b — mode-aware tone strip. TL/POJ/English drop
    // ASCII digits (byte-identical to legacy `strip_ascii_tone_digits`),
    // TPS drops the 8 Bopomofo tone marks so the body matches the
    // `tps:<tps_notone>` family from C-0.
    let toneless = strip_tones_for_mode(&shadow[start..end], mode);
    if toneless.is_empty() {
        return None;
    }
    let raw_span = (
        shadow_to_raw_end[start] as u32,
        shadow_to_raw_end[end] as u32,
    );
    // Explicit-tone fix — the DICT lookup is tone-aware: a fully-toned
    // edge (`tai5`) looks up the verbatim `tl:tai5` key so slot 0 can
    // only be synthesized from the typed tone, matching the span-local
    // list (a split — span-local toned, walker toneless — would let a
    // wrong-tone word reappear at slot 0). The §35 Final-only
    // restriction and the §17 / §41 tone pin come from the same
    // [`crate::shadow::span_key`] derivation the span-local keys use.
    let span_key = crate::shadow::span_key(
        shadow,
        start,
        end,
        mode,
        barriers,
        hyphen_runs,
        one_syllable_segments,
    )?;
    Some(EdgeLookup {
        toneless,
        raw_span,
        span_key,
    })
}

/// Every word the dictionary offers any edge of `continuous_keys`' lattice,
/// deduplicated, in edge order — the words whose user counts can change what
/// [`walk_buffer`] picks (the homophone it shows, and the user weight that
/// prices the edge). The dictionary's only: the caller adds the custom and
/// learned rows' words.
///
/// [`walk_buffer`]: super::walk::walk_buffer
pub(crate) fn edge_homophone_words(
    continuous_keys: &crate::shadow::ContinuousKeys,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut words = Vec::new();
    for &(start, end) in continuous_keys.lattice.edges() {
        let Some(EdgeLookup {
            raw_span, span_key, ..
        }) = edge_lookup(continuous_keys, start, end, ctx.mode)
        else {
            continue;
        };
        let edge_words = homophone_words_for_key(
            &span_key.key,
            &span_key.final_only,
            &span_key.tone_pin,
            raw_span,
            ctx,
        );
        words.extend(
            edge_words
                .into_iter()
                .filter(|word| seen.insert(word.clone())),
        );
    }
    words
}

/// The toneless body of every edge of `continuous_keys`' lattice,
/// deduplicated, in edge order — the keys a custom word or a learned phrase
/// is filed under when it can be one of the walk's edges.
pub(crate) fn edge_toneless_bodies(
    continuous_keys: &crate::shadow::ContinuousKeys,
    mode: phonetics::InputMode,
) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    continuous_keys
        .lattice
        .edges()
        .iter()
        .filter_map(|&(start, end)| edge_lookup(continuous_keys, start, end, mode))
        .map(|lookup| lookup.toneless)
        .filter(|body| seen.insert(body.clone()))
        .collect()
}
