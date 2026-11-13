//! Span-local candidate fetch for continuous input.
//!
//! Given a **mode-canonical** input (TL ASCII for TL/English, POJ ASCII
//! for POJ, Bopomofo for TPS — all canonicalized upstream by
//! `composing::shadow::canonicalize_poj_shadow`), a starting byte
//! position, and the valid syllable-end byte offsets produced by
//! `composing::syllabifier`, return every dictionary candidate whose
//! toneless key (`<prefix>:<toneless>`, `prefix ∈ {tl, poj, tps}`) equals
//! `input[pos..end]` for some `end`. Each candidate is scored via
//! [`ranking::calculate_continuous_score`] and tagged with the consumed
//! byte span and syllable count so the UI can decide what to commit.
//!
//! # Why span-local lookup
//!
//! Pure longest-match (khiin-rs `khiin/src/data/segmenter.rs`) would
//! commit `tsua` to span 4 and lose the `珠 (tsu, span=3)` candidate. A
//! global lattice (librime `src/rime/algo/syllabifier.cc`) is over-built
//! for our scope. Multi-cut span-local fetch is the middle ground.
//!
//! Multi-syllable candidates (e.g. `珠仔`) and single-syllable candidates
//! (e.g. `紙`) under the same toneless key (`tl:tsua`) are distinguished
//! by `DictionaryRecord::syllable_count`.
//!
//! # Contracts
//!
//! - **`input` MUST be mode-canonical** (lowercase or mixed-case): TL
//!   ASCII under TL/English, POJ ASCII under POJ, Bopomofo under TPS. All
//!   modes walk the same shadow → lattice path in `composing` and emit
//!   keys in their own FST family (`phonetics::KeyFamily::for_input_mode`).
//! - `endings` SHOULD be ascending UTF-8 char boundaries within
//!   `input[pos..]`. Out-of-range or non-boundary endings are silently
//!   skipped (matches the syllabifier's safe contract).
//! - `enabled_sources_bitmask` follows the same wire format as
//!   `lexicon::search()` — bit 12 = variant gate, bit 9 = khiin gate,
//!   bits 0..=11 = per-source enables, `u32::MAX` = all sources on.
//! - User-frequency input is plumbed through [`ContinuousFetchCtx`] —
//!   `freq_map` (`FrequencyData` keyed by the `(display_text,
//!   canonical_tl)` PAIR identity, Core Principle #7) plus `now_ms`
//!   (platform epoch-ms). The engine derives `user_freq_boost(count)` per
//!   candidate inside `record_to_candidate` / `custom_entry_to_candidate`
//!   using `BOOST_ALPHA` / `MAX_BOOST` from `ranking::score`; the
//!   platform's `user_frequency.db` stays native. Cold-start safe
//!   defaults = `&FrequencyMap::new()` + `now_ms = 0` (boost = 1.0,
//!   user_weight = 0.0 everywhere).
//!
//! # Ordering
//!
//! Returned candidates are sorted by the `CandidateSortKey` documented at
//! [`fetch_candidates_for_keys_with_barriers`]; the corpus `walker_cost` and
//! `calculate_continuous_score` provide two of its dimensions.
//! Within-tier ties keep insertion order (`endings` order, then
//! `prefix_index` rowid order — `lookup_exact` is deterministic per
//! build). The comparator coerces `NaN` scores to `f64::MIN` so even a
//! contract-violating boost cannot break the ordering invariant.
//!
//! The production entry is [`fetch_candidates_for_keys_with_barriers`],
//! called from `composing::continuous::assemble_candidates`; the
//! test-only `fetch_candidates_for_endings` wrapper (pre-computed
//! syllabifier endings → `(span, key)` pairs) lives in
//! `engine/lexicon/tests/common/mod.rs`.

use crate::dictionary_reader::DictionaryReader;
use crate::prefix_index::PrefixIndex;
use ranking::{CandidateRankFacts, ContextRanks, FrequencyMap};

mod abbrev;
mod candidate;
mod compound;
mod edge;
mod exact;
mod partial_prefix;
mod reach;
mod tone_pin;
mod toneless_match;

use reach::{custom_abbreviation_matches, custom_reach, fused_custom_typed_key, CustomReach};

pub use abbrev::fetch_abbrev_candidates;
pub use compound::compound_hanji_exists;
pub use edge::{best_candidate_for_key_with_barriers, homophone_words_for_key, EdgeBest};
pub use exact::fetch_candidates_for_keys_with_barriers;
pub use partial_prefix::{
    fetch_partial_prefix_candidates, fetch_partial_prefix_candidates_unbounded,
};
pub use tone_pin::{TonePin, TpsClose, TpsCloseKind, TypedBoundary};

/// `RawCandidate.coverage_kind` ordinals, owned by the sort that reads them.
/// Which fetcher stamps which: `fetch_candidates_for_keys_with_barriers`
/// (`prefix_index.lookup_exact`) → [`COVERAGE_KIND_FULL`];
/// [`fetch_partial_prefix_candidates`] (`prefix_index.lookup_prefix`, the
/// syllabifier found no ending) → [`COVERAGE_KIND_PARTIAL_PREFIX`];
/// [`fetch_abbrev_candidates`] (the buffer equals a record's
/// `tl_abbrev` / `poj_abbrev` / `tps_abbrev` face) → [`COVERAGE_KIND_ABBREV`].
pub use ranking::{COVERAGE_KIND_ABBREV, COVERAGE_KIND_FULL, COVERAGE_KIND_PARTIAL_PREFIX};

/// Worst-case rowid hydration budget per partial-prefix lookup.
/// Single-char prefixes (`tl:k`, `tl:t`) match hundreds of FST entries;
/// this caps `dict.record(rowid)` calls so per-keystroke work stays
/// bounded on the iOS keyboard-extension RAM/latency budget. Set well
/// above [`PARTIAL_PREFIX_OUTPUT_CAP`] so high-frequency short
/// candidates landing past the legacy byte-sort first 30 still reach
/// the [`CandidateSortKey`] sort and can win on score / freq rather than be
/// silently dropped at the rowid stage. See
/// `docs/engine/continuous-candidate-display.md` §15.8.
///
/// Resolves the pre-v3.5.9 R6 limitation: the old single `take(30)`
/// pre-cap was ranking-blind, so for input `tl:k` the FST byte-sort
/// front-loaded `tl:ka-*` multi-syllable phrases and evicted
/// high-frequency single-syllable entries like `tl:ki` before any
/// scoring ran.
///
/// [`CandidateSortKey`]: ranking::CandidateSortKey
pub const PARTIAL_PREFIX_HYDRATE_CAP: usize = 500;

/// Maximum candidates returned from [`fetch_partial_prefix_candidates`]
/// to the platform candidate strip. Applied AFTER hydration, dedupe,
/// and [`CandidateSortKey`] sort — so the top-N visible to the user is the
/// globally best-scoring subset of the (up to
/// [`PARTIAL_PREFIX_HYDRATE_CAP`]) hydrated pool, not the FST
/// byte-sort prefix. Matches the legacy `LexiconService` per-request
/// output size to keep the candidate strip visually stable.
///
/// [`CandidateSortKey`]: ranking::CandidateSortKey
pub const PARTIAL_PREFIX_OUTPUT_CAP: usize = 30;

/// One span-local candidate. Mirrors the 5-field shape pinned by
/// `docs/releases/v3.5.8/plan.md` § Phase 5 — Each Candidate carries (5-field shape).
#[derive(Debug, Clone, PartialEq)]
pub struct RawCandidate {
    /// The `dictionary.bin` rowid this candidate was read from
    /// ([`DictionaryRecord::rowid`](crate::dictionary_reader::DictionaryRecord::rowid));
    /// [`NO_ROWID`](crate::dictionary_reader::NO_ROWID) for a candidate that
    /// is no dictionary row. The walker looks its pick's bigram word up by it
    /// (`docs/architecture/bigram-lm-roadmap.md` D6). Internal — NOT emitted
    /// on `CandidateMessage`.
    pub rowid: u32,
    /// Byte span `(start, end)` in the original input that this candidate
    /// consumes on commit. `start` always equals the `pos` plumbed
    /// through [`fetch_candidates_for_keys_with_barriers`] (the production
    /// entry; the test-only `fetch_candidates_for_endings` wrapper in
    /// `engine/lexicon/tests/common/mod.rs` preserves the same contract);
    /// `end` is one of the offsets in `endings`.
    pub consumed_span: (u32, u32),
    /// Number of TL syllables in the matched dictionary entry, copied
    /// from `DictionaryRecord::syllable_count` (1..=4 by builder cap).
    pub syllable_count: u8,
    /// What the user sees / what gets committed: hanji if available,
    /// otherwise the stored TL romanization. Engine-authoritative
    /// commit key + `user_frequency.db` write key on both platforms.
    pub display_text: String,
    /// v3.5.8 Phase 9 Item 5 — display romanization carried alongside
    /// `display_text` so platform UI can render dual-line cells
    /// (roman line + hanji line) the same way the legacy lexicon
    /// path does. At this lexicon layer it equals the underlying
    /// `DictionaryRecord.tl`; `requests::handle_fetch_at_pos` then
    /// applies the presentation transforms — per-segment recasing and,
    /// in POJ input mode, a TL→POJ-display rewrite (`oo`→`o͘`,
    /// `nn`→`ⁿ`, …) — before emission. NEVER consulted for the engine
    /// commit (which goes through `display_text`); the platform formats
    /// its document string from this presentation roman.
    pub roman: String,
    /// v3.5.8 Phase 9 Item 5 — hanji display carried alongside
    /// `display_text`. `None` iff `DictionaryRecord.hanji.is_none()`
    /// (TAILO candidate); `Some` otherwise. On the wire this maps to
    /// `optional string hanji` so consumers can distinguish "TAILO
    /// — no hanji exists" from "wire-frame defect / absent field"
    /// (per `docs/engine/continuous-candidate-display.md` §4.2). UI
    /// uses this as the dual-line cell subtitle; engine commit still
    /// goes through `display_text`.
    pub hanji: Option<String>,
    /// v3.6.1 R2 — canonical TL romanization, the identity sidechannel
    /// for the `(hanji, canonical-TL)` word-identity pair (Core
    /// Principle #7). Unlike [`roman`] (the DISPLAY romanization that
    /// `requests::handle_fetch_at_pos` recases per typed segment and
    /// rewrites TL→POJ in POJ mode), this stays the canonical TL:
    /// `DictionaryRecord.tl` for `dict.bin` hits ([`record_to_candidate`]),
    /// `phonetics::api::canonical_tl_form(roman, mode)` for custom
    /// ([`custom_entry_to_candidate`](candidate::custom_entry_to_candidate)) and walker-synth candidates.
    /// Emitted on `CandidateMessage.canonical_tl`; the platform
    /// round-trips it back into `CommitContinuous.association_tl` so the
    /// NextWord association learns the same TL a normal candidate commit
    /// records (fixes continuous-vs-normal `next_tl` fragmentation). Empty
    /// only when no canonical TL is recoverable (TPS-OOV hanji-absent) —
    /// platform omits `association_tl` and the engine falls back to the
    /// raw committed slice. NEVER consulted for the document commit.
    ///
    /// [`record_to_candidate`]: candidate::record_to_candidate
    pub canonical_tl: String,
    /// Result of [`ranking::calculate_continuous_score`].
    pub score: f32,
    /// Raw dictionary frequency before any bias / boost. Carried
    /// alongside the multiplicative `score` so the v3.5.8 Phase 9.1
    /// `CandidateSortKey` can use raw freq as an explicit tie-break dimension
    /// distinct from `adjusted_score`. Always equals
    /// `DictionaryRecord::frequency` for candidates produced by
    /// `fetch_candidates_for_keys_with_barriers`.
    pub frequency: u32,
    /// The record's `dict.bin` `walker_cost` (corpus `−ln p`, milli-nats) —
    /// the [`CandidateSortKey`] dimension above `score` that the list and the
    /// walker's edge pick share (E1 P5b). [`WALKER_COST_UNPRICED`] for a
    /// candidate the corpus does not price: custom, learned, the walker's
    /// slot-0 synth, the literal roman row. Internal — NOT emitted on
    /// `CandidateMessage`.
    ///
    /// [`CandidateSortKey`]: ranking::CandidateSortKey
    /// [`WALKER_COST_UNPRICED`]: crate::dictionary_reader::WALKER_COST_UNPRICED
    pub walker_cost: u16,
    /// Dictionary source bitmask copied verbatim from
    /// [`DictionaryRecord::bitmask`](crate::dictionary_reader::DictionaryRecord::bitmask). Used by the v3.5.8 Phase 9.1
    /// `CandidateSortKey` to derive `source_tier_rank` at sort time per
    /// `docs/releases/v3.5.8/plan.md` § Phase 9. Carrying it on the candidate (vs.
    /// re-reading the dictionary record) lets the sort be a pure
    /// function of the returned `RawCandidate` vector.
    pub bitmask: u16,
    /// Time-decayed user-selection weight for this candidate's
    /// `(display_text, canonical_tl)` pair
    /// ([`ranking::FrequencyData::user_weight`]; `0.0` = never selected,
    /// no wall clock, or clock-skew). Leading user-preference dimension
    /// of [`CandidateSortKey`] and of the walker's per-edge pick
    /// ([`best_candidate_for_key_with_barriers`]) — any selected word
    /// outranks every never-selected homophone. Rationale:
    /// `docs/engine/continuous-input-ranking.md` §3.2. Internal — NOT
    /// emitted on `CandidateMessage`.
    ///
    /// [`CandidateSortKey`]: ranking::CandidateSortKey
    pub user_weight: f64,
    /// Whether the previous word's bigram continuations include this
    /// candidate, and from which layer (`ranking::CONTEXT_RANK_*`): the
    /// `CandidateSortKey` dimension below `user_weight` and above `score` (§56).
    pub context_rank: u8,
    /// v3.5.8 Phase 9 Item 10 — coverage kind for the new partial-prefix
    /// path. [`COVERAGE_KIND_FULL`] for the existing
    /// `fetch_candidates_for_keys_with_barriers` lookup-exact path;
    /// [`COVERAGE_KIND_PARTIAL_PREFIX`] for
    /// [`fetch_partial_prefix_candidates`] hits.
    ///
    /// Internal axis only — does NOT enter `CandidateMessage` (same
    /// pattern as [`user_weight`](Self::user_weight); see
    /// `docs/engine/continuous-candidate-display.md` §15.5). Consumed
    /// solely by [`CandidateSortKey`], below `tier`, `abbreviation` and
    /// `preference_class`: within a tier and preference class a
    /// partial-prefix candidate follows every full-syllable one,
    /// irrespective of user weight, score, dict freq, or source rank.
    ///
    /// [`CandidateSortKey`]: ranking::CandidateSortKey
    pub coverage_kind: u8,
    /// v3.5.8 Phase 9 Item 12 — `true` for candidates synthesized from
    /// a `custom_dictionary.db` entry ([`custom_entry_to_candidate`](candidate::custom_entry_to_candidate)),
    /// `false` for `dict.bin` FST hits ([`record_to_candidate`]). Read
    /// by [`CandidateSortKey::new`] (`preference_class` places a
    /// never-selected custom word after the selected words and before the
    /// rest; `source_tier_rank(bitmask, is_custom)` returns rank `0` when
    /// `true`, ahead of every `dict.bin` source tier) and by the
    /// `(canonical_tl, hanji)` dedupe winner policy
    /// (`docs/engine/continuous-commit-and-display.md` §10.10): on a
    /// duplicate `(canonical_tl, hanji)` identity the lowest `source_tier_rank`
    /// survivor wins, so a custom entry always beats a `dict.bin`
    /// duplicate. Internal axis only — NOT emitted on
    /// `CandidateMessage` (same pattern as [`coverage_kind`] /
    /// `user_weight`).
    ///
    /// [`CandidateSortKey::new`]: ranking::CandidateSortKey::new
    /// [`record_to_candidate`]: candidate::record_to_candidate
    pub is_custom: bool,
}

impl RawCandidate {
    /// The fields [`CandidateSortKey`] orders by.
    ///
    /// [`CandidateSortKey`]: ranking::CandidateSortKey
    pub fn rank_facts(&self) -> CandidateRankFacts {
        CandidateRankFacts {
            coverage_kind: self.coverage_kind,
            consumed_span: self.consumed_span,
            user_weight: self.user_weight,
            context_rank: self.context_rank,
            walker_cost: self.walker_cost,
            score: self.score,
            frequency: self.frequency,
            bitmask: self.bitmask,
            is_custom: self.is_custom,
        }
    }
}

/// v3.5.8 Phase 9 Item 12 — one `custom_dictionary.db` row, as the engine
/// reads it from its store (`composing::UserRows`). `roman` / `hanji` are
/// the raw stored columns, verbatim (no display capitalization); the
/// engine folds `roman` to `canonical_tl` (NFC, POJ → TL) so its
/// `(canonical_tl, hanji)` dedupe key collides correctly against
/// `dict.bin`'s `DictionaryRecord.tl` / `.hanji`. `hanji = None`
/// is a romanization-only custom entry (mirrors
/// `DictionaryRecord.hanji` / `RawCandidate.hanji` `Option` semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomEntry {
    pub roman: String,
    pub hanji: Option<String>,
}

/// Learned phrases (§50) — one auto-learned `(Hanji, canonical-TL)` pair, as
/// the engine reads it from its store (`composing::UserRows`) and as a final
/// commit teaches it (`composing::Applied`). Unlike [`CustomEntry`] both
/// fields are canonical (the engine derived them), so a learned row is ranked exactly like a
/// `dict.bin` record with no frequency and no source bits: it competes in
/// the same [`CandidateSortKey`] pick and never overrides
/// (`docs/architecture/behavioral-invariants.md` §50).
///
/// [`CandidateSortKey`]: ranking::CandidateSortKey
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnedEntry {
    pub hanji: String,
    pub canonical_tl: String,
}

/// Shared context for the continuous-input fetch entry points
/// ([`fetch_candidates_for_keys_with_barriers`],
/// [`fetch_partial_prefix_candidates`]): the concerns every fetch needs
/// (filter / freq-map / clock / custom / readers / mode / space pin)
/// bundled into one borrowed struct so call sites pass three or four
/// args instead of eight.
///
/// All fields are `pub` — construction is always a stack-local struct
/// literal at the call site (production builds it inside the composing
/// seam; integration tests build it once per test). No constructor is
/// needed.
pub struct ContinuousFetchCtx<'a> {
    /// `Filter::from_enabled_bitmask` input — production passes the user's
    /// source filter (`composing::requests::fetch_at_pos_intent`,
    /// `u32::MAX` = every source); tests narrow it to verify filter behaviour.
    pub enabled_sources_bitmask: u32,
    /// User-selection snapshot keyed by the `(display_text, canonical_tl)`
    /// pair (Core Principle #7). Empty map + `now_ms = 0` is the
    /// cold-start neutral.
    pub freq_map: &'a FrequencyMap,
    /// Platform epoch-ms wall clock at fetch time.
    pub now_ms: i64,
    /// The previous word's continuations (`dispatch::context`), ranked by
    /// layer; `ContextRanks::empty()` = no context, the context-free order.
    pub context: &'a ContextRanks,
    /// `custom_dictionary.db` hits to merge into the candidate list.
    /// Empty slice = no custom merge (the production wiring's
    /// cold-start default).
    pub custom: &'a [CustomEntry],
    /// Learned phrases (§50) whose whole-buffer key the platform matched
    /// exactly against the raw buffer; merged as `(0, raw_len)` rows like
    /// `custom`. Empty slice = feature off / un-wired.
    pub learned: &'a [LearnedEntry],
    /// FST prefix index reader.
    pub prefix_index: &'a PrefixIndex,
    /// Dictionary record reader (mmap-backed).
    pub dict: &'a DictionaryReader,
    /// v3.5.9 B-4 — active input mode for the fetch. Threaded through
    /// so [`custom_entry_to_candidate`](candidate::custom_entry_to_candidate) can canonicalize
    /// hanji-absent `display_text` to TL form via
    /// [`phonetics::api::canonical_tl_form`], keeping the
    /// `user_frequency.db` commit key mode-invariant. Lattice / FST
    /// key prefix selection (`tl:` vs `poj:`) is decided upstream and
    /// embedded in `keys` already — this field exists solely to fold
    /// the romanization fallback for the freq key, NOT to alter
    /// dictionary lookup.
    pub mode: phonetics::InputMode,
    /// The whole typed buffer's [`TonePin`] — the tone constraint the
    /// sources that carry no span key of their own answer to:
    /// `custom_dictionary.db` entries are synthesized at `(0, raw_len)`,
    /// and partial-prefix extensions are hydrated from a strict prefix of
    /// the buffer, so neither has a per-key pin to align against.
    /// Dictionary hits on the exact / walker paths carry their own
    /// per-key pin instead. [`TonePin::None`] is the legacy all-tones
    /// behavior.
    pub tone_pin: TonePin,
    /// The whole typed buffer's partial-prefix key (`tl:tankiant`) — what a
    /// `custom_dictionary.db` entry is measured against on the exact
    /// span-local path, which has no whole-buffer key of its own
    /// ([`custom_reach`]). `None` admits no custom entry there: there is
    /// nothing typed to match it to.
    pub whole_buffer_key: Option<String>,
}

/// Span aliases for [`fetch_candidates_for_keys_with_barriers`]: `(start_byte, end_byte)`
/// in the user-facing input buffer (TL ASCII or TPS Bopomofo bytes,
/// depending on caller). The engine only stores these verbatim in the
/// returned `RawCandidate.consumed_span`; FST lookup uses the paired key.
pub type ConsumedSpan = (u32, u32);
