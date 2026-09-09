//! Public façade for the composing crate. Defines `Engine`, `EngineState`,
//! `Phase`, `Intent`, and `ComposingError`. Implementation of state
//! transitions lives in `transition.rs`; this module is the stable surface
//! that `dispatch.rs` and external crates consume.

use lexicon::{compound_hanji_exists, EngineHandle as LexiconHandle};
use protos::engine::{AppConfig, ComposingResponse};
use thiserror::Error;

/// Composition phase. `Idle` means no preedit; `Composing { raw, caret }`
/// carries the numeric-tone ASCII raw input that the platform-side state used
/// to shadow; `Continuous { raw, caret, nailed }` is the v3.5.8 multi-segment
/// state.
///
/// `caret` is the editing position inside the pending `raw`: a UTF-8 byte
/// offset on a char boundary, `0..=raw.len()`. Every mutator edits there;
/// only `Intent::MoveCaret` (desktop) moves it away from `raw.len()`, so on
/// mobile it is always the end. It lives beside `raw` rather than on
/// `EngineState` so replacing the phase can never leave a stale offset.
///
/// **Model B (mainstream-aligned, see `docs/engine/continuous-input-ranking.md`
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
    Composing {
        raw: String,
        caret: usize,
    },
    Continuous {
        raw: String,
        caret: usize,
        nailed: Vec<NailedSegment>,
    },
}

/// One step of `Intent::MoveCaret`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretDirection {
    Left,
    Right,
}

impl Phase {
    /// Pending-tail display form rendered through the derived-display chain
    /// (POJ doubletap → tone marks → nasal-case adjust; TPS pass-through).
    /// For `Phase::Continuous` this is strictly the pending `raw` tail, not
    /// the original keystroke history nor the nailed prefix; for
    /// `Phase::Composing` it is the single-segment raw; for `Phase::Idle` it
    /// is the empty string.
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
            Phase::Composing { raw, .. } | Phase::Continuous { raw, .. } => {
                crate::derived::derived_display(raw, config)
            }
        }
    }

    /// The composing-buffer surface the host renders in its single
    /// marked / composing region (`docs/engine/continuous-input-ranking.md`
    /// §10.2 / §10.4 invariant I1, Model B).
    ///
    /// - `Idle` → empty string.
    /// - `Composing { raw }` → derived display of `raw` (identity with
    ///   [`Phase::raw_input`]; no nailed segments exist).
    /// - `Continuous { raw, nailed }` → `Σ nailed[i].display_text`
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
            Phase::Composing { raw, .. } => crate::derived::derived_display(raw, config),
            Phase::Continuous { raw, nailed, .. } => combined_display(nailed, raw, config),
        }
    }
}

/// One **nailed** segment inside `Phase::Continuous`. "Nailed" means the
/// user accepted a candidate for this part of the buffer, but — under
/// Model B (`docs/engine/continuous-input-ranking.md` §10) — it is **NOT**
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
    pub raw_span: (usize, usize),
    pub syllable_count: u8,
}

/// v3.5.8 — word-boundary separator policy for the Model B continuous
/// composing buffer (`docs/engine/continuous-input-ranking.md` §10.2
/// segmented-spacing contract). A single ASCII space joins adjacent
/// nailed segments (and the nailed prefix ↔ pending tail) **only when
/// the rendered script is roman-ish**: roman-first, or both-scripts
/// (`hit (彼)`). Hanji-first (`is_translate_swapped` without
/// `output_both_scripts`) and TPS render the hanji/bopomofo as-is with
/// no inter-segment space. This mirrors the platform
/// `appendAutoSpaceIfApplicable` predicate so the marked region and the
/// final-commit auto-space stay consistent. `is_translate_swapped`
/// alone cannot distinguish hanji-first from both-scripts (both set it
/// `true`) — hence the `output_both_scripts` AppConfig field
/// (Codex pre-impl 2026-05-18).
fn continuous_word_space(config: &AppConfig) -> bool {
    let effective_swapped = config.is_translate_swapped || config.input_mode == "tps";
    // De Morgan of the platform `appendAutoSpaceIfApplicable` guard
    // `if (effectiveSwapped && !outputBothScripts) return`: roman-ish =
    // not swapped, OR both-scripts is on.
    !effective_swapped || config.output_both_scripts
}

/// Pure `Σ nailed[i].display_text` join, parameterized by the
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
///   internal `-` between run members; otherwise advance one segment
///   and emit a single space at the boundary.
///
/// Anti-overgluing (overlapping bigrams `AB` + `BC` without trigram
/// `ABC` → `A-B C`, not `A-B-C`) is a natural consequence of
/// leftmost longest-match; the unit tests below pin the matrix.
///
/// The separator is a render/commit join concern, **dictionary-informed**
/// for known n-syllable compounds, and is NEVER stored in
/// `NailedSegment.display_text` / `raw_text` — backspace-pop restores the
/// editable tail from `raw_text`, so segment data stays separator-free;
/// the hyphen is derived fresh on every render from the segments' own
/// `canonical_text` + `syllable_count`.
fn nailed_prefix_with_oracle(
    nailed: &[NailedSegment],
    space: bool,
    is_compound: impl Fn(&str, u8) -> bool,
) -> String {
    let mut s = String::new();
    let mut j = 0;
    while j < nailed.len() {
        let prev_hyphen = s.ends_with('-');
        let run_len = if !space || prev_hyphen {
            1
        } else {
            longest_compound_run(&nailed[j..], &is_compound)
        };

        if j > 0 && space && !prev_hyphen {
            s.push(' ');
        }
        for k in 0..run_len {
            if k > 0 {
                s.push('-');
            }
            s.push_str(&nailed[j + k].display_text);
        }
        j += run_len;
    }
    s
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
fn longest_compound_run<F: Fn(&str, u8) -> bool>(segs: &[NailedSegment], is_compound: &F) -> usize {
    let max_n = segs
        .iter()
        .take(MAX_COMPOUND_RUN)
        .take_while(|s| s.syllable_count == 1 && !s.display_text.ends_with('-'))
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
    let eligible = space
        && nailed.len() >= 2
        && nailed
            .windows(2)
            .any(|w| w[0].syllable_count == 1 && w[1].syllable_count == 1);
    if !eligible {
        return nailed_prefix_with_oracle(nailed, space, |_, _| false);
    }
    LexiconHandle::with_state(|state| {
        let (Some(prefix), Some(dict)) = (state.prefix_index.as_ref(), state.dictionary.as_ref())
        else {
            return Ok(nailed_prefix_with_oracle(nailed, space, |_, _| false));
        };
        Ok(nailed_prefix_with_oracle(nailed, space, |h, n| {
            compound_hanji_exists(h, n, prefix, dict)
        }))
    })
    .unwrap_or_else(|_| nailed_prefix_with_oracle(nailed, space, |_, _| false))
}

/// The Model B composing-buffer surface for a `(nailed, raw)` pair:
/// `Σ nailed[i].display_text` followed by the derived display of the
/// pending `raw` tail. **Single source of truth** — both
/// [`Phase::composing_display`] and the `transition.rs` Continuous paths
/// route through this so the rendered preedit and the hard-finalize commit
/// can never diverge (Codex post-impl review point).
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
    let mut s = nailed_prefix(nailed, config);
    let derived = crate::derived::derived_display(raw, config);
    // §10.2 word boundary between the nailed prefix and the pending
    // tail (the tail is the next word). Same predicate + trailing-`-`
    // suppression as the inter-segment join.
    if !s.is_empty() && !derived.is_empty() && continuous_word_space(config) && !s.ends_with('-') {
        s.push(' ');
    }
    let tail_start = s.len();
    s.push_str(&derived);
    (s, tail_start)
}

/// Engine state — the platform no longer shadows this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineState {
    pub phase: Phase,
    pub selected_candidate_index: i32,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            selected_candidate_index: -1,
        }
    }
}

/// Mirrors the iOS `ComposingState.Intent` / Android `ComposingState.Intent`
/// case set 1:1. Decoded from `protos::engine::ComposingRequest::method`
/// inside `dispatch::handle`.
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
    CommitDerived,
    CommitRaw,
    SelectSuggestion {
        text: String,
    },
    CommitPreeditThenInsertExternal {
        text: String,
    },
    Reset,
    SetSelectedCandidateIndex {
        index: i32,
    },
    QueryState,
    EnterContinuous,
    /// v3.5.8 Phase 6 — pure read of span-local continuous-input
    /// candidates for the current `Phase::Continuous { raw }` starting
    /// at `position` (always `0` in v3.5.8; non-zero short-circuits
    /// to an empty candidate list). Resolved by `dispatch::handle`
    /// outside the `transition::apply` pure path because the fetch
    /// needs lexicon state — see `dispatch::handle_fetch_at_pos`.
    /// `transition.rs` only sees this variant via a defensive snapshot
    /// arm; production callers always go through dispatch.
    ///
    /// v3.5.8 Phase 9.3a — carries the per-candidate
    /// `user_frequency.db` snapshot (`frequency_entries`, keyed by
    /// `display_text_key = hanji ?? roman`) and the platform wall
    /// clock (`now_ms`, epoch-ms). Both fields are decoded verbatim
    /// from `FetchAtPos { frequency_entries, now_ms }` and threaded
    /// straight to `handle_fetch_at_pos`. Empty list + `now_ms = 0`
    /// is the backward-compatible "no user-freq plumbing yet" mode
    /// that reproduces PR-9.2 behavior (neutral 1.0 boost, rank 1
    /// everywhere).
    ///
    /// v3.5.8 Phase 9 Item 12 — `custom_entries` carries the
    /// platform's `custom_dictionary.db` matches for the current raw
    /// buffer (raw stored `(roman, hanji)` columns; DB stays native).
    /// Decoded verbatim from `FetchAtPos.custom_entries` and threaded
    /// to `handle_fetch_at_pos`, which synthesizes a full-buffer
    /// `RawCandidate` per entry and dedupes `(roman, hanji)` against
    /// the FST hits. Empty list = no custom matches / feature
    /// disabled — backward-compatible no-op.
    ///
    /// **v3.5.9 B-4** — `roman` may legitimately be either TL or POJ
    /// display form (whichever the user typed when storing the
    /// entry). The engine treats it as raw on the lattice / dedupe
    /// axis (`composing::shadow::custom_toneless_key` canonicalizes
    /// per-mode for the FST family) and folds it through
    /// `phonetics::api::canonical_tl_form` only when synthesizing the
    /// `user_frequency.db` commit key (`display_text`), keeping the
    /// commit key mode-invariant.
    /// PR-9.6 — `enabled_sources_bitmask` carries the user's dictionary
    /// source-toggle state so continuous candidates honour the same
    /// toggles as Tab3 browse. Decoded verbatim from
    /// `FetchAtPos.enabled_sources_bitmask`; the `0`-means-absent →
    /// `u32::MAX` sentinel is resolved in `handle_fetch_at_pos`. Full
    /// wire/sentinel contract: the `FetchAtPos` proto comment.
    /// §34 / S22 — `literal_roman_candidate_disabled` gates the always-on
    /// preedit-literal roman candidate (index-0 `derived_display` WYSIWYG row
    /// for 漢羅 one-tap). Decoded verbatim from `FetchAtPos`; OFF suppresses
    /// only the §34 forced prepend, not the natural roman candidates. Full
    /// wire/sentinel contract: the `FetchAtPos` proto comment.
    FetchAtPos {
        position: u32,
        frequency_entries: Vec<protos::engine::FrequencyEntry>,
        now_ms: i64,
        custom_entries: Vec<protos::engine::CustomDictEntry>,
        enabled_sources_bitmask: u32,
        literal_roman_candidate_disabled: bool,
    },
    /// Nail a candidate segment in `Phase::Continuous`. The engine takes
    /// `pending[..consumed_bytes]` as the nailed segment's raw text and
    /// keeps `pending[consumed_bytes..]` as the new pending tail. When
    /// `consumed_bytes >= pending.len()`, this becomes a final commit and
    /// exits to Idle. Caller (Phase 6+ proto layer) is responsible for
    /// `consumed_bytes` aligning with both UTF-8 char boundaries and TL
    /// syllable boundaries returned by the syllabifier.
    CommitContinuous {
        display_text: String,
        // v3.5.8 Phase 9 Bug 1 (Option A): canonical key for freq/NextWord.
        // Empty → engine falls back to `display_text` (legacy callers).
        canonical_text: String,
        // v3.6.1 R2: canonical TL of the committed candidate. Becomes the
        // NextWord `roman` arg (→ `prev_tl`/`next_tl`); empty → engine
        // falls back to the raw committed slice (legacy / TPS-OOV).
        association_tl: String,
        consumed_bytes: usize,
        syllable_count: u8,
    },
    ResetContinuous,
    /// Desktop Telex scheme — one tone / affricate / hyphen letter applied
    /// to the pending tail (`telex::apply_telex_key`). Unknown keys and
    /// edits that change nothing answer with a no-op.
    TelexKey {
        key: String,
    },
    /// Desktop caret — step one char inside the pending tail; see the
    /// `MoveCaret` proto comment for the contract (no refetch, edge = no-op).
    /// `None` is a wire direction the engine does not know (unspecified or
    /// newer than this build) and steps nowhere.
    MoveCaret {
        direction: Option<CaretDirection>,
    },
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

    /// Pure read — no state mutation, no effects emitted. Used by
    /// `Intent::QueryState` and the generation-mismatch path's post-drop
    /// re-snapshot. `config` is the request's `AppConfig` so
    /// `Preedit.display_text` reflects the caller's actual mode/toggles
    /// (per Codex PR #197 r3169707395).
    pub fn snapshot(&self, config: &AppConfig) -> ComposingResponse {
        crate::transition::apply(&mut self.state.clone(), Intent::QueryState, config)
    }

    /// Apply `intent` against the current state, mutate, and return the
    /// resulting response (preedit + ordered effects + new index +
    /// is_composing). Delegates to the pure transition table in
    /// `transition.rs`.
    pub fn apply(&mut self, intent: Intent, config: &AppConfig) -> ComposingResponse {
        crate::transition::apply(&mut self.state, intent, config)
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
    /// `dispatch::handle`. NOT public API — external callers always go
    /// through `dispatch::handle`. The user-initiated `Intent::Reset` path
    /// goes through `apply(Intent::Reset, ...)`, which emits the
    /// `ClearPreeditWithoutCommit + ResetAutocomplete` effects when
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
        combined_display, nailed_prefix, nailed_prefix_with_oracle, AppConfig, NailedSegment,
    };

    fn seg(display: &str) -> NailedSegment {
        NailedSegment {
            display_text: display.to_owned(),
            canonical_text: display.to_owned(),
            raw_text: display.to_owned(),
            association_tl: display.to_owned(),
            raw_span: (0, display.len()),
            syllable_count: 1,
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
            raw_span: (0, display.len()),
            syllable_count,
        }
    }

    /// `input_mode` + the two swap flags are the only fields the
    /// separator predicate reads; the rest stay at proto defaults.
    fn cfg(input_mode: &str, swapped: bool, both: bool) -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: input_mode.to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: swapped,
            is_association_recording_enabled: false,
            platform_id: 0,
            output_both_scripts: both,
            candidate_display_mode: 0,
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
        // is_translate_swapped without output_both_scripts → hanji-first.
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
        // input_mode == "tps" → effective_swapped regardless of flag.
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
        assert_eq!(nailed_prefix_with_oracle(&n, true, |_, _| false), "hit tui");
        // Hanji-first / TPS (space = false) → no separator at all,
        // oracle irrelevant.
        let h = [seg("彼"), seg("隻")];
        assert_eq!(nailed_prefix_with_oracle(&h, false, |_, _| true), "彼隻");
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
            nailed_prefix_with_oracle(&n, true, |h, n| n == 2 && h == "查某"),
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
            nailed_prefix_with_oracle(&n, true, |h, n| matches!(
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
            nailed_prefix_with_oracle(&n, true, |h, n| n == 2 && (h == "查某" || h == "某人")),
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
            nailed_prefix_with_oracle(&n, true, |h, n| n == 2 && (h == "查某" || h == "麻煩")),
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
            nailed_prefix_with_oracle(&n, true, |_, _| true),
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
            nailed_prefix_with_oracle(&n, true, |h, n| n == 2 && h == "台灣"),
            "tai-uan"
        );
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
            nailed_prefix_with_oracle(&n, true, |h, n| {
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
            nailed_prefix_with_oracle(&n, true, |h, n| n == 2 && h == "灣國"),
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
}
