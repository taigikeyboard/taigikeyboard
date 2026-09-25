// Pure shadow pipeline — mode-aware canonicalize (POJ→POJ ASCII under POJ
// mode, POJ→TL ASCII under TL/English mode; v3.5.9 B-2 PR #309), hyphen
// strip, lattice build, and derived span / partial-prefix / custom-key
// helpers.

use lexicon::{ConsumedSpan, SyllableInventory, TonePin, TypedBoundary};
use phonetics::InputMode;
use unicode_normalization::UnicodeNormalization;

use crate::lattice::{build_lattice_with_barriers, Lattice};
use crate::syllabifier::tl::is_tl_tone_digit;
use crate::syllabifier::valid_span_endings_lowered_with_barriers;

/// v3.5.9 B-2 — map `mode` to its FST key family prefix. The tagged-single-FST
/// (`syllables.fst` + `dictionary.fst`) carries `tl:` / `poj:` / `tps:`
/// families; English shares the TL family because English buffers do not
/// have their own inventory and the syllabifier is not invoked there.
///
/// v3.5.9 D / C-3b — TPS promoted to a first-class family. The continuous
/// walker now emits `tps:<bopomofo_toneless>` keys against the C-0 emit
/// of `dictionary.fst`; the legacy `build_keys_tps` path (which folded
/// TPS into `tl:` keys via `phonetics::tps_to_tl`) is retired.
pub(crate) fn mode_key_prefix(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Poj => "poj",
        InputMode::Tps => "tps",
        InputMode::Tl | InputMode::English => "tl",
    }
}

/// v3.5.9 D / C-3b — mode-aware tone-mark stripping. Generalises the
/// TL/POJ-only [`strip_ascii_tone_digits`] so a TPS shadow slice strips
/// its 8 Bopomofo tone scalars (per `phonetics::tps::is_tps_tone_mark`)
/// instead of (no-op) ASCII digits, producing a key body that matches the
/// `tps:<tps_notone>` family. Used by every shadow-derived key / roman
/// emit site: `left_anchored_keys_and_restrictions`, the walker edge key in
/// `composing::continuous::fetch_walker_slot0_inner`, the walker's
/// no-dict roman synth, `custom_toneless_key`, and `build_partial_prefix_key`.
///
/// TL/POJ/English: ASCII-digit strip — byte-identical to the legacy
/// [`strip_ascii_tone_digits`] path.
/// TPS: TPS tone-mark strip — drops the 8 scalars listed in
/// `phonetics::tps::is_tps_tone_mark` (U+02C6 ˆ, U+02C7 ˇ, U+02CA ˊ,
/// U+02CB ˋ, U+02D9 ˙, U+02EA ˪, U+02EB ˫, U+0307 combining dot above).
/// Hyphens / whitespace are NOT stripped here because
/// [`build_hyphen_shadow`] already drops ASCII `-` upstream and the TPS
/// buffer rarely contains them; matches `_TPS_TONE_AND_SEP_RE`'s tone
/// half (build pipeline `dictionary/common/notone.py::remove_tps_tone`).
pub(crate) fn strip_tones_for_mode(s: &str, mode: InputMode) -> String {
    match mode {
        InputMode::Tps => s
            .chars()
            .filter(|c| !phonetics::is_tps_tone_mark(*c))
            .collect(),
        InputMode::Tl | InputMode::Poj | InputMode::English => strip_ascii_tone_digits(s),
    }
}

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
/// ([`span_end_pins_unmarked_tone`]). Surfacing all tones stays correct only
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
///   ([`phonetics::normalize_tps_tone8_scalar`]) so it matches the
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
            .map(phonetics::normalize_tps_tone8_scalar)
            .collect(),
        _ => strip_tones_for_mode(span, mode),
    }
}

/// Cap on syllabifier BFS depth for Phase 6 fetches. Matches the
/// `max_syllables=8` budget called out in `docs/releases/v3.5.8/plan.md` § Phase 3 — Performance and
/// keeps the worst-case lookup at O(n × 3 × 8) FST hits. Owned by
/// [`build_shadow_lattice_with_barriers`] (lattice BFS budget); since v3.5.9 D / C-3b
/// the legacy per-mode `continuous::build_keys_tps` is retired and all
/// four modes (TL/POJ/English/TPS) share the unified shadow → lattice
/// path through this single cap.
pub(crate) const MAX_SYLLABLES: usize = 8;

/// The per-fetch segmentation input: the shadow, its offset map, the
/// DAG, and the barriers (all, and the `--` subset) in shadow coordinates.
pub(crate) struct ShadowLattice {
    pub shadow: String,
    pub shadow_to_raw_end: Vec<usize>,
    pub lattice: Lattice,
    /// Stripped-separator / hyphen barriers (§35).
    pub barriers: Vec<usize>,
    /// The typed `-` runs among them, `(shadow offset, run length)` (§52 kind,
    /// §55 rendering).
    pub hyphen_runs: Vec<(usize, usize)>,
}

/// Run the canonicalize → hyphen-shadow → (TPS-only) space-strip
/// pipeline and build the segmentation lattice over the resulting
/// shadow. The TPS space-strip ([`build_separator_shadow`]) folds the
/// keyboard's tone-1 / syllable-separator space out of the shadow so a
/// first-tone phrase (`ㄍㄠ ㄉㄞ`) yields a cross-space edge; it is a
/// no-op for TL/POJ/English.
/// Returns a [`ShadowLattice`] — the shadow, its raw offset map, the
/// DAG, the stripped-separator / hyphen barrier set in shadow coordinates
/// (§35) and the `--` subset of it (§52). Shared by [`build_continuous_keys`] (left-anchored projection —
/// byte-identical to pre-S1, the S1 pinning tests guard this),
/// `continuous::fetch_walker_slot0_inner` (S2 whole-sentence walker) and
/// the `dispatch::build_keys_tl_with_inventory` test seam, so the
/// shadow + offset map + DAG are constructed exactly once per fetch and
/// the consumers cannot drift.
pub(crate) fn build_shadow_lattice_with_barriers(
    raw: &str,
    inv: &SyllableInventory,
    mode: InputMode,
) -> ShadowLattice {
    let lower = raw.to_ascii_lowercase();
    let (canonical, canonical_to_raw_end) = canonicalize_poj_shadow(&lower, mode);
    lattice_from_canonical_with_barriers(&canonical, &canonical_to_raw_end, inv, mode)
}

/// The separator layers of a canonical shadow: the compound-hyphen strip, the
/// TPS space strip, and their barriers merged into final-shadow
/// coordinates. One body for the lattice pipeline and the whole-buffer
/// key (`fused_shadow_with_barriers`) so a typed `-` pins both the same
/// way (§52).
struct SeparatorLayers {
    /// hyphenless byte → canonical byte.
    hyphenless_to_canonical: Vec<usize>,
    shadow: String,
    /// shadow byte → hyphenless byte.
    shadow_to_hyphenless: Vec<usize>,
    /// Hyphen + space barriers, shadow coordinates, sorted.
    barriers: Vec<usize>,
    /// The TPS space barriers alone — the §41 whole-buffer pin signal.
    space_barriers: Vec<usize>,
    /// The hyphen barriers with their run length, `(shadow offset, run)`:
    /// two or more is the khinsiann `--` the §52 pin tells from a plain
    /// `-`, and the run is what §55 renders at the boundary.
    hyphen_runs: Vec<(usize, usize)>,
}

fn separator_layers(canonical: &str, mode: InputMode) -> SeparatorLayers {
    let (hyphenless, hyphenless_to_canonical, hyphen_barriers) =
        strip_char_shadow_with_barriers(canonical, '-');
    // TPS-only — the ASCII space is the keyboard's tone-1 / syllable
    // separator (appended on `space` while composing so the next
    // dual-form consonant stays an initial), NOT a literal space. Strip
    // it before lattice construction so a first-tone phrase `ㄍㄠ ㄉㄞ`
    // produces one cross-space `(0, full)` edge (交代) instead of
    // dead-ending at the space; the offset map keeps the separator byte
    // in the full-span commit. No-op for TL/POJ/English (space is a real
    // word boundary). See `build_separator_shadow`.
    let (shadow, shadow_to_hyphenless, space_barriers) =
        build_separator_shadow_with_barriers(&hyphenless, mode);
    // Merge the two barrier layers into final-shadow coordinates. Hyphen
    // barriers are hyphenless offsets; project them through the separator
    // strip (count the shadow bytes whose hyphenless source is below the
    // barrier — equivalently, find the shadow offset whose map value first
    // reaches the barrier).
    let mut barriers: Vec<usize> = space_barriers.clone();
    let mut hyphen_runs: Vec<(usize, usize)> = Vec::new();
    for (hyphen_barrier, run) in hyphen_barriers {
        // The projected shadow offset is the LAST map index whose consumed
        // hyphenless prefix still fits under the barrier — i.e. how many
        // hyphenless bytes BEFORE the barrier survived the space strip. A
        // first-index-≥ search is off by one when a stripped space sits
        // immediately before the hyphen (`A␠-B`): the space consumes a
        // hyphenless byte without producing a shadow byte, and ≥ would land
        // the barrier after B's first glyph — a phantom barrier inside the
        // next syllable (Codex post-impl 2026-08-19 BLOCK 3).
        let projected = shadow_to_hyphenless
            .iter()
            .rposition(|&hyphenless_idx| hyphenless_idx <= hyphen_barrier)
            .unwrap_or(0);
        if !barriers.contains(&projected) {
            barriers.push(projected);
        }
        if !hyphen_runs.iter().any(|&(at, _)| at == projected) {
            hyphen_runs.push((projected, run));
        }
    }
    barriers.sort_unstable();
    SeparatorLayers {
        hyphenless_to_canonical,
        shadow,
        shadow_to_hyphenless,
        barriers,
        space_barriers,
        hyphen_runs,
    }
}

/// [`lattice_from_canonical`] plus the merged barrier set: every shadow
/// byte offset where a user separator (TPS space) or compound hyphen was
/// stripped. Barriers gate the §35 ambiguity expansion — a single
/// syllable may not cross one, and the glyph before one is Final-only —
/// and the TPS syllabifier receives them so an expanded probe cannot
/// fuse across the user's explicit boundary.
fn lattice_from_canonical_with_barriers(
    canonical: &str,
    canonical_to_raw_end: &[usize],
    inv: &SyllableInventory,
    mode: InputMode,
) -> ShadowLattice {
    let SeparatorLayers {
        hyphenless_to_canonical,
        shadow,
        shadow_to_hyphenless,
        barriers,
        hyphen_runs,
        ..
    } = separator_layers(canonical, mode);
    // Compose the three offset maps: shadow → hyphenless → canonical → raw.
    let shadow_to_raw_end: Vec<usize> = shadow_to_hyphenless
        .iter()
        .map(|&hyphenless_idx| canonical_to_raw_end[hyphenless_to_canonical[hyphenless_idx]])
        .collect();
    // §41 — consume a TRAILING separator marker into the full-span end.
    //
    // `strip_char_shadow_with_barriers` only records map entries for the
    // characters it KEEPS, so an interior space is swept up by the next
    // glyph's entry (`build_separator_shadow_tps_strips_space_and_maps_to_raw`
    // pins that) but a TRAILING one falls past the last entry: `ㄒㄧ␣` maps
    // its full span to raw 6 of 7 bytes. `commit_continuous` then keeps
    // `pending[consumed_bytes..]` — a lone `" "` the user never sees (the
    // display seam hides it) but that leaves the engine composing a phantom
    // buffer: the next backspace deletes the invisible marker, the
    // final-commit test misreads, and the NextWord terminal segment can get
    // an empty display.
    //
    // Fixed HERE, after the barrier merge, rather than inside the shared
    // strip primitive: that primitive also serves the hyphen layer, where a
    // trailing hyphen MUST stay pending
    // (`build_hyphen_shadow_trailing_hyphen_is_not_consumed`), and moving the
    // endpoint earlier would shift the hyphen-barrier `rposition` projection
    // above. Barrier offsets, lattice edges and `key_final_only_offsets` are
    // all untouched — only the full-span shadow→raw endpoint moves.
    let mut shadow_to_raw_end = shadow_to_raw_end;
    if matches!(mode, InputMode::Tps) {
        let kept_end = hyphenless_to_canonical[shadow_to_hyphenless[shadow.len()]];
        if kept_end < canonical.len() && canonical[kept_end..].chars().all(|c| c == ' ') {
            if let Some(full_span_end) = shadow_to_raw_end.last_mut() {
                *full_span_end = canonical_to_raw_end[canonical.len()];
            }
        }
    }
    // v3.5.9 B-2 — thread `mode` into the lattice builder; the inventory is
    // mode-aware (`SyllableInventory::contains_in(mode, …)`), so a POJ-mode
    // shadow now resolves against the `poj:` family of `syllables.fst` and
    // emits POJ-shaped syllable boundaries (`chiah`, `goa`, …) rather than
    // collapsing onto the TL forms.
    let lattice = build_lattice_with_barriers(&shadow, inv, mode, MAX_SYLLABLES, &barriers);
    ShadowLattice {
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        hyphen_runs,
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
/// `dispatch::build_continuous_keys_with_inventory` test seam, so the
/// two cannot drift. Empty `final_only` for TL / POJ / English — their
/// pipeline strips no ambiguity glyph; `barriers` there are the typed
/// hyphens (§52).
pub(crate) struct ContinuousKeys {
    pub keys: Vec<(ConsumedSpan, String)>,
    pub final_only: Vec<Vec<usize>>,
    /// `tone_pins[i]` is the post-lookup tone constraint for `keys[i]`
    /// ([`span_key`] → [`span_tone_pin`]): the §17 typed digits of a
    /// TL/POJ span, or the §41 unmarked tone of a TPS span closed by the
    /// keyboard's space. The lookup layer keeps only candidates whose
    /// reading honors it.
    pub tone_pins: Vec<TonePin>,
    pub shadow: String,
    pub shadow_to_raw_end: Vec<usize>,
    pub lattice: Lattice,
    pub barriers: Vec<usize>,
    /// The typed `-` runs among them, `(shadow offset, run length)`.
    pub hyphen_runs: Vec<(usize, usize)>,
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
    let ShadowLattice {
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        hyphen_runs,
        ..
    } = build_shadow_lattice_with_barriers(raw, inv, mode);
    let LeftAnchoredKeys {
        keys,
        final_only,
        tone_pins,
    } = left_anchored_keys_and_restrictions(
        &shadow,
        &shadow_to_raw_end,
        &lattice,
        inv,
        mode,
        &barriers,
        &hyphen_runs,
    );
    ContinuousKeys {
        keys,
        final_only,
        tone_pins,
        shadow,
        shadow_to_raw_end,
        lattice,
        barriers,
        hyphen_runs,
    }
}

/// v3.5.9 A1 — extracted from the pre-A1 `build_keys_tl_with_inventory`
/// loop. Emit ONLY the lattice's left-anchored (`start == 0`)
/// projection as keys, byte-identical to the pre-S1 single-start
/// `valid_span_endings(shadow, 0, …)` output (`build_lattice` sorts
/// edges so the `start == 0` ones come first in ascending-`end`
/// order), so the span-local candidate / commit path is unchanged
/// from S1.
///
/// Interior (`start > 0`) edges are NOT emitted as user-facing keys:
/// under Model B (`docs/engine/continuous-input-ranking.md`
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
/// tone pin. `barriers` are shadow coordinates from
/// [`build_shadow_lattice_with_barriers`]; empty for TL/POJ/English.
pub(crate) fn left_anchored_keys_and_restrictions(
    shadow: &str,
    shadow_to_raw_end: &[usize],
    lattice: &Lattice,
    inv: &SyllableInventory,
    mode: InputMode,
    barriers: &[usize],
    hyphen_runs: &[(usize, usize)],
) -> LeftAnchoredKeys {
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
    // the longest single syllable, AND (c) has NO multi-syllable phrase
    // reading — i.e. no interior edge `(m, end)` with `m > 0` reaches it. (c)
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
    // (`iah`, `iakau3`) live in `tests/build_keys_tl_lattice.rs`.
    let lowered = shadow.to_ascii_lowercase();
    // §18 recompute is barrier-aware (Codex post-impl 2026-08-19 BLOCK 2):
    // the lattice above was built with barriers, so re-deriving the
    // single-syllable ends WITHOUT them can manufacture a longer
    // "single syllable" that fuses across the user's separator (`ㄍㄚ`␣`ㄉ`
    // mid-typing: a barrier-blind recompute reads ㄍㄚㆵ as the longest
    // single and wrongly suppresses the legitimate ㄍㄚ).
    let single_ends = crate::syllabifier::valid_span_endings_lowered_with_barriers(
        &lowered, 0, inv, mode, 1, barriers,
    );
    let max_single_end = single_ends.iter().copied().max();
    let has_phrase_reading = |end: usize| lattice.edges().iter().any(|&(s, e)| e == end && s > 0);
    let is_closed_dead_end = |end: usize| {
        lattice.edges().iter().all(|&(s, _)| s != end)
            && remainder_has_closed_syllable(shadow, end, mode, barriers)
    };
    let survives_as_phrase = |end: usize| has_phrase_reading(end) && !is_closed_dead_end(end);

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
        // (a)/(b)/(c)/(d) rule above). Longest single, phrase ends, and
        // phrase-reachable shorter spans that can still continue are kept.
        if single_ends.contains(&end) && Some(end) != max_single_end && !survives_as_phrase(end) {
            continue;
        }
        // Tone-aware lookup body + §35 / §41 barrier metadata, same
        // derivation as the walker edge ([`span_key`]); a digit-only /
        // bare-tone-mark span has no key.
        let Some(SpanKey {
            key,
            final_only,
            tone_pin,
        }) = span_key(shadow, 0, end, mode, barriers, hyphen_runs)
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
    /// `"{prefix}:{body}"` — [`mode_key_prefix`] + [`fst_body_for_span`].
    pub key: String,
    /// [`key_final_only_offsets`] — KEY byte offsets of the glyph before
    /// each barrier inside / at the end of the span.
    pub final_only: Vec<usize>,
    /// [`span_tone_pin`] — the §17 typed digits of a TL/POJ span, or the
    /// §41 unmarked tone of a TPS span closed on a stripped space.
    pub tone_pin: TonePin,
}

/// Derive the lookup key triple for `shadow[start..end]`. `None` when the
/// tone-ruled body is empty (digit-only / bare-tone-mark span).
///
/// Two barrier coordinate conventions meet here — keep them straight:
/// - **Final-only offsets are span-local.** `barriers` (whole-shadow byte
///   offsets) are narrowed to `start < b <= end` and shifted by `-start`
///   before [`key_final_only_offsets`] maps them into the emitted key. A
///   barrier exactly at `start` is a boundary the span opens on, not one
///   it contains, so it is excluded; one at `end` is included (the user
///   closed that syllable).
/// - **The tone pin is global.** [`span_end_pins_unmarked_tone`] (inside
///   [`span_tone_pin`]) compares the whole-shadow `end` against the
///   unshifted `barriers`.
///
/// The left-anchored path passes `start = 0`, where the two conventions
/// coincide.
pub(crate) fn span_key(
    shadow: &str,
    start: usize,
    end: usize,
    mode: InputMode,
    barriers: &[usize],
    hyphen_runs: &[(usize, usize)],
) -> Option<SpanKey> {
    let span = &shadow[start..end];
    let body = fst_body_for_span(span, mode);
    if body.is_empty() {
        return None;
    }
    let prefix = mode_key_prefix(mode);
    let span_barriers = span_local_barriers(barriers, start, end);
    let final_only = key_final_only_offsets(span, &body, mode, &span_barriers, prefix.len() + 1);
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
    Some(SpanKey {
        key: format!("{prefix}:{body}"),
        final_only,
        tone_pin: span_tone_pin(span, &body, end, mode, barriers, boundaries),
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

/// The post-lookup constraint for `span` (whose lookup body is `body`),
/// one rule per mode family:
/// - **TL / POJ**: a span carrying at least one ASCII tone digit or a
///   typed `-` boundary pins what it typed — [`TonePin::TypedTones`]
///   over the hyphenless span verbatim (`teng5sek`) plus `boundaries`
///   (§52: a reading must end a syllable on every one with the same
///   separator kind, so `khi|ah` drops 隙 `khiah`). The lookup body is
///   toneless for a partial-tone span (there is no partial-tone FST
///   family, §17 case 3) and verbatim for a fully-toned one (§17 case
///   1); the pin re-applies the typed digits after the lookup either
///   way, and is what keeps a toneless-matched custom entry honest for a
///   fully-toned span. A span with neither stays unpinned — the "show
///   all tones" affordance.
/// - **TPS**: [`span_end_pins_unmarked_tone`] → [`TonePin::TpsSpaceEnd`]
///   over the toneless body (§41).
/// - **English**: never pinned — a digit there is not a tone.
pub(crate) fn span_tone_pin(
    span: &str,
    body: &str,
    span_end: usize,
    mode: InputMode,
    barriers: &[usize],
    boundaries: Vec<TypedBoundary>,
) -> TonePin {
    match mode {
        InputMode::Tl | InputMode::Poj
            if span.bytes().any(|b| b.is_ascii_digit()) || !boundaries.is_empty() =>
        {
            TonePin::TypedTones {
                mode,
                typed: span.to_owned(),
                boundaries,
            }
        }
        InputMode::Tps if span_end_pins_unmarked_tone(span, span_end, mode, barriers) => {
            TonePin::TpsSpaceEnd(body.to_owned())
        }
        _ => TonePin::None,
    }
}

/// The whole typed buffer's [`TonePin`] — the counterpart of
/// [`span_tone_pin`] for the sources that carry no span key of their own:
/// custom-dictionary entries are synthesized at `(0, raw_len)` and the
/// partial-prefix extensions are hydrated from a strict prefix of the
/// buffer, so the lookup layer aligns their readings against this pin
/// instead. One projection of [`whole_buffer_span_key`], so it is
/// exactly the pin a span-local key over the whole buffer would carry:
/// a TL/POJ buffer with a typed digit anywhere → [`TonePin::TypedTones`];
/// a TPS buffer whose TAIL syllable was closed by the keyboard's space
/// (a trailing-space barrier at the shadow end, no mark of its own) →
/// [`TonePin::TpsSpaceEnd`] over the fused toneless body (§41); English,
/// toneless TL/POJ and an un-closed TPS tail → none.
pub(crate) fn whole_buffer_tone_pin(raw: &str, mode: InputMode) -> TonePin {
    whole_buffer_span_key(raw, mode)
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
fn whole_buffer_span_key(raw: &str, mode: InputMode) -> Option<SpanKey> {
    let (shadow, barriers, hyphen_runs) = fused_shadow_with_barriers(raw, mode);
    span_key(&shadow, 0, shadow.len(), mode, &barriers, &hyphen_runs)
}

/// A3 (§41) — true when the span ending at `span_end` (shadow
/// coordinates) closes on a barrier the TPS separator strip left behind
/// and carries no tone mark of its own. That is exactly the shape the
/// reported bug needs: the user pressed the keyboard's space to close an
/// unmarked syllable, so the candidate list must narrow to that
/// syllable's no-mark tone (1 open rime / 4 stop coda) instead of every
/// tone of the toneless key.
///
/// **Span-end only.** A barrier strictly INSIDE the span is a plain
/// syllable boundary and must stay one: S18's `ㄉㄞ`␣`ㄍㄧ` → 台機 opens
/// on `tai5` and S23's `ㄇ`␣`ㄒㄧ` → 毋是 opens on `m7`, so reading every
/// interior space as a tone-1 instruction would delete both phrases.
///
/// TPS-only. In TL/POJ a space is a literal word boundary (the strip is a
/// no-op there, so `barriers` carries no space offsets anyway) and their
/// tones are ASCII digits, which the fully-toned path already pins. A
/// span whose last char IS a tone mark needs no pinning either — it took
/// the verbatim toned key in [`fst_body_for_span`].
pub(crate) fn span_end_pins_unmarked_tone(
    span: &str,
    span_end: usize,
    mode: InputMode,
    barriers: &[usize],
) -> bool {
    matches!(mode, InputMode::Tps)
        && barriers.contains(&span_end)
        && !span
            .chars()
            .next_back()
            .is_some_and(phonetics::is_tps_tone_mark)
}

/// §35 barrier contract part (b) for one emitted key: for every barrier
/// inside (or at the end of) the span, the KEY byte offset of the glyph
/// just before it — the pattern slot that keeps only Final-role
/// readings. `body` is the already-computed lookup body for the span
/// (verbatim when fully toned, tone-stripped otherwise), so the offsets
/// account for stripped tone marks; `prefix_len` shifts them past the
/// `"tps:"` family prefix. A trailing barrier (separator at shadow end)
/// counts — the user closed that syllable, unlike a plain buffer end
/// which stays unrestricted.
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

/// Build a hyphenless shadow of `raw` paired with a byte-indexed map
/// from shadow byte offsets back to raw byte offsets, mirroring the
/// hyphen half of `dictionary/common/notone.py::remove_tone` regex
/// `[\d\-]`. The digit half stays at [`strip_ascii_tone_digits`].
///
/// Contract:
/// - `shadow` is `raw` with every ASCII `-` (U+002D) removed; all other
///   bytes (including non-ASCII bytes from accidental POJ diacritics)
///   pass through unchanged.
/// - `shadow_to_raw_end` has length `shadow.len() + 1`; index `k` is the
///   raw byte offset RIGHT AFTER the last raw char that contributed the
///   `k`-th shadow byte. `shadow_to_raw_end[0] = 0`.
/// - Leading hyphens before the first surviving raw char ARE folded
///   into the consumed prefix: every shadow ending whose raw mapping
///   passes byte index 0 inherits the preceding hyphens in its
///   `consumed_span`.
/// - Trailing hyphens AFTER the last surviving raw char are NOT
///   folded: a shadow ending at `shadow.len()` maps to the raw byte
///   AFTER the last non-hyphen char, leaving any trailing hyphen in the
///   pending raw buffer for platform UI to retain post-commit.
///
/// Examples:
/// - `"tai-bak"` → shadow `"taibak"`, map `[0, 1, 2, 3, 5, 6, 7]`
/// - `"-tai"`    → shadow `"tai"`,    map `[0, 2, 3, 4]`
/// - `"tai-"`    → shadow `"tai"`,    map `[0, 1, 2, 3]`
/// - `"goa--si"` → shadow `"goasi"`,  map `[0, 1, 2, 3, 6, 7]`
/// - `"---"`     → shadow `""`,       map `[0]`
#[cfg(test)]
pub(crate) fn build_hyphen_shadow(raw: &str) -> (String, Vec<usize>) {
    strip_char_shadow(raw, '-')
}

/// Shared strip + offset-map mechanism behind [`build_hyphen_shadow`]
/// and [`build_separator_shadow`]: drop every `skip` char from `input`
/// and return the stripped string paired with a byte map where index `k`
/// is the input byte offset RIGHT AFTER the last char that contributed
/// the `k`-th output byte. `map[0] = 0`; `map.len() == output.len() + 1`.
/// The two callers differ only in which char they strip (`-` vs ` `) and
/// in their mode gating; the loop body is identical, so it lives here.
#[cfg(test)]
fn strip_char_shadow(input: &str, skip: char) -> (String, Vec<usize>) {
    let (shadow, map, _) = strip_char_shadow_with_barriers(input, skip);
    (shadow, map)
}

/// [`strip_char_shadow`] plus the OUTPUT byte offsets where a stripped
/// char sat — the "barrier" positions the §35 ambiguity expansion needs:
/// a stripped separator / hyphen is the user's explicit syllable close, so
/// (a) no single-syllable probe may cross it and (b) the glyph just
/// before it may only read as a Final form. Offsets are in shadow
/// coordinates (`0 ≤ b ≤ shadow.len()`); consecutive stripped chars
/// dedupe to one barrier carrying the run length.
fn strip_char_shadow_with_barriers(
    input: &str,
    skip: char,
) -> (String, Vec<usize>, Vec<(usize, usize)>) {
    let mut shadow = String::with_capacity(input.len());
    let mut map: Vec<usize> = Vec::with_capacity(input.len() + 1);
    // `(shadow offset, run length)` — the run tells `--` from `-` (§52).
    let mut barriers: Vec<(usize, usize)> = Vec::new();
    map.push(0);
    for (idx, ch) in input.char_indices() {
        if ch == skip {
            match barriers.last_mut() {
                Some((at, run)) if *at == shadow.len() => *run += 1,
                _ => barriers.push((shadow.len(), 1)),
            }
            continue;
        }
        let end_after = idx + ch.len_utf8();
        for _ in 0..ch.len_utf8() {
            map.push(end_after);
        }
        shadow.push(ch);
    }
    (shadow, map, barriers)
}

/// Mode-aware syllable-separator strip, paired with a shadow→input
/// byte-offset map (mirrors [`build_hyphen_shadow`]'s shape).
///
/// In TPS the ASCII space (U+0020) is the keyboard's tone-1 / syllable
/// boundary marker — iOS `ActionHandler+KeyActions` / Android
/// `TextInputKeyHandler` append it on the `space` key WHILE composing
/// precisely so the next dual-form consonant stays an initial
/// (`tps_adjust::adjust_initial_key` treats space as a syllable
/// boundary). First tone has no tone mark, so the space is the only
/// delimiter it has. The continuous segmenter must therefore treat that
/// space as a ZERO-WIDTH separator: strip it from the shadow so the
/// lattice produces a cross-space phrase edge (`ㄍㄠ ㄉㄞ` → one
/// `(0, full)` span → 交代) instead of dead-ending at the space, while
/// the returned offset map keeps every shadow byte anchored to the raw
/// byte AFTER the run it came from — so a full-span commit's
/// `consumed_span` still consumes the separator byte and fully replaces
/// the preedit.
///
/// TPS-only: for TL/POJ/English a space is a real word boundary / literal
/// space and MUST stay a hard segment boundary, so this returns the
/// identity shadow + map (byte-identical to not calling it).
///
/// Contract mirrors [`build_hyphen_shadow`]: `map` has length
/// `shadow.len() + 1`; index `k` is the input byte offset right after the
/// last input char that contributed the `k`-th shadow byte; `map[0] = 0`.
#[cfg(test)]
fn build_separator_shadow(input: &str, mode: InputMode) -> (String, Vec<usize>) {
    let (shadow, map, _) = build_separator_shadow_with_barriers(input, mode);
    (shadow, map)
}

/// [`build_separator_shadow`] plus the stripped-space barrier offsets
/// (shadow coordinates). Non-TPS returns the identity shadow and no
/// barriers.
fn build_separator_shadow_with_barriers(
    input: &str,
    mode: InputMode,
) -> (String, Vec<usize>, Vec<usize>) {
    if !matches!(mode, InputMode::Tps) {
        return (input.to_owned(), (0..=input.len()).collect(), Vec::new());
    }
    let (shadow, map, barriers) = strip_char_shadow_with_barriers(input, ' ');
    (
        shadow,
        map,
        barriers.into_iter().map(|(at, _)| at).collect(),
    )
}

/// Drop every ASCII digit from `s`. Equivalent to the digit half of
/// `dictionary/common/notone.py::remove_tone()` regex `[\d\-]` under
/// the canonical-ASCII TL input contract — Python `\d` matches every
/// Unicode decimal digit, but TL canonical input only ever uses
/// ASCII `0..=9`, so `is_ascii_digit()` is sound here. Hyphens are
/// stripped one layer up by [`build_hyphen_shadow`] (Phase 9 Item 8),
/// so callers feed this fn a hyphenless shadow slice already.
fn strip_ascii_tone_digits(s: &str) -> String {
    s.chars().filter(|c| !c.is_ascii_digit()).collect()
}

/// The whole-buffer fused shadow — `lowercase → canonicalize_poj_shadow
/// → build_hyphen_shadow → build_separator_shadow` — with no tone rule
/// applied and every offset map discarded. This is the same pipeline
/// [`build_shadow_lattice_with_barriers`] runs (there with the maps kept
/// for span slicing), so a key built from it is byte-identical to the
/// walker / left-anchored edge keys built from the same buffer. Callers
/// apply their own tone rule: [`custom_toneless_key`] always strips
/// ([`strip_tones_for_mode`]), [`build_partial_prefix_key`] keeps a fully
/// toned buffer verbatim ([`fst_body_for_span`]).
fn fused_shadow(raw: &str, mode: InputMode) -> String {
    fused_shadow_with_barriers(raw, mode).0
}

/// [`fused_shadow`] plus the barriers (shadow coordinates) a whole-buffer
/// key pins on, and the typed `-` runs among them: for TL / POJ the typed `-` boundaries (§52, the custom /
/// learned / partial-prefix pin); for TPS the stripped-space barriers
/// alone — the §41 pin signal — never a typed hyphen, which there only
/// gates the §35 ambiguity expansion the whole-buffer callers discard.
fn fused_shadow_with_barriers(
    raw: &str,
    mode: InputMode,
) -> (String, Vec<usize>, Vec<(usize, usize)>) {
    let lower = raw.to_ascii_lowercase();
    let (canonical, _) = canonicalize_poj_shadow(&lower, mode);
    let layers = separator_layers(&canonical, mode);
    let (barriers, hyphen_runs) = match mode {
        InputMode::Tps => (layers.space_barriers, Vec::new()),
        InputMode::Tl | InputMode::Poj | InputMode::English => {
            (layers.barriers, layers.hyphen_runs)
        }
    };
    (layers.shadow, barriers, hyphen_runs)
}

/// v3.5.8 S6 (Codex pre-impl S6 Q2, 2026-05-17, BLOCK condition) —
/// derive the walker lattice-edge match key for a
/// `custom_dictionary.db` entry's romanization, or `None` when the
/// derived form does not land in `[a-z]+` after canonicalization.
///
/// MUST produce a key byte-identical to the one
/// `continuous::fetch_walker_slot0_inner`'s edge provider builds for a
/// syllable span (`<prefix>:{toneless}` with `prefix ∈ {tl, poj}` per
/// `mode_key_prefix(mode)`; v3.5.9 B-2 PR #309 promoted POJ to a
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
pub(crate) fn custom_toneless_key(roman: &str, mode: InputMode) -> Option<String> {
    let toneless = strip_tones_for_mode(&fused_shadow(roman, mode), mode);
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
    let prefix = mode_key_prefix(mode);
    Some(format!("{prefix}:{toneless}"))
}

/// Canonicalize POJ-display input (`pe̍h-ōe-jī`, `chóa`, `peⁿ`, `so͘`)
/// into ASCII spelling for its mode's FST key family, paired with a
/// byte-indexed map from canonical byte offsets back to original `input`
/// byte offsets. v3.5.8 Phase 9 Item 9; v3.5.9 B-2 reshape so the output
/// is **POJ ASCII** under POJ mode (`chiah` stays `chiah`) and **TL literal**
/// under TL mode (POJ-shaped input is NOT folded into the TL chain).
///
/// Mode-gated Phase 2 rule list (v3.5.9 B-2, refined by PR #309 Codex
/// P1 `r3276402303`; TL-literal pass added 2026-06-05):
/// - `mode == InputMode::Poj` → [`phonetics::NORMALIZE_TO_POJ_GLYPH_RULES`]:
///   the glyph-only subset (`o͘→oo`, `ⁿ→nn`, `ᴺ→nn`). The legacy
///   `ou→oo` alias is **excluded** because it would mis-fire across
///   syllable boundaries on hyphenless multi-syllable user input — e.g.
///   typing `toui` for POJ `tó-uī` (indexed `poj_notone=toui`) would
///   get folded to `tooi` and lose the lattice match. Per-syllable
///   callers (build-pipeline `canonicalize_poj_syllable`, runtime
///   `derive_poj_notone_for_match`) still consume the full
///   [`phonetics::NORMALIZE_TO_POJ_RULES`] — they apply per-token so
///   the `ou` alias only ever sees a single syllable.
/// - `mode != InputMode::Poj` (TL / TPS / English) →
///   [`phonetics::TL_ENCODING_RULES`]: encoding-only (`o͘→oo`, `ⁿ→nn`,
///   `ᴺ→nn`, `oonn→onn`), **no POJ→TL spelling fold**. TL input is taken
///   literally so a valid TL special final `eng` [ɛŋ] survives (not
///   collapsed to `ing` [iŋ]) and POJ-shaped TL-mode input (`teng`/`goa`/
///   `chiah`) is not auto-corrected into a `tl:` hit. The ASCII branch is
///   still identity (F3C gate); since the encoding rules are no-ops on
///   ASCII, ASCII identity and `TL_ENCODING_RULES` agree on ASCII input.
///
/// Phase 1 — char-level NFD walk over the original input. Each NFD
/// scalar that is a tone mark per `phonetics::is_combining_tone_mark`
/// (`U+0300, U+0301, U+0302, U+0304, U+0306, U+030B, U+030C, U+030D`) is
/// dropped, with its UTF-8 byte width absorbed into the preceding base
/// char's `raw_end` so the offset map stays anchored at the right of
/// each consumed run. Other NFD scalars pass through unchanged
/// (including `\u{0358}` and `\u{207f}` / `\u{1d3a}`, which Phase 2
/// turns into ASCII regardless of which rule list is active — both
/// lists carry the same encoding rules).
///
/// Phase 2 — apply the mode-selected rule list with offset-aware
/// substring replace. Shrinking rules (`o\u{0358}→oo`, `\u{207f}|\u{1d3a}
/// →nn`, `oonn→onn` for TL only) drain the dropped trailing byte's map
/// entry; byte-count-preserving rules leave the offset map invariant.
///
/// Output contract (mirrors [`build_hyphen_shadow`]):
/// - `canonical` is ASCII (after Phase 2 all non-ASCII codepoints have
///   been replaced with ASCII spellings).
/// - `canonical_to_raw_end` has length `canonical.len() + 1`. Index `k`
///   is the original-input byte offset right after the last original
///   byte that contributed the first `k` canonical bytes.
///   `canonical_to_raw_end[0] = 0`.
///
/// Examples (NFC inputs):
/// - `"pe\u{030d}h"` (5 bytes) → `"peh"`, map `[0, 1, 4, 5]` — the
///   dropped `\u{030d}` (2 bytes) is folded into the preceding `e`'s
///   raw_end.
/// - `"so\u{0358}"` (4 bytes) → `"soo"`, map `[0, 1, 4, 4]` — the
///   `o\u{0358}→oo` substitution emits two ASCII bytes for the original
///   non-ASCII pair, with EVERY new byte anchored at raw_end 4 so a
///   partial-prefix syllabifier hit (`so` toneless) still consumes the
///   whole `o\u{0358}` source spelling. Without this fold a `tl:so`
///   candidate against the live `dictionary.csv` `so` / `soo` sibling
///   pair would commit leaving `\u{0358}` dangling in the pending
///   buffer (Codex post-impl P1 2026-05-15).
/// - `"pe\u{207f}"` (5 bytes) → `"penn"`, map `[0, 1, 2, 5, 5]` — the
///   `\u{207f}→nn` substitution starts AFTER the `pe`, so the
///   atomic-fold rule only touches map indices strictly inside the
///   substitution span (`map[3]` and `map[4]`). The byte BEFORE the
///   substitution (`map[2] = 2`) is left alone — a syllabifier match
///   ending exactly at the substitution boundary (e.g. `pe` toneless)
///   legitimately consumes only `pe` raw bytes and leaves `\u{207f}`
///   pending; the user's deliberate tap on the shorter candidate
///   opted into that.
pub(crate) fn canonicalize_poj_shadow(input: &str, mode: InputMode) -> (String, Vec<usize>) {
    if input.is_ascii() {
        // Identity offset map: Phase 1's NFD walk is a no-op for ASCII,
        // so every canonical byte maps straight back to its own raw
        // offset regardless of which branch we take below.
        let map: Vec<usize> = (0..=input.len()).collect();
        if matches!(mode, InputMode::Poj) {
            // v3.5.9 B-2 — POJ mode: apply the glyph-only POJ rule
            // subset. All 3 rules are non-ASCII → ASCII substitutions
            // that are no-ops on already-ASCII input, so the ASCII
            // POJ path is now **identity** — `chiah` stays `chiah`,
            // `toui` stays `toui` (PR #309 Codex P1 `r3276402303`:
            // applying the legacy `ou→oo` alias whole-buffer mis-fired
            // on multi-syllable hyphenless typing like `toui` for
            // POJ `tó-uī`). Dirty-row `ou` protection moves to the
            // per-syllable build pipeline where it is structurally
            // safe (one syllable per application).
            return apply_normalize_with_offsets(
                input.to_owned(),
                map,
                phonetics::NORMALIZE_TO_POJ_GLYPH_RULES,
            );
        }
        // TL / non-POJ: F3C identity fast-path (protects `toui`).
        return (input.to_owned(), map);
    }

    // Phase 1: NFD walk per original char so we can pair every NFD scalar
    // with the byte range of the original char it came from.
    let mut intermediate = String::with_capacity(input.len());
    let mut map: Vec<usize> = Vec::with_capacity(input.len() + 1);
    map.push(0);
    let mut nfd_buf = [0u8; 4];

    for (raw_idx, ch) in input.char_indices() {
        let raw_end_after = raw_idx + ch.len_utf8();
        for nfd_ch in ch.nfd() {
            // `\u{0358}` (POJ `o\u{0358}` dot) is not a tone mark and must
            // survive Phase 1 so the Phase 2 `o\u{0358}→oo` rule can fire.
            if phonetics::is_combining_tone_mark(nfd_ch) {
                // Drop: absorb the dropped scalar's raw bytes into the
                // preceding emitted byte's raw_end so platform commit
                // does not leave a dangling combining mark in the
                // pending buffer. Guard against a leading standalone
                // combining mark (map has only the baseline `0` entry,
                // no emitted byte yet to absorb into): leave the
                // baseline at `0` per the documented contract; the
                // next emitted char's `raw_end_after` will already
                // account for the dropped mark's byte width via its
                // own `raw_idx + len_utf8()` (Codex post-impl P3
                // 2026-05-15).
                if map.len() > 1 {
                    if let Some(last) = map.last_mut() {
                        *last = raw_end_after;
                    }
                }
                continue;
            }
            let nfd_str = nfd_ch.encode_utf8(&mut nfd_buf);
            intermediate.push_str(nfd_str);
            for _ in 0..nfd_ch.len_utf8() {
                map.push(raw_end_after);
            }
        }
    }

    // Lowercase the ASCII letters that survived the NFD walk so the
    // Phase 2 substitutions actually match. Non-ASCII bytes left over
    // (`\u{0358}`, `\u{207f}`, `\u{1d3a}`) are unaffected by
    // `to_ascii_lowercase` and get replaced into ASCII by Phase 2 below.
    let intermediate_lower = intermediate.to_ascii_lowercase();

    // v3.5.9 B-2 (PR #309 Codex P1 `r3276402303` refinement) — mode
    // selects Phase 2 rule list. POJ mode runs the glyph-only POJ
    // subset (no `ou→oo` alias, no `ch→ts` chain) so non-ASCII POJ
    // input like `pe\u{030d}h` / `chia\u{030d}h` / `so\u{0358}` lands
    // as POJ ASCII (`peh` / `chiah` / `soo`). The `ou→oo` alias is
    // dropped here too because the same hyphenless multi-syllable
    // failure mode applies to non-ASCII input (e.g. `t\u{f3}u\u{12b}`
    // typed without a hyphen would have folded `toui → tooi`). TL /
    // TPS / English keep the pre-B-2 chain so dictionary hits routed
    // through the TL family stay byte-identical.
    // POJ keeps POJ shape (glyph-only); TL / English / TPS take input
    // literally — encoding-only normalization, NO POJ→TL spelling fold — so a
    // valid TL special final `eng` [ɛŋ] is not collapsed to `ing` [iŋ] and a
    // POJ-spelled syllable typed in TL mode (`teng`/`goa`/`chiah`) is not
    // auto-corrected into a `tl:` family hit. English non-ASCII stays literal
    // (`hello` not reinterpreted as Taigi); TPS Bopomofo never matches these
    // Latin rules.
    let rules = if matches!(mode, InputMode::Poj) {
        phonetics::NORMALIZE_TO_POJ_GLYPH_RULES
    } else {
        phonetics::TL_ENCODING_RULES
    };
    apply_normalize_with_offsets(intermediate_lower, map, rules)
}

/// Apply an ordered list of `(find, replace)` rules with offset-map
/// maintenance, returning the mutated string + updated map. v3.5.9 B-2
/// generalization of the pre-B-2 `apply_normalize_to_tl_with_offsets`:
/// the caller now passes the rule list. The two production rule lists
/// reaching this entry are [`phonetics::TL_ENCODING_RULES`]
/// (TL / English / TPS mode — encoding-only, no POJ→TL spelling fold) and
/// [`phonetics::NORMALIZE_TO_POJ_GLYPH_RULES`]
/// (POJ mode — the glyph-only subset of `NORMALIZE_TO_POJ_RULES` without
/// the `ou→oo` alias, which would mis-fire across syllable boundaries
/// at whole-buffer scope; see B-2 PR #309 Codex P1 `r3276402303`). This
/// makes `canonicalize_poj_shadow` mode-aware without duplicating the
/// offset-map maintenance loop. Same in-order iteration + same patterns
/// as the rule lists in `phonetics::syllable`; non-shrinking rules
/// leave the offset map invariant, shrinking rules drain the dropped
/// trailing byte's map entry instead of producing a new `String`.
fn apply_normalize_with_offsets(
    s: String,
    map: Vec<usize>,
    rules: &[(&str, &str)],
) -> (String, Vec<usize>) {
    let mut s = s;
    let mut map = map;
    for (find, repl) in rules {
        offset_aware_replace(&mut s, &mut map, find, repl);
    }
    (s, map)
}

/// Walk `s` left-to-right, replacing every occurrence of `find` with
/// `repl`, and update `map` so each post-replacement byte still points
/// at the correct original-input `raw_end`. Used by
/// [`apply_normalize_with_offsets`].
///
/// Semantics:
/// - Equal-length replacements (`ch→ts`, `oa→ua`, ...) leave `map`
///   untouched at every byte position because the replaced bytes
///   inherit the same `raw_end` slots.
/// - Shrinking replacements (`o\u{0358}→oo`, `\u{207f}→nn`, `oonn→onn`)
///   drain `map[pos + repl_len .. pos + find_len]` so the new last
///   byte of the replacement inherits the original `find`'s trailing
///   `raw_end` — i.e. the commit consumes everything `find` covered.
/// - Right-to-left replace order keeps already-computed positions
///   stable while we mutate `s` / `map`.
///
/// Caller must ensure `find` is non-empty and `repl.len() <=
/// find.len()` (asserted in debug builds — Codex pre-impl scope guard
/// 2026-05-15: we only need shrinking here; an expanding rule would
/// require allocating new map entries and is YAGNI).
fn offset_aware_replace(s: &mut String, map: &mut Vec<usize>, find: &str, repl: &str) {
    debug_assert!(!find.is_empty(), "offset_aware_replace: empty find pattern");
    debug_assert!(
        repl.len() <= find.len(),
        "offset_aware_replace: expanding replacement {find:?}→{repl:?} not supported"
    );
    let find_len = find.len();
    let repl_len = repl.len();
    if find_len == 0 || !s.contains(find) {
        return;
    }
    // Collect all match start byte positions left-to-right without
    // overlap (mirrors `str::replace`).
    let mut positions: Vec<usize> = Vec::new();
    let mut start = 0;
    while let Some(pos) = s[start..].find(find) {
        let abs = start + pos;
        positions.push(abs);
        start = abs + find_len;
    }
    for &pos in positions.iter().rev() {
        s.replace_range(pos..pos + find_len, repl);
        if find_len != repl_len {
            // `map[k]` = raw_end AFTER canonical byte k-1, so map has
            // length canonical.len() + 1. To preserve the load-bearing
            // contract that no syllabifier match ever leaves an
            // upstream-substituted source codepoint dangling in the
            // pending buffer (Codex post-impl P1 2026-05-15, against
            // `dictionary/output/dictionary.csv` `so` + `soo` siblings):
            //   1. Drain the trailing `(find_len - repl_len)` interior
            //      map entries inside the matched range — these are
            //      the bytes the substitution dropped.
            //   2. Force every surviving interior entry inside the
            //      match span (`map[pos + 1 .. pos + repl_len + 1]`)
            //      to the original `raw_end_of_match`. The replacement
            //      now represents the FULL match atomically, so any
            //      partial-prefix syllable candidate that lands at a
            //      shadow_end inside the substituted span still
            //      consumes every raw byte of the source spelling.
            let raw_end_of_match = map[pos + find_len];
            map.drain(pos + repl_len..pos + find_len);
            for slot in map.iter_mut().take(pos + repl_len + 1).skip(pos + 1) {
                *slot = raw_end_of_match;
            }
        }
    }
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
/// matching `mode` via [`mode_key_prefix`] so the byte-range scan in
/// [`crate::continuous::fetch_via_lexicon_partial_inner`] hits the right
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
    // whole buffer is one span; [`whole_buffer_tone_pin`] carries the pin.
    let key = whole_buffer_span_key(raw, mode)?.key;
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
    shaped.then(|| {
        format!(
            "{}{}:{}",
            mode_key_prefix(mode),
            lexicon::key_normalizer::ABBREV_FAMILY_SUFFIX,
            raw.to_ascii_lowercase()
        )
    })
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure shadow pipeline. Dispatch-level integration
    //! (decode round-trip, degraded paths) lives in
    //! `engine/composing/tests/dispatch_continuous.rs`; the byte-exact
    //! cross-slice golden lives in `engine/composing/tests/golden_fetch_at_pos.rs`.

    // `span_key` barrier conventions (E4.2). Shadow `ㄎㄛㆻㄫㄉㄞ` is six
    // 3-byte Bopomofo scalars (char starts 0/3/6/9/12/15, len 18); the
    // `tps` family prefix shifts key offsets by `"tps".len() + 1 = 4`.
    //
    // trace (non-zero start): span 6..18 = `ㆻㄫㄉㄞ`, global barrier 12 →
    // local 6 → body[..6] = `ㆻㄫ`, last glyph starts at 3 → key offset
    // 4 + 3 = 7; end 18 is not a barrier → not pinned.
    #[test]
    fn span_key_shifts_barriers_into_span_coordinates() {
        let k = span_key("ㄎㄛㆻㄫㄉㄞ", 6, 18, InputMode::Tps, &[12], &[]).expect("body");
        assert_eq!(k.key, "tps:ㆻㄫㄉㄞ");
        assert_eq!(k.final_only, vec![7]);
        assert_eq!(k.tone_pin, TonePin::None);
    }

    // trace (barrier at start): span 12..18 = `ㄉㄞ`; barrier 12 is the
    // boundary the span opens on → excluded from final-only; barrier 18
    // → local 6 → body[..6] = `ㄉㄞ`, last glyph at 3 → offset 7, and
    // the span closes on it with no tone mark → pinned.
    #[test]
    fn span_key_excludes_barrier_at_start_and_pins_barrier_at_end() {
        let k = span_key("ㄎㄛㆻㄫㄉㄞ", 12, 18, InputMode::Tps, &[12, 18], &[]).expect("body");
        assert_eq!(k.key, "tps:ㄉㄞ");
        assert_eq!(k.final_only, vec![7]);
        assert_eq!(k.tone_pin, TonePin::TpsSpaceEnd("ㄉㄞ".to_owned()));

        let opens_on_barrier =
            span_key("ㄎㄛㆻㄫㄉㄞ", 12, 18, InputMode::Tps, &[12], &[]).expect("body");
        assert!(opens_on_barrier.final_only.is_empty());
        assert_eq!(opens_on_barrier.tone_pin, TonePin::None);
    }

    // trace (cross-barrier): span 0..18 runs THROUGH barrier 9 → the
    // glyph before it (`ㆻ`, starts at 6) is Final-only at key offset
    // 4 + 6 = 10, but the span does not END on the barrier → not pinned.
    // The span that stops exactly at 9 carries the same offset and IS
    // pinned (global `end` vs global barrier).
    #[test]
    fn span_key_cross_barrier_restricts_but_does_not_pin() {
        let through = span_key("ㄎㄛㆻㄫㄉㄞ", 0, 18, InputMode::Tps, &[9], &[]).expect("body");
        assert_eq!(through.key, "tps:ㄎㄛㆻㄫㄉㄞ");
        assert_eq!(through.final_only, vec![10]);
        assert_eq!(through.tone_pin, TonePin::None);

        let stops = span_key("ㄎㄛㆻㄫㄉㄞ", 0, 9, InputMode::Tps, &[9], &[]).expect("body");
        assert_eq!(stops.key, "tps:ㄎㄛㆻ");
        assert_eq!(stops.final_only, vec![10]);
        assert_eq!(stops.tone_pin, TonePin::TpsSpaceEnd("ㄎㄛㆻ".to_owned()));
    }

    // A3 (§41) — space-pin predicates. `ㄒㄧ` is two 3-byte Bopomofo
    // scalars, so its shadow end is `"ㄒㄧ".len()`; the barrier the TPS
    // separator strip leaves for a trailing space sits at exactly that
    // offset.
    #[test]
    fn span_end_pins_when_barrier_lands_on_an_unmarked_tail() {
        let span = "ㄒㄧ";
        assert!(span_end_pins_unmarked_tone(
            span,
            span.len(),
            InputMode::Tps,
            &[span.len()],
        ));
    }

    #[test]
    fn span_end_does_not_pin_without_a_barrier_at_its_end() {
        let span = "ㄒㄧ";
        // No barrier at all, and a barrier strictly INSIDE the span: an
        // interior boundary is a plain syllable split (§31 台機 / §35 毋是),
        // never a tone instruction.
        assert!(!span_end_pins_unmarked_tone(
            span,
            span.len(),
            InputMode::Tps,
            &[]
        ));
        assert!(!span_end_pins_unmarked_tone(
            span,
            span.len(),
            InputMode::Tps,
            &["ㄒ".len()],
        ));
    }

    #[test]
    fn span_end_does_not_pin_a_tone_marked_tail() {
        // A marked tail took the verbatim toned key, which already filters
        // by tone — pinning it as unmarked would empty the strip.
        let span = "ㄒㄧˋ";
        assert!(!span_end_pins_unmarked_tone(
            span,
            span.len(),
            InputMode::Tps,
            &[span.len()],
        ));
    }

    #[test]
    fn span_end_pin_is_tps_only() {
        let span = "tai";
        for mode in [InputMode::Tl, InputMode::Poj, InputMode::English] {
            assert!(!span_end_pins_unmarked_tone(
                span,
                span.len(),
                mode,
                &[span.len()]
            ));
        }
    }

    #[test]
    fn whole_buffer_pin_strips_the_trailing_space() {
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ ", InputMode::Tps),
            TonePin::TpsSpaceEnd("ㄒㄧ".to_owned())
        );
        // Repeated trailing spaces are one pin, not several.
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ  ", InputMode::Tps),
            TonePin::TpsSpaceEnd("ㄒㄧ".to_owned())
        );
        // Interior spaces are boundaries; the body is the fused surface.
        assert_eq!(
            whole_buffer_tone_pin("ㄍㄠ ㄉㄞ ", InputMode::Tps),
            TonePin::TpsSpaceEnd("ㄍㄠㄉㄞ".to_owned())
        );
    }

    #[test]
    fn whole_buffer_pin_is_none_without_a_trailing_space() {
        assert_eq!(whole_buffer_tone_pin("ㄒㄧ", InputMode::Tps), TonePin::None);
        // An interior space alone is a boundary, not a pin.
        assert_eq!(
            whole_buffer_tone_pin("ㄍㄠ ㄉㄞ", InputMode::Tps),
            TonePin::None
        );
    }

    #[test]
    fn whole_buffer_pin_is_none_for_a_marked_tail() {
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧˋ ", InputMode::Tps),
            TonePin::None
        );
    }

    #[test]
    fn whole_buffer_pin_is_none_for_toneless_or_english_buffers() {
        for mode in [InputMode::Tl, InputMode::Poj, InputMode::English] {
            assert_eq!(whole_buffer_tone_pin("tai ", mode), TonePin::None);
            assert_eq!(whole_buffer_tone_pin("taigi", mode), TonePin::None);
        }
        // English digits are not tones.
        assert_eq!(
            whole_buffer_tone_pin("tai5", InputMode::English),
            TonePin::None
        );
    }

    #[test]
    fn whole_buffer_pin_ignores_a_typed_hyphen_under_tps() {
        // A TPS buffer closed by `-` is not closed by the keyboard space:
        // the §41 pin stays off (Codex post-impl 2026-09-22 P2).
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ-", InputMode::Tps),
            TonePin::None
        );
        assert_eq!(
            whole_buffer_tone_pin("ㄒㄧ ", InputMode::Tps),
            TonePin::TpsSpaceEnd("ㄒㄧ".to_owned())
        );
    }

    #[test]
    fn whole_buffer_pin_is_none_for_a_space_only_buffer() {
        assert_eq!(whole_buffer_tone_pin(" ", InputMode::Tps), TonePin::None);
        assert_eq!(whole_buffer_tone_pin("", InputMode::Tps), TonePin::None);
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
            whole_buffer_tone_pin("teng5-sek", InputMode::Poj),
            TonePin::TypedTones {
                mode: InputMode::Poj,
                typed: "teng5sek".to_owned(),
                boundaries: vec![TypedBoundary {
                    at: 5,
                    khinsiann: false,
                }],
            }
        );
        assert_eq!(
            whole_buffer_tone_pin("Teng5sek4", InputMode::Tl),
            typed_tones(InputMode::Tl, "teng5sek4")
        );
        // Digit-only buffer has no body → no pin.
        assert_eq!(whole_buffer_tone_pin("5", InputMode::Tl), TonePin::None);
    }

    fn typed_tones(mode: InputMode, typed: &str) -> TonePin {
        TonePin::TypedTones {
            mode,
            typed: typed.to_owned(),
            boundaries: Vec::new(),
        }
    }

    #[test]
    fn span_key_pins_typed_tones_only_for_a_digit_bearing_tl_poj_span() {
        // Partial: toneless lookup body + typed-tone pin.
        let k = span_key("teng5sek", 0, 8, InputMode::Poj, &[], &[]).expect("body");
        assert_eq!(k.key, "poj:tengsek");
        assert_eq!(k.tone_pin, typed_tones(InputMode::Poj, "teng5sek"));
        // Fully toned: verbatim toned key AND the pin.
        let k = span_key("teng5sek4", 0, 9, InputMode::Tl, &[], &[]).expect("body");
        assert_eq!(k.key, "tl:teng5sek4");
        assert_eq!(k.tone_pin, typed_tones(InputMode::Tl, "teng5sek4"));
        // Toneless: unpinned (the all-tones affordance).
        let k = span_key("tengsek", 0, 7, InputMode::Poj, &[], &[]).expect("body");
        assert_eq!(k.tone_pin, TonePin::None);
        // English: digits are not tones.
        let k = span_key("teng5sek", 0, 8, InputMode::English, &[], &[]).expect("body");
        assert_eq!(k.key, "tl:tengsek");
        assert_eq!(k.tone_pin, TonePin::None);
        // Interior sub-span of a longer shadow pins its own slice only.
        let k = span_key("teng5sek", 5, 8, InputMode::Poj, &[], &[]).expect("body");
        assert_eq!(k.key, "poj:sek");
        assert_eq!(k.tone_pin, TonePin::None);
    }

    use super::*;

    // Barrier metadata — the §35 contract's raw material. The pipeline
    // strips the TPS space / hyphen but records where they sat, and the
    // ambiguity-aware lookup uses those offsets for the Final-only
    // restriction (direction tests live in lexicon/tests/tps_readings.rs).
    #[test]
    fn barriers_are_recorded_where_separators_were_stripped() {
        let ShadowLattice {
            shadow, barriers, ..
        } = {
            // No inventory needed for the strip half — build a tiny one.
            let inv = test_inventory(&["tps:ㄎㄛ"]);
            build_shadow_lattice_with_barriers("ㄎㄛㆻ ㄫ", &inv, InputMode::Tps)
        };
        assert_eq!(shadow, "ㄎㄛㆻㄫ");
        assert_eq!(barriers, vec![9], "barrier at the stripped-space offset");
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

    // §41 — a trailing separator marker is consumed by the full span, so a
    // whole-buffer commit leaves nothing pending. Interior separators were
    // already consumed via the next glyph's map entry; this pins the tail
    // case the reported bug exposed.
    #[test]
    fn trailing_separator_is_consumed_by_the_full_span() {
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let raw = "ㄒㄧ "; // 3 + 3 + 1 bytes
        assert_eq!(raw.len(), 7, "raw byte length precondition");
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            barriers,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄒㄧ");
        assert_eq!(barriers, vec![6], "barrier where the space was stripped");
        assert_eq!(
            shadow_to_raw_end[shadow.len()],
            raw.len(),
            "full-span end must reach raw len so the commit eats the marker"
        );
    }

    #[test]
    fn repeated_trailing_separators_are_all_consumed() {
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let raw = "ㄒㄧ  ";
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄒㄧ");
        assert_eq!(shadow_to_raw_end[shadow.len()], raw.len());
    }

    #[test]
    fn interior_separator_keeps_its_shorter_span_end_unchanged() {
        // The tail fix must not move a span that ends BEFORE the separator:
        // `ㄍㄠ` in `ㄍㄠ␣ㄉㄞ` still ends at raw 6, leaving the space pending
        // for the rest of the phrase (existing S18 contract).
        let inv = test_inventory(&["tps:ㄍㄠ", "tps:ㄉㄞ"]);
        let raw = "ㄍㄠ ㄉㄞ";
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄍㄠㄉㄞ");
        assert_eq!(shadow_to_raw_end[6], 6, "first-syllable end is unmoved");
        assert_eq!(shadow_to_raw_end[shadow.len()], raw.len());
    }

    // Mixed tails the endpoint rule must NOT consume: the moment a hyphen
    // appears in the tail the hyphen contract wins, whichever order the two
    // separators came in. Codex post-impl 2026-08-21 asked for these
    // explicitly — the comment claimed the `␠-` / `-␠` shapes were covered
    // when only the bare trailing hyphen was.
    #[test]
    fn mixed_separator_tails_keep_the_hyphen_contract() {
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        for raw in ["ㄒㄧ -", "ㄒㄧ- ", "ㄒㄧ - "] {
            let ShadowLattice {
                shadow,
                shadow_to_raw_end,
                ..
            } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
            assert_eq!(shadow, "ㄒㄧ", "{raw}");
            assert_eq!(
                shadow_to_raw_end[shadow.len()],
                6,
                "{raw}: a hyphen in the tail keeps the span short"
            );
        }
    }

    #[test]
    fn separator_only_and_empty_buffers_are_safe() {
        // Degenerate shapes must not panic or invent coverage: an empty
        // buffer keeps the baseline map, an all-separator buffer produces an
        // empty shadow (no lattice edge, so no candidate can claim the span).
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers("", &inv, InputMode::Tps);
        assert_eq!(shadow, "");
        assert_eq!(shadow_to_raw_end, vec![0]);

        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            barriers,
            ..
        } = build_shadow_lattice_with_barriers("  ", &inv, InputMode::Tps);
        assert_eq!(shadow, "");
        assert_eq!(barriers, vec![0], "one barrier at the collapsed offset");
        assert_eq!(shadow_to_raw_end[0], 2, "the whole buffer is consumable");
    }

    #[test]
    fn trailing_hyphen_is_still_not_consumed() {
        // The hyphen contract is untouched: a trailing hyphen stays pending
        // (the tail rule accepts ASCII spaces only).
        let inv = test_inventory(&["tps:ㄒㄧ"]);
        let raw = "ㄒㄧ-";
        let ShadowLattice {
            shadow,
            shadow_to_raw_end,
            ..
        } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
        assert_eq!(shadow, "ㄒㄧ");
        assert_eq!(
            shadow_to_raw_end[shadow.len()],
            6,
            "trailing hyphen must stay pending"
        );
    }

    #[test]
    fn mixed_separator_orders_project_to_the_same_barrier() {
        // `A␠-B` and `A-␠B` must both yield ONE barrier right after A —
        // a first-index-≥ projection put the `A␠-B` hyphen barrier after
        // B's first glyph (a phantom barrier inside the next syllable).
        let inv = test_inventory(&["tps:ㄎㄛ"]);
        for raw in ["ㄎㄛ -ㄫ", "ㄎㄛ- ㄫ"] {
            let ShadowLattice {
                shadow, barriers, ..
            } = build_shadow_lattice_with_barriers(raw, &inv, InputMode::Tps);
            assert_eq!(shadow, "ㄎㄛㄫ", "{raw}");
            assert_eq!(barriers, vec![6], "{raw}: one barrier after ㄎㄛ");
        }
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

    fn test_inventory(keys: &[&str]) -> SyllableInventory {
        use fst::SetBuilder;
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("taigi_shadow_inv_{}_{n}.fst", std::process::id()));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut sorted: Vec<&str> = keys.to_vec();
        sorted.sort_unstable();
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in sorted {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
        SyllableInventory::open(&path).expect("open inventory")
    }

    #[test]
    fn strip_ascii_tone_digits_drops_all_ascii_digits() {
        // Equivalent to digit half of `notone.py::remove_tone()` regex
        // `[\d\-]` under the canonical-ASCII TL contract.
        assert_eq!(strip_ascii_tone_digits("tsua"), "tsua");
        assert_eq!(strip_ascii_tone_digits("tsua7"), "tsua");
        assert_eq!(strip_ascii_tone_digits("tai1bak4"), "taibak");
        // '0' is not a tone marker per phonetics::syllable.rs:18-20 but
        // notone.py drops every ASCII digit; mirror that here.
        assert_eq!(strip_ascii_tone_digits("a0b"), "ab");
        // Hyphen NOT stripped at this layer — `build_hyphen_shadow`
        // (Phase 9 Item 8) handles the `[\d\-]` regex's hyphen half
        // upstream, so by the time a slice reaches this fn it is
        // already hyphenless. The literal-passthrough assertion stays
        // as a behavioural pin so a refactor cannot quietly fold the
        // hyphen strip into both layers.
        assert_eq!(strip_ascii_tone_digits("tai-bak"), "tai-bak");
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

    // ----- v3.5.8 Phase 9 Item 8 — hyphen-shadow contract pins -----

    #[test]
    fn build_hyphen_shadow_no_hyphen_is_identity() {
        let (shadow, map) = build_hyphen_shadow("taibak");
        assert_eq!(shadow, "taibak");
        // No hyphens means every shadow byte maps to its own raw position
        // — pinning this protects existing hyphenless `consumed_span` values
        // from any future drift.
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn build_hyphen_shadow_internal_hyphen_collapses() {
        // `tai-bak` collapses to `taibak`; shadow byte 3 (`b`) maps to
        // raw end 5 because raw byte 4 was the consumed `-`.
        let (shadow, map) = build_hyphen_shadow("tai-bak");
        assert_eq!(shadow, "taibak");
        assert_eq!(map, vec![0, 1, 2, 3, 5, 6, 7]);
    }

    #[test]
    fn build_hyphen_shadow_leading_hyphen_consumes_into_prefix() {
        // Leading `-` is consumed into the prefix: shadow byte 0 (`t`)
        // maps to raw end 2 so commits at any shadow ending cover the
        // leading hyphen.
        let (shadow, map) = build_hyphen_shadow("-tai");
        assert_eq!(shadow, "tai");
        assert_eq!(map, vec![0, 2, 3, 4]);
    }

    #[test]
    fn build_hyphen_shadow_trailing_hyphen_is_not_consumed() {
        // Trailing `-` stays in pending: shadow ending at len() maps to
        // raw byte AFTER the last non-hyphen char, never to raw_len.
        let (shadow, map) = build_hyphen_shadow("tai-");
        assert_eq!(shadow, "tai");
        // map.last() = raw end after the final `i`, NOT raw_len.
        assert_eq!(map, vec![0, 1, 2, 3]);
    }

    #[test]
    fn build_hyphen_shadow_double_hyphen_collapses() {
        // `goa--si` → `goasi`; shadow byte 3 (`s`) maps to raw end 6
        // because raw bytes 3 & 4 were the consumed `--` pair.
        let (shadow, map) = build_hyphen_shadow("goa--si");
        assert_eq!(shadow, "goasi");
        assert_eq!(map, vec![0, 1, 2, 3, 6, 7]);
    }

    #[test]
    fn build_hyphen_shadow_all_hyphens_yields_empty_shadow() {
        let (shadow, map) = build_hyphen_shadow("---");
        assert_eq!(shadow, "");
        // Only the baseline entry survives (`map[0] = 0`).
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn build_hyphen_shadow_empty_input_yields_single_baseline() {
        let (shadow, map) = build_hyphen_shadow("");
        assert_eq!(shadow, "");
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn build_hyphen_shadow_trailing_hyphen_after_internal_hyphen_excluded() {
        // `tai-bak-`: the internal `-` folds into the consumed prefix,
        // but the trailing `-` stays pending — so map.last() = 7 (after
        // `k`), NOT 8 (the raw_len). Catches a regression on the
        // off-by-one risk Codex flagged in pre-impl consult.
        let (shadow, map) = build_hyphen_shadow("tai-bak-");
        assert_eq!(shadow, "taibak");
        assert_eq!(map, vec![0, 1, 2, 3, 5, 6, 7]);
    }

    // ----- TPS syllable-separator space strip (build_separator_shadow) -----

    #[test]
    fn build_separator_shadow_tps_strips_space_and_maps_to_raw() {
        // `ㄍㄠ ㄉㄞ` (kau-tai with the keyboard's tone-1 / boundary
        // space). TPS mode strips the ASCII space so the lattice can
        // build a cross-space phrase edge, but the offset map keeps the
        // full-span end anchored at raw len (13) so commit consumes the
        // space byte. Per-char byte widths: ㄍㄠㄉㄞ each 3 bytes, space 1.
        let raw = "\u{310d}\u{3120} \u{3109}\u{311e}";
        assert_eq!(raw.len(), 13, "raw byte length precondition");
        let (shadow, map) = build_separator_shadow(raw, InputMode::Tps);
        assert_eq!(shadow, "\u{310d}\u{3120}\u{3109}\u{311e}");
        assert_eq!(shadow.len(), 12);
        // map[shadow.len()] == raw.len(): the full-span commit consumes
        // the stripped separator byte.
        assert_eq!(map.len(), 13);
        assert_eq!(
            map[12], 13,
            "full-span end must map to raw len (incl space)"
        );
        // First-syllable end (shadow offset 6 = end of ㄠ) maps to raw 6,
        // right before the space — a first-syllable commit leaves the
        // space pending, which is correct.
        assert_eq!(map[6], 6);
        // The ㄉ that followed the space starts at raw 7 (after the
        // 1-byte space), so shadow offset 9 (end of ㄉ) maps to raw 10.
        assert_eq!(map[9], 10);
    }

    #[test]
    fn build_separator_shadow_non_tps_is_identity() {
        // TL/POJ/English: a space is a real word boundary / literal
        // space and MUST stay a hard segment boundary — identity shadow
        // + identity map, byte-identical to the pre-fix pipeline.
        for mode in [InputMode::Tl, InputMode::Poj, InputMode::English] {
            let (shadow, map) = build_separator_shadow("tai uan", mode);
            assert_eq!(shadow, "tai uan", "{mode:?} must not strip space");
            assert_eq!(
                map,
                (0..=7).collect::<Vec<usize>>(),
                "{mode:?} identity map"
            );
        }
    }

    #[test]
    fn build_separator_shadow_tps_keeps_entering_tone_coda() {
        // Scope guard: only the ASCII space is stripped. A real
        // entering-tone stop coda ㆵ (U+31B5, `kat` = ㄍㄚㆵ) is a
        // Bopomofo body char, NOT a separator — it must survive so a
        // deliberate single-syllable entering-tone word is unaffected.
        let kat = "\u{310d}\u{311a}\u{31b5}";
        let (shadow, map) = build_separator_shadow(kat, InputMode::Tps);
        assert_eq!(shadow, kat, "entering-tone coda must not be stripped");
        // No char stripped → shadow byte-for-byte == raw; the map is the
        // per-char-end map (same convention as build_hyphen_shadow), with
        // map.last() == raw len so a full-span commit consumes everything.
        // trace: ㄍ/ㄚ/ㆵ each 3 bytes → [0,3,3,3,6,6,6,9,9,9].
        assert_eq!(map, vec![0, 3, 3, 3, 6, 6, 6, 9, 9, 9]);
        assert_eq!(map[map.len() - 1], kat.len(), "full span maps to raw len");
    }

    // ----- v3.5.8 Phase 9 Item 9 — canonicalize_poj_shadow contract pins -----

    #[test]
    fn canonicalize_poj_shadow_pure_ascii_is_identity_fast_path() {
        // The F3C gate: pure-ASCII TL input MUST pass through with an
        // identity offset map, otherwise downstream Item 8 contract pins
        // (offset map = vec![0, 1, 2, ...]) regress.
        let (canonical, map) = canonicalize_poj_shadow("tai-bak", InputMode::Tl);
        assert_eq!(canonical, "tai-bak");
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn canonicalize_poj_shadow_empty_ascii_is_identity() {
        let (canonical, map) = canonicalize_poj_shadow("", InputMode::Tl);
        assert_eq!(canonical, "");
        assert_eq!(map, vec![0]);
    }

    #[test]
    fn canonicalize_poj_shadow_combining_tone_mark_drops_and_absorbs() {
        // `pe\u{030d}h` (POJ `pe̍h` for 白): combining tone-8 mark on
        // `e`; canonical drops it and `e`'s raw_end inherits the 2
        // bytes the mark would have consumed.
        let (canonical, map) = canonicalize_poj_shadow("pe\u{030d}h", InputMode::Tl);
        assert_eq!(canonical, "peh");
        // `p` stays at raw_end 1; `e` jumps to 4 (skipping the
        // 2-byte `\u{030d}`); `h` lands at 5.
        assert_eq!(map, vec![0, 1, 4, 5]);
    }

    #[test]
    fn canonicalize_poj_shadow_o_with_dot_above_right_emits_oo() {
        // `so\u{0358}` (POJ `so͘` for 嫂): combining dot-above-right is
        // NOT a tone mark per `phonetics::is_combining_tone_mark`; it survives
        // Phase 1 and Phase 2 collapses `o\u{0358}` → `oo`.
        let (canonical, map) = canonicalize_poj_shadow("so\u{0358}", InputMode::Tl);
        assert_eq!(canonical, "soo");
        // Both new `o` bytes anchor at raw_end 4 (after the full
        // `o\u{0358}` source spelling). Codex post-impl P1: partial-
        // prefix `so` against the live dictionary's `so` / `soo`
        // sibling pair must still consume the full source spelling.
        assert_eq!(map, vec![0, 1, 4, 4]);
    }

    #[test]
    fn canonicalize_poj_shadow_leading_combining_mark_preserves_baseline_zero() {
        // Standalone leading combining mark (no base char to absorb
        // into): Codex post-impl P3 — baseline map[0] = 0 stays, the
        // next emitted char accounts for the dropped mark's bytes via
        // its own raw_idx + len_utf8.
        let (canonical, map) = canonicalize_poj_shadow("\u{030d}h", InputMode::Tl);
        assert_eq!(canonical, "h");
        assert_eq!(map, vec![0, 3]);
    }

    #[test]
    fn canonicalize_poj_shadow_superscript_nasal_marker_emits_nn() {
        // `pe\u{207f}` (POJ `peⁿ`): superscript-n collapses to `nn`.
        let (canonical, map) = canonicalize_poj_shadow("pe\u{207f}", InputMode::Tl);
        assert_eq!(canonical, "penn");
        // `p` → 1, `e` → 2, both new `n` bytes anchor at raw_end 5
        // (after `\u{207f}`).
        assert_eq!(map, vec![0, 1, 2, 5, 5]);
    }

    #[test]
    fn canonicalize_poj_shadow_tl_non_ascii_is_literal_no_spelling_fold() {
        // TL-literal (2026-06-05): non-ASCII `chóa` (POJ glyph for 紙) keeps
        // its literal spelling after the tone-2 acute drop — NO `ch→ts` /
        // `oa→ua` POJ→TL fold. Pre-2026-06-05 this folded to `tsua`.
        let (canonical, _map) = canonicalize_poj_shadow("ch\u{f3}a", InputMode::Tl);
        assert_eq!(canonical, "choa");
    }

    #[test]
    fn canonicalize_poj_shadow_precomposed_uppercase_lowercases_via_phase2() {
        // `\u{00d3}` (`Ó`, precomposed UPPERCASE) → NFD `O\u{0301}` →
        // drop combining → `O` (uppercase) → Phase 2 lowercase pass
        // makes it `o`. This pins the ordering: NFD walk must come
        // BEFORE the lowercase pass, otherwise uppercase precomposed
        // diacritic chars would survive into Phase 2 substitutions.
        // TL-literal (2026-06-05): no `oa→ua` spelling fold, so the result
        // is `oa` — still proving the uppercase `Ó` lowercased to `o`.
        let (out, _map) = canonicalize_poj_shadow("\u{00d3}a", InputMode::Tl);
        assert_eq!(out, "oa", "{out:?}");
    }

    #[test]
    fn canonicalize_poj_shadow_glyph_fold_keeps_oonn_and_drains_offsets() {
        // `o\u{0358}\u{207f}` (`o͘ⁿ`) = `o` (1) + `\u{0358}` (2) +
        // `\u{207f}` (3) = 6 raw bytes. Both glyph rules SHRINK
        // (`o͘`→`oo` is 3→2, `ⁿ`→`nn` is 3→2), so the offset map still
        // exercises the drain path in `offset_aware_replace`.
        //
        // The result stays `oonn`: the nasal fold is NOT applied at
        // whole-buffer scope, where it fires across a syllable seam and
        // destroys real keys (滷卵 `lo͘nng` → `loonng` → `lonng`). The
        // `o͘ⁿ` spelling is served by the alias keys the dictionary build
        // emits (`phonetics::nasal_oo_alias_spelling`), so this shadow can
        // stay literal.
        let (out, map) = canonicalize_poj_shadow("o\u{0358}\u{207f}", InputMode::Tl);
        assert_eq!(out, "oonn", "{out:?}");
        assert_eq!(*map.last().unwrap(), 6, "{map:?}");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_preserves_poj_shape() {
        // v3.5.9 B-2 (PR #309 refinement) — POJ mode applies the
        // glyph-only NORMALIZE_TO_POJ_GLYPH_RULES subset (`o\u{0358}
        // →oo`, `\u{207f}→nn`, `\u{1d3a}→nn`; no `ou→oo` alias
        // whole-buffer). Pure ASCII POJ syllables stay POJ-shaped —
        // they are then looked up against the `poj:` family of the
        // tagged-single-FST.
        let (poj, map) = canonicalize_poj_shadow("chiah", InputMode::Poj);
        assert_eq!(poj, "chiah", "POJ mode preserves POJ ASCII (no ch→ts fold)");
        // ASCII identity map: Phase 1 NFD is identity for ASCII; no
        // POJ rule fires on `chiah` (no `ou`, no non-ASCII chars), so
        // the map is invariant.
        assert_eq!(map, vec![0, 1, 2, 3, 4, 5]);
        // TL mode keeps the F3C identity unchanged from pre-B-2.
        let (tl, _) = canonicalize_poj_shadow("chiah", InputMode::Tl);
        assert_eq!(tl, "chiah");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_no_tl_chain_substitutions() {
        // v3.5.9 B-2 — none of the POJ→TL chain rules (`chh→tsh`,
        // `oa→ua`, `oe→ue`, `eng→ing`, `ek→ik`) fire in POJ mode:
        // NORMALIZE_TO_POJ_RULES is encoding-only. Each input below
        // stays as itself, routes to the `poj:` family.
        for input in ["chhia", "goa", "hoe", "peng", "tek"] {
            let (out, _) = canonicalize_poj_shadow(input, InputMode::Poj);
            assert_eq!(out, input, "POJ `{input}` must stay POJ-shaped");
        }
    }

    #[test]
    fn canonicalize_poj_shadow_poj_ascii_ou_is_boundary_preserving() {
        // v3.5.9 B-2 PR #309 (Codex P1 `r3276402303`) — shadow
        // canonicalize MUST NOT apply the `ou → oo` alias whole-buffer:
        // hyphenless POJ user input like `toui` (intended POJ `tó-uī`,
        // indexed `poj_notone=toui`) would mis-fold to `tooi` and lose
        // the lattice match. Under the glyph-only rule subset, ASCII
        // POJ input is identity — the lattice + syllabifier handle
        // boundaries via the `poj:` family inventory.
        let (toui, toui_map) = canonicalize_poj_shadow("toui", InputMode::Poj);
        assert_eq!(
            toui, "toui",
            "POJ `toui` must stay `toui` (boundary preserved)"
        );
        // ASCII identity → offset map is the identity sequence.
        assert_eq!(toui_map, vec![0, 1, 2, 3, 4]);
        // Same boundary-preservation guarantee for typical dirty
        // single-syllable inputs (`sou`, `kou`): these now stay as
        // themselves (pre-fix they folded). Per-syllable dirty-row
        // protection lives in the build pipeline + per-token matching
        // guard derive — both consume the full NORMALIZE_TO_POJ_RULES
        // and remain safe (one syllable per application).
        let (sou, _) = canonicalize_poj_shadow("sou", InputMode::Poj);
        assert_eq!(sou, "sou");
        let (kou, _) = canonicalize_poj_shadow("kou", InputMode::Poj);
        assert_eq!(kou, "kou");
        // TL mode unchanged — F3C identity fast-path on ASCII.
        let (tl_sou, _) = canonicalize_poj_shadow("sou", InputMode::Tl);
        assert_eq!(tl_sou, "sou", "TL ASCII path stays identity (F3C)");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_non_ascii_keeps_poj_shape() {
        // v3.5.9 B-2 (PR #309 refinement) — non-ASCII POJ-display input
        // under POJ mode runs Phase 1 (NFD + tone-mark drop) then
        // Phase 2 with NORMALIZE_TO_POJ_GLYPH_RULES (glyph encoding
        // only, no `ou→oo` alias whole-buffer). Result preserves POJ
        // shape — `chia\u{030d}h` (POJ `chia̍h` for 食) stays `chiah`,
        // NOT folded to TL `tsiah`.
        let (out, _) = canonicalize_poj_shadow("chia\u{030d}h", InputMode::Poj);
        assert_eq!(
            out, "chiah",
            "POJ mode keeps POJ shape after tone-mark drop"
        );
        // TL mode is literal too (2026-06-05) — no `ch→ts` spelling fold;
        // both modes converge to `chiah` on this glyph-only input.
        let (tl, _) = canonicalize_poj_shadow("chia\u{030d}h", InputMode::Tl);
        assert_eq!(tl, "chiah", "TL literal: no ch→ts spelling fold");
    }

    #[test]
    fn canonicalize_poj_shadow_poj_non_ascii_o_with_dot_above_right() {
        // v3.5.9 B-2 SHOULD #2 (Codex pre-impl) — the shrinking
        // `o\u{0358}→oo` rule fires in BOTH lists; verify the offset
        // map drain stays consistent under POJ mode (commit-span
        // contract): the two emitted `o` bytes both anchor at raw_end
        // 4 so a partial-prefix `so` candidate still consumes the full
        // `o\u{0358}` source spelling on commit.
        let (poj, poj_map) = canonicalize_poj_shadow("so\u{0358}", InputMode::Poj);
        assert_eq!(poj, "soo");
        assert_eq!(poj_map, vec![0, 1, 4, 4]);
        // TL mode produces the same shape (the rule is shared) — pin
        // both to lock cross-mode byte-identity on this commit-span-
        // sensitive shrinking rule.
        let (tl, tl_map) = canonicalize_poj_shadow("so\u{0358}", InputMode::Tl);
        assert_eq!(tl, "soo");
        assert_eq!(tl_map, poj_map);
    }

    #[test]
    fn canonicalize_poj_shadow_poj_non_ascii_superscript_nasal() {
        // v3.5.9 B-2 SHOULD #2 — `\u{207f}→nn` (POJ `ⁿ` → ASCII `nn`)
        // shrinks from 3 bytes to 2. Verify the POJ-mode offset map
        // matches the TL-mode one byte-for-byte (commit-span contract).
        let (poj, poj_map) = canonicalize_poj_shadow("pe\u{207f}", InputMode::Poj);
        assert_eq!(poj, "penn");
        assert_eq!(poj_map, vec![0, 1, 2, 5, 5]);
        let (tl, tl_map) = canonicalize_poj_shadow("pe\u{207f}", InputMode::Tl);
        assert_eq!(tl, "penn");
        assert_eq!(tl_map, poj_map);
    }

    #[test]
    fn canonicalize_poj_shadow_tl_ascii_chiah_stays_identity_f3c_guard() {
        // Regression guard for the F3C gate: the SAME ASCII input in TL
        // mode (`mode = InputMode::Tl`) must NOT be rewritten, so the `tó-uī`
        // (`toui`) class of real TL entries is never garbled.
        let (out, map) = canonicalize_poj_shadow("chiah", InputMode::Tl);
        assert_eq!(out, "chiah", "TL-mode ASCII must stay identity");
        assert_eq!(map, (0..="chiah".len()).collect::<Vec<_>>());
        // `toui` (佗位) must survive — `ou→oo` must NOT fire in TL mode.
        let (toui, _) = canonicalize_poj_shadow("toui", InputMode::Tl);
        assert_eq!(toui, "toui", "F3C: TL-mode `toui` must not become `tooi`");
    }

    #[test]
    fn canonicalize_poj_shadow_non_ascii_both_modes_literal() {
        // 2026-06-05 TL-literal pass — non-ASCII POJ-display input now keeps
        // its literal shape in BOTH modes (POJ glyph-only, TL encoding-only,
        // neither runs the POJ→TL spelling chain). `chóa` (POJ for 紙) →
        // `choa` either way. The pre-B-2 TL `tsua` fold AND the B-2-era
        // mode divergence are both gone.
        let (poj, _) = canonicalize_poj_shadow("ch\u{f3}a", InputMode::Poj);
        let (tl, _) = canonicalize_poj_shadow("ch\u{f3}a", InputMode::Tl);
        assert_eq!(poj, "choa", "POJ keeps POJ ASCII shape");
        assert_eq!(tl, "choa", "TL literal: no ch→ts / oa→ua fold");
    }

    #[test]
    fn combining_tone_mark_predicate_excludes_non_tone_combiners() {
        // `\u{0358}` (combining dot above right) is in the
        // U+0300-U+036F combining block but is NOT a tone mark; it has
        // to survive Phase 1 so Phase 2 `o\u{0358}→oo` can fire.
        assert!(!phonetics::is_combining_tone_mark('\u{0358}'));
        // ASCII letters / digits / hyphens / common Latin diacritics
        // must obviously not be flagged either.
        for c in ['a', '0', '-', '\u{00e2}', '\u{014d}'] {
            assert!(
                !phonetics::is_combining_tone_mark(c),
                "{c:?} must not be a tone mark"
            );
        }
    }

    #[test]
    fn offset_aware_replace_same_length_leaves_map_invariant() {
        let mut s = String::from("choa");
        let mut map = vec![0, 1, 2, 3, 4];
        offset_aware_replace(&mut s, &mut map, "ch", "ts");
        assert_eq!(s, "tsoa");
        assert_eq!(map, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn offset_aware_replace_shrinking_drains_middle_entries() {
        // Synthetic: `xxoonnyy` shrinks `oonn` (4 bytes) → `onn` (3),
        // dropping one map entry from inside the matched range. The
        // entry that survives at the new end position must equal the
        // original map[pos + find_len] (raw_end of the full match).
        let mut s = String::from("xxoonnyy");
        let mut map = vec![0, 10, 20, 30, 40, 50, 60, 70, 80];
        offset_aware_replace(&mut s, &mut map, "oonn", "onn");
        assert_eq!(s, "xxonnyy");
        // Post-replace map length = canonical.len() + 1 = 8.
        // Slot for "after the entire `onn` collapse" (map index 5)
        // must equal the original map[6] = 60 (raw_end of the full
        // `oonn` match).
        assert_eq!(map.len(), 8);
        assert_eq!(map[0], 0);
        assert_eq!(map[2], 20, "byte before `oo` must be unchanged");
        assert_eq!(
            map[5], 60,
            "byte after `onn` must equal raw_end of full match"
        );
        assert_eq!(map[6], 70, "tail must shift left by one slot");
    }

    #[test]
    fn offset_aware_replace_skips_when_pattern_absent() {
        let mut s = String::from("xyz");
        let mut map = vec![0, 1, 2, 3];
        offset_aware_replace(&mut s, &mut map, "ab", "cd");
        assert_eq!(s, "xyz");
        assert_eq!(map, vec![0, 1, 2, 3]);
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

    // Hermetic `SyllableInventory` builder — same inline pattern as the
    // `lattice::builder` unit tests (inline duplication preferred over a
    // shared test-utils crate). Pins the carve-out helper without the
    // `LexiconHandle` singleton (Codex post-impl S5 P3, 2026-05-17).
    fn build_inventory(samples: &[&str]) -> SyllableInventory {
        use std::path::PathBuf;

        use fst::SetBuilder;
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
        keys.sort();
        keys.dedup();

        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf = std::env::temp_dir().join(format!(
            "taigi_shadow_carveout_{}_{n}.fst",
            std::process::id()
        ));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in &keys {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
        SyllableInventory::open(&path).expect("open inventory")
    }

    /// v3.5.9 B-2 — POJ-only inventory builder. Emits `poj:<canonical>`
    /// keys via `phonetics::canonicalize_poj_syllable` (which preserves
    /// POJ ASCII shape, distinct from `canonicalize_syllable`'s TL fold).
    /// Used by the B-2 mode-aware unit tests to prove POJ shadow helpers
    /// route to the `poj:` family of the tagged-single-FST.
    fn build_poj_inventory(samples: &[&str]) -> SyllableInventory {
        use std::path::PathBuf;

        use fst::SetBuilder;
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
        keys.sort();
        keys.dedup();

        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf = std::env::temp_dir().join(format!(
            "taigi_shadow_carveout_poj_{}_{n}.fst",
            std::process::id()
        ));
        let file = std::fs::File::create(&path).expect("create fst");
        let mut builder = SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
        for key in &keys {
            builder.insert(key.as_bytes()).expect("insert");
        }
        builder.finish().expect("finish");
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
        };
        let boundary = |mode, typed: &str, at: Vec<usize>| {
            boundary_kinds(mode, typed, at.into_iter().map(|at| (at, false)).collect())
        };
        // trace: shadow `khiah`, barrier at 3 → span (0,5) carries it.
        let k = span_key("khiah", 0, 5, InputMode::Tl, &[3], &[]).expect("body");
        assert_eq!(k.key, "tl:khiah");
        assert_eq!(k.tone_pin, boundary(InputMode::Tl, "khiah", vec![3]));
        // Digits and a boundary together: offsets count the typed digit.
        let k = span_key("khi3ah", 0, 6, InputMode::Tl, &[4], &[]).expect("body");
        assert_eq!(k.tone_pin, boundary(InputMode::Tl, "khi3ah", vec![4]));
        // POJ reads the same shape.
        let k = span_key("khiah", 0, 5, InputMode::Poj, &[3], &[]).expect("body");
        assert_eq!(k.tone_pin, boundary(InputMode::Poj, "khiah", vec![3]));
        // The barrier at a span's own start is carried as `at: 0` with its
        // kind — read only by a reading that opens with `--` itself.
        let k = span_key("khiah", 3, 5, InputMode::Tl, &[3], &[(3, 2)]).expect("body");
        assert_eq!(
            k.tone_pin,
            boundary_kinds(InputMode::Tl, "ah", vec![(0, true)])
        );
        let k = span_key("khiah", 3, 5, InputMode::Tl, &[3], &[]).expect("body");
        assert_eq!(
            k.tone_pin,
            boundary_kinds(InputMode::Tl, "ah", vec![(0, false)])
        );
        // …and the `--` subset marks an interior boundary's kind.
        let k = span_key("khiah", 0, 5, InputMode::Tl, &[3], &[(3, 2)]).expect("body");
        assert_eq!(
            k.tone_pin,
            boundary_kinds(InputMode::Tl, "khiah", vec![(3, true)])
        );
        // No barrier, no digit → unpinned as before; English never pins.
        assert_eq!(
            span_key("khiah", 0, 5, InputMode::Tl, &[], &[])
                .unwrap()
                .tone_pin,
            TonePin::None
        );
        assert_eq!(
            span_key("khiah", 0, 5, InputMode::English, &[3], &[])
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
