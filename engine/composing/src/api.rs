//! Public façade for the composing crate. Defines `Engine`, `EngineState`,
//! `Phase`, `Intent`, and `ComposingError`. Implementation of state
//! transitions lives in `transition.rs`; this module is the stable surface
//! that `requests.rs` and external crates consume.

pub use crate::conversion::{Conversion, ConversionFrequency, ConvertedSegment};
use crate::handle::{ListContext, PendingSnapshot};
use crate::nailed::combined_display;
pub use crate::nailed::NailedSegment;
use crate::WordListStart;
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
    /// The other script of the lead (desktop Output the Other Script); a
    /// one-script pick writes its one script.
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
    /// §10.2 segmented-spacing contract: adjacent pieces (nailed segments,
    /// and the nailed prefix ↔ pending tail) are joined by a single space
    /// when both meeting edges are romanization (or `output_both_scripts`
    /// annotates every pick in brackets); Hanji, Bopomofo, and Hanji↔roman
    /// script transitions join with no separator. See
    /// [`has_word_space_boundary`] / [`nailed_prefix`]. This is the exact
    /// string a hard finalize (Enter / final-commit) writes to the
    /// document, so the separator policy applies identically there.
    ///
    /// [`has_word_space_boundary`]: crate::nailed::has_word_space_boundary
    /// [`nailed_prefix`]: crate::nailed::nailed_prefix
    pub fn composing_display(&self, config: &AppConfig) -> String {
        match self {
            Phase::Idle => String::new(),
            Phase::Continuous { raw, nailed, .. } => combined_display(nailed, raw, config),
        }
    }
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

/// A candidate accepted by a commit-and-insert key. Values come from the
/// same fetched candidate as an ordinary `CommitContinuous` pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinuousPick {
    pub canonical_text: String,
    pub association_tl: String,
    pub hanji: Option<String>,
    pub consumed_bytes: usize,
    pub syllable_count: u8,
    pub script: Option<CommitScript>,
    pub roman: String,
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
    /// Accept a highlighted candidate (when present), finish the composition
    /// as shown and append the literal with its auto space in one write.
    ConfirmCompositionThenInsert {
        candidate: Option<ContinuousPick>,
        text: String,
        auto_space_enabled: bool,
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
    /// The `dictionary.bin` source filter the fetch applies, the same as Tab3
    /// browse, is the request config's `dictionary_sources`, resolved when the
    /// fetch is answered (`crate::requests::source_filter_bitmask`).
    /// §34 / S22 — `literal_roman_candidate_disabled` gates the always-on
    /// preedit-literal roman candidate (index-0 `derived_display` WYSIWYG row
    /// for Hanji-romanization one-tap). Decoded verbatim from `FetchAtPos`; OFF suppresses
    /// only the §34 forced prepend, not the natural roman candidates. Full
    /// wire/sentinel contract: the `FetchAtPos` proto comment.
    FetchAtPos {
        now_ms: i64,
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
    /// A NextWord prediction tap (D1) — contract on the `CommitPrediction` proto.
    CommitPrediction {
        /// `None` (the wire's `UNSPECIFIED`, or a newer script) ignores it.
        script: Option<CommitScript>,
        /// The prediction's display romanization; empty for Hanji-only.
        roman: String,
        /// Identity and the Hanji half; empty ignores the tap.
        hanji: String,
        /// The canonical TL the usage is counted under.
        canonical_tl: String,
    },
    /// Desktop Telex scheme — one tone letter applied
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

    /// What a `FetchAtPos` ranks against (`PendingSnapshot`): the text it
    /// lists ([`crate::WordListStart::listed`] — part of the pending tail, or
    /// the last nailed segment's raw text followed by the tail) and the word
    /// that text follows — the converted word ending where it starts, else the
    /// segment nailed before it — when that is a word
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
        let start = self
            .word_list_start(config)
            .unwrap_or(WordListStart::InTail(0));
        let (listed_raw, list_start) = start.listed(nailed, raw);
        // The segments nailed before the listed text: the last nailed
        // segment's list reads that segment itself.
        let nailed_before = match start {
            WordListStart::LastNailed => nailed.split_last().map_or(&[][..], |(_, rest)| rest),
            WordListStart::InTail(_) => &nailed[..],
        };
        let segment_before = if list_start == 0 {
            nailed_before.last().cloned()
        } else {
            conversion
                .as_ref()
                .and_then(|conversion| conversion.word_ending_at(list_start))
                .map(|word| word.nailed_as_shown(raw, &[]))
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
            listed_raw,
            context,
        }
    }

    /// Where the list of the word before the caret starts — in the pending
    /// tail, or at the last nailed segment — under a Hanji conversion the
    /// request asks for (`crate::conversion::word_list_start`); `None` for the
    /// whole tail's list. A pick resolves the same start
    /// (`transition::commit_continuous`).
    pub fn word_list_start(&self, config: &AppConfig) -> Option<WordListStart> {
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
