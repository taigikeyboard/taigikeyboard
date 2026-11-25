//! Hàn-Lô lām (漢羅濫, `AppConfig.han_lo_mixed`): every dictionary word also
//! offers its romanization as a word of its own.
//!
//! For each reading a fetch found — one `(consumed_span, canonical-TL)` among
//! its Hanji rows — [`add_roman_twins`] adds one romanization-only row whose
//! identity is the `(canonical-TL, canonical-TL)` pair a roman-only dictionary
//! row already has (Core Principle #6). So:
//!
//! - its selection weight and context rank are its own — picking `tâi-gí`
//!   lifts the romanization, never 台語, and the reverse;
//! - its prior is its best Hanji homophone's corpus cost plus
//!   [`ROMAN_TWIN_COST_PENALTY`];
//! - a romanization-only row the dictionary already carries for the reading
//!   is that reading's row, repriced from the same homophone — minus
//!   [`LKK_ROMAN_COST_BONUS`] for an LKK recommendation (李江却台語文教基金會
//!   漢羅合用建議用字: `kah`, `ka-tsiah`, `mā`), so it leads its Hanji
//!   homophones — never worse than its own corpus price — and shares a
//!   bundled previous-word context hit on them (`我` → 毋 lifts `m̄` too).
//!
//! Every phonetic fetch runs it after its custom / learned merge and dedupe,
//! so the user's words get their romanization too and a romanization-only
//! user row answers its reading: the span-local list and the partial-prefix
//! extensions ([`merge_custom_dedupe_sort`](super::candidate::merge_custom_dedupe_sort)),
//! the abbreviation block, and the walker's edge pick
//! ([`best_candidate_for_key_with_barriers`](super::best_candidate_for_key_with_barriers)).
//! A romanization is thus a word of the whole-sentence path exactly as a Hanji
//! word is — it wins its edge on the user's picks, the previous word's context
//! or an LKK recommendation — while the edge keeps the price it has without
//! it (the dictionary's span cost and the Hanji winner's bigram), so the
//! segmentation never moves.

use std::collections::HashMap;

use ranking::{CONTEXT_RANK_BUNDLED, COVERAGE_KIND_ABBREV};

use super::{ContinuousFetchCtx, RawCandidate};
use crate::dictionary_reader::{LKK_BIT, WALKER_COST_UNPRICED};

/// How much less likely the romanization of a reading is written than its
/// most written Hanji, in walker milli-nats (`−ln p × 1000`): 1000 ≈ ×0.37.
/// A tuning knob — no Han-Lo corpus prices it yet.
pub const ROMAN_TWIN_COST_PENALTY: u16 = 1000;

/// How far an LKK romanization-only recommendation sorts ahead of its best
/// Hanji homophone, in walker milli-nats.
pub const LKK_ROMAN_COST_BONUS: u16 = 1;

fn has_hanji(candidate: &RawCandidate) -> bool {
    candidate
        .hanji
        .as_deref()
        .is_some_and(|hanji| !hanji.is_empty())
}

/// The prior cost of a romanization whose best Hanji homophone costs
/// `sibling`, capped below [`WALKER_COST_UNPRICED`] so a priced reading stays
/// priced.
fn roman_prior(sibling: u16, recommended: bool) -> u16 {
    if sibling == WALKER_COST_UNPRICED {
        WALKER_COST_UNPRICED
    } else if recommended {
        sibling.saturating_sub(LKK_ROMAN_COST_BONUS)
    } else {
        sibling
            .saturating_add(ROMAN_TWIN_COST_PENALTY)
            .min(WALKER_COST_UNPRICED - 1)
    }
}

/// When `ctx.roman_twins` is on: gives every reading among `rows`' Hanji rows
/// its romanization-only row, and reprices a romanization-only dictionary row
/// that has a Hanji homophone. Order is left to the caller's sort.
pub(super) fn add_roman_twins(rows: &mut Vec<RawCandidate>, ctx: &ContinuousFetchCtx<'_>) {
    if !ctx.roman_twins {
        return;
    }
    // The pricing Hanji row per reading: the one the corpus writes most
    // (lowest `walker_cost`; the first listed on a tie). Never the user's
    // preference order — a picked, custom or learned homophone (§42: picking
    // Hanji never affects its romanization), nor an unpriced user row over a
    // priced dictionary one.
    let mut best: HashMap<((u32, u32), &str), usize> = HashMap::new();
    for (index, row) in rows.iter().enumerate().filter(|(_, row)| has_hanji(row)) {
        let reading = (row.consumed_span, row.canonical_tl.as_str());
        best.entry(reading)
            .and_modify(|at| {
                if row.walker_cost < rows[*at].walker_cost {
                    *at = index;
                }
            })
            .or_insert(index);
    }
    if best.is_empty() {
        return;
    }
    let best: HashMap<((u32, u32), String), usize> = best
        .into_iter()
        .map(|((span, tl), at)| ((span, tl.to_owned()), at))
        .collect();
    // Readings a romanization-only row (dictionary, custom, learned) already
    // answers get no twin.
    let answered: std::collections::HashSet<((u32, u32), String)> = rows
        .iter()
        .filter(|row| !has_hanji(row))
        .map(|row| (row.consumed_span, row.canonical_tl.clone()))
        .filter(|reading| best.contains_key(reading))
        .collect();
    // Reprice the dictionary's own romanization rows from their homophone.
    let sibling_cost: HashMap<&((u32, u32), String), u16> = best
        .iter()
        .map(|(reading, &at)| (reading, rows[at].walker_cost))
        .collect();
    // Readings whose Hanji the bundled previous-word context names (`我` →
    // 毋 / `m̄`): the LKK recommendation shares that hit, so the bundled
    // bigram, which keys Hanji only, does not undo the recommendation. The
    // user's own context (`CONTEXT_RANK_USER`) is theirs and stays the
    // Hanji's.
    let bundled_context: std::collections::HashSet<((u32, u32), &str)> = rows
        .iter()
        .filter(|row| has_hanji(row) && row.context_rank == CONTEXT_RANK_BUNDLED)
        .map(|row| (row.consumed_span, row.canonical_tl.as_str()))
        .collect();
    let repriced: Vec<(usize, u16, u8)> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| !has_hanji(row) && !row.is_custom)
        .filter_map(|(index, row)| {
            let reading = (row.consumed_span, row.canonical_tl.clone());
            let sibling = *sibling_cost.get(&reading)?;
            let recommended = row.bitmask & LKK_BIT != 0;
            let context_rank = if recommended
                && bundled_context.contains(&(row.consumed_span, row.canonical_tl.as_str()))
            {
                row.context_rank.min(CONTEXT_RANK_BUNDLED)
            } else {
                row.context_rank
            };
            Some((
                index,
                row.walker_cost.min(roman_prior(sibling, recommended)),
                context_rank,
            ))
        })
        .collect();
    for (index, cost, context_rank) in repriced {
        rows[index].walker_cost = cost;
        rows[index].context_rank = context_rank;
    }
    let mut twins: Vec<(usize, RawCandidate)> = best
        .iter()
        .filter(|(reading, _)| !answered.contains(*reading))
        .map(|(_, &at)| (at, twin_of(&rows[at], ctx)))
        .collect();
    // Deterministic insertion order: the sibling's.
    twins.sort_by_key(|(at, _)| *at);
    rows.extend(twins.into_iter().map(|(_, twin)| twin));
}

/// The romanization-only row of `sibling`'s reading, with the romanization's
/// own selection weight and context rank. It keeps the sibling's `rowid`,
/// so the walker's bigram prices it as the word it writes.
fn twin_of(sibling: &RawCandidate, ctx: &ContinuousFetchCtx<'_>) -> RawCandidate {
    let display_text = sibling.canonical_tl.clone();
    let abbreviation = sibling.coverage_kind == COVERAGE_KIND_ABBREV;
    RawCandidate {
        user_weight: ctx
            .freq_map
            .get(&display_text, &sibling.canonical_tl)
            .weight_for(abbreviation, ctx.now_ms),
        context_rank: ctx.context.rank(&display_text, &sibling.canonical_tl),
        walker_cost: roman_prior(sibling.walker_cost, false),
        hanji: None,
        roman: sibling.roman.clone(),
        display_text,
        is_custom: false,
        ..sibling.clone()
    }
}
