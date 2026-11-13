// Pure shadow pipeline — mode-aware canonicalize (POJ→POJ ASCII under POJ
// mode, POJ→TL ASCII under TL/English mode; v3.5.9 B-2 PR #309), hyphen
// and separator strips with their barriers. The lattice build lives in
// `lattice`; the span / partial-prefix / custom keys and tone pins in `keys`.

use phonetics::InputMode;
use unicode_normalization::UnicodeNormalization;

mod keys;
mod lattice;

pub(crate) use keys::{
    abbrev_query_key, build_continuous_keys, build_partial_prefix_key, custom_toneless_key,
    greedy_longest_syllabification, oov_reading_barriers, span_key, span_min_syllable_count,
    whole_buffer_tone_pin, ContinuousKeys, SpanKey,
};
pub(crate) use lattice::{build_shadow_lattice_with_barriers, ShadowLattice};

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

/// The raw buffer as every lookup reads it: ASCII letters lowercased and the
/// TPS glyph alias folded ([`phonetics::fold_tps_glyph_alias`]). Byte-length
/// preserving, so an offset into the result indexes `raw` unchanged; the
/// display keeps the typed glyph.
pub(crate) fn lookup_form(raw: &str) -> String {
    raw.chars()
        .map(|c| phonetics::fold_tps_glyph_alias(c.to_ascii_lowercase()))
        .collect()
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
///
/// [`fst_body_for_span`]: keys::fst_body_for_span
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
    let lower = lookup_form(raw);
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
///   `phonetics::KeyFamily::Poj.toneless_face`) still consume the full
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

#[cfg(test)]
mod tests {
    //! Unit tests for the pure shadow pipeline. Dispatch-level integration
    //! (decode round-trip, degraded paths) lives in
    //! `engine/composing/tests/dispatch_continuous.rs`; the byte-exact
    //! cross-slice golden lives in `engine/composing/tests/golden_fetch_at_pos.rs`.

    use super::*;

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
}
