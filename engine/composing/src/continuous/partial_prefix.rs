//! The partial-prefix fetch (step 2 empty-keys arm, step 4b extension scan).

use super::recase::recase_all;

use crate::shadow::build_partial_prefix_key;
use lexicon::{ConsumedSpan, ContinuousFetchCtx, RawCandidate};

/// A lexicon partial-prefix fetch: the bounded
/// [`fetch_partial_prefix_candidates`] or the un-truncated
/// [`fetch_partial_prefix_candidates_unbounded`].
///
/// [`fetch_partial_prefix_candidates`]: lexicon::fetch_partial_prefix_candidates
/// [`fetch_partial_prefix_candidates_unbounded`]: lexicon::fetch_partial_prefix_candidates_unbounded
pub(super) type PartialPrefixFetch =
    fn(&(ConsumedSpan, String), u32, &ContinuousFetchCtx<'_>) -> Vec<RawCandidate>;

/// v3.5.9 A2 — partial-prefix fetch inner. Pre-A2 `fetch_via_lexicon_partial`'s
/// body minus the `LexiconHandle::with_state` opener.
/// [`build_partial_prefix_key`] (renamed from `_tl` suffix in v3.5.9
/// B-2 since the emitter is now mode-aware) is pure (shadow primitives
/// only) and runs ahead of any state borrow.
///
/// v3.5.9 D7 — takes [`ContinuousFetchCtx`]; `mode` is kept as a
/// separate arg because it is shadow-key construction input, not part
/// of the shared lexicon ctx.
///
/// v3.5.9 B-0c — `mode: phonetics::InputMode` replaces the prior
/// `is_poj: bool` (`mode == InputMode::Poj` preserves the old gate).
/// `mode` flows untouched through to [`build_partial_prefix_key`],
/// keeping the seam's single-source-of-truth invariant. v3.5.9 B-2
/// makes the emitter mode-aware so POJ partial-prefix keys land in the
/// `poj:` family rather than folding through the TL chain.
///
/// `fetch`: the empty-keys branch passes the bounded
/// [`fetch_partial_prefix_candidates`] (no FULL block to exclude against).
/// v3.5.9 Step 4b passes [`fetch_partial_prefix_candidates_unbounded`]: it
/// applies its FULL-block exclude on the un-truncated pool, then truncates
/// itself — addresses Codex PR #351 r3321758666 (exact-key homophones
/// would otherwise eat the entire output cap before the dedupe ran).
///
/// [`fetch_partial_prefix_candidates`]: lexicon::fetch_partial_prefix_candidates
/// [`fetch_partial_prefix_candidates_unbounded`]: lexicon::fetch_partial_prefix_candidates_unbounded
pub(super) fn fetch_via_lexicon_partial_inner(
    raw: &str,
    raw_len: u32,
    mode: phonetics::InputMode,
    ctx: &ContinuousFetchCtx<'_>,
    fetch: PartialPrefixFetch,
) -> Vec<RawCandidate> {
    let Some(key) = build_partial_prefix_key(raw, mode) else {
        return Vec::new();
    };
    let mut out = fetch(&key, raw_len, ctx);
    // Partial-prefix candidates carry `consumed_span = (0, raw_len)` by
    // construction (`build_partial_prefix_key` pins the span). Recase against
    // the full raw buffer so the keys-empty branch and the Step 4b extension
    // path apply the SAME per-candidate case mapping the span-local Step 3
    // loop applies to its FULL-coverage hits. Without this, the case display
    // of a partial-prefix hit flips between branches as the user crosses the
    // first syllable boundary (e.g. typing `T` → keys.is_empty() arm,
    // unrecased; then `Ta` → else-arm + Step 4b, recased).
    if let Some(seg) = raw.get(..raw_len as usize) {
        recase_all(&mut out, seg, mode);
    }
    out
}
