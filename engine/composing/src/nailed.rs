//! Nailed segments and the Model B composing-buffer join: the nailed prefix,
//! the learned reading and the combined preedit display.

use lexicon::dictionary_reader::Filter;
use lexicon::{compound_hanji_exists, EngineHandle as LexiconHandle};
use protos::engine::AppConfig;

/// One **nailed** segment inside `Phase::Continuous`. "Nailed" means the
/// user accepted a candidate for this part of the buffer, but — under
/// Model B (`docs/engine/continuous-commit-and-display.md` §10) — it is **NOT**
/// yet written to the host document; it lives inside the active marked /
/// composing region until a hard finalize. `raw_span` records the byte
/// offsets in the original raw input the user typed (start = end of the
/// previous segment, end = start + raw_text.len()). `syllable_count` lets
/// span-local fetch distinguish e.g. `tsua` → 紙(1) vs 珠仔(2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NailedSegment {
    // Formatted for the current display mode (swap / TPS / both-scripts); a hard finalize writes
    // the whole region at once.
    pub display_text: String,
    // Canonical dictionary key (`hanji.unwrap_or(roman)`). The backspace-pop NextWord
    // last-selected fix reads this so association learning stays display-mode independent
    // (v3.5.8 Phase 9 Bug 1, Option A). Equals `display_text` when not swapped.
    pub canonical_text: String,
    pub raw_text: String,
    // v3.6.1 R2 — canonical TL romanization of the committed candidate
    // (the chosen `CandidateMessage.canonical_tl`). The NextWord
    // `WordSelected` / `UpdateLastSelectedWord` `roman` arg is built from
    // this (so the learned `prev_tl` / `next_tl` matches a normal
    // candidate commit), falling back to `raw_text` when empty (a picked
    // candidate with no canonical TL: TPS-OOV, hanji-absent). Kept SEPARATE from `raw_text`,
    // which stays the authority for span / unnail mechanics (Codex
    // pre-impl 2026-06-03 SHOULD).
    pub association_tl: String,
    // Learned phrases (§50) — the chosen `CandidateMessage.hanji`, present
    // only when the platform picked a hanji-bearing candidate (any output
    // script). `None` for a hanji-less pick; the final
    // commit learns the composition only when every nailed segment has one.
    pub hanji: Option<String>,
    pub raw_span: (usize, usize),
    pub syllable_count: u8,
    // Whether the user picked this segment from the list. A Hanji conversion's
    // pick nails the words before it as shown (`roadmap` H4), not picked: such
    // a segment records no usage, keeps the composition from teaching a §50
    // phrase, and is never one end of a next-word pair (B4, H7).
    pub is_picked: bool,
}

impl NailedSegment {
    /// The segment as a context word — `(canonical text, association roman)`,
    /// the identity a final commit's `preceding` carries — or `None` for one
    /// with no word identity: nailed as shown without a canonical TL (glyphs,
    /// a reading the dictionary has no word for).
    pub(crate) fn context_word(&self) -> Option<(String, String)> {
        (self.is_picked || !self.association_tl.is_empty()).then(|| {
            (
                self.canonical_text.clone(),
                crate::transition::association_roman(&self.association_tl, &self.raw_text),
            )
        })
    }
}

/// v3.5.8 — word-boundary separator policy for the Model B continuous
/// composing buffer (`docs/engine/continuous-commit-and-display.md` §10.2
/// segmented-spacing contract). A single ASCII space joins adjacent
/// nailed segments (and the nailed prefix ↔ pending tail) **only when
/// the rendered script is roman-ish**: roman-first, or both-scripts
/// (`hit (彼)`). Hanji-first (`is_hanji_first` without
/// `output_both_scripts`) and TPS render the hanji/bopomofo as-is with
/// no inter-segment space. This mirrors the platform
/// `appendAutoSpaceIfApplicable` predicate so the marked region and the
/// final-commit auto-space stay consistent. `is_hanji_first`
/// alone cannot distinguish hanji-first from both-scripts (both set it
/// `true`) — hence the `output_both_scripts` AppConfig field
/// (Codex pre-impl 2026-05-18).
pub(crate) fn continuous_word_space(config: &AppConfig) -> bool {
    // De Morgan of the platform `appendAutoSpaceIfApplicable` guard
    // `if (effectiveSwapped && !outputBothScripts) return`: roman-ish =
    // not Hanji-first (swap or TPS layout), OR both-scripts is on.
    !config.renders_hanji_first() || config.output_both_scripts
}

/// Pure `Σ text.of(nailed[i])` join (the display, or the learned TL), parameterized by the
/// roman-ish `space` predicate and an `is_compound` oracle. **Single
/// source of truth** for the nailed-prefix concatenation; do not
/// re-inline this loop.
///
/// Inter-segment boundary policy (only when `space`, i.e. roman-ish —
/// hanji-first / TPS have no separator at all):
/// - boundary after a hyphen-continuation segment (`tai-`, `s` ends
///   with `-`): emit **nothing** and skip the compound check for the
///   segment that follows (mirrors the platform
///   `appendAutoSpaceIfApplicable` `endsWith("-")` guard);
/// - else, find the **longest run** `[j, j+n)` (`n >= 2`) starting at
///   the current position such that every member is single-syllable
///   AND `is_compound(Σ canonical_text, Σ association_tl, n)` is true (#772:
///   the same Hanji and reading, from a source the user has on). If found, emit
///   `joiner` (`-`, or the Syllable Separator's `""` / `" "`) between run
///   members; otherwise advance one segment and emit a single space at the
///   boundary.
///
/// Anti-overgluing (overlapping bigrams `AB` + `BC` without trigram
/// `ABC` → `A-B C`, not `A-B-C`) is a natural consequence of
/// leftmost longest-match; the unit tests below pin the matrix.
///
/// A `-` run the user typed before a segment (it folds into that
/// segment's `raw_text`) overrides either boundary's output — the word
/// space and the compound joiner alike — while the compound grouping stays
/// (`tng--lai` → 轉 + 來 → `tńg--lâi`). Full contract:
/// `docs/architecture/behavioral-invariants.md` §51.
///
/// The compound hyphen is **dictionary-informed** and is never stored in
/// `NailedSegment.display_text` — it is derived fresh on every render from
/// the segments' own `canonical_text` + `association_tl` + `syllable_count`
/// and the request's `AppConfig.dictionary_sources`; `raw_text` keeps
/// the user's typed characters as the authority for backspace-pop.
fn nailed_prefix_with_oracle(
    nailed: &[NailedSegment],
    text: SegmentText,
    space: bool,
    syllable_joiner: Option<&str>,
    is_compound: impl Fn(&str, &str, u8) -> bool,
) -> String {
    // Syllable Separator: the engine-synthesised compound joiner is the one
    // hyphen in this render that no user typed and no platform committed, so
    // it is the one the setting owns here. Segment text arrives already
    // rendered.
    let joiner = syllable_joiner.unwrap_or("-");
    let mut s = String::new();
    let mut j = 0;
    while j < nailed.len() {
        let prev_hyphen = s.ends_with('-');
        let run_len = if !space || prev_hyphen {
            1
        } else {
            longest_compound_run(&nailed[j..], text, &is_compound)
        };

        if j > 0 && space && !prev_hyphen {
            push_boundary(&mut s, &nailed[j], text, " ", syllable_joiner);
        }
        for k in 0..run_len {
            if k > 0 {
                push_boundary(&mut s, &nailed[j + k], text, joiner, syllable_joiner);
            }
            s.push_str(text.of(&nailed[j + k]));
        }
        j += run_len;
    }
    s
}

/// The boundary in front of `next` (§51). A rendering that opens with
/// `-` / `·` is a dictionary khinsiann piece or a §34 literal carrying
/// its own separator, so a typed run adds nothing to it — except the Space
/// joiner in front of a rendered `·`, which separates syllables, not marks
/// the neutral tone (`hōo ·guá`).
fn push_boundary(
    s: &mut String,
    next: &NailedSegment,
    text: SegmentText,
    default: &str,
    syllable_joiner: Option<&str>,
) {
    let run = typed_separator_run(&next.raw_text);
    if run.is_empty() {
        s.push_str(default);
        return;
    }
    let rendered = text.of(next);
    if rendered.starts_with('-') {
        return;
    }
    // `syllable_joiner`: `None` = the Hyphen setting (write the run as typed);
    // `Some` = the Space / None joiner (`AppConfig::rendered_syllable_joiner`).
    // A rendered `·guá` piece already carries its dot, so it takes the joiner
    // alone — the half of `phonetics::api::syllable_joiner_display`'s rule
    // that a piece rendered on its own cannot see.
    let opens_with_dot = rendered.starts_with('·');
    match syllable_joiner {
        None if opens_with_dot => {}
        None => s.push_str(run),
        Some(joiner) => {
            let run_len = if opens_with_dot { 0 } else { run.len() };
            phonetics::api::push_syllable_joiner_run(s, run_len, joiner);
        }
    }
}

/// The ASCII `-` run a raw segment opens with (`--lai` → `--`).
pub(crate) fn typed_separator_run(raw_text: &str) -> &str {
    let n = raw_text.bytes().take_while(|&b| b == b'-').count();
    &raw_text[..n]
}

/// Upper bound on the longest-match compound scan. Matches the
/// dictionary builder cap `MAX_SYLLABLES = 4` at
/// `dictionary/build/dictionary_records.py:43` — records with more
/// syllables are dropped at build time, so probing the FST for `n > 4`
/// is wasted work. The cap also keeps the `n as u8` proto cast safely
/// inside `u8` for any nail-history length (the dispatch-layer
/// `clamp_syllable_count` already saturates platform input at `u8`).
const MAX_COMPOUND_RUN: usize = 4;

/// Largest `n` in `2..=MAX_COMPOUND_RUN` such that `segs[..n]` are all
/// single-syllable segments whose `display_text` does NOT end with
/// `-` (a user-typed hyphen continuation — a `tai-` segment carries
/// its own boundary and cannot be folded into an automatic compound
/// run without duplicating the hyphen), AND
/// `is_compound(Σ canonical_text, Σ association_tl, n)` is true — the
/// run's Hanji and its picked reading (a segment without a reading leaves
/// the run's reading short, so it matches no record). Returns `1` when no
/// run qualifies (caller treats it as "advance one segment, normal
/// boundary").
///
/// Builds the full eligible-segment concatenations once, then truncates
/// from the right per iteration — O(max_n) string ops instead of
/// rebuilding each attempt.
fn longest_compound_run<F: Fn(&str, &str, u8) -> bool>(
    segs: &[NailedSegment],
    text: SegmentText,
    is_compound: &F,
) -> usize {
    let max_n = segs
        .iter()
        .take(MAX_COMPOUND_RUN)
        .take_while(|s| s.syllable_count == 1 && !text.of(s).ends_with('-'))
        .count();
    if max_n < 2 {
        return 1;
    }
    let mut hanji = String::new();
    let mut reading = String::new();
    for s in &segs[..max_n] {
        hanji.push_str(&s.canonical_text);
        reading.push_str(&s.association_tl);
    }
    for n in (2..=max_n).rev() {
        if is_compound(&hanji, &reading, n as u8) {
            return n;
        }
        if n > 2 {
            let last = &segs[n - 1];
            hanji.truncate(hanji.len() - last.canonical_text.len());
            reading.truncate(reading.len() - last.association_tl.len());
        }
    }
    1
}

/// `Σ nailed[i].display_text` joined with the §10.2 word-boundary
/// separator (see [`continuous_word_space`]), with a contiguous run of
/// single-syllable segments that reconstructs a **known n-syllable
/// dictionary compound** joined by internal hyphens instead
/// (紅尾冬 → `Âng-bóe-tang`, 查某 → `tsa-bóo`). v3.5.9 extends the
/// v3.5.8 §10.2 Option A bigram-only oracle to longest-match `n >= 2`.
///
/// One lexicon lock for the whole join (Codex pre-impl R2 2026-05-18).
/// Cheap pre-gate first: a compound hyphen can only fire when the
/// render is roman-ish AND there is at least one adjacent
/// single-syllable pair — otherwise the lexicon is never touched. When
/// no dictionary is installed (`with_state` → `Err`, e.g. unit tests)
/// the join degrades to the pure space-join: byte-identical to the
/// pre-Option-A behaviour.
pub(crate) fn nailed_prefix(nailed: &[NailedSegment], config: &AppConfig) -> String {
    let space = continuous_word_space(config);
    nailed_join(
        nailed,
        SegmentText::Display,
        space,
        config.rendered_syllable_joiner(),
        config,
    )
}

/// Learned phrases (§50) — the canonical TL a composition of nailed
/// picks learns: the segments' `association_tl` under the same word
/// boundaries the roman commit renders ([`nailed_prefix_with_oracle`]),
/// whatever the display settings — a dictionary compound run joins with
/// `-`, other words with a space, a typed `-` run overrides. 做 + 進 +
/// 出 + 口 learns `tsò tsìn-tshut-kháu`, never `tsò-tsìn-tshut-kháu`
/// (USER 2026-09-23: 做 and 進出口 are two words). The compound runs come
/// from the same source-filtered oracle as the render (#772), so a phrase
/// learns the separators the preedit showed.
pub(crate) fn learned_reading(nailed: &[NailedSegment], config: &AppConfig) -> String {
    nailed_join(nailed, SegmentText::AssociationTl, true, None, config)
}

/// Which per-segment text a nailed join renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SegmentText {
    /// `display_text` — the marked region and the commit.
    Display,
    /// `association_tl` — the canonical TL a learned phrase stores.
    AssociationTl,
}

impl SegmentText {
    fn of(self, seg: &NailedSegment) -> &str {
        match self {
            Self::Display => &seg.display_text,
            Self::AssociationTl => &seg.association_tl,
        }
    }
}

/// The compound oracle reads the user's dictionary switches from
/// `config.dictionary_sources` (absent = every source, an un-wired sender).
fn nailed_join(
    nailed: &[NailedSegment],
    text: SegmentText,
    space: bool,
    syllable_joiner: Option<&str>,
    config: &AppConfig,
) -> String {
    let eligible = space
        && nailed.len() >= 2
        && nailed
            .windows(2)
            .any(|w| w[0].syllable_count == 1 && w[1].syllable_count == 1);
    if !eligible {
        return nailed_prefix_with_oracle(nailed, text, space, syllable_joiner, |_, _, _| false);
    }
    let filter = Filter::from_enabled_bitmask(crate::requests::source_filter_bitmask(config));
    LexiconHandle::with_state(|state| {
        let (Some(prefix), Some(dict)) = (state.prefix_index.as_ref(), state.dictionary.as_ref())
        else {
            return Ok(nailed_prefix_with_oracle(
                nailed,
                text,
                space,
                syllable_joiner,
                |_, _, _| false,
            ));
        };
        Ok(nailed_prefix_with_oracle(
            nailed,
            text,
            space,
            syllable_joiner,
            |hanji, reading, n| compound_hanji_exists(hanji, reading, n, &filter, prefix, dict),
        ))
    })
    .unwrap_or_else(|_| {
        nailed_prefix_with_oracle(nailed, text, space, syllable_joiner, |_, _, _| false)
    })
}

/// The Model B composing-buffer surface for a `(nailed, raw)` pair:
/// `Σ nailed[i].display_text` followed by the derived display of the
/// pending `raw` tail. **Single source of truth** — both
/// [`Phase::composing_display`] and the `transition.rs` Continuous paths
/// route through this so the rendered preedit and the hard-finalize commit
/// can never diverge (Codex post-impl review point). The one preedit that
/// is not this string is a shown Hanji conversion (`transition::phase_preedit`),
/// which `CommitAsShown` and `CommitPreeditThenInsertExternal` write; the
/// other commits write this one.
///
/// [`Phase::composing_display`]: crate::api::Phase::composing_display
pub(crate) fn combined_display(nailed: &[NailedSegment], raw: &str, config: &AppConfig) -> String {
    combined_display_with_tail(nailed, raw, config).0
}

/// [`combined_display`] plus the byte offset where the pending tail's derived
/// form starts inside it — what a caret inside the tail is projected from.
pub(crate) fn combined_display_with_tail(
    nailed: &[NailedSegment],
    raw: &str,
    config: &AppConfig,
) -> (String, usize) {
    join_nailed_prefix_and_tail(
        nailed,
        &crate::derived::derived_display(raw, config),
        config,
    )
}

/// The nailed prefix followed by `derived`, the pending tail as displayed,
/// plus the byte offset where the tail starts.
pub(crate) fn join_nailed_prefix_and_tail(
    nailed: &[NailedSegment],
    derived: &str,
    config: &AppConfig,
) -> (String, usize) {
    let mut s = nailed_prefix(nailed, config);
    // §10.2 word boundary between the nailed prefix and the pending
    // tail (the tail is the next word). Same predicate + trailing-`-`
    // suppression as the inter-segment join; a tail that opens with a
    // typed `-` run carries its own boundary (`tńg--lai`, not `tńg --lai`).
    if !s.is_empty()
        && !derived.is_empty()
        && continuous_word_space(config)
        && !s.ends_with('-')
        && !derived.starts_with('-')
    {
        s.push(' ');
    }
    let tail_start = s.len();
    s.push_str(derived);
    (s, tail_start)
}

#[cfg(test)]
mod tests {
    //! v3.5.8 §10.2 segmented-spacing contract pins for the Model B
    //! composing-buffer join (`nailed_prefix` / `combined_display`).
    //! Behavioral mid/final-commit coverage lives in
    //! `tests/continuous_phase.rs` + `tests/raw_input_pending_tail.rs`;
    //! these unit-pin the predicate matrix directly.

    use super::{
        combined_display, combined_display_with_tail, nailed_prefix, nailed_prefix_with_oracle,
        AppConfig, NailedSegment, SegmentText,
    };

    fn seg(display: &str) -> NailedSegment {
        NailedSegment {
            display_text: display.to_owned(),
            canonical_text: display.to_owned(),
            raw_text: display.to_owned(),
            association_tl: display.to_owned(),
            hanji: None,
            raw_span: (0, display.len()),
            syllable_count: 1,
            is_picked: true,
        }
    }

    /// Distinct `display_text` vs `canonical_text` (roman-display,
    /// hanji-canonical) + explicit syllable count — the §10.2 Option A
    /// compound-hyphen tests key the oracle off `canonical_text`.
    fn seg_dc(display: &str, canonical: &str, syllable_count: u8) -> NailedSegment {
        NailedSegment {
            display_text: display.to_owned(),
            canonical_text: canonical.to_owned(),
            raw_text: display.to_owned(),
            association_tl: canonical.to_owned(),
            hanji: None,
            raw_span: (0, display.len()),
            syllable_count,
            is_picked: true,
        }
    }

    /// `input_mode` + the two swap flags are the only fields the
    /// separator predicate reads; the rest stay at proto defaults.
    fn cfg(input_mode: &str, swapped: bool, both: bool) -> AppConfig {
        AppConfig {
            input_mode: input_mode.to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_hanji_first: swapped,
            output_both_scripts: both,
            candidate_display_mode: 0,
            syllable_separator: 0,
            force_lowercase_nasal_marker: false,
            tps_or_maps_to_er: false,
            hanji_conversion: None,
            dictionary_sources: None,
            custom_dictionary_disabled: false,
            c_types_affricate: false,
            f_types_hyphen: false,
        }
    }

    #[test]
    fn roman_first_inserts_word_boundary_space_between_segments() {
        let n = [seg("hit"), seg("tui")];
        assert_eq!(nailed_prefix(&n, &cfg("tl", false, false)), "hit tui");
    }

    #[test]
    fn hanji_first_has_no_inter_segment_space() {
        let n = [seg("彼"), seg("隻")];
        // is_hanji_first without output_both_scripts → hanji-first.
        assert_eq!(nailed_prefix(&n, &cfg("tl", true, false)), "彼隻");
    }

    #[test]
    fn both_scripts_is_roman_ish_and_spaced_even_when_swapped() {
        let n = [seg("hit (彼)"), seg("tui (隻)")];
        assert_eq!(
            nailed_prefix(&n, &cfg("tl", true, true)),
            "hit (彼) tui (隻)"
        );
    }

    #[test]
    fn tps_renders_as_is_no_space() {
        let n = [seg("ㄏㄧㆵ"), seg("ㄉㄨㄧ")];
        // input_mode == "tps" → Hanji-first regardless of the stored swap.
        assert_eq!(nailed_prefix(&n, &cfg("tps", false, false)), "ㄏㄧㆵㄉㄨㄧ");
    }

    #[test]
    fn trailing_hyphen_segment_suppresses_the_following_space() {
        // A hyphen-continuation segment (`tai-`) is mid-word; no space
        // after it even in roman-first (mirrors the platform
        // `appendAutoSpaceIfApplicable` `endsWith("-")` rule).
        let n = [seg("tai-"), seg("uan")];
        assert_eq!(nailed_prefix(&n, &cfg("tl", false, false)), "tai-uan");
    }

    #[test]
    fn empty_and_single_segment_have_no_leading_or_trailing_space() {
        assert_eq!(nailed_prefix(&[], &cfg("tl", false, false)), "");
        assert_eq!(
            nailed_prefix(&[seg("hit")], &cfg("tl", false, false)),
            "hit"
        );
    }

    #[test]
    fn combined_display_spaces_nailed_prefix_against_pending_tail() {
        let n = [seg("珠")];
        // derived_display("a", tl) == "a" (verbatim, no tone digit) →
        // roman-first inserts the §10.2 boundary space.
        assert_eq!(combined_display(&n, "a", &cfg("tl", false, false)), "珠 a");
        // hanji-first: no boundary space.
        assert_eq!(combined_display(&n, "a", &cfg("tl", true, false)), "珠a");
        // No pending tail → no dangling separator.
        assert_eq!(combined_display(&n, "", &cfg("tl", false, false)), "珠");
        // No nailed prefix → tail only, no leading separator.
        assert_eq!(combined_display(&[], "a", &cfg("tl", false, false)), "a");
    }

    // ---- §10.2 dictionary-compound hyphen join (longest-match) ----
    // `nailed_prefix_with_oracle` is the pure join; these pin the
    // separator policy against a hermetic compound oracle (the live
    // lexicon-backed path is locked in `engine/lexicon/tests/
    // compound_hanji.rs` + the graceful no-lexicon path by the §10.2
    // predicate-matrix tests above, which now route through this fn).
    // v3.5.9 extended the v3.5.8 §10.2 Option A bigram-only oracle to
    // longest-match `n >= 2`.

    #[test]
    fn oracle_false_everywhere_is_byte_identical_to_plain_space_join() {
        // Regression pin: with no compound ever, the roman-ish join is
        // exactly the pre-Option-A behaviour.
        let n = [seg("hit"), seg("tui")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _, _| false),
            "hit tui"
        );
        // Hanji-first / TPS (space = false) → no separator at all,
        // oracle irrelevant.
        let h = [seg("彼"), seg("隻")];
        assert_eq!(
            nailed_prefix_with_oracle(&h, SegmentText::Display, false, None, |_, _, _| true),
            "彼隻"
        );
    }

    #[test]
    fn known_two_syllable_compound_renders_internal_hyphen() {
        // hit ê tsa bóo → hit ê tsa-bóo (查某 is the only 2-syll
        // compound; "彼个" / others are not in the oracle). Regression
        // pin: the v3.5.9 longest-match refactor must preserve the
        // v3.5.8 §10.2 Option A bigram path.
        let n = [
            seg_dc("hit", "彼", 1),
            seg_dc("ê", "个", 1),
            seg_dc("tsa", "查", 1),
            seg_dc("bóo", "某", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 2
                && h == "查某"),
            "hit ê tsa-bóo"
        );
    }

    #[test]
    fn known_three_syllable_compound_renders_internal_hyphens() {
        // 紅尾冬 (red-tail fish) is a known 3-syll compound; both 紅尾
        // (n=2) and 紅尾冬 (n=3) are in the oracle. Longest-match-left
        // picks n=3, emitting hyphens between all three members:
        // "Âng-bóe-tang", NOT "Âng-bóe tang".
        let n = [
            seg_dc("Âng", "紅", 1),
            seg_dc("bóe", "尾", 1),
            seg_dc("tang", "冬", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| matches!(
                (h, n),
                ("紅尾冬", 3) | ("紅尾", 2)
            )),
            "Âng-bóe-tang"
        );
    }

    #[test]
    fn overlapping_bigrams_do_not_imply_trigram() {
        // Oracle says both 查某 (n=2) and 某人 (n=2) are compounds, but
        // 查某人 (n=3) is NOT. Longest-match tries n=3 (false), then
        // n=2 from the leftmost position (true) → consumes 查-某,
        // advances to 人, emits ` 人` → "tsa-bóo lâng", never
        // "tsa-bóo-lâng". Anti-overgluing is a natural consequence of
        // leftmost longest-match, not a separate `prev_hyphen` guard.
        let n = [
            seg_dc("tsa", "查", 1),
            seg_dc("bóo", "某", 1),
            seg_dc("lâng", "人", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 2
                && (h == "查某" || h == "某人")),
            "tsa-bóo lâng"
        );
    }

    #[test]
    fn disjoint_bigrams_each_render_internal_hyphen() {
        // A-B-C-D with AB and CD both 2-syll compounds, ABC and BCD
        // and ABCD none → "A-B C-D". Disjoint runs don't block each
        // other; only an in-flight run is anti-extended.
        let n = [
            seg_dc("tsa", "查", 1),
            seg_dc("bóo", "某", 1),
            seg_dc("mâ", "麻", 1),
            seg_dc("huân", "煩", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 2
                && (h == "查某" || h == "麻煩")),
            "tsa-bóo mâ-huân"
        );
    }

    #[test]
    fn multi_syllable_segment_is_not_a_compound_run_member() {
        // A 2-syllable nailed segment (e.g. the compound was tapped
        // whole) is never folded into an n-syll compound run even if
        // the oracle would match the canonical concatenation. The
        // longest-match scan stops counting at the first non-single-
        // syllable segment.
        let n = [seg_dc("tsa-bóo", "查某", 2), seg_dc("lâng", "人", 1)];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _, _| true),
            "tsa-bóo lâng"
        );
    }

    #[test]
    fn user_typed_trailing_hyphen_inside_run_disqualifies_compound() {
        // `tai-` (user hyphen-continuation) appears first in the run.
        // Oracle DOES hit `(台灣, 2)`, but `longest_compound_run` must
        // exclude segments whose `display_text` already ends with `-`
        // (otherwise the in-run auto-hyphen would render `tai--uan`).
        // Expected: push `tai-` alone, then the next iteration sees
        // `s.ends_with('-')` and pushes `uan` with no boundary →
        // `tai-uan` (Codex PR #349 r3311725114).
        let n = [seg_dc("tai-", "台", 1), seg_dc("uan", "灣", 1)];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 2
                && h == "台灣"),
            "tai-uan"
        );
    }

    #[test]
    fn syllable_joiner_joins_the_compound_run_and_keeps_the_typed_hyphen() {
        // Syllable Separator None / Space: the compound run is still detected
        // (台灣 stays one word) but joined with the setting's joiner; the
        // following word boundary and a user-typed `tai-` continuation are
        // untouched.
        let n = [
            seg_dc("tai", "台", 1),
            seg_dc("uan", "灣", 1),
            seg_dc("lang", "人", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(""), |h, _, n| n == 2
                && h == "台灣"),
            "taiuan lang"
        );
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(" "), |h, _, n| n == 2
                && h == "台灣"),
            "tai uan lang"
        );
        let typed = [seg_dc("tai-", "台", 1), seg_dc("uan", "灣", 1)];
        for joiner in ["", " "] {
            assert_eq!(
                nailed_prefix_with_oracle(
                    &typed,
                    SegmentText::Display,
                    true,
                    Some(joiner),
                    |h, _, n| n == 2 && h == "台灣"
                ),
                "tai-uan",
                "{joiner:?}"
            );
        }
    }

    #[test]
    fn compound_run_capped_at_max_dictionary_phrase_length() {
        // Builder caps `syllable_count` at 4 (`dictionary/build/
        // dictionary_records.py:43`); the longest-match scan mirrors
        // the cap. Even if the oracle would happily say "yes" for a
        // 5-syllable concat, `longest_compound_run` never asks at
        // `n=5` — the result is the longest 4-syllable match (n=4
        // here), and the 5th segment renders as a separate word with
        // a space boundary (Codex PR #349 r3311725130).
        let n = [
            seg_dc("a", "A", 1),
            seg_dc("b", "B", 1),
            seg_dc("c", "C", 1),
            seg_dc("d", "D", 1),
            seg_dc("e", "E", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| {
                // ALL prefixes match — without the cap we'd get a
                // single 5-syll run "a-b-c-d-e".
                matches!((h, n), ("ABCDE", 5) | ("ABCD", 4) | ("ABC", 3) | ("AB", 2))
            }),
            "a-b-c-d e"
        );
    }

    #[test]
    fn user_typed_trailing_hyphen_suppresses_then_blocks_next_compound() {
        // "tai-" is a user hyphen-continuation: boundary emits nothing
        // and the following segment skips its compound check (run_len
        // forced to 1 by `s.ends_with('-')`), so even though 灣國 is in
        // the oracle the (uan, X) pair does NOT auto-compound.
        let n = [
            seg_dc("tai-", "台", 1),
            seg_dc("uan", "灣", 1),
            seg_dc("X", "國", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 2
                && h == "灣國"),
            "tai-uan X"
        );
    }

    #[test]
    fn no_lexicon_installed_keeps_compound_pairs_space_joined() {
        // `nailed_prefix` (lexicon-wired) with no dictionary installed
        // in the unit-test process → graceful pure space-join.
        let n = [seg_dc("tsa", "查", 1), seg_dc("bóo", "某", 1)];
        assert_eq!(nailed_prefix(&n, &cfg("tl", false, false)), "tsa bóo");
    }

    // ---- typed separator run at a boundary (USER 2026-09-22) ----
    // The `-` run typed before a pick folds into that pick's `raw_text`;
    // the boundary in front of it renders the run, whatever the oracle
    // or the word space would have emitted.

    fn seg_raw(display: &str, canonical: &str, raw: &str) -> NailedSegment {
        NailedSegment {
            raw_text: raw.to_owned(),
            ..seg_dc(display, canonical, 1)
        }
    }

    #[test]
    fn typed_run_renders_verbatim_over_space_and_compound_joiner() {
        let no_compound = |_: &str, _: &str, _: u8| false;
        let compound = |h: &str, _: &str, n: u8| n == 2 && h == "轉來";
        for (raw, expected) in [
            ("-lâi", "tńg-lâi"),
            ("--lâi", "tńg--lâi"),
            ("---lâi", "tńg---lâi"),
        ] {
            let n = [seg_dc("tńg", "轉", 1), seg_raw("lâi", "來", raw)];
            assert_eq!(
                nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, no_compound),
                expected,
                "{raw}"
            );
            assert_eq!(
                nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, compound),
                expected,
                "{raw} compound"
            );
        }
        // Nothing typed → the oracle decides, as before.
        let n = [seg_dc("tńg", "轉", 1), seg_raw("lâi", "來", "lâi")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, no_compound),
            "tńg lâi"
        );
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, compound),
            "tńg-lâi"
        );
    }

    #[test]
    fn typed_run_inside_a_compound_keeps_the_rest_of_the_run_grouped() {
        // Only 台灣人 is a compound; `-` typed between 台 and 灣 must not
        // turn 灣|人 into a word boundary (`tâi-uân lâng`).
        let n = [
            seg_dc("tâi", "台", 1),
            seg_raw("uân", "灣", "--uan"),
            seg_raw("lâng", "人", "lang"),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 3
                && h == "台灣人"),
            "tâi--uân-lâng"
        );
    }

    #[test]
    fn the_oracle_sees_the_run_reading_beside_its_hanji_at_every_length() {
        // #772: the reading truncates with the Hanji — 紅尾冬 is asked as
        // (紅尾冬, ângbuétang, 3), then (紅尾, ângbué, 2) — so a run is a
        // compound only under the reading the user picked.
        let seg_tl = |display: &str, hanji: &str| NailedSegment {
            association_tl: display.to_owned(),
            ..seg_dc(display, hanji, 1)
        };
        let n = [
            seg_tl("âng", "紅"),
            seg_tl("bué", "尾"),
            seg_tl("tang", "冬"),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, r, n| {
                (h, r, n) == ("紅尾冬", "ângbuétang", 3)
            }),
            "âng-bué-tang"
        );
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, r, n| {
                (h, r, n) == ("紅尾", "ângbué", 2)
            }),
            "âng-bué tang"
        );
        // The same Hanji under another reading is no compound.
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, r, n| {
                (h, r, n) == ("紅尾", "ângbé", 2)
            }),
            "âng bué tang"
        );
    }

    #[test]
    fn typed_run_renders_through_the_syllable_joiner() {
        for (joiner, raw, expected) in [
            ("", "-lâi", "tńglâi"),
            ("", "--lâi", "tńg·lâi"),
            ("", "----lâi", "tńg··lâi"),
            (" ", "-lâi", "tńg lâi"),
            (" ", "--lâi", "tńg ·lâi"),
            (" ", "---lâi", "tńg ·lâi"),
        ] {
            let n = [seg_dc("tńg", "轉", 1), seg_raw("lâi", "來", raw)];
            assert_eq!(
                nailed_prefix_with_oracle(
                    &n,
                    SegmentText::Display,
                    true,
                    Some(joiner),
                    |_, _, _| { false }
                ),
                expected,
                "{joiner:?} {raw}"
            );
        }
    }

    #[test]
    fn space_joiner_separates_a_rendered_dot_piece_after_a_typed_run() {
        // The `·guá` piece carries its neutral-tone dot, not the syllable
        // boundary: Space still writes one in front of it, None adds nothing.
        let n = [seg_dc("hōo", "予", 1), seg_raw("·guá", "我", "--gua")];
        let render = |joiner| {
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(joiner), |_, _, _| {
                false
            })
        };
        assert_eq!(render(" "), "hōo ·guá");
        assert_eq!(render(""), "hōo·guá");
        // A literal `--guá` piece keeps carrying its own separator.
        let literal = [seg_dc("hōo", "予", 1), seg_raw("--guá", "我", "--gua")];
        assert_eq!(
            nailed_prefix_with_oracle(
                &literal,
                SegmentText::Display,
                true,
                Some(" "),
                |_, _, _| { false }
            ),
            "hōo--guá"
        );
    }

    #[test]
    fn typed_run_is_not_doubled_when_the_segment_already_carries_it() {
        // A dictionary khinsiann piece, a §34 literal, and their None-mode
        // `·` forms open with their own separator.
        for display in ["--khí-lâi", "--khilai", "·khílâi"] {
            let n = [seg_dc("kì", "記", 1), seg_raw(display, "起來", "--khilai")];
            assert_eq!(
                nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _, _| false),
                format!("kì{display}"),
                "{display}"
            );
        }
        // Inside a compound too: the oracle's joiner must not stack onto
        // a segment that renders its own `--` (`tńg---lâi`).
        let n = [seg_dc("tńg", "轉", 1), seg_raw("--lâi", "來", "--lai")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, _, n| n == 2
                && h == "轉來"),
            "tńg--lâi"
        );
        // No run typed: a self-separated segment keeps today's boundary.
        let n = [
            seg_dc("kì", "記", 1),
            seg_raw("--khí-lâi", "起來", "khilai"),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _, _| false),
            "kì --khí-lâi"
        );
        // A continuation segment (`tai-`) already ends the boundary.
        let n = [seg_dc("tai-", "台", 1), seg_raw("uan", "灣", "-uan")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _, _| false),
            "tai-uan"
        );
    }

    #[test]
    fn typed_run_is_ignored_without_a_word_space() {
        // Hanji-first / TPS render no separator; the run stays in raw only.
        let n = [seg_dc("轉", "轉", 1), seg_raw("來", "來", "--lai")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, false, None, |_, _, _| true),
            "轉來"
        );
    }

    #[test]
    fn pending_tail_opening_with_a_typed_run_gets_no_boundary_space() {
        let n = [seg_dc("tńg", "轉", 1)];
        let c = cfg("tl", false, false);
        assert_eq!(combined_display(&n, "--lai", &c), "tńg--lai");
        assert_eq!(combined_display(&n, "-lai", &c), "tńg-lai");
        assert_eq!(combined_display(&n, "lai", &c), "tńg lai");
        let (_, tail_start) = combined_display_with_tail(&n, "--lai", &c);
        assert_eq!(tail_start, "tńg".len());
    }
}
