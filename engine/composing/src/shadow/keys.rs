// Shadow-derived lookup keys: tone body, continuous / span keys, tone pins,
// syllabification helpers, custom / partial-prefix / abbreviation keys.

use lexicon::{ConsumedSpan, SyllableInventory, TonePin, TpsClose, TpsCloseKind, TypedBoundary};
use phonetics::{InputMode, KeyFamily};

use crate::lattice::Lattice;
use crate::syllabifier::tl::is_tl_tone_digit;
use crate::syllabifier::valid_span_endings_lowered_with_barriers;

use super::lattice::{build_shadow_lattice_with_barriers, ShadowLattice};
use super::{fused_shadow, fused_shadow_with_barriers, lookup_form, strip_tones_for_mode};

/// True iff `span` is a fully-toned TL/POJ reading: a non-empty sequence
/// of `(syllable digit)` groups — every syllable carries an ASCII tone
/// digit (e.g. `tai5`, `tai5gi2`, `kak4`) — with no orphan/leading digit
/// and a trailing tone digit. Such a span maps verbatim onto the
/// digit-separated, hyphenless `tl_num` / `poj_num` FST key family
/// (`dictionary/build/create_fst.py::collect_pairs` emits `tl:<tl_num>` for
/// every record), so an exact lookup on the verbatim span filters
/// candidates to exactly the typed tone(s).
///
/// Syllable-aware: each letter group a digit closes must be ONE
/// phonotactically valid syllable ([`phonetics::is_valid_syllable`], the
/// `TL_INITIALS × TL_FINALS` gate with POJ spelling folded to TL). A fused
/// group carrying an untoned syllable (`kokbin5tong2`, `tengsek4`) is a
/// PARTIAL-tone span with no FST key of its own → false → toneless key +
/// [`TonePin::TypedTones`], the path a tail-untoned `kokbin5tong` already
/// takes (§17 case 3). Phonotactics rather than the syllable inventory:
/// this is a property of the typed SHAPE and must not drift with the
/// dictionary build. A group that is a valid syllable yet could also
/// split (`ai5` as `a`+`i5`) still keys verbatim — unchanged.
///
/// The span-local + walker callers pass a lattice edge (which proves the
/// SPAN splits into syllables, not that every digit-closed group is one);
/// the partial-prefix caller ([`build_partial_prefix_key`]) passes the raw
/// whole-buffer shadow, possibly an INCOMPLETE syllable (`ts5`, `abc1`) —
/// that keys toneless + pin, and the pin is fail-closed on a spelling
/// mismatch, so it can widen the prefix scan but never admit a wrong tone.
fn span_is_fully_toned_ascii(span: &str) -> bool {
    if span.is_empty() {
        return false;
    }
    let mut group_start = 0;
    for (i, b) in span.bytes().enumerate() {
        if b.is_ascii_digit() {
            // A tone digit closes the current syllable group; an empty
            // group is an orphan digit (`5tai`, `tai55`).
            let group = &span[group_start..i];
            if group.is_empty() || !phonetics::is_valid_syllable(group) {
                return false;
            }
            group_start = i + 1;
        } else if !b.is_ascii_alphabetic() {
            return false; // leaked hyphen / non-ASCII — not a clean toned reading
        }
    }
    // A trailing letter leaves a group open (un-toned) → not fully toned.
    group_start == span.len()
}

/// TPS analogue of [`span_is_fully_toned_ascii`]: true iff `span` matches the
/// `(bopomofo-body+ mark-bearing-tone)+` grammar — a non-empty sequence of
/// Bopomofo bodies each closed by a standalone TPS tone scalar (tones
/// 2/3/5/6/7/8/9 per `phonetics::is_tps_tone_mark`, including the
/// `U+02D9`/`U+0307` tone-8 pair) — with no orphan/leading mark and a
/// **trailing** mark. Such a span maps verbatim, after tone-8 scalar
/// normalization, onto the `tps:<tps_num>` FST family, so an exact lookup
/// filters candidates to the typed tone(s).
///
/// **Tones 1 and 4 produce no mark, so a span whose LAST syllable is tone-1/4
/// returns false → toneless all-tones key.** Tone-1 carries no mark (its
/// keyboard space separator is stripped upstream by [`build_separator_shadow`])
/// and tone-4 is a bare stop-coda glyph (ㆴㆵㆻㆷ, part of the body, NOT in
/// `is_tps_tone_mark`); neither has a distinguishing `tps:<tps_num>` key
/// (`tps_num == tps_notone`), so the toneless key is the only key they can
/// take. A3 (§41) narrows what that means at the CANDIDATE layer: when the
/// span ends on the stripped keyboard space, the same toneless lookup runs
/// but the results are filtered to the unmarked tone
/// ([`tps_span_closes`]). Surfacing all tones stays correct only
/// for a tone-1/4 span the user has NOT closed with a space.
///
/// **Text-only, unlike [`span_is_fully_toned_ascii`]**: with no syllable
/// splitter it cannot split a fused multi-syllable body, so a tone-1/4
/// syllable that is *leading or interior* (followed later by a marked
/// syllable) is NOT detected — `ㄍㄠㄉㄞˊ` (kau1+tai5, e.g. the §18 `ㄍㄠ ␣ ㄉㄞˊ`
/// space-phrase after the separator strip) reads as fully toned and keys the
/// verbatim `tps:ㄍㄠㄉㄞˊ`. This is exact, NOT a wrong-tone hit: a tone-1/4
/// syllable contributes no mark to `tps_num` either, so the verbatim span
/// equals the phrase's real toned key (kau**1**-tai5) — the unmarked leading
/// syllable resolves to its no-mark tone, mirroring how the TL/POJ path keys
/// `tl:taigi5` for `taigi5`. The per-syllable all-tones affordance for the
/// unmarked syllable is preserved separately via the shorter single-syllable
/// span (`tps:ㄍㄠ`, toneless), which §18 keeps alongside the phrase; only the
/// multi-syllable PHRASE candidate is tone-pinned. A fully-toned-LOOKING but
/// nonexistent body simply MISSES the FST (zero candidates).
///
/// The span reaching here is already hyphen- and separator-stripped, so it is
/// pure Bopomofo + tone marks; any other char is a leak and returns false.
fn span_is_fully_toned_tps(span: &str) -> bool {
    if span.is_empty() {
        return false;
    }
    let mut group_has_body = false;
    for c in span.chars() {
        if phonetics::is_tps_tone_mark(c) {
            if !group_has_body {
                return false; // orphan tone mark — no body opened this group
            }
            group_has_body = false; // tone mark closes the current syllable group
        } else if phonetics::is_tps_char(c) {
            group_has_body = true;
        } else {
            return false; // leaked separator / non-TPS — not a clean toned reading
        }
    }
    // A trailing un-marked body (tone 1 / tone 4) leaves a group open.
    !group_has_body
}

/// FST lookup body for a continuous-input span. When `span` is a
/// fully-toned reading, return its verbatim toned key so
/// `lookup_exact` / `lookup_prefix` filters by the typed tone(s) — the fix
/// for the bug where explicit `tai5` surfaced every tone of `tai`:
/// - **TL / POJ** ([`span_is_fully_toned_ascii`]): digits kept verbatim,
///   matching the `tl:<tl_num>` / `poj:<poj_num>` family.
/// - **TPS** ([`span_is_fully_toned_tps`]): Bopomofo + tone marks kept, with
///   the tone-8 dot normalized `U+02D9 → U+0307`
///   ([`phonetics::normalize_tps_lookup_scalar`]) so it matches the
///   `tps:<tps_num>` family the build pipeline stores. Tones 1/4 carry no
///   mark and fall through to the toneless branch (see
///   [`span_is_fully_toned_tps`]).
///
/// Otherwise return the toneless form ([`strip_tones_for_mode`]): toneless
/// continuous input intentionally surfaces all tones, and mixed/partial-tone
/// spans have no fully-toned FST key family. This is a key-SELECTION rule
/// (one branch per span shape), NOT a runtime fallback — there is no
/// "toned miss → retry toneless" path.
///
/// `English` mode has no tone semantics (a trailing digit in an English
/// buffer is not a tone), so it keeps the legacy digit-strip.
pub(crate) fn fst_body_for_span(span: &str, mode: InputMode) -> String {
    match mode {
        InputMode::Tl | InputMode::Poj if span_is_fully_toned_ascii(span) => span.to_string(),
        InputMode::Tps if span_is_fully_toned_tps(span) => span
            .chars()
            .map(phonetics::normalize_tps_lookup_scalar)
            .collect(),
        _ => strip_tones_for_mode(span, mode),
    }
}

/// The continuous-input lookup keys for `raw`, plus the per-key
/// barrier metadata the §35 ambiguity-aware lookup needs.
///
/// Returns `(keys, per-key final-only offsets, base (shadow, map,
/// lattice), barriers)`:
/// - `keys[i]` is the literal left-anchored span key exactly as before
///   this round — ambiguity resolution happens at LOOKUP time
///   (`lexicon::PrefixIndex::lookup_exact_tps_readings`), so the key
///   text stays the user's letters and the substitution-count ordering
///   has a stable baseline.
/// - `final_only[i]` = byte offsets into `keys[i]` (family prefix
///   included) of glyphs immediately before a stripped separator / hyphen
///   barrier: those pattern slots keep only Final-role readings (§31 —
///   the user's explicit boundary is never re-read as an onset).
/// - the base triple feeds the whole-sentence walker, which resolves
///   its edge keys through the same expanded lookup.
/// - `barriers` (shadow coordinates) let the walker compute the same
///   per-edge restriction for interior edges; `hyphen_runs` are the typed
///   `-` runs among them, read by [`span_key`] for the boundary kinds
///   (§52) and by the §55 render for what to write there.
///
/// Single source for `continuous::assemble_candidates` and the
/// `requests::build_continuous_keys_with_inventory` test seam, so the
/// two cannot drift. Empty `final_only` for TL / POJ / English — their
/// pipeline strips no ambiguity glyph; `barriers` there are the typed
/// hyphens (§52).
pub(crate) struct ContinuousKeys {
    pub keys: Vec<(ConsumedSpan, String)>,
    pub final_only: Vec<Vec<usize>>,
    /// `tone_pins[i]` is the post-lookup tone constraint for `keys[i]`
    /// ([`span_key`] → [`span_tone_pin`]): the §17 typed digits of a
    /// TL/POJ span, or the syllable closes (tone marks, keyboard spaces)
    /// of a TPS span (§17 case 4, §41). The lookup layer keeps only
    /// candidates whose reading honors it.
    pub tone_pins: Vec<TonePin>,
    pub shadow: String,
    pub shadow_to_raw_end: Vec<usize>,
    pub lattice: Lattice,
    pub barriers: Vec<usize>,
    /// The typed `-` runs among them, `(shadow offset, run length)`.
    pub hyphen_runs: Vec<(usize, usize)>,
    /// [`ShadowLattice::one_syllable_segments`].
    pub one_syllable_segments: Vec<(usize, usize)>,
}

/// The three parallel per-key vectors [`left_anchored_keys_and_restrictions`]
/// emits, kept together so the indices cannot drift apart at a call site.
/// [`ContinuousKeys`] carries the same three plus the shadow/lattice base.
pub(crate) struct LeftAnchoredKeys {
    pub keys: Vec<(ConsumedSpan, String)>,
    pub final_only: Vec<Vec<usize>>,
    pub tone_pins: Vec<TonePin>,
}

pub(crate) fn build_continuous_keys(
    raw: &str,
    inv: &SyllableInventory,
    mode: InputMode,
) -> ContinuousKeys {
    let shadow_lattice = build_shadow_lattice_with_barriers(raw, inv, mode);
    let LeftAnchoredKeys {
        keys,
        final_only,
        tone_pins,
    } = left_anchored_keys_and_restrictions(&shadow_lattice, inv, mode);
    let ShadowLattice {
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        hyphen_runs,
        one_syllable_segments,
        ..
    } = shadow_lattice;
    ContinuousKeys {
        keys,
        final_only,
        tone_pins,
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        hyphen_runs,
        one_syllable_segments,
    }
}

/// v3.5.9 A1 — extracted from the pre-A1 key-building
/// loop. Emit ONLY the lattice's left-anchored (`start == 0`)
/// projection as keys, byte-identical to the pre-S1 single-start
/// `valid_span_endings(shadow, 0, …)` output (`build_lattice` sorts
/// edges so the `start == 0` ones come first in ascending-`end`
/// order), so the span-local candidate / commit path is unchanged
/// from S1.
///
/// Interior (`start > 0`) edges are NOT emitted as user-facing keys:
/// under Model B (`docs/engine/continuous-commit-and-display.md`
/// §10.3/§10.4) commit is forward-only `pending[..consumed_bytes]`,
/// so an independently tappable interior candidate has no
/// Model-B-consistent commit. S2's whole-sentence walker consumes
/// the interior edges INTERNALLY (via
/// [`build_shadow_lattice_with_barriers`] + `crate::lattice::walk_best`
/// in `fetch_walker_slot0`) and emits one
/// synthesized full-buffer best path explicitly prepended at slot 0
/// by `handle_fetch_at_pos`; the user-facing commit span stays
/// `(0, end)`. Interior `台語`-style words remain reachable as the
/// next path-step after the prefix is nailed (Codex pre-impl S2
/// Q1c = option ii, 2026-05-16; `docs/releases/v3.5.8/plan.md` §整句 lattice + walker).
///
/// Each key also carries its §35 barrier restriction — byte offsets
/// (into the emitted key string, family prefix included) of glyphs
/// immediately before a stripped separator / hyphen barrier — and its §41
/// tone pin. The barriers are shadow coordinates from
/// [`build_shadow_lattice_with_barriers`]; for TL/POJ/English they are the
/// typed hyphens (§52).
pub(crate) fn left_anchored_keys_and_restrictions(
    shadow_lattice: &ShadowLattice,
    inv: &SyllableInventory,
    mode: InputMode,
) -> LeftAnchoredKeys {
    let ShadowLattice {
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        hyphen_runs,
        one_syllable_segments,
        ..
    } = shadow_lattice;
    // v3.5.9 B-2 — `mode` selects the FST key family the emitted keys are
    // namespaced into ([`span_key`]). The lattice itself was already built
    // against the matching `SyllableInventory` family
    // ([`build_shadow_lattice_with_barriers`] → [`build_lattice`]), so the
    // syllabification and the key namespace come from a single mode
    // parameter — they cannot drift.

    // Longest-match prefix suppression (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`,
    // USER 2026-05-31: "suppress even without a tone"): among the SINGLE-syllable spans anchored at
    // offset 0, surface only the LONGEST. A shorter single syllable that is a
    // strict prefix of a longer one (`ta`⊂`tai`⊂`tai5`, `tsu`⊂`tsua`) is
    // dropped — fixing the reported bug where typing a complete syllable
    // (`tai` / `tai5`) surfaced 2-letter `ta` candidates. Keys on span length,
    // not tone, so it covers toned + toneless alike and subsumes the
    // explicit-tone case (§17).
    //
    // The single-syllable ends are recomputed via the SAME `max_syllables = 1`
    // primitive `build_lattice` chains (`lattice.edges()` flattens depth, so a
    // `(0, end)` edge cannot be told apart from a chain reaching `end` —
    // re-running the depth-1 walk is the only way to isolate true single
    // syllables). Lowercasing once mirrors `build_lattice`, which also
    // re-lowercases the shadow before walking.
    //
    // An end is suppressed only when it is (a) a single-syllable end, (b) not
    // the longest single syllable, (c) has NO multi-syllable phrase
    // reading — i.e. no interior edge `(m, end)` with `m > 0` reaches it —
    // AND (e) under TPS is not a straddled prefix (below). (c)
    // is the safety guard: a shorter span that ALSO parses as a phrase
    // (`a`+`i` ending where `ai` is a single syllable too) is a legitimate
    // different-word candidate and must survive. In practice (b)+(c) coincide
    // for the reported bug (`ta`/`tsu` have no interior predecessor), but (c)
    // makes the rule provably never drop a phrase candidate. Display layer
    // only: the lattice keeps every edge, so the walker
    // (`fetch_walker_slot0_inner`) + min-hop `span_min_syllable_count` still
    // see every split (non-greedy `ta`+`nia` recovery, Codex PR #290 P1,
    // unaffected).
    //
    // (d) The phrase reading does NOT rescue a closed dead end (USER report
    // 2026-09-19: `iah8` / `ioh8` / `iok8` trailed the whole `ia` / `io`
    // family). `iah8` parses as `i`+`a` at end 2, so (c) kept `ia` — but no
    // lattice edge leaves end 2 and the remainder `h8` already carries a
    // typed tone, so it can never grow into a syllable: committing 也 would
    // strand it. Both halves are required; the mid-typing controls
    // (`iah`, `iakau3`) live in `tests/continuous_keys_tl_lattice.rs`.
    //
    // (e) TPS only — straddled prefix (user report 2026-10-10: toneless
    // `ㄌㄧㄏㄛ` listed li̍h 裂 but never li 你, while the walker cut li | ho).
    // A shorter single survives when one syllable leaving its end runs past
    // the longest single's end: the longest single then ate the start of that
    // syllable (`ㄌㄧㄏ` eats the ㄏ of `ㄏㄛ`; `ㄍㄚㄉㄚ` keeps ka beside kat;
    // `ㄍㄧㄨㄧ` keeps ki beside kiu). A syllable typed alone (`ㄍㄚㆵ`,
    // `ㄍㄧㄨ`) has nothing past it, so the prefix is still dropped. The
    // shape is not TPS-specific (TL `liho` has it too); the mode gate is a
    // product decision — TL / POJ keep (a)–(d) unchanged (maintainer
    // 2026-10-10). Depth-1 recompute again: lattice edges leaving `end` also
    // hold chains, which would rescue `ta` in `taigi` (`i`+`gi` runs past
    // `tai`).
    let lowered = shadow.to_ascii_lowercase();
    // §18 recompute is barrier-aware (Codex post-impl 2026-08-19 BLOCK 2):
    // the lattice above was built with barriers, so re-deriving the
    // single-syllable ends WITHOUT them can manufacture a longer
    // "single syllable" that fuses across the user's separator (`ㄍㄚ`␣`ㄉ`
    // mid-typing: a barrier-blind recompute reads ㄍㄚㆵ as the longest
    // single and wrongly suppresses the legitimate ㄍㄚ).
    let single_ends = valid_span_endings_lowered_with_barriers(&lowered, 0, inv, mode, 1, barriers);
    let max_single_end = single_ends.iter().copied().max();
    let has_phrase_reading = |end: usize| lattice.edges().iter().any(|&(s, e)| e == end && s > 0);
    let is_closed_dead_end = |end: usize| {
        lattice.edges().iter().all(|&(s, _)| s != end)
            && remainder_has_closed_syllable(shadow, end, mode, barriers)
    };
    let survives_as_phrase = |end: usize| has_phrase_reading(end) && !is_closed_dead_end(end);
    let is_straddled_prefix = |end: usize| {
        matches!(mode, InputMode::Tps)
            && valid_span_endings_lowered_with_barriers(&lowered, end, inv, mode, 1, barriers)
                .into_iter()
                .any(|next_end| Some(next_end) > max_single_end)
    };

    let mut out = Vec::with_capacity(lattice.edges().len());
    let mut restrictions: Vec<Vec<usize>> = Vec::with_capacity(lattice.edges().len());
    let mut tone_pins: Vec<TonePin> = Vec::with_capacity(lattice.edges().len());
    for &(start, end) in lattice.edges() {
        if start != 0 {
            continue;
        }
        // Guards mirror the pre-S1 single-start loop exactly (`start`
        // is 0 here, so the slice / offset map is identical).
        if end == 0 || end > shadow.len() || !shadow.is_char_boundary(end) {
            continue;
        }
        // Drop a strictly-shorter single-syllable-only prefix span (see the
        // (a)–(e) rule above). Longest single, phrase ends, phrase-reachable
        // shorter spans that can still continue, and TPS straddled prefixes
        // are kept.
        if single_ends.contains(&end)
            && Some(end) != max_single_end
            && !survives_as_phrase(end)
            && !is_straddled_prefix(end)
        {
            continue;
        }
        // Tone-aware lookup body + §35 / §41 barrier metadata, same
        // derivation as the walker edge ([`span_key`]); a digit-only /
        // bare-tone-mark span has no key.
        let Some(SpanKey {
            key,
            final_only,
            tone_pin,
        }) = span_key(
            shadow,
            0,
            end,
            mode,
            barriers,
            hyphen_runs,
            one_syllable_segments,
        )
        else {
            continue;
        };
        let raw_end = shadow_to_raw_end[end];
        restrictions.push(final_only);
        tone_pins.push(tone_pin);
        out.push(((0u32, raw_end as u32), key));
    }
    LeftAnchoredKeys {
        keys: out,
        final_only: restrictions,
        tone_pins,
    }
}

/// True when the user has already closed a syllable somewhere in
/// `shadow[end..]`, so that remainder can no longer be an open pending
/// tail (§18 guard (d)). What closes a syllable per family: a typed TL /
/// POJ tone digit, a TPS tone mark, or a §41 stripped-space barrier
/// strictly after `end` (one AT `end` closes the span itself, not its
/// remainder; the barrier closes the syllable on its unmarked tone).
/// English carries no tone marks, so its remainder is never closed.
fn remainder_has_closed_syllable(
    shadow: &str,
    end: usize,
    mode: InputMode,
    barriers: &[usize],
) -> bool {
    let remainder = &shadow[end..];
    let barrier_after_end = || barriers.iter().any(|&b| b > end);
    match mode {
        // §52: a typed `-` past `end` closes the syllable before it the
        // way a tone digit does (`iah-` can no longer grow past `h`).
        InputMode::Tl | InputMode::Poj => {
            remainder.bytes().any(is_tl_tone_digit) || barrier_after_end()
        }
        InputMode::Tps => remainder.chars().any(phonetics::is_tps_tone_mark) || barrier_after_end(),
        InputMode::English => false,
    }
}

/// One span's FST lookup key plus its §35 / §41 barrier metadata, as
/// [`span_key`] derives it for both the left-anchored keys and the walker
/// edges.
pub(crate) struct SpanKey {
    /// `"{prefix}{body}"` — [`KeyFamily::prefix`] + [`fst_body_for_span`].
    pub key: String,
    /// [`key_final_only_offsets`] — KEY byte offsets of the glyph before
    /// each barrier inside / at the end of the span.
    pub final_only: Vec<usize>,
    /// [`span_tone_pin`] — the §17 typed digits of a TL/POJ span, or the
    /// typed syllable closes of a TPS span (§17 case 4, §41).
    pub tone_pin: TonePin,
}

/// Derive the lookup key triple for `shadow[start..end]`. `None` when the
/// tone-ruled body is empty (digit-only / bare-tone-mark span).
///
/// Barrier coordinates are span-local for both consumers:
/// - **Final-only offsets.** `barriers` (whole-shadow byte
///   offsets) are narrowed to `start < b <= end` and shifted by `-start`
///   before [`key_final_only_offsets`] maps them into the emitted key. A
///   barrier exactly at `start` is a boundary the span opens on, not one
///   it contains, so it is excluded; one at `end` is included (the user
///   closed that syllable).
/// - **The tone pin reads the same span-local barriers.** [`span_tone_pin`]
///   gets them as `boundaries`; its TPS closes sit in toneless-body offsets
///   of the span.
pub(crate) fn span_key(
    shadow: &str,
    start: usize,
    end: usize,
    mode: InputMode,
    barriers: &[usize],
    hyphen_runs: &[(usize, usize)],
    one_syllable_segments: &[(usize, usize)],
) -> Option<SpanKey> {
    let span = &shadow[start..end];
    let body = fst_body_for_span(span, mode);
    if body.is_empty() {
        return None;
    }
    let prefix = KeyFamily::for_input_mode(mode).prefix();
    let span_barriers = span_local_barriers(barriers, start, end);
    let final_only = key_final_only_offsets(span, &body, mode, &span_barriers, prefix.len());
    // §52 — the typed runs this span answers to: each interior / trailing
    // barrier with its kind, plus the run typed right before the span
    // (`at: 0`), which only a reading that itself opens with `--` reads.
    let leading = (start > 0 && barriers.contains(&start)).then_some(start);
    let boundaries: Vec<TypedBoundary> = leading
        .into_iter()
        .chain(span_barriers.iter().map(|b| b + start))
        .map(|at| TypedBoundary {
            at: at - start,
            khinsiann: hyphen_runs.iter().any(|&(b, run)| b == at && run >= 2),
        })
        .collect();
    // The one-syllable stretches inside the span, in span coordinates: a
    // reading may not split one (`ai-` drops 阿姨 `a-î`).
    let one_syllable_stretches: Vec<(usize, usize)> = one_syllable_segments
        .iter()
        .filter(|&&(segment_start, segment_end)| start <= segment_start && segment_end <= end)
        .map(|&(segment_start, segment_end)| (segment_start - start, segment_end - start))
        .collect();
    Some(SpanKey {
        key: format!("{prefix}{body}"),
        final_only,
        tone_pin: span_tone_pin(span, mode, boundaries, one_syllable_stretches),
    })
}

/// [`span_local_barriers`] for the OOV synth readings
/// ([`greedy_longest_syllabification`] / [`span_min_syllable_count`]),
/// under the `typed_hyphen_is_boundary` policy (§52).
pub(crate) fn oov_reading_barriers(
    barriers: &[usize],
    start: usize,
    end: usize,
    mode: InputMode,
) -> Vec<usize> {
    if crate::syllabifier::typed_hyphen_is_boundary(mode) {
        span_local_barriers(barriers, start, end)
    } else {
        Vec::new()
    }
}

/// The barriers strictly inside or at the end of `shadow[start..end]`,
/// shifted to span coordinates — the boundary in front of a span belongs
/// to the span before it, so a barrier AT `start` is dropped.
fn span_local_barriers(barriers: &[usize], start: usize, end: usize) -> Vec<usize> {
    barriers
        .iter()
        .filter(|&&b| b > start && b <= end)
        .map(|&b| b - start)
        .collect()
}

/// The post-lookup constraint for `span`, one rule per mode family:
/// - **TL / POJ**: a span carrying at least one ASCII tone digit or a
///   typed `-` boundary pins what it typed — [`TonePin::TypedTones`]
///   over the hyphenless span verbatim (`teng5sek`) plus `boundaries`
///   (§52: a reading must end a syllable on every one with the same
///   separator kind, so `khi|ah` drops 隙 `khiah`) and the one-syllable
///   stretches no reading may split (`ai-` drops 阿姨). The lookup body is
///   toneless for a partial-tone span (there is no partial-tone FST
///   family, §17 case 3) and verbatim for a fully-toned one (§17 case
///   1); the pin re-applies the typed digits after the lookup either
///   way, and is what keeps a toneless-matched custom entry honest for a
///   fully-toned span. A span with neither stays unpinned — the "show
///   all tones" affordance.
/// - **TPS**: a span carrying a tone mark or a keyboard space pins every
///   syllable close it typed — [`TonePin::TpsTyped`] over the toneless
///   body with [`tps_span_closes`] (§17 case 4, §41). The lookup body is
///   toneless for a partial-tone span exactly as in TL/POJ, so the pin is
///   what keeps `ㄉㄞ˫ㄨㄢ` from surfacing 台灣 `tâi` and `ㄌㄧˋㄫ` from
///   surfacing 靈 `lîng` through the §35 `ㄫ`→`ㄥ` reading.
/// - **English**: never pinned — a digit there is not a tone.
pub(crate) fn span_tone_pin(
    span: &str,
    mode: InputMode,
    boundaries: Vec<TypedBoundary>,
    one_syllable_stretches: Vec<(usize, usize)>,
) -> TonePin {
    match mode {
        InputMode::Tl | InputMode::Poj
            if span.bytes().any(|b| b.is_ascii_digit()) || !boundaries.is_empty() =>
        {
            TonePin::TypedTones {
                mode,
                typed: span.to_owned(),
                boundaries,
                one_syllable_stretches,
            }
        }
        InputMode::Tps => {
            let closes = tps_span_closes(span, &boundaries);
            if closes.is_empty() {
                TonePin::None
            } else {
                TonePin::TpsTyped {
                    body: strip_tones_for_mode(span, mode),
                    closes,
                }
            }
        }
        _ => TonePin::None,
    }
}

/// §17 case 4 + §41 — every syllable close typed in the TPS `span`, in
/// toneless-body offsets and in order: each tone mark
/// ([`TpsCloseKind::Marked`] with its tone), each space strictly inside the
/// span ([`TpsCloseKind::Boundary`], any tone — S18 `ㄉㄞ`␣`ㄍㄧ` opens on
/// tai5, S23 `ㄇ`␣`ㄒㄧ` 毋是 on m7, so an interior space is never a tone-1
/// instruction), and the space ending the span after an unmarked syllable
/// ([`TpsCloseKind::SpaceClosed`], §41: tone 1 / 4). A close needs a glyph
/// since the previous one, so an orphan mark, or a space right after a
/// mark, adds nothing. `boundaries` are the span-local barriers
/// [`span_key`] built (TPS barriers are spaces); the one at 0 is the
/// boundary the span opens on and closes nothing in it.
fn tps_span_closes(span: &str, boundaries: &[TypedBoundary]) -> Vec<TpsClose> {
    let is_barrier = |at: usize| at > 0 && boundaries.iter().any(|b| b.at == at);
    let mut closes: Vec<TpsClose> = Vec::new();
    let mut body_end = 0usize;
    for (at, c) in span.char_indices() {
        if is_barrier(at) {
            push_tps_close(&mut closes, body_end, TpsCloseKind::Boundary);
        }
        match phonetics::tps_tone_mark_number(c) {
            Some(tone) => push_tps_close(&mut closes, body_end, TpsCloseKind::Marked(tone)),
            None => body_end += c.len_utf8(),
        }
    }
    if is_barrier(span.len()) {
        push_tps_close(&mut closes, body_end, TpsCloseKind::SpaceClosed);
    }
    closes
}

/// Append a close at `body_end` unless no glyph was typed since the last one.
fn push_tps_close(closes: &mut Vec<TpsClose>, body_end: usize, kind: TpsCloseKind) {
    if body_end > closes.last().map_or(0, |last| last.body_end) {
        closes.push(TpsClose { body_end, kind });
    }
}

/// The whole typed buffer's [`TonePin`] — the counterpart of
/// [`span_tone_pin`] for the sources that carry no span key of their own:
/// custom-dictionary entries are synthesized at `(0, raw_len)` and the
/// partial-prefix extensions are hydrated from a strict prefix of the
/// buffer, so the lookup layer aligns their readings against this pin
/// instead. One projection of [`whole_buffer_span_key`], so it is
/// exactly the pin a span-local key over the whole buffer would carry:
/// a TL/POJ buffer with a typed digit or a typed `-` anywhere →
/// [`TonePin::TypedTones`]; a TPS buffer carrying a tone mark or a
/// keyboard space → [`TonePin::TpsTyped`] over the fused toneless body
/// (§17 case 4, §41); English, TL/POJ with neither and a TPS buffer with
/// neither → none. `inv` reads which typed-`-` stretches are one syllable (§52);
/// `None` marks none.
pub(crate) fn whole_buffer_tone_pin(
    raw: &str,
    mode: InputMode,
    inv: Option<&SyllableInventory>,
) -> TonePin {
    whole_buffer_span_key(raw, mode, inv)
        .map(|k| k.tone_pin)
        .unwrap_or_default()
}

/// The whole buffer as ONE span through [`span_key`]: the
/// [`fused_shadow_with_barriers`] pipeline (the same passes the lattice
/// path runs, offset maps discarded) and the tone rule of every other
/// key site. Shared by [`build_partial_prefix_key`] (takes the key) and
/// [`whole_buffer_tone_pin`] (takes the pin) so the two cannot drift.
/// `None` when the tone-ruled body is empty (empty / digit-only /
/// bare-tone-mark / space-only buffer).
fn whole_buffer_span_key(
    raw: &str,
    mode: InputMode,
    inv: Option<&SyllableInventory>,
) -> Option<SpanKey> {
    let (shadow, barriers, hyphen_runs) = fused_shadow_with_barriers(raw, mode);
    let one_syllable_segments = inv
        .map(|inv| crate::syllabifier::one_syllable_segments(&shadow, inv, mode, &barriers))
        .unwrap_or_default();
    span_key(
        &shadow,
        0,
        shadow.len(),
        mode,
        &barriers,
        &hyphen_runs,
        &one_syllable_segments,
    )
}

/// §35 barrier contract part (b) for one emitted key: for every barrier
/// inside (or at the end of) the span, the KEY byte offset of the glyph
/// just before it — the pattern slot that keeps only Final-role
/// readings. `body` is the already-computed lookup body for the span
/// (verbatim when fully toned, tone-stripped otherwise), so the offsets
/// account for stripped tone marks; `prefix_len` shifts them past the
/// `"tps:"` family prefix. A trailing barrier (separator at shadow end)
/// counts — the user closed that syllable, unlike a plain buffer end
/// which stays unrestricted. This only prunes the automaton; the span's
/// [`TpsCloseKind::Boundary`] / `SpaceClosed` close in [`span_tone_pin`] is
/// what requires a reading to end a syllable there (§17 case 4).
pub(crate) fn key_final_only_offsets(
    span_shadow: &str,
    body: &str,
    mode: InputMode,
    barriers: &[usize],
    prefix_len: usize,
) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for &barrier in barriers {
        if barrier == 0 || barrier > span_shadow.len() {
            continue;
        }
        let body_prefix_len = if body.len() == span_shadow.len() {
            barrier
        } else {
            strip_tones_for_mode(&span_shadow[..barrier], mode).len()
        };
        let Some((last_start, _)) = body[..body_prefix_len.min(body.len())]
            .char_indices()
            .last()
        else {
            continue;
        };
        let offset = prefix_len + last_start;
        if !out.contains(&offset) {
            out.push(offset);
        }
    }
    out
}

/// v3.5.8 S5 (Codex pre-impl Q2, 2026-05-17) — greedy longest-syllable
/// segmentation of `shadow`, the no-dict carve-out's user-facing
/// romanization reading.
///
/// From each offset, take the **longest** valid single syllable
/// (`valid_span_endings_lowered(.., max_syllables = 1)` → max ending)
/// and advance. This is the canonical romanization reading (khiin
/// longest-match family, `references/khiin-rs/khiin/src/data/segmenter.rs`):
/// `taiuantai → [tai, uan, tai]`. Literal *maximal-syllable-count*
/// would instead over-split into sub-syllables (`ta i u an …`),
/// reproducing the very over-segmentation S5 removes — so greedy-LONGEST
/// is deliberate, not max-segment.
///
/// Returns `None` when some offset has no valid syllable (the buffer
/// cannot be cleanly read syllable-by-syllable) — the caller then
/// suppresses the slot-0 synth and leaves the span-local list
/// untouched (pre-S2 behavior, same contract as
/// `continuous::synth_consumed_span`'s trailing-hyphen suppression).
/// `shadow` is ASCII-lowercased here; lowercasing is byte-length and
/// char-boundary preserving, so the returned offsets index `shadow`
/// identically.
pub(crate) fn greedy_longest_syllabification(
    shadow: &str,
    inv: &SyllableInventory,
    mode: InputMode,
    barriers: &[usize],
) -> Option<Vec<(usize, usize)>> {
    let lowered = shadow.to_ascii_lowercase();
    let mut segs: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0usize;
    while pos < lowered.len() {
        // v3.5.9 B-2: `mode` selects the inventory family that gates the
        // single-syllable step. The shadow is already in the matching
        // family's canonical ASCII form (B-2 reshape of
        // `canonicalize_poj_shadow` preserves POJ ASCII when `mode ==
        // Poj`), so a TL shadow walks the `tl:` family and a POJ shadow
        // walks the `poj:` family — both produce shadow-aligned offsets
        // because the inventory family's syllable boundaries match the
        // shadow form.
        let end = valid_span_endings_lowered_with_barriers(&lowered, pos, inv, mode, 1, barriers)
            .into_iter()
            .max()?;
        segs.push((pos, end));
        pos = end;
    }
    // Forward-only single-syllable steps land exactly on `len`; the
    // guard is belt-and-suspenders.
    (pos == lowered.len()).then_some(segs)
}

/// v3.5.8 OOV-cost fix (Codex PR #290 P1 `r3255035136`, 2026-05-18) —
/// the **guaranteed** syllable count of `shadow_span`: the minimum
/// number of single-syllable hops to cover it. Each hop is one valid
/// syllable from `valid_span_endings_lowered(.., max_syllables = 1)` —
/// the same per-syllable step `lattice::builder::build_lattice` relies
/// on (it calls `valid_span_endings_lowered(.., max_syllables = 8)`,
/// whose internal BFS itself advances exactly one valid syllable per
/// depth level, so an emitted edge is a chain of these single hops).
///
/// Sets the no-dict edge's `syllable_count`. **v3.5.8 RC0**: the OOV
/// edge *cost* is now `OOV_PER_CHAR_PENALTY * toneless_len`
/// (char-keyed, khiin's per-char `BIG`), so `syllable_count` no longer
/// feeds OOV pricing — it is metadata that flows into the synthesized
/// slot-0 candidate's syllable sum. Kept honest (not a hardcoded `1`)
/// anyway so that sum stays correct and the dispatch invariant is not
/// weakened (Codex pre-impl RC0 Q3). Replaces
/// `greedy_longest_syllabification(span).len()`: greedy-longest is not
/// a global segmentation guarantee — it can dead-end (`None`) on a
/// span still lattice-syllabifiable via a *non-greedy* split, and the
/// old `unwrap_or(1)` then under-counted a multi-syllable OOV span. A
/// min-hop BFS over the builder's own single-syllable steps cannot
/// dead-end on a real lattice edge: `build_lattice` emits
/// `(start, end)` only by chaining exactly those hops, so a hop-path
/// `0 → len` provably exists and the BFS returns `Some(>= 1)`. Min-hop
/// (not greedy / not max) is the fewest-syllable valid reading — it is
/// `1` only when the whole span is itself one valid syllable
/// (correct), never collapsing a genuinely multi-syllable span to `1`.
///
/// `None` only if `len` is not single-syllable-reachable at all —
/// impossible for an edge this same `build_lattice` produced over the
/// same `shadow`/`inv` (Codex pre-impl Q1/Q2 OK); the caller treats
/// `None` as a broken edge/provider invariant and fail-closed **drops
/// the edge** rather than mispricing it (the buffer is still spanned
/// via finer edges).
pub(crate) fn span_min_syllable_count(
    shadow_span: &str,
    inv: &SyllableInventory,
    mode: InputMode,
    barriers: &[usize],
) -> Option<usize> {
    let lowered = shadow_span.to_ascii_lowercase();
    let end = lowered.len();
    if end == 0 {
        return None;
    }
    // Unweighted shortest path (in #hops) from offset 0 to `end`. Each
    // hop is one valid syllable from
    // `valid_span_endings_lowered(.., pos, inv, mode, 1)` — the
    // single-hop primitive the lattice builder chains under the same
    // `mode`. v3.5.9 B-2 plumbs `mode` through here so a POJ shadow
    // walks the `poj:` family for hop counts, matching the family used
    // by [`build_lattice`] to produce the edge in the first place; the
    // hop-count invariant (`build_lattice` emits `(start, end)` only by
    // chaining single hops) holds per-family.
    use std::collections::{BTreeMap, VecDeque};
    let mut dist: BTreeMap<usize, usize> = BTreeMap::new();
    dist.insert(0, 0);
    let mut queue: VecDeque<usize> = VecDeque::new();
    queue.push_back(0);
    while let Some(pos) = queue.pop_front() {
        let hops = dist[&pos];
        if pos == end {
            return Some(hops);
        }
        for nxt in valid_span_endings_lowered_with_barriers(&lowered, pos, inv, mode, 1, barriers) {
            if nxt > pos && nxt <= end && !dist.contains_key(&nxt) {
                dist.insert(nxt, hops + 1);
                queue.push_back(nxt);
            }
        }
    }
    None
}

/// v3.5.8 S6 (Codex pre-impl S6 Q2, 2026-05-17, BLOCK condition) —
/// derive the walker lattice-edge match key for a
/// `custom_dictionary.db` entry's romanization, or `None` when the
/// derived form does not land in `[a-z]+` after canonicalization.
///
/// MUST produce a key byte-identical to the one
/// `continuous::fetch_walker_slot0_inner`'s edge provider builds for a
/// syllable span (`{prefix}{toneless}`, the colon-carrying `tl:` / `poj:` /
/// `tps:` prefix of `KeyFamily::for_input_mode(mode)`; v3.5.9 B-2 PR #309 promoted POJ to a
/// first-class FST key family, pre-B-2 every key prefixed `tl:`).
/// `toneless` is the hyphen-stripped, mode-canonicalized,
/// (TPS-only) space-stripped, tone-stripped shadow slice — the shared
/// [`fused_shadow`] pipeline followed by [`strip_tones_for_mode`], the
/// single normalization source the walker edge keys use.
/// Codex pre-impl S6 Q2 **BLOCK**ed a plain `strip_ascii_tone_digits`:
/// it cannot fold a POJ/diacritic custom roman (`tâi-uân`, `tâi-gí`)
/// into `taiuan` / `taigi`; the canonicalize pass is load-bearing.
///
/// **Roman-only** (Codex pre-impl S6 Q2): `hanji` is edge *payload*
/// resolved after the edge is chosen, never an edge key — the lattice
/// is keyed by toneless romanization spans. Returns `None` for an
/// empty result or any residue outside ASCII `a..=z`: punctuation /
/// CJK / digit-only custom roman, or a POJ-shape token that does not
/// canonicalize cleanly, can never equal a syllabifier-built lattice
/// edge key, so it simply stays a span-local candidate and never
/// enters the walker.
///
/// `mode` MUST be the same value `continuous::fetch_walker_slot0_inner`
/// passes to [`build_shadow_lattice_with_barriers`] for this fetch: the canonicalize
/// step is mode-gated — TL/English mode folds POJ→TL (`ch→ts`,
/// `oa→ua`, ...) and emits `tl:`, POJ mode keeps POJ ASCII (no fold)
/// and emits `poj:`. A mismatch would make a custom roman key
/// `poj:chiah` while the lattice edge keys `tl:tsiah` (or the
/// converse), silently breaking the S6 byte-identity match.
///
/// **Stored readings fold the word space.** A stored row's reading may
/// separate its words with a space — a custom roman committed from a
/// multi-word candidate (`tshì-giām sû`), a learned phrase's multi-word
/// dictionary TL (`iā sī`, §50). Between syllables of one stored reading
/// that space is the same boundary a hyphen is, so under TL / POJ /
/// English it folds to `-` before the shadow pipeline and keys like the
/// unspaced typed span (`tl:tshigiamsu`). Without the fold the space
/// survives the TL / POJ separator pass and fails the body gate, so the
/// row never reaches the walker. This is a rule for STORED readings only:
/// a typed TL / POJ space stays a word boundary in the lattice. TPS keeps
/// its own space handling (the separator pass strips it).
pub(crate) fn custom_toneless_key(roman: &str, mode: InputMode) -> Option<String> {
    let reading = match mode {
        // A TL / POJ stored reading is keyed by its Bopomofo face — what a
        // TPS user types for it, and the dictionary's own `tps:` face
        // (`phonetics::tps_notone_from_tl`). A Bopomofo row keys as typed.
        InputMode::Tps if phonetics::api::stored_roman_mode(roman, mode) == InputMode::Tl => {
            phonetics::tps_notone_from_tl(&phonetics::api::custom_search_reading(roman))
        }
        InputMode::Tps => roman.to_owned(),
        InputMode::Tl | InputMode::Poj | InputMode::English => roman.replace(' ', "-"),
    };
    let toneless = strip_tones_for_mode(&fused_shadow(&reading, mode), mode);
    if toneless.is_empty() {
        return None;
    }
    // v3.5.9 D / C-3b — gate body shape by mode:
    // - TL/POJ/English: body must be all ASCII lowercase a..=z
    //   (no Bopomofo / digit / punctuation residue can collide with a
    //   syllabifier-built TL/POJ lattice edge key).
    // - TPS: body must be all Bopomofo (TPS char) AND carry no leftover
    //   TPS tone mark (strip should have caught them; the guard is
    //   defensive — non-Bopomofo residue cannot equal a `tps:<tps_notone>`
    //   edge key built from the same shadow pipeline).
    let body_ok = match mode {
        InputMode::Tps => toneless
            .chars()
            .all(|c| phonetics::is_tps_char(c) && !phonetics::is_tps_tone_mark(c)),
        InputMode::Tl | InputMode::Poj | InputMode::English => {
            toneless.bytes().all(|b| b.is_ascii_lowercase())
        }
    };
    if !body_ok {
        return None;
    }
    // v3.5.9 B-2 — mode-aware family prefix. The contract still requires
    // byte-identity with the edge provider's emitted key (the S6
    // invariant), so this MUST consume the same `mode` and the same
    // shadow pipeline — both are now mode-aware in lockstep.
    let prefix = KeyFamily::for_input_mode(mode).prefix();
    Some(format!("{prefix}{toneless}"))
}

/// v3.5.8 Phase 9 Item 10 / v3.5.9 B-2 + D Fork 7b — partial-prefix
/// mode-aware key builder. Runs the same [`fused_shadow`] →
/// [`fst_body_for_span`] chain (mode-aware tone strip since v3.5.9 D / C-3b:
/// TL/POJ/English drop ASCII tone digits, TPS drops the 8 Bopomofo tone
/// scalars per `phonetics::is_tps_tone_mark`) as the production
/// [`left_anchored_keys_and_restrictions`] / walker edge providers but
/// **skips the syllabifier** (the partial-prefix path is reached
/// precisely because the syllabifier returned no valid ending — TL `g`,
/// TPS `ㄉ`, etc.). Emits the `tl:` / `poj:` / `tps:` family prefix
/// matching `mode` via [`KeyFamily::for_input_mode`] so the byte-range scan in
/// `continuous::partial_prefix::fetch_via_lexicon_partial_inner` hits the right
/// FST family. Returns `None` when the resulting toneless key is empty
/// (raw was hyphen-only / digit-only for TL/POJ, bare tone mark for TPS)
/// so the caller can short-circuit without firing an unbounded prefix
/// scan.
///
/// `consumed_span` is fixed to `(0, raw.len())` — partial-prefix
/// candidates always final-commit per Q15.4 (the offset maps from
/// Items 8 + 9 are intentionally discarded here because there is no
/// per-syllable mid-commit semantics to preserve).
pub(crate) fn build_partial_prefix_key(
    raw: &str,
    mode: InputMode,
) -> Option<(ConsumedSpan, String)> {
    // Explicit-tone fix — tone-aware body, same [`span_key`] rule as
    // `left_anchored_keys_and_restrictions` / the walker edge: a fully-toned
    // whole buffer (`tai5`) yields the verbatim `tl:tai5` prefix so the
    // Step 4b `lookup_prefix` extension scan only surfaces tone-5-initial
    // keys, never the all-tone `tl:tai` range. Toneless / mixed buffers
    // keep the toneless prefix (the partial-prefix path's normal "typing
    // toward the first boundary" behavior; a mixed buffer's typed digits
    // travel as the whole-buffer [`TonePin`] instead). See
    // [`fst_body_for_span`]. v3.5.9 D / C-3b + D Fork 7b — TPS reaches
    // this builder via the unified `assemble_candidates` empty-keys
    // fallthrough and always takes the toneless branch (`is_tps_tone_mark`
    // strip), so a raw `ㄉㄧˊ` shadow still yields the `tps:ㄉㄧ` toneless
    // key. The §35 / §41 metadata of the key is discarded here — the
    // whole buffer is one span; [`whole_buffer_tone_pin`] carries the pin,
    // so no inventory is needed for it here.
    let key = whole_buffer_span_key(raw, mode, None)?.key;
    Some(((0u32, raw.len() as u32), key))
}

/// Whole-buffer abbreviation query key (`tl-abbrev:ss`, `tps-abbrev:ㄙㄒ`)
/// for [`lexicon::fetch_abbrev_candidates`], or `None` when the buffer
/// cannot be an abbreviation — `behavioral-invariants.md` §46.
///
/// An abbreviation is one leading spelling unit per syllable (one glyph in
/// TPS, one to three letters in TL / POJ), so the buffer must be ≥ 2
/// glyphs of letter material only: TL / POJ ASCII letters (vowels included —
/// a zero-initial syllable abbreviates to its first vowel, 紅嬰仔 `aea`),
/// TPS initial / vowel glyphs (`is_tps_char`, no tone mark). A digit, tone
/// mark, hyphen or space disqualifies the buffer, so no [`TonePin`] or
/// barrier ever applies to this path. English has no abbreviation family.
/// Whether the buffer IS an abbreviation is decided by the index, not by
/// shape: the `*-abbrev:` family holds nothing else, and a buffer that is
/// also a reading (`ai` 愛 / 阿姨) simply gets both — the syllabic block
/// first, the abbreviation block after it.
///
/// Runs every keystroke; the per-char screen rejects before allocating.
pub(crate) fn abbrev_query_key(raw: &str, mode: InputMode) -> Option<String> {
    if raw.chars().count() < 2 {
        return None;
    }
    let shaped = match mode {
        InputMode::Tl | InputMode::Poj => raw.chars().all(|c| c.is_ascii_alphabetic()),
        InputMode::Tps => raw
            .chars()
            .all(|c| phonetics::is_tps_char(c) && !phonetics::is_tps_tone_mark(c)),
        InputMode::English => false,
    };
    if !shaped {
        return None;
    }
    let key = format!(
        "{}{}",
        KeyFamily::for_input_mode(mode).prefix(),
        lookup_form(raw)
    );
    phonetics::abbrev_family_key(&key)
}

#[cfg(test)]
mod tests {
    use super::super::strip_ascii_tone_digits;
    use super::*;

    // `span_key` barrier conventions (E4.2). Shadow `ㄎㄛㆻㄫㄉㄞ` is six
    // 3-byte Bopomofo scalars (char starts 0/3/6/9/12/15, len 18); the
    // `tps` family prefix shifts key offsets by `"tps".len() + 1 = 4`.
    //
    // trace (non-zero start): span 6..18 = `ㆻㄫㄉㄞ`, global barrier 12 →
    // local 6 → body[..6] = `ㆻㄫ`, last glyph starts at 3 → key offset
    // 4 + 3 = 7; the interior space closes `ㆻㄫ` with any tone (a
    // Boundary at toneless 6); end 18 is not a barrier → no SpaceClosed.
    #[test]
    fn span_key_shifts_barriers_into_span_coordinates() {
        let k = span_key("ㄎㄛㆻㄫㄉㄞ", 6, 18, InputMode::Tps, &[12], &[], &[]).expect("body");
        assert_eq!(k.key, "tps:ㆻㄫㄉㄞ");
        assert_eq!(k.final_only, vec![7]);
        assert_eq!(
            k.tone_pin,
            tps_pin("ㆻㄫㄉㄞ", &[(6, TpsCloseKind::Boundary)])
        );
    }

    fn tps_pin(body: &str, closes: &[(usize, TpsCloseKind)]) -> TonePin {
        TonePin::TpsTyped {
            body: body.to_owned(),
            closes: closes
                .iter()
                .map(|&(body_end, kind)| TpsClose { body_end, kind })
                .collect(),
        }
    }

    // trace (barrier at start): span 12..18 = `ㄉㄞ`; barrier 12 is the
    // boundary the span opens on → excluded from final-only; barrier 18
    // → local 6 → body[..6] = `ㄉㄞ`, last glyph at 3 → offset 7, and
    // the span closes on it with no tone mark → pinned.
    #[test]
    fn span_key_excludes_barrier_at_start_and_pins_barrier_at_end() {
        let k =
            span_key("ㄎㄛㆻㄫㄉㄞ", 12, 18, InputMode::Tps, &[12, 18], &[], &[]).expect("body");
        assert_eq!(k.key, "tps:ㄉㄞ");
        assert_eq!(k.final_only, vec![7]);
        assert_eq!(
            k.tone_pin,
            tps_pin("ㄉㄞ", &[(6, TpsCloseKind::SpaceClosed)])
        );

        let opens_on_barrier =
            span_key("ㄎㄛㆻㄫㄉㄞ", 12, 18, InputMode::Tps, &[12], &[], &[]).expect("body");
        assert!(opens_on_barrier.final_only.is_empty());
        assert_eq!(opens_on_barrier.tone_pin, TonePin::None);
    }

    // trace (cross-barrier): span 0..18 runs THROUGH barrier 9 → the
    // glyph before it (`ㆻ`, starts at 6) is Final-only at key offset
    // 4 + 6 = 10, and a reading must end a syllable there (Boundary, any
    // tone), but the span does not END on the barrier → no SpaceClosed.
    // The span that stops exactly at 9 carries the same offset and IS
    // pinned (global `end` vs global barrier).
    #[test]
    fn span_key_cross_barrier_restricts_but_does_not_pin() {
        let through =
            span_key("ㄎㄛㆻㄫㄉㄞ", 0, 18, InputMode::Tps, &[9], &[], &[]).expect("body");
        assert_eq!(through.key, "tps:ㄎㄛㆻㄫㄉㄞ");
        assert_eq!(through.final_only, vec![10]);
        assert_eq!(
            through.tone_pin,
            tps_pin("ㄎㄛㆻㄫㄉㄞ", &[(9, TpsCloseKind::Boundary)])
        );

        let stops = span_key("ㄎㄛㆻㄫㄉㄞ", 0, 9, InputMode::Tps, &[9], &[], &[]).expect("body");
        assert_eq!(stops.key, "tps:ㄎㄛㆻ");
        assert_eq!(stops.final_only, vec![10]);
        assert_eq!(
            stops.tone_pin,
            tps_pin("ㄎㄛㆻ", &[(9, TpsCloseKind::SpaceClosed)])
        );
    }

    #[test]
    fn whole_buffer_pin_strips_the_trailing_space() {
        let si = tps_pin("ㄒㄧ", &[(6, TpsCloseKind::SpaceClosed)]);
        assert_eq!(whole_buffer_tone_pin("ㄒㄧ ", InputMode::Tps, None), si);
        // Repeated trailing spaces are one pin, not several.
        assert_eq!(whole_buffer_tone_pin("ㄒㄧ  ", InputMode::Tps, None), si);
        // Interior spaces are any-tone boundaries; the body is the fused surface.
        assert_eq!(
            whole_buffer_tone_pin("ㄍㄠ ㄉㄞ ", InputMode::Tps, None),
            tps_pin(
                "ㄍㄠㄉㄞ",
                &[(6, TpsCloseKind::Boundary), (12, TpsCloseKind::SpaceClosed)]
            )
        );
    }

    #[test]
    fn whole_buffer_pin_is_none_without_a_mark_or_space() {
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ", InputMode::Tps, None),
            TonePin::None
        );
        // An interior space alone is a boundary of any tone, not a tone-1 pin.
        assert_eq!(
            whole_buffer_tone_pin("ㄍㄠ ㄉㄞ", InputMode::Tps, None),
            tps_pin("ㄍㄠㄉㄞ", &[(6, TpsCloseKind::Boundary)])
        );
    }

    #[test]
    fn whole_buffer_pin_keeps_a_marked_tail_tone_without_a_space_close() {
        // The mark closed the syllable; the space after it adds nothing.
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧˋ ", InputMode::Tps, None),
            tps_pin("ㄒㄧ", &[(6, TpsCloseKind::Marked('2'))])
        );
    }

    // trace: `ㄉㄞ˫ㄨㄢ` → ˫ (7) after `ㄉㄞ` = toneless 6; `ㄌㄧˋㄫ` → ˋ (2)
    // at 6; `ㄏㄛˋㄇㄚ`␣ → ˋ at 6 + the space closing `ㄇㄚ` at 12;
    // `ㄐㄧㆵ˙ㆤˊ` → ˙ (8) at 9, ˊ (5) at 12.
    #[test]
    fn whole_buffer_pin_closes_every_typed_tone_mark() {
        use TpsCloseKind::{Marked, SpaceClosed};
        for (raw, body, closes) in [
            ("ㄉㄞ˫ㄨㄢ", "ㄉㄞㄨㄢ", vec![(6, Marked('7'))]),
            ("ㄌㄧˋㄫ", "ㄌㄧㄫ", vec![(6, Marked('2'))]),
            (
                "ㄏㄛˋㄇㄚ ",
                "ㄏㄛㄇㄚ",
                vec![(6, Marked('2')), (12, SpaceClosed)],
            ),
            (
                "ㄐㄧㆵ˙ㆤˊ",
                "ㄐㄧㆵㆤ",
                vec![(9, Marked('8')), (12, Marked('5'))],
            ),
        ] {
            assert_eq!(
                whole_buffer_tone_pin(raw, InputMode::Tps, None),
                tps_pin(body, &closes),
                "{raw}"
            );
        }
    }

    // A mark with no glyph since the last close closes nothing: a leading
    // `˫` (or a doubled one) must not demand a syllable end at offset 0,
    // which no reading has (Codex post-impl 2026-10-11).
    #[test]
    fn whole_buffer_pin_ignores_orphan_marks() {
        use TpsCloseKind::Marked;
        for (raw, body, closes) in [
            ("˫ㄉㄞㄨㄢ", "ㄉㄞㄨㄢ", vec![]),
            ("˫˫ㄉㄞ", "ㄉㄞ", vec![]),
            ("ㄉㄞ˫ˊㄨㄢ", "ㄉㄞㄨㄢ", vec![(6, Marked('7'))]),
        ] {
            let expected = if closes.is_empty() {
                TonePin::None
            } else {
                tps_pin(body, &closes)
            };
            assert_eq!(
                whole_buffer_tone_pin(raw, InputMode::Tps, None),
                expected,
                "{raw}"
            );
        }
    }

    #[test]
    fn whole_buffer_pin_is_none_for_toneless_or_english_buffers() {
        for mode in [InputMode::Tl, InputMode::Poj, InputMode::English] {
            assert_eq!(whole_buffer_tone_pin("tai ", mode, None), TonePin::None);
            assert_eq!(whole_buffer_tone_pin("taigi", mode, None), TonePin::None);
        }
        // English digits are not tones.
        assert_eq!(
            whole_buffer_tone_pin("tai5", InputMode::English, None),
            TonePin::None
        );
    }

    #[test]
    fn whole_buffer_pin_ignores_a_typed_hyphen_under_tps() {
        // A TPS buffer closed by `-` is not closed by the keyboard space:
        // the §41 pin stays off (Codex post-impl 2026-09-22 P2).
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ-", InputMode::Tps, None),
            TonePin::None
        );
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ ", InputMode::Tps, None),
            tps_pin("ㄒㄧ", &[(6, TpsCloseKind::SpaceClosed)])
        );
    }

    #[test]
    fn whole_buffer_pin_is_none_for_a_space_only_buffer() {
        assert_eq!(
            whole_buffer_tone_pin(" ", InputMode::Tps, None),
            TonePin::None
        );
        assert_eq!(
            whole_buffer_tone_pin("", InputMode::Tps, None),
            TonePin::None
        );
    }

    // §17 case 3 — a TL/POJ buffer with any typed digit pins the typed
    // tones: hyphens are stripped, case folded, the digits kept verbatim,
    // and the family prefix follows the mode. Partial (`teng5-sek`) and
    // fully-toned (`teng5sek4`) buffers pin alike — the pin is what keeps
    // toneless-matched custom entries honest for the fully-toned case.
    #[test]
    fn whole_buffer_pin_carries_typed_tones_for_tl_poj() {
        // §52: the typed `-` after `teng5` is a boundary too (offset 5,
        // past the digit) — exactly what Codex predicted would flip.
        assert_eq!(
            whole_buffer_tone_pin("teng5-sek", InputMode::Poj, None),
            TonePin::TypedTones {
                mode: InputMode::Poj,
                typed: "teng5sek".to_owned(),
                boundaries: vec![TypedBoundary {
                    at: 5,
                    khinsiann: false,
                }],
                one_syllable_stretches: Vec::new(),
            }
        );
        assert_eq!(
            whole_buffer_tone_pin("Teng5sek4", InputMode::Tl, None),
            typed_tones(InputMode::Tl, "teng5sek4")
        );
        // Digit-only buffer has no body → no pin.
        assert_eq!(
            whole_buffer_tone_pin("5", InputMode::Tl, None),
            TonePin::None
        );
    }

    fn typed_tones(mode: InputMode, typed: &str) -> TonePin {
        TonePin::TypedTones {
            mode,
            typed: typed.to_owned(),
            boundaries: Vec::new(),
            one_syllable_stretches: Vec::new(),
        }
    }

    #[test]
    fn span_key_pins_typed_tones_only_for_a_digit_bearing_tl_poj_span() {
        // Partial: toneless lookup body + typed-tone pin.
        let k = span_key("teng5sek", 0, 8, InputMode::Poj, &[], &[], &[]).expect("body");
        assert_eq!(k.key, "poj:tengsek");
        assert_eq!(k.tone_pin, typed_tones(InputMode::Poj, "teng5sek"));
        // Fully toned: verbatim toned key AND the pin.
        let k = span_key("teng5sek4", 0, 9, InputMode::Tl, &[], &[], &[]).expect("body");
        assert_eq!(k.key, "tl:teng5sek4");
        assert_eq!(k.tone_pin, typed_tones(InputMode::Tl, "teng5sek4"));
        // Toneless: unpinned (the all-tones affordance).
        let k = span_key("tengsek", 0, 7, InputMode::Poj, &[], &[], &[]).expect("body");
        assert_eq!(k.tone_pin, TonePin::None);
        // English: digits are not tones.
        let k = span_key("teng5sek", 0, 8, InputMode::English, &[], &[], &[]).expect("body");
        assert_eq!(k.key, "tl:tengsek");
        assert_eq!(k.tone_pin, TonePin::None);
        // Interior sub-span of a longer shadow pins its own slice only.
        let k = span_key("teng5sek", 5, 8, InputMode::Poj, &[], &[], &[]).expect("body");
        assert_eq!(k.key, "poj:sek");
        assert_eq!(k.tone_pin, TonePin::None);
    }

    #[test]
    fn key_final_only_offsets_marks_the_glyph_before_a_barrier() {
        // Span ㄎㄛㆻㄫ with a barrier after ㆻ (byte 9): the ㆻ slot (key
        // offset 4 + 6) is Final-only; nothing else is restricted.
        let body = "ㄎㄛㆻㄫ"; // toneless span: body == span
        assert_eq!(
            key_final_only_offsets("ㄎㄛㆻㄫ", body, InputMode::Tps, &[9], 4),
            vec![4 + 6],
        );
        // No barriers → no restriction; barrier at 0 → nothing before it.
        assert!(key_final_only_offsets("ㄎㄛㆻㄫ", body, InputMode::Tps, &[], 4).is_empty());
        assert!(key_final_only_offsets("ㄎㄛㆻㄫ", body, InputMode::Tps, &[0], 4).is_empty());
    }

    #[test]
    fn key_final_only_offsets_accounts_for_stripped_tone_marks() {
        // Mixed-tone span ㄎㄛˋㆻㄫ with a barrier after ㆻ (shadow byte 11):
        // the toneless body drops the 2-byte ˋ, so the ㆻ slot lands at
        // body offset 6 (prefix 4 → key offset 10), not at the shadow-based
        // 8. The barrier also counts when TRAILING (span ends at it).
        let span = "ㄎㄛˋㆻㄫ"; // 3+3+2+3+3 bytes
        let body = "ㄎㄛㆻㄫ"; // tone-stripped
        assert_eq!(
            key_final_only_offsets(span, body, InputMode::Tps, &[11], 4),
            vec![4 + 6],
        );
        // Trailing barrier: span exactly ends at the barrier.
        assert_eq!(
            key_final_only_offsets("ㄎㄛㆻ", "ㄎㄛㆻ", InputMode::Tps, &[9], 4),
            vec![4 + 6],
        );
    }

    #[test]
    fn custom_toneless_key_numeric_tl_strips_tone_digits() {
        assert_eq!(
            custom_toneless_key("tai5gi2", InputMode::Tl).as_deref(),
            Some("tl:taigi")
        );
    }

    #[test]
    fn custom_toneless_key_numeric_with_hyphen_strips_both() {
        // Same `tl:taigi` key the edge provider builds for the `taigi`
        // span — proves the byte-identical-match contract.
        assert_eq!(
            custom_toneless_key("tai5-gi2", InputMode::Tl).as_deref(),
            Some("tl:taigi")
        );
    }

    #[test]
    fn custom_toneless_key_tl_strips_tone_keeps_literal_spelling() {
        // The canonicalize pass still strips tone marks + folds glyph
        // encoding, so a diacritic custom roman keys to the same toneless
        // form as the numeric one when there is NO spelling difference:
        // `tâi-gí` → `tl:taigi` == numeric `tai5gi2`.
        assert_eq!(
            custom_toneless_key("tâi-gí", InputMode::Tl).as_deref(),
            Some("tl:taigi"),
            "tone-mark + hyphen strip (no spelling change) → tl:taigi"
        );
        // TL-literal (2026-06-05): the POJ→TL SPELLING fold is NOT applied,
        // so POJ-spelled `oân` (oa) keys to `tl:taioan`, distinct from the
        // TL-spelled `uan5` → `tl:taiuan`. Both the lattice edge key and this
        // custom key run the SAME `canonicalize_poj_shadow(mode)`, so they
        // stay byte-identical and the continuous custom match still holds.
        assert_eq!(
            custom_toneless_key("tâi-oân", InputMode::Tl).as_deref(),
            Some("tl:taioan"),
        );
        assert_eq!(
            custom_toneless_key("tai5uan5", InputMode::Tl).as_deref(),
            Some("tl:taiuan"),
        );
    }

    #[test]
    fn custom_toneless_key_folds_a_stored_word_space_like_a_hyphen() {
        // trace: "tshì-giām sû" → space→'-' → "tshì-giām-sû" → hyphen +
        // tone strip → "tshigiamsu"; the same key the unspaced typed span
        // `tshigiamsu` / `tshi3giam7su5` builds.
        for roman in [
            "tshì-giām sû",
            "tshì-giām-sû",
            "tshi3giam7su5",
            "tshì  giām sû ",
        ] {
            assert_eq!(
                custom_toneless_key(roman, InputMode::Tl).as_deref(),
                Some("tl:tshigiamsu"),
                "{roman}"
            );
        }
        assert_eq!(
            custom_toneless_key("chhì-giām sû", InputMode::Poj).as_deref(),
            Some("poj:chhigiamsu")
        );
        // TPS: the separator pass already strips the space; a Bopomofo
        // reading keys the same with or without it.
        assert_eq!(
            custom_toneless_key("ㄍㄠ ㄉㄞ", InputMode::Tps),
            custom_toneless_key("ㄍㄠㄉㄞ", InputMode::Tps)
        );
    }

    #[test]
    fn custom_toneless_key_tps_keys_a_latin_reading_by_its_bopomofo_face() {
        // trace: 陳三 `Tân Sann` → custom_search_reading (TL, unchanged) →
        // tps_notone_from_tl → `ㄉㄢㄙㆩ`; the typed `ㄉㄢˊㄙㆩ` strips its ˊ
        // to the same body. POJ `Tân Saⁿ` folds to the same TL first.
        let typed = custom_toneless_key("ㄉㄢˊㄙㆩ", InputMode::Tps);
        assert_eq!(typed.as_deref(), Some("tps:ㄉㄢㄙㆩ"));
        assert_eq!(custom_toneless_key("Tân Sann", InputMode::Tps), typed);
        assert_eq!(custom_toneless_key("tân-sann", InputMode::Tps), typed);
        assert_eq!(custom_toneless_key("Tân Saⁿ", InputMode::Tps), typed);
    }

    #[test]
    fn custom_toneless_key_rejects_empty_and_non_tl_residue() {
        // Empty / whitespace / punctuation / CJK / digit-only custom
        // roman can never equal a syllabifier-built lattice edge key,
        // so it returns None and stays a span-local-only candidate.
        assert_eq!(custom_toneless_key("", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("   ", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("!!!", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("123", InputMode::Tl), None);
        assert_eq!(custom_toneless_key("台語", InputMode::Tl), None);
    }

    #[test]
    fn custom_toneless_key_poj_ascii_matches_walker_edge_key() {
        // S6 byte-identity: a custom-dict roman `chiah` keyed under the
        // SAME `mode` the walker edge provider uses must equal the
        // lattice edge key for that mode. v3.5.9 B-2 — POJ now emits
        // `poj:` family keys preserving POJ ASCII (pre-B-2 this folded
        // to TL `tl:tsiah`; B-2 keeps POJ first-class via the `poj:`
        // family of the tagged-single-FST).
        assert_eq!(
            custom_toneless_key("chiah", InputMode::Poj).as_deref(),
            Some("poj:chiah"),
        );
        // TL mode keeps the F3C identity (un-canonicalized).
        assert_eq!(
            custom_toneless_key("chiah", InputMode::Tl).as_deref(),
            Some("tl:chiah"),
        );
    }

    // ----- v3.5.8 Phase 9 Item 10 — partial-prefix key derivation -----

    #[test]
    fn build_partial_prefix_key_passes_ascii_through() {
        let (span, key) = build_partial_prefix_key("gu", InputMode::Tl).unwrap();
        // partial-prefix candidates always final-commit (Q15.4) →
        // consumed_span covers the whole pending tail.
        assert_eq!(span, (0u32, 2u32));
        assert_eq!(key, "tl:gu");
    }

    #[test]
    fn build_partial_prefix_key_tone_aware_lowercases() {
        // Explicit-tone fix — a fully-toned partial buffer KEEPS its tone
        // digit so the prefix scan filters to the typed tone: `GU5` →
        // `tl:gu5` (was `tl:gu` pre-fix). Toneless input still strips to
        // the all-tone fused prefix: `GU` → `tl:gu`.
        // trace: "GU5" → lower "gu5" → shadow "gu5" → fully-toned (`gu`+`5`)
        //   → verbatim body "gu5" → "tl:gu5".
        let (_, key) = build_partial_prefix_key("GU5", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:gu5");
        let (_, key) = build_partial_prefix_key("GU", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:gu");
    }

    #[test]
    fn build_partial_prefix_key_strips_internal_hyphen_via_item8_shadow() {
        // Item 8's hyphen-shadow chain runs on the partial-prefix path
        // too — `tai-` collapses to `tai` (trailing `-` stays in the
        // pending raw buffer per build_hyphen_shadow contract), and
        // `-tai` collapses to `tai`.
        let (_, key) = build_partial_prefix_key("tai-", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:tai");
        let (_, key) = build_partial_prefix_key("-tai", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:tai");
    }

    #[test]
    fn build_partial_prefix_key_canonicalizes_poj_diacritic_via_item9() {
        // Item 9's canonicalize chain runs on partial-prefix input too
        // — `pe\u{030d}` (POJ `pe̍h` minus the trailing `h`) folds to
        // `pe` after the tone-mark drop, giving FST key `tl:pe`.
        let (_, key) = build_partial_prefix_key("pe\u{030d}", InputMode::Tl).unwrap();
        assert_eq!(key, "tl:pe");
    }

    #[test]
    fn build_partial_prefix_key_returns_none_for_empty_after_strip() {
        // Hyphen-only or digit-only raw produces an empty toneless
        // key — return None so the caller skips the FST scan.
        assert!(build_partial_prefix_key("", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("-", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("--", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("5", InputMode::Tl).is_none());
        assert!(build_partial_prefix_key("-5-", InputMode::Tl).is_none());
    }

    #[test]
    fn build_partial_prefix_key_tone_policy_pins_unvalidated_partial() {
        // Explicit-tone fix — the partial-prefix builder skips the
        // syllabifier (it is reached precisely when no valid ending
        // exists), so it can receive a NON-validated whole buffer. Pin
        // the tone-policy on each shape so the safety reasoning in
        // `fst_body_for_span` stays honest:
        //   - fully-toned → verbatim toned prefix (filters by tone).
        let (_, k) = build_partial_prefix_key("tai5", InputMode::Tl).unwrap();
        assert_eq!(k, "tl:tai5");
        //   - mixed (trailing letter) → toneless prefix (no regression).
        let (_, k) = build_partial_prefix_key("tai5g", InputMode::Tl).unwrap();
        assert_eq!(k, "tl:taig");
        //   - fully-toned-LOOKING but not a syllable → toneless prefix;
        //     the typed digit travels as the whole-buffer `TypedTones`
        //     pin, fail-closed on a spelling mismatch, so junk input
        //     still yields zero candidates (NEVER a wrong-tone hit).
        let (_, k) = build_partial_prefix_key("abc1", InputMode::Tl).unwrap();
        assert_eq!(k, "tl:abc");
    }

    #[test]
    fn span_is_fully_toned_ascii_requires_one_syllable_per_group() {
        // Every digit-closed group is one valid syllable → verbatim key.
        assert!(span_is_fully_toned_ascii("tai5"));
        assert!(span_is_fully_toned_ascii("kok4bin5tong2"));
        assert!(
            span_is_fully_toned_ascii("chit4"),
            "POJ spelling folds to TL"
        );
        // A fused group carrying an untoned syllable is PARTIAL tone.
        assert!(!span_is_fully_toned_ascii("kokbin5tong2"));
        assert!(!span_is_fully_toned_ascii("kok4bintong2"));
        assert!(!span_is_fully_toned_ascii("tengsek4"));
        // Open tail, orphan digit, non-syllable group, separator leak.
        assert!(!span_is_fully_toned_ascii("kokbin5tong"));
        assert!(!span_is_fully_toned_ascii("5tai"));
        assert!(!span_is_fully_toned_ascii("tai55"));
        assert!(!span_is_fully_toned_ascii("abc1"));
        assert!(!span_is_fully_toned_ascii("tai5-gi2"));
        assert!(!span_is_fully_toned_ascii(""));
    }

    #[test]
    fn build_partial_prefix_key_poj_emits_poj_family() {
        // v3.5.9 B-2 — POJ mode emits `poj:` family keys preserving POJ
        // ASCII spelling (pre-B-2 this test asserted `tl:tsi` after a
        // POJ→TL fold; B-2 makes POJ first-class so `chi` stays `chi`
        // and routes to the `poj:` family of the FST).
        let (_, key) = build_partial_prefix_key("chi", InputMode::Poj).unwrap();
        assert_eq!(key, "poj:chi");
        let (_, tl_key) = build_partial_prefix_key("chi", InputMode::Tl).unwrap();
        assert_eq!(tl_key, "tl:chi", "TL mode keeps F3C identity");
    }

    #[test]
    fn build_partial_prefix_key_tps_emits_tps_family_for_leading_initial() {
        // v3.5.9 D Fork 7b — TPS partial-prefix activated. A leading
        // lone Bopomofo initial `ㄉ` (3 bytes UTF-8) emits `tps:ㄉ` so
        // `prefix_index.n("tps:ㄉ")` byte-range scans every dictionary
        // row whose `tps_notone` starts with `ㄉ` (佇/著/丁/同/單/...).
        let (span, key) = build_partial_prefix_key("\u{3109}", InputMode::Tps).unwrap();
        assert_eq!(span, (0u32, 3u32));
        assert_eq!(key, "tps:\u{3109}");
    }

    #[test]
    fn build_partial_prefix_key_tps_keeps_explicit_tone_mark() {
        // B2 (§17 TPS) — a fully-toned TPS span keeps its tone mark verbatim
        // so the `tps:<tps_num>` family filters to the typed tone, mirroring
        // the TL/POJ digit-keeping path. `ㄉㄞˊ` (U+02CA tone-5) → `tps:ㄉㄞˊ`,
        // NOT the toneless `tps:ㄉㄞ` (pre-B2 unconditional strip, the bug).
        let (_, key) =
            build_partial_prefix_key("\u{3109}\u{311e}\u{02ca}", InputMode::Tps).unwrap();
        assert_eq!(key, "tps:\u{3109}\u{311e}\u{02ca}");
    }

    #[test]
    fn build_partial_prefix_key_tps_normalizes_tone8_dot_to_combining() {
        // B2 — tone-8 byte-identity: the keyboard types the standalone dot
        // `U+02D9` (˙) but the stored `tps:<tps_num>` key uses combining
        // `U+0307`. The verbatim toned key must normalize so it matches.
        // `ㄍㄚㆵ˙` (U+02D9) → `tps:ㄍㄚㆵ̇` (U+0307).
        let (_, key) =
            build_partial_prefix_key("\u{310d}\u{311a}\u{31b5}\u{02d9}", InputMode::Tps).unwrap();
        assert_eq!(key, "tps:\u{310d}\u{311a}\u{31b5}\u{0307}");
    }

    #[test]
    fn build_partial_prefix_key_tps_tone4_coda_stays_toneless() {
        // B2 — tone-4 is a bare stop-coda glyph (ㆵ U+31B5), part of the
        // syllable body with no tone mark; it has no distinguishing
        // `tps:<tps_num>` key, so the span stays on the toneless all-tones
        // key. `ㄍㄚㆵ` (no dot) → `tps:ㄍㄚㆵ` (NOT a toned key).
        let (_, key) =
            build_partial_prefix_key("\u{310d}\u{311a}\u{31b5}", InputMode::Tps).unwrap();
        assert_eq!(key, "tps:\u{310d}\u{311a}\u{31b5}");
    }

    #[test]
    fn build_partial_prefix_key_tps_bare_tone_mark_returns_none() {
        // A bare tone-mark only buffer (no Bopomofo body) strips to
        // empty → None, preventing an unbounded `tps:` namespace scan.
        // Mirrors the TL `digit-only` / `hyphen-only` guard above.
        assert!(build_partial_prefix_key("\u{02ca}", InputMode::Tps).is_none());
        assert!(build_partial_prefix_key("\u{02cb}", InputMode::Tps).is_none());
        assert!(build_partial_prefix_key("\u{0307}", InputMode::Tps).is_none());
    }

    // ----- B2 (§17 TPS) span_is_fully_toned_tps + fst_body_for_span -----

    #[test]
    fn span_is_fully_toned_tps_single_marked_tone_is_toned() {
        // Each mark-bearing tone (2/3/5/6/7/8/9) closes the syllable.
        for tone_mark in [
            '\u{02cb}', // 2 ˋ
            '\u{02ea}', // 3 ˪
            '\u{02ca}', // 5 ˊ
            '\u{02c7}', // 6 ˇ
            '\u{02eb}', // 7 ˫
            '\u{0307}', // 8 combining dot
            '\u{02d9}', // 8 standalone dot (keyboard form)
            '\u{02c6}', // 9 ˆ
        ] {
            let span = format!("\u{3109}\u{311e}{tone_mark}"); // ㄉㄞ + tone
            assert!(
                span_is_fully_toned_tps(&span),
                "tone mark U+{:04X} should mark the span fully toned",
                tone_mark as u32
            );
        }
    }

    #[test]
    fn span_is_fully_toned_tps_multi_syllable_each_toned() {
        // ㄍㄠˋㄉㄞˊ — two syllables, both marked → fully toned.
        assert!(span_is_fully_toned_tps(
            "\u{310d}\u{3120}\u{02cb}\u{3109}\u{311e}\u{02ca}"
        ));
    }

    #[test]
    fn span_is_fully_toned_tps_excludes_tone1_and_tone4_and_mixed() {
        // tone-1 / tone-4 (no trailing mark) and mixed partial-tone spans are
        // NOT fully toned → toneless all-tones key.
        assert!(!span_is_fully_toned_tps("\u{3109}\u{311e}")); // ㄉㄞ tone-1 (no mark)
        assert!(!span_is_fully_toned_tps("\u{310d}\u{311a}\u{31b5}")); // ㄍㄚㆵ tone-4 coda
                                                                       // ㄍㄠˋㄉㄞ — first syllable toned, second toneless → mixed.
        assert!(!span_is_fully_toned_tps(
            "\u{310d}\u{3120}\u{02cb}\u{3109}\u{311e}"
        ));
        assert!(!span_is_fully_toned_tps("")); // empty
        assert!(!span_is_fully_toned_tps("\u{02ca}")); // orphan tone mark
    }

    #[test]
    fn fst_body_for_span_tps_toned_keeps_mark_toneless_strips() {
        // Toned span kept verbatim (tone-8 normalized); toneless stripped.
        assert_eq!(
            fst_body_for_span("\u{3109}\u{311e}\u{02ca}", InputMode::Tps),
            "\u{3109}\u{311e}\u{02ca}"
        );
        assert_eq!(
            fst_body_for_span("\u{310d}\u{311a}\u{31b5}\u{02d9}", InputMode::Tps),
            "\u{310d}\u{311a}\u{31b5}\u{0307}"
        );
        assert_eq!(
            fst_body_for_span("\u{3109}\u{311e}", InputMode::Tps),
            "\u{3109}\u{311e}"
        );
    }

    #[test]
    fn span_is_fully_toned_tps_leading_untoned_syllable_reads_as_toned() {
        // Documented text-only limitation: a LEADING tone-1 syllable
        // followed by a marked syllable is not split, so `ㄍㄠㄉㄞˊ` (kau1+tai5, the
        // §18 `ㄍㄠ ␣ ㄉㄞˊ` space-phrase after the separator strip) reads as
        // fully toned and keys the verbatim `tps:ㄍㄠㄉㄞˊ`. This is EXACT, not a
        // wrong-tone hit — kau's tone-1 contributes no mark to `tps_num` either,
        // so the verbatim span equals the real kau1-tai5 phrase key. The
        // per-syllable all-tones affordance for kau stays available via the
        // shorter single-syllable `tps:ㄍㄠ` span (§18).
        assert!(span_is_fully_toned_tps(
            "\u{310d}\u{3120}\u{3109}\u{311e}\u{02ca}"
        ));
        assert_eq!(
            fst_body_for_span("\u{310d}\u{3120}\u{3109}\u{311e}\u{02ca}", InputMode::Tps),
            "\u{310d}\u{3120}\u{3109}\u{311e}\u{02ca}"
        );
        // A leading tone-4 stop-coda syllable behaves the same way.
        assert!(span_is_fully_toned_tps(
            "\u{310d}\u{311a}\u{31b5}\u{3109}\u{311e}\u{02ca}"
        ));
    }

    // ----- v3.5.8 S5 — no-dict carve-out (greedy + min-hop helpers) -----

    // Hermetic `SyllableInventory` builder. Pins the carve-out helper
    // without the `LexiconHandle` singleton (Codex post-impl S5 P3,
    // 2026-05-17).
    fn build_inventory(samples: &[&str]) -> SyllableInventory {
        use phonetics::canonicalize_syllable;

        let mut keys: Vec<String> = Vec::new();
        for s in samples {
            let (canonical, tone) = canonicalize_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_syllable"));
            if tone.is_empty() {
                keys.push(format!("tl:{canonical}"));
            } else {
                keys.push(format!("tl:{canonical}{tone}"));
                keys.push(format!("tl:{canonical}"));
            }
        }
        let path = test_support::write_fst_set(
            "inventory.fst",
            keys.into_iter().map(String::into_bytes).collect(),
        );
        SyllableInventory::open(&path).expect("open inventory")
    }

    /// v3.5.9 B-2 — POJ-only inventory builder. Emits `poj:<canonical>`
    /// keys via `phonetics::canonicalize_poj_syllable` (which preserves
    /// POJ ASCII shape, distinct from `canonicalize_syllable`'s TL fold).
    /// Used by the B-2 mode-aware unit tests to prove POJ shadow helpers
    /// route to the `poj:` family of the tagged-single-FST.
    fn build_poj_inventory(samples: &[&str]) -> SyllableInventory {
        use phonetics::canonicalize_poj_syllable;

        let mut keys: Vec<String> = Vec::new();
        for s in samples {
            let (canonical, tone) = canonicalize_poj_syllable(s)
                .unwrap_or_else(|| panic!("sample {s:?} failed canonicalize_poj_syllable"));
            if tone.is_empty() {
                keys.push(format!("poj:{canonical}"));
            } else {
                keys.push(format!("poj:{canonical}{tone}"));
                keys.push(format!("poj:{canonical}"));
            }
        }
        let path = test_support::write_fst_set(
            "inventory.fst",
            keys.into_iter().map(String::into_bytes).collect(),
        );
        SyllableInventory::open(&path).expect("open inventory")
    }

    // ---- §52 typed `-` is a syllable boundary (USER 2026-09-22) ----
    // Inventory mirrors production around `khi|ah`: every strict prefix
    // of `khiah` that is itself a syllable (`khi`, `khia`) is present, so
    // the greedy reading has something wrong to prefer.

    fn khiah_inventory() -> SyllableInventory {
        build_inventory(&["khi3", "khia7", "khiah4", "ah4", "a1", "i1"])
    }

    #[test]
    fn typed_hyphen_barrier_blocks_a_single_syllable_from_crossing_it() {
        use crate::syllabifier::valid_span_endings_lowered_with_barriers as endings;
        let inv = khiah_inventory();
        // One syllable from 0: only `khi` may end before the barrier.
        assert_eq!(endings("khiah", 0, &inv, InputMode::Tl, 1, &[3]), vec![3]);
        // Two syllables: chains still meet AT the barrier (`khi`+`a`,
        // `khi`+`ah`) — only the crossing single is gone.
        assert_eq!(
            endings("khiah", 0, &inv, InputMode::Tl, 2, &[3]),
            vec![3, 4, 5]
        );
        // No barrier: byte-identical to before (`khi`, `khia`, `khiah`).
        assert_eq!(
            endings("khiah", 0, &inv, InputMode::Tl, 1, &[]),
            vec![3, 4, 5]
        );
        // English never honours one.
        assert_eq!(
            endings("khiah", 0, &inv, InputMode::English, 1, &[3]),
            vec![3, 4, 5]
        );
    }

    #[test]
    fn typed_hyphen_closing_one_syllable_admits_no_cut_inside_it() {
        use crate::syllabifier::valid_span_endings_lowered_with_barriers as endings;
        // trace: shadow `siamtioh`, barrier 4 closes `siam` (one syllable).
        // Unclosed, `sia`+`m` and `si`+`a` cut inside it at 2 and 3.
        let inv = build_inventory(&["siam2", "sia2", "si5", "a1", "am1", "m7", "tioh8"]);
        assert_eq!(
            endings("siamtioh", 0, &inv, InputMode::Tl, 3, &[]),
            vec![2, 3, 4, 8]
        );
        assert_eq!(
            endings("siamtioh", 0, &inv, InputMode::Tl, 3, &[4]),
            vec![4, 8]
        );
        // A link never starts inside the stretch either (`m` from 3).
        assert!(endings("siamtioh", 3, &inv, InputMode::Tl, 3, &[4]).is_empty());
        // A digit typed after the `-` (`siam-2` → shadow `siam2`) is no tone
        // of `siam`: the stretch stays closed and one syllable.
        assert_eq!(endings("siam2", 0, &inv, InputMode::Tl, 3, &[4]), vec![4]);
        // The open tail after the last barrier is unconstrained (`tioh` then
        // `si` 6, `sia` 7, `siam` 8), and a stretch that is no single
        // syllable (`siamtioh` closed at 8) keeps its inner cuts.
        assert_eq!(
            endings("tiohsiam", 0, &inv, InputMode::Tl, 3, &[4]),
            vec![4, 6, 7, 8]
        );
        assert_eq!(
            endings("siamtioh", 0, &inv, InputMode::Tl, 3, &[8]),
            vec![2, 3, 4, 8]
        );
        // English never closes one.
        assert_eq!(
            endings("siamtioh", 0, &inv, InputMode::English, 3, &[4]),
            vec![2, 3, 4, 8]
        );
    }

    #[test]
    fn oov_readings_split_on_a_typed_hyphen_in_tl_poj_only() {
        let inv = khiah_inventory();
        assert_eq!(
            greedy_longest_syllabification("khiah", &inv, InputMode::Tl, &[3]),
            Some(vec![(0, 3), (3, 5)])
        );
        assert_eq!(
            greedy_longest_syllabification("khiah", &inv, InputMode::Tl, &[]),
            Some(vec![(0, 5)])
        );
        assert_eq!(
            span_min_syllable_count("khiah", &inv, InputMode::Tl, &[3]),
            Some(2)
        );
        assert_eq!(
            span_min_syllable_count("khiah", &inv, InputMode::Tl, &[]),
            Some(1)
        );
        // Span-local shift, and the mode gate: TPS / English get none.
        assert_eq!(
            oov_reading_barriers(&[3, 5], 2, 5, InputMode::Tl),
            vec![1, 3]
        );
        assert_eq!(
            oov_reading_barriers(&[3, 5], 0, 5, InputMode::Poj),
            vec![3, 5]
        );
        assert!(oov_reading_barriers(&[3], 0, 5, InputMode::Tps).is_empty());
        assert!(oov_reading_barriers(&[3], 0, 5, InputMode::English).is_empty());
    }

    #[test]
    fn span_key_pins_typed_boundaries_for_a_hyphenated_tl_poj_span() {
        let boundary_kinds = |mode, typed: &str, at: Vec<(usize, bool)>| TonePin::TypedTones {
            mode,
            typed: typed.to_owned(),
            boundaries: at
                .into_iter()
                .map(|(at, khinsiann)| TypedBoundary { at, khinsiann })
                .collect(),
            one_syllable_stretches: Vec::new(),
        };
        let boundary = |mode, typed: &str, at: Vec<usize>| {
            boundary_kinds(mode, typed, at.into_iter().map(|at| (at, false)).collect())
        };
        // A one-syllable stretch rides on the pin in span coordinates; one
        // the span does not contain is left out.
        let k = span_key("siamtioh", 0, 8, InputMode::Tl, &[4], &[], &[(0, 4)]).expect("body");
        assert_eq!(
            k.tone_pin,
            TonePin::TypedTones {
                mode: InputMode::Tl,
                typed: "siamtioh".to_owned(),
                boundaries: vec![TypedBoundary {
                    at: 4,
                    khinsiann: false,
                }],
                one_syllable_stretches: vec![(0, 4)],
            }
        );
        let k = span_key("siamtioh", 4, 8, InputMode::Tl, &[4], &[], &[(0, 4)]).expect("body");
        assert!(matches!(
            k.tone_pin,
            TonePin::TypedTones { ref one_syllable_stretches, .. }
                if one_syllable_stretches.is_empty()
        ));
        // trace: shadow `khiah`, barrier at 3 → span (0,5) carries it.
        let k = span_key("khiah", 0, 5, InputMode::Tl, &[3], &[], &[]).expect("body");
        assert_eq!(k.key, "tl:khiah");
        assert_eq!(k.tone_pin, boundary(InputMode::Tl, "khiah", vec![3]));
        // Digits and a boundary together: offsets count the typed digit.
        let k = span_key("khi3ah", 0, 6, InputMode::Tl, &[4], &[], &[]).expect("body");
        assert_eq!(k.tone_pin, boundary(InputMode::Tl, "khi3ah", vec![4]));
        // POJ reads the same shape.
        let k = span_key("khiah", 0, 5, InputMode::Poj, &[3], &[], &[]).expect("body");
        assert_eq!(k.tone_pin, boundary(InputMode::Poj, "khiah", vec![3]));
        // The barrier at a span's own start is carried as `at: 0` with its
        // kind — read only by a reading that opens with `--` itself.
        let k = span_key("khiah", 3, 5, InputMode::Tl, &[3], &[(3, 2)], &[]).expect("body");
        assert_eq!(
            k.tone_pin,
            boundary_kinds(InputMode::Tl, "ah", vec![(0, true)])
        );
        let k = span_key("khiah", 3, 5, InputMode::Tl, &[3], &[], &[]).expect("body");
        assert_eq!(
            k.tone_pin,
            boundary_kinds(InputMode::Tl, "ah", vec![(0, false)])
        );
        // …and the `--` subset marks an interior boundary's kind.
        let k = span_key("khiah", 0, 5, InputMode::Tl, &[3], &[(3, 2)], &[]).expect("body");
        assert_eq!(
            k.tone_pin,
            boundary_kinds(InputMode::Tl, "khiah", vec![(3, true)])
        );
        // No barrier, no digit → unpinned as before; English never pins.
        assert_eq!(
            span_key("khiah", 0, 5, InputMode::Tl, &[], &[], &[])
                .unwrap()
                .tone_pin,
            TonePin::None
        );
        assert_eq!(
            span_key("khiah", 0, 5, InputMode::English, &[3], &[], &[])
                .unwrap()
                .tone_pin,
            TonePin::None
        );
    }

    #[test]
    fn greedy_longest_syllabification_taiuantai_reads_tai_uan_tai() {
        // The documented no-dict carve-out expectation: `taiuantai`
        // (no dict hit anywhere) must read as the canonical
        // longest-syllable segmentation `tai uan tai`, NOT the
        // sub-syllable over-split `ta i u an ta i` (literal
        // max-syllable-count) and NOT the min-cost fewest-edge blob.
        let inv = build_inventory(&["tai1", "uan1", "ta1", "i1", "u1", "an1"]);
        let segs = greedy_longest_syllabification("taiuantai", &inv, InputMode::Tl, &[])
            .expect("buffer is fully syllabifiable");
        assert_eq!(segs, vec![(0, 3), (3, 6), (6, 9)]);
        let roman = segs
            .iter()
            .map(|&(s, e)| strip_ascii_tone_digits(&"taiuantai"[s..e]))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(roman, "tai uan tai");
    }

    #[test]
    fn greedy_longest_syllabification_returns_none_when_unsegmentable() {
        // A trailing byte with no valid syllable → `None`, so the
        // caller suppresses the slot-0 synth and leaves the span-local
        // list untouched (pre-S2 behavior).
        let inv = build_inventory(&["tai1"]);
        assert!(greedy_longest_syllabification("taix", &inv, InputMode::Tl, &[]).is_none());
    }

    #[test]
    fn greedy_longest_syllabification_poj_mode_uses_poj_family() {
        // v3.5.9 B-2 — under POJ mode the carve-out walks the `poj:`
        // family of the inventory. With a `poj:`-only inventory (no
        // matching `tl:` entries), TL-mode probing must FAIL while
        // POJ-mode probing succeeds — proves the mode parameter selects
        // the right family end-to-end.
        let inv = build_poj_inventory(&["chiah4", "goa2"]);
        let segs = greedy_longest_syllabification("chiahgoa", &inv, InputMode::Poj, &[])
            .expect("POJ shadow must syllabify against `poj:` family");
        assert_eq!(segs, vec![(0, 5), (5, 8)]);
        // TL mode against the same inventory finds nothing — the `tl:`
        // family is empty, so the very first step has no valid ending.
        assert!(
            greedy_longest_syllabification("chiahgoa", &inv, InputMode::Tl, &[]).is_none(),
            "TL mode must NOT see `poj:`-only inventory entries"
        );
    }

    // ----- v3.5.8 OOV-cost fix — span_min_syllable_count
    //       (Codex PR #290 P1 r3255035136) -----

    #[test]
    fn span_min_syllable_count_simple_spans() {
        let inv = build_inventory(&["tai1", "uan1", "ta1"]);
        // Whole span is itself one valid syllable → 1 (correct, not a
        // collapse).
        assert_eq!(
            span_min_syllable_count("tai", &inv, InputMode::Tl, &[]),
            Some(1)
        );
        // `taiuanta` = tai|uan|ta → 3 (the synth syllable-sum metadata).
        assert_eq!(
            span_min_syllable_count("taiuanta", &inv, InputMode::Tl, &[]),
            Some(3)
        );
        // Not single-syllable-reachable → None (caller fail-closes).
        assert_eq!(
            span_min_syllable_count("taix", &inv, InputMode::Tl, &[]),
            None
        );
        assert_eq!(span_min_syllable_count("", &inv, InputMode::Tl, &[]), None);
    }

    #[test]
    fn span_min_syllable_count_recovers_a_greedy_dead_end() {
        // The exact P1: greedy-longest dead-ends but the span IS
        // lattice-syllabifiable via a non-greedy split, so the old
        // `greedy…unwrap_or(1)` mispriced this 2-syllable OOV blob as
        // ONE syllable. Inventory = {ta, tan, nia}; input `tania`:
        //   greedy from 0 takes the LONGEST prefix `tan` → tail `ia`
        //   has no valid syllable → greedy = None → old code = 1.
        //   non-greedy `ta` then `nia` spans it → real count = 2.
        let inv = build_inventory(&["ta1", "tan1", "nia1"]);
        assert!(
            greedy_longest_syllabification("tania", &inv, InputMode::Tl, &[]).is_none(),
            "precondition: greedy-longest must dead-end on this span"
        );
        assert_eq!(
            span_min_syllable_count("tania", &inv, InputMode::Tl, &[]),
            Some(2),
            "min-hop walk must recover the real 2-syllable count, not collapse to 1"
        );
    }

    #[test]
    fn span_min_syllable_count_poj_mode_uses_poj_family() {
        // v3.5.9 B-2 — POJ mode min-hop walks the `poj:` family. A
        // `poj:`-only inventory: POJ-mode hop count succeeds while
        // TL-mode probe must return None.
        let inv = build_poj_inventory(&["chiah4", "goa2"]);
        assert_eq!(
            span_min_syllable_count("chiahgoa", &inv, InputMode::Poj, &[]),
            Some(2),
        );
        assert_eq!(
            span_min_syllable_count("chiahgoa", &inv, InputMode::Tl, &[]),
            None,
            "TL mode must NOT see `poj:`-only entries"
        );
    }

    // `abbrev_query_key` — the whole-buffer abbreviation gate.
    #[test]
    fn abbrev_query_key_accepts_letter_only_buffers_of_two_or_more() {
        // trace: consonant acronyms, vowel-initial abbreviations (紅嬰仔
        // `aea`) and plain readings (`ai`, `tai`) all pass — the index
        // decides which of them are abbreviations.
        for raw in [
            "ss", "tk", "mk", "ngs", "ts", "SS", "Tk", "aea", "iii", "ai", "tai",
        ] {
            assert_eq!(
                abbrev_query_key(raw, InputMode::Tl).as_deref(),
                Some(format!("tl-abbrev:{}", raw.to_lowercase()).as_str()),
                "{raw}"
            );
        }
        assert_eq!(
            abbrev_query_key("chp", InputMode::Poj).as_deref(),
            Some("poj-abbrev:chp")
        );
    }

    #[test]
    fn abbrev_query_key_rejects_short_digit_and_separated_buffers() {
        // trace: one glyph; a digit / hyphen / space; English mode.
        for raw in ["s", "", "ss2", "s-s", "s s", "tai5"] {
            assert_eq!(abbrev_query_key(raw, InputMode::Tl), None, "{raw:?}");
        }
        assert_eq!(abbrev_query_key("ss", InputMode::English), None);
    }

    #[test]
    fn abbrev_query_key_tps_glyphs_without_tone_marks() {
        // trace: ㄙ + ㄒ initials, ㄤ + ㆤ + ㄚ vowels (紅嬰仔) both pass.
        assert_eq!(
            abbrev_query_key("ㄙㄒ", InputMode::Tps).as_deref(),
            Some("tps-abbrev:ㄙㄒ")
        );
        assert_eq!(
            abbrev_query_key("ㄤㆤㄚ", InputMode::Tps).as_deref(),
            Some("tps-abbrev:ㄤㆤㄚ")
        );
        // A tone mark, a space, a lone glyph or Latin letters disqualify.
        for raw in ["ㄙㄒˊ", "ㄙ ㄒ", "ㄙ", "ㄙs"] {
            assert_eq!(abbrev_query_key(raw, InputMode::Tps), None, "{raw:?}");
        }
    }
}
