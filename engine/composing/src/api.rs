//! Public façade for the composing crate. Defines `Engine`, `EngineState`,
//! `Phase`, `Intent`, and `ComposingError`. Implementation of state
//! transitions lives in `transition.rs`; this module is the stable surface
//! that `requests.rs` and external crates consume.

pub use crate::conversion::{Conversion, ConversionFrequency, ConvertedSegment};
use crate::handle::{ListContext, PendingSnapshot};
use lexicon::{compound_hanji_exists, EngineHandle as LexiconHandle};
use protos::engine::{AppConfig, ComposingResponse};
use thiserror::Error;

/// Composition phase. `Idle` means no preedit; every composition is
/// `Continuous { raw, caret, nailed }` (the v3.5.8 multi-segment state) from
/// its first keystroke — the former single-segment `Composing` phase only
/// ever lived between that keystroke and the platform's former `EnterContinuous`
/// (removed R12, 2026-10-01). A Continuous phase is never empty in both
/// `raw` and `nailed`.
///
/// `caret` is the editing position inside the pending `raw`: a UTF-8 byte
/// offset on a char boundary, `0..=raw.len()`. Every mutator edits there;
/// only `Intent::MoveCaret` (desktop) moves it away from `raw.len()`, so on
/// mobile it is always the end. Under a conversion it is never inside a
/// converted word. It lives beside `raw` rather than on
/// `EngineState` so replacing the phase can never leave a stale offset.
///
/// `conversion` is the Hanji conversion of the pending `raw`
/// ([`crate::conversion`]), kept beside it for the same reason. Only a
/// mutation whose `AppConfig` asks for one (`hanji_conversion`) on a TPS
/// buffer stores it, so every TL / POJ and mobile composition carries `None`.
/// A request that does not ask for it is shown the glyphs even while the
/// state holds one. It changes what the preedit shows, never what a commit
/// writes.
///
/// **Model B (mainstream-aligned, see `docs/engine/continuous-commit-and-display.md`
/// §10):** `nailed` segments are **NOT** in the host document. The whole
/// composition — `Σ nailed[i].display_text` followed by the derived display
/// of the pending `raw` tail — lives in **one** marked / composing region
/// until a hard finalize (Enter / final-commit / external-suggestion commit).
/// A candidate tap *nails* a segment inside the composition; only a hard
/// finalize writes literal text to the document. Reset/abort clears the
/// whole region and drops all `nailed` (nothing was written).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Continuous {
        raw: String,
        caret: usize,
        nailed: Vec<NailedSegment>,
        conversion: Option<Conversion>,
    },
}

/// One move of `Intent::MoveCaret`: a step left / right, or a jump to the
/// start / end of the pending tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretDirection {
    Left,
    Right,
    Start,
    End,
}

/// Which rendering of a pick the document gets — the wire `CommitScript`
/// minus `UNSPECIFIED`, which a commit is ignored under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitScript {
    /// What the output settings lead with (Enter / a tap).
    Lead,
    /// The other script of the lead (desktop Space).
    Other,
    /// A §42 split Hanji cell.
    Hanji,
    /// A §42 split romanization cell.
    Roman,
}

/// One pick the engine counts on the R5 path — the triple each platform's
/// tap handler recorded itself: the canonical display text (the identity
/// sidechannel, never the document rendering), the canonical TL, and the
/// Hanji when the pick carried one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    pub display_text: String,
    pub canonical_tl: String,
    pub hanji: Option<String>,
}

impl Phase {
    /// Pending-tail display form rendered through the derived-display chain
    /// (POJ doubletap → tone marks → nasal-case adjust; TPS pass-through).
    /// For `Phase::Continuous` this is strictly the pending `raw` tail, not
    /// the original keystroke history nor the nailed prefix; for
    /// `Phase::Idle` it is the empty string.
    ///
    /// **Model B (§10):** this is **no longer the composing-buffer surface** —
    /// it is one internal *component* of it. The composing buffer the host
    /// renders is [`Phase::composing_display`] (`Σ nailed.display_text` +
    /// this pending-tail form). `raw_input` remains the still-editable raw
    /// tail used by span-local candidate fetch and by callers that need the
    /// pending-only form.
    ///
    /// User-typed hyphens are preserved as conversion boundaries (the
    /// derived-display chain splits on `-` for tone-mark application); the
    /// engine does NOT validate whether each chunk is a real syllable, and
    /// does NOT auto-insert hyphens. See §10.2 amendment 2026-05-13.
    pub fn raw_input(&self, config: &AppConfig) -> String {
        match self {
            Phase::Idle => String::new(),
            Phase::Continuous { raw, .. } => crate::derived::derived_display(raw, config),
        }
    }

    /// The composing-buffer surface the host renders in its single
    /// marked / composing region (`docs/engine/continuous-commit-and-display.md`
    /// §10.2 / §10.4 invariant I1, Model B) — except while a Hanji
    /// `conversion` is shown, when the preedit carries the converted tail
    /// and this stays the text a commit writes.
    ///
    /// - `Idle` → empty string.
    /// - `Continuous { raw, nailed, .. }` → `Σ nailed[i].display_text`
    ///   concatenated with the derived display of the pending `raw` tail.
    ///   Nailed segments are **not** in the document; they are part of the
    ///   marked region until a hard finalize.
    ///
    /// v3.5.8 §10.2 segmented-spacing contract: adjacent segments (and
    /// the nailed prefix ↔ pending tail) are joined by a single space
    /// when the rendered script is roman-ish (roman-first / both-scripts);
    /// hanji-first / TPS render as-is with no separator. See
    /// [`continuous_word_space`] / [`nailed_prefix`]. This is the exact
    /// string a hard finalize (Enter / final-commit) writes to the
    /// document, so the separator policy applies identically there.
    pub fn composing_display(&self, config: &AppConfig) -> String {
        match self {
            Phase::Idle => String::new(),
            Phase::Continuous { raw, nailed, .. } => combined_display(nailed, raw, config),
        }
    }
}

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
    // candidate commit), falling back to `raw_text` when empty (legacy
    // callers / TPS-OOV hanji-absent). Kept SEPARATE from `raw_text`,
    // which stays the authority for span / unnail mechanics (Codex
    // pre-impl 2026-06-03 SHOULD).
    pub association_tl: String,
    // Learned phrases (§50) — the chosen `CandidateMessage.hanji`, present
    // only when the platform picked a hanji-bearing candidate (any output
    // script). `None` for a hanji-less pick or a legacy caller; the final
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
fn continuous_word_space(config: &AppConfig) -> bool {
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
///   AND `is_compound(Σ canonical_text, n)` is true. If found, emit
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
/// the segments' own `canonical_text` + `syllable_count`; `raw_text` keeps
/// the user's typed characters as the authority for backspace-pop.
fn nailed_prefix_with_oracle(
    nailed: &[NailedSegment],
    text: SegmentText,
    space: bool,
    syllable_joiner: Option<&str>,
    is_compound: impl Fn(&str, u8) -> bool,
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
/// `is_compound(Σ canonical_text, n)` is true. Returns `1` when no
/// run qualifies (caller treats it as "advance one segment, normal
/// boundary").
///
/// Builds the full eligible-segment concatenation once, then truncates
/// from the right per iteration — O(max_n) string ops instead of
/// rebuilding each attempt.
fn longest_compound_run<F: Fn(&str, u8) -> bool>(
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
    let mut concat = String::new();
    for s in &segs[..max_n] {
        concat.push_str(&s.canonical_text);
    }
    for n in (2..=max_n).rev() {
        if is_compound(&concat, n as u8) {
            return n;
        }
        if n > 2 {
            let last_len = segs[n - 1].canonical_text.len();
            concat.truncate(concat.len() - last_len);
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
    )
}

/// Learned phrases (§50) — the canonical TL a composition of nailed
/// picks learns: the segments' `association_tl` under the same word
/// boundaries the roman commit renders ([`nailed_prefix_with_oracle`]),
/// whatever the display settings — a dictionary compound run joins with
/// `-`, other words with a space, a typed `-` run overrides. 做 + 進 +
/// 出 + 口 learns `tsò tsìn-tshut-kháu`, never `tsò-tsìn-tshut-kháu`
/// (USER 2026-09-23: 做 and 進出口 are two words).
pub(crate) fn learned_reading(nailed: &[NailedSegment]) -> String {
    nailed_join(nailed, SegmentText::AssociationTl, true, None)
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

fn nailed_join(
    nailed: &[NailedSegment],
    text: SegmentText,
    space: bool,
    syllable_joiner: Option<&str>,
) -> String {
    let eligible = space
        && nailed.len() >= 2
        && nailed
            .windows(2)
            .any(|w| w[0].syllable_count == 1 && w[1].syllable_count == 1);
    if !eligible {
        return nailed_prefix_with_oracle(nailed, text, space, syllable_joiner, |_, _| false);
    }
    LexiconHandle::with_state(|state| {
        let (Some(prefix), Some(dict)) = (state.prefix_index.as_ref(), state.dictionary.as_ref())
        else {
            return Ok(nailed_prefix_with_oracle(
                nailed,
                text,
                space,
                syllable_joiner,
                |_, _| false,
            ));
        };
        Ok(nailed_prefix_with_oracle(
            nailed,
            text,
            space,
            syllable_joiner,
            |h, n| compound_hanji_exists(h, n, prefix, dict),
        ))
    })
    .unwrap_or_else(|_| {
        nailed_prefix_with_oracle(nailed, text, space, syllable_joiner, |_, _| false)
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

/// Engine state — the platform no longer shadows this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineState {
    pub phase: Phase,
}

impl Default for EngineState {
    fn default() -> Self {
        Self { phase: Phase::Idle }
    }
}

/// The user's own rows a `FetchAtPos` ranks with, for the pending buffer.
/// The engine reads them from its user-data stores (`dispatch` crate,
/// `user_data::handle_composing`); no platform sends them
/// (`docs/architecture/user-data-engine-roadmap.md` P9). Empty = neutral:
/// no boost, no custom or learned candidates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserRows {
    /// The learned counts for the candidates a neutral fetch offered, keyed
    /// by the `(display_text, canonical_tl)` pair.
    pub frequency: ranking::FrequencyMap,
    /// The custom-dictionary entries matching the buffer — raw stored
    /// `(roman, hanji)` columns. `roman` may be TL or POJ display form
    /// (whichever the user typed, v3.5.9 B-4): the lattice / dedupe axis
    /// treats it as raw (`shadow::custom_toneless_key` canonicalizes per
    /// mode) and only the synthesized commit key folds it to canonical TL.
    pub custom: Vec<lexicon::CustomEntry>,
    /// The learned phrases (§50) matching the buffer, most-learned first —
    /// the order decides which of two separator forms of one reading wins.
    pub learned: Vec<lexicon::LearnedEntry>,
}

/// What a mutating intent produced: the response the platform gets, the
/// phrase a final commit taught (§50), and the pick an R5 commit counts. The
/// engine keeps both in its own stores (`dispatch` crate,
/// `user_data::handle_composing`); neither crosses the FFI.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    pub response: ComposingResponse,
    pub learned: Option<lexicon::LearnedEntry>,
    /// Set by a `CommitContinuous` that nailed or finalized (R5).
    pub usage: Option<Usage>,
}

impl From<ComposingResponse> for Applied {
    fn from(response: ComposingResponse) -> Self {
        Self {
            response,
            learned: None,
            usage: None,
        }
    }
}

/// What a `ComposingRequest` asks the engine to do. Decoded from
/// `protos::engine::ComposingRequest::method` inside `requests::handle`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Start {
        text: String,
    },
    Append {
        ch: String,
    },
    AppendHyphen,
    ReplaceLast {
        replacement: String,
    },
    DeleteBackward,
    CommitRaw,
    CommitPreeditThenInsertExternal {
        text: String,
    },
    Reset,
    /// v3.5.8 Phase 6 — pure read of span-local continuous-input
    /// candidates for the current `Phase::Continuous { raw }` starting
    /// at `position` (always `0` in v3.5.8; non-zero short-circuits
    /// to an empty candidate list). Resolved by `requests::handle`
    /// outside the `transition::apply` pure path because the fetch
    /// needs lexicon state — see `requests::handle_fetch_at_pos`.
    /// `transition.rs` only sees this variant via a defensive snapshot
    /// arm; production callers always go through dispatch.
    ///
    /// `now_ms` is the platform wall clock (epoch-ms) the recency ranking
    /// reads; `user_rows` the user's own data the fetch ranks with
    /// ([`UserRows`]) — the engine reads them from its stores, a wire
    /// request decodes with none.
    ///
    /// `enabled_sources_bitmask` is the `dictionary.bin` source filter the
    /// fetch applies, the same as Tab3 browse — resolved from
    /// `FetchAtPos.toggles` at decode ([`crate::requests::fetch_at_pos_intent`]);
    /// `u32::MAX` = every source, `0` = none.
    /// §34 / S22 — `literal_roman_candidate_disabled` gates the always-on
    /// preedit-literal roman candidate (index-0 `derived_display` WYSIWYG row
    /// for Hanji-romanization one-tap). Decoded verbatim from `FetchAtPos`; OFF suppresses
    /// only the §34 forced prepend, not the natural roman candidates. Full
    /// wire/sentinel contract: the `FetchAtPos` proto comment.
    FetchAtPos {
        now_ms: i64,
        enabled_sources_bitmask: u32,
        literal_roman_candidate_disabled: bool,
        user_rows: UserRows,
        /// The previous word's continuations, user-learned and bundled
        /// (`dispatch::context`), ranked into the candidate order (§56).
        /// Empty = no context. The platform sends none; a proto decode
        /// leaves it empty.
        context: ranking::ContextRanks,
    },
    /// Nail a candidate segment in `Phase::Continuous`. The engine takes
    /// `pending[..consumed_bytes]` as the nailed segment's raw text and
    /// keeps `pending[consumed_bytes..]` as the new pending tail. When
    /// `consumed_bytes >= pending.len()`, this becomes a final commit and
    /// exits to Idle. Caller (Phase 6+ proto layer) is responsible for
    /// `consumed_bytes` aligning with both UTF-8 char boundaries and TL
    /// syllable boundaries returned by the syllabifier. The engine resolves
    /// what the pick writes itself (R5, `commit_text::resolve_commit_text`).
    CommitContinuous {
        // v3.5.8 Phase 9 Bug 1 (Option A): canonical key for freq/NextWord.
        // Empty → the commit is ignored.
        canonical_text: String,
        // v3.6.1 R2: canonical TL of the committed candidate. Becomes the
        // NextWord `roman` arg (→ `prev_tl`/`next_tl`); empty → engine
        // falls back to the raw committed slice (TPS-OOV).
        association_tl: String,
        // Learned phrases (§50): the picked candidate's hanji, `None` when
        // the pick carried none (contract on the `CommitContinuous` proto).
        hanji: Option<String>,
        consumed_bytes: usize,
        syllable_count: u8,
        /// Which rendering of the pick the document gets; `None` (the wire's
        /// `UNSPECIFIED`, or a script newer than this engine) ignores the
        /// commit.
        script: Option<CommitScript>,
        /// The pick's display romanization (`CandidateMessage.roman`).
        roman: String,
    },
    /// Desktop Telex scheme — one tone / affricate / hyphen letter applied
    /// to the pending tail (`telex::apply_telex_key`). Unknown keys and
    /// edits that change nothing answer with a no-op.
    TelexKey {
        key: String,
    },
    /// One TPS key at the caret — auto-correct, replacement and insertion in
    /// one step, or the Space separator; see the `TpsKey` proto comment.
    TpsKey {
        key: String,
    },
    /// Desktop caret — step one char inside the pending tail, one word over
    /// a converted word, or jump to the tail's start / end; see the
    /// `MoveCaret` proto comment for the contract (no refetch, a move the
    /// caret cannot make = no effects unless a conversion is rebuilt, re-open
    /// under a conversion).
    /// `None` is a wire direction the engine does not know (unspecified or
    /// newer than this build) and steps nowhere.
    MoveCaret {
        direction: Option<CaretDirection>,
    },
    /// Desktop, under a Hanji conversion — commit the composition as the
    /// preedit shows it; see the `CommitAsShown` proto comment.
    CommitAsShown,
    /// Desktop, under a Hanji conversion — commit the glyphs of the whole
    /// composition as typed; see the `CommitAsTyped` proto comment.
    CommitAsTyped,
}

impl Intent {
    /// `true` for intents that only read engine state. `requests::query`
    /// answers them from `&Engine`; `EngineHandle` runs them against a
    /// clone with its locks released and never lets them reset state on a
    /// generation mismatch (a stale worker-thread fetch must not wipe a
    /// newer context).
    pub fn is_read_only(&self) -> bool {
        matches!(self, Intent::FetchAtPos { .. })
    }
}

#[derive(Debug, Error)]
pub enum ComposingError {
    #[error("composing request missing method")]
    MissingMethod,
}

/// State machine. Held inside `Mutex<Engine>` at the FFI boundary.
#[derive(Clone, Debug, Default)]
pub struct Engine {
    state: EngineState,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// The response a fresh (Idle) engine gives: what a generation-mismatch
    /// reset would have produced. `EngineHandle` answers read-only intents
    /// carrying a stale generation with this instead of resetting.
    pub fn idle_snapshot(config: &AppConfig) -> ComposingResponse {
        Self::default().snapshot(config)
    }

    /// Pure read — no state mutation, no effects emitted. Used by the
    /// `FetchAtPos` read path and the generation-mismatch idle snapshot. `config` is the request's `AppConfig` so
    /// `Preedit.display_text` reflects the caller's actual mode/toggles
    /// (per Codex PR #197 r3169707395).
    pub fn snapshot(&self, config: &AppConfig) -> ComposingResponse {
        crate::transition::snapshot(&self.state, config)
    }

    /// What a `FetchAtPos` ranks against (`PendingSnapshot`): the part of the
    /// pending tail it lists ([`Self::word_list_start`]) and the word that part
    /// follows — the converted word ending where it starts, else, at the start
    /// of the tail, the last nailed segment — when that is a word
    /// ([`NailedSegment::context_word`]). With no segment before it, the list
    /// starts the composition and follows the committed context (§56);
    /// anything else before it — glyphs — cuts the context.
    pub fn pending_snapshot(&self, config: &AppConfig) -> PendingSnapshot {
        let Phase::Continuous {
            raw,
            nailed,
            conversion,
            ..
        } = &self.state.phase
        else {
            return PendingSnapshot {
                listed_raw: String::new(),
                context: ListContext::Committed,
            };
        };
        let list_start = self.word_list_start(config).unwrap_or(0);
        let segment_before = if list_start > 0 {
            conversion
                .as_ref()
                .and_then(|conversion| conversion.word_ending_at(list_start))
                .map(|word| word.nailed_as_shown(raw, &[]))
        } else {
            nailed.last().cloned()
        };
        let context = match segment_before {
            None if list_start == 0 => ListContext::Committed,
            segment => segment
                .and_then(|segment| segment.context_word())
                .map_or(ListContext::Cut, |(word, roman)| {
                    ListContext::Word(word, roman)
                }),
        };
        PendingSnapshot {
            listed_raw: raw[list_start..].to_string(),
            context,
        }
    }

    /// Byte offset in the pending tail where the list of the word before the
    /// caret starts, under a Hanji conversion the request asks for
    /// (`crate::conversion::word_list_start`); `None` for the whole tail's
    /// list. A pick resolves the same start (`transition::commit_continuous`).
    pub fn word_list_start(&self, config: &AppConfig) -> Option<usize> {
        match &self.state.phase {
            Phase::Continuous {
                raw,
                caret,
                nailed,
                conversion,
            } => {
                crate::conversion::word_list_start(nailed, raw, *caret, conversion.as_ref(), config)
            }
            Phase::Idle => None,
        }
    }

    /// Apply `intent` against the current state, mutate, and return the
    /// resulting response (preedit + ordered effects + is_composing). Delegates to the pure transition table in
    /// `transition.rs`.
    pub fn apply(&mut self, intent: Intent, config: &AppConfig) -> ComposingResponse {
        self.apply_learning(intent, config).response
    }

    /// [`apply`](Self::apply), with the phrase a final commit taught (§50).
    pub fn apply_learning(&mut self, intent: Intent, config: &AppConfig) -> Applied {
        self.apply_ranked(intent, config, None)
    }

    /// [`apply_learning`](Self::apply_learning), a Hanji conversion walk
    /// ranking with the user's learned counts `frequency` supplies (H5);
    /// `None` walks neutral.
    pub fn apply_ranked(
        &mut self,
        intent: Intent,
        config: &AppConfig,
        frequency: Option<&dyn ConversionFrequency>,
    ) -> Applied {
        crate::transition::apply(&mut self.state, intent, config, frequency)
    }

    /// Pure-Rust observability of the engine's `EngineState`. Returns a
    /// clone so callers cannot mutate internal state. Phase 4 adds this so
    /// `tests/continuous_phase.rs` can assert `Phase::Continuous`'s
    /// `nailed` / `raw` fields without a corresponding proto carrier
    /// (the proto-side response shape lands in Phase 6). `#[doc(hidden)]`
    /// because this is a Phase-4-internal escape hatch — production
    /// callers should reach state through `apply` / `snapshot`'s
    /// `ComposingResponse` carrier (Codex post-impl note 1).
    #[doc(hidden)]
    pub fn snapshot_state(&self) -> EngineState {
        self.state.clone()
    }

    /// Idempotent reset. Called from the generation-mismatch path inside
    /// `requests::handle`. NOT public API — external callers always go
    /// through `requests::handle`. The user-initiated `Intent::Reset` path
    /// goes through `apply(Intent::Reset, ...)`, which emits the
    /// `ClearPreeditWithoutCommit + ClearCandidates` effects when
    /// composing; this helper is silent (no effects) for the
    /// generation-mismatch drop. Call site is `EngineHandle::handle`.
    pub(crate) fn reset(&mut self) {
        self.state = EngineState::default();
    }
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
            platform_id: 0,
            output_both_scripts: both,
            candidate_display_mode: 0,
            syllable_separator: 0,
            force_lowercase_nasal_marker: false,
            tps_or_maps_to_er: false,
            hanji_conversion: None,
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _| false),
            "hit tui"
        );
        // Hanji-first / TPS (space = false) → no separator at all,
        // oracle irrelevant.
        let h = [seg("彼"), seg("隻")];
        assert_eq!(
            nailed_prefix_with_oracle(&h, SegmentText::Display, false, None, |_, _| true),
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 2
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| matches!(
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 2
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 2
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _| true),
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 2
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(""), |h, n| n == 2
                && h == "台灣"),
            "taiuan lang"
        );
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(" "), |h, n| n == 2
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
                    |h, n| n == 2 && h == "台灣"
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| {
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 2
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
        let no_compound = |_: &str, _: u8| false;
        let compound = |h: &str, n: u8| n == 2 && h == "轉來";
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 3
                && h == "台灣人"),
            "tâi--uân-lâng"
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
                nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(joiner), |_, _| {
                    false
                }),
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
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, Some(joiner), |_, _| false)
        };
        assert_eq!(render(" "), "hōo ·guá");
        assert_eq!(render(""), "hōo·guá");
        // A literal `--guá` piece keeps carrying its own separator.
        let literal = [seg_dc("hōo", "予", 1), seg_raw("--guá", "我", "--gua")];
        assert_eq!(
            nailed_prefix_with_oracle(&literal, SegmentText::Display, true, Some(" "), |_, _| {
                false
            }),
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
                nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _| false),
                format!("kì{display}"),
                "{display}"
            );
        }
        // Inside a compound too: the oracle's joiner must not stack onto
        // a segment that renders its own `--` (`tńg---lâi`).
        let n = [seg_dc("tńg", "轉", 1), seg_raw("--lâi", "來", "--lai")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |h, n| n == 2
                && h == "轉來"),
            "tńg--lâi"
        );
        // No run typed: a self-separated segment keeps today's boundary.
        let n = [
            seg_dc("kì", "記", 1),
            seg_raw("--khí-lâi", "起來", "khilai"),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _| false),
            "kì --khí-lâi"
        );
        // A continuation segment (`tai-`) already ends the boundary.
        let n = [seg_dc("tai-", "台", 1), seg_raw("uan", "灣", "-uan")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, true, None, |_, _| false),
            "tai-uan"
        );
    }

    #[test]
    fn typed_run_is_ignored_without_a_word_space() {
        // Hanji-first / TPS render no separator; the run stays in raw only.
        let n = [seg_dc("轉", "轉", 1), seg_raw("來", "來", "--lai")];
        assert_eq!(
            nailed_prefix_with_oracle(&n, SegmentText::Display, false, None, |_, _| true),
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
