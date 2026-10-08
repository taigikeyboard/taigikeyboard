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

use phonetics::{KeyFamily, HANJI_KEY_PREFIX};
use unicode_normalization::UnicodeNormalization;

use crate::dictionary_reader::{DictionaryReader, Filter, WALKER_COST_UNPRICED};
use crate::prefix_index::PrefixIndex;
use ranking::{
    sort_by_candidate_key, CandidateRankFacts, CandidateSortKey, ContextRanks, FrequencyMap,
};

mod candidate;
mod toneless_match;

use candidate::{
    dedupe_by_roman_hanji_span, learned_entry_to_candidate, merge_custom_dedupe_sort,
    record_to_candidate,
};
use toneless_match::{
    matches_continuous_toneless_key, matches_continuous_toneless_prefix_key, tps_face_starts_with,
    with_tps_or_variant, KeyFace,
};

/// `RawCandidate.form` discriminator. Every candidate this module emits
/// carries the notone form: span-local keys are `<prefix>:<toneless>`
/// bodies, and the tone-pinned (`tl_num` / `poj_num`) keys the same
/// guards accept resolve to the same records. The other form ordinals
/// (hanji 0 / numeric 2 / abbrev 3) are reserved for carriers the proto
/// side does not have.
pub const FORM_NOTONE: u8 = 1;

/// v3.5.8 Phase 9 Item 10 — `RawCandidate.coverage_kind` ordinal for
/// full-syllable hits (the pre-Item-10 path: `valid_span_endings`
/// returned at least one ending and `fetch_candidates_for_keys_with_barriers`
/// produced the candidate via `prefix_index.lookup_exact`).
pub const COVERAGE_KIND_FULL: u8 = 0;

/// v3.5.8 Phase 9 Item 10 — `RawCandidate.coverage_kind` ordinal for
/// partial-prefix hits (syllabifier returned no ending, engine fell
/// through to [`fetch_partial_prefix_candidates`] via
/// `prefix_index.lookup_prefix`). Ranks strictly below
/// [`COVERAGE_KIND_FULL`] in [`CandidateSortKey`] regardless of any other
/// dimension; see `docs/engine/continuous-candidate-display.md` §15.5.
pub const COVERAGE_KIND_PARTIAL_PREFIX: u8 = 1;

/// `RawCandidate.coverage_kind` ordinal for whole-buffer abbreviation hits
/// ([`fetch_abbrev_candidates`]): the typed buffer is an acronym-shaped
/// string (`ss`, `tk`, `ㄙㄒ`) and the record's per-syllable-initial face
/// (`tl_abbrev` / `poj_abbrev` / `tps_abbrev`) equals it. Ranks strictly
/// below [`COVERAGE_KIND_PARTIAL_PREFIX`] in [`CandidateSortKey`]: an acronym-shaped
/// buffer that is also a valid onset (`ts`, `kh`) is mid-syllable at least
/// as often as it is an abbreviation, so the single-syllable partial hits
/// the user was reaching for keep their positions and the abbreviated
/// words trail them.
pub const COVERAGE_KIND_ABBREV: u8 = 2;

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
pub const PARTIAL_PREFIX_HYDRATE_CAP: usize = 500;

/// Maximum candidates returned from [`fetch_partial_prefix_candidates`]
/// to the platform candidate strip. Applied AFTER hydration, dedupe,
/// and [`CandidateSortKey`] sort — so the top-N visible to the user is the
/// globally best-scoring subset of the (up to
/// [`PARTIAL_PREFIX_HYDRATE_CAP`]) hydrated pool, not the FST
/// byte-sort prefix. Matches the legacy `LexiconService` per-request
/// output size to keep the candidate strip visually stable.
pub const PARTIAL_PREFIX_OUTPUT_CAP: usize = 30;

/// MOE-aligned candidate-type discriminator (`VocType` analog). Carried
/// on every [`RawCandidate`] and wire-encoded onto
/// `protos::taigi::engine::CandidateMessage.script_kind` (Phase 9.2). Derived
/// from `DictionaryRecord.hanji` presence + NFKD-normalized Latin-letter
/// detection by [`derive_script_kind`]; never emitted as
/// [`CandidateScriptKind::Unspecified`] from Rust.
///
/// **Metadata-only in v3.5.8 Phase 9.2** — does NOT enter the nine-
/// dimension [`CandidateSortKey`] tie-break (per `docs/releases/v3.5.8/plan.md` § Phase 9 R2
/// Q3.a "reserve rank use until real collisions are measured"). The
/// existing `form` axis remains orthogonal (toneless / numeric / hanji
/// / abbrev) and unaffected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CandidateScriptKind {
    /// Proto3 default — Rust never emits this; platforms reading the
    /// wire treat it as "unknown carrier, ignore" rather than HANT.
    Unspecified = 0,
    /// Hanji-only display (no Latin letters after NFKD normalization).
    Hant = 1,
    /// Roman/romanization-only display — `DictionaryRecord.hanji` was
    /// `None`, so `display_text` fell back to the TL field.
    Tailo = 2,
    /// Hanji display containing at least one Latin letter after NFKD
    /// (e.g. `iáu未`, `ê早`, `屎î`, hypothetical fullwidth `Ａ字`).
    Mixed = 3,
}

impl CandidateScriptKind {
    /// Wire-format integer matching `protos::CandidateScriptKind`'s prost
    /// representation. Kept as a method so a future reshuffle of the
    /// proto enum values would fail this cast at compile time via the
    /// `as u32` discriminant.
    pub const fn to_proto_i32(self) -> i32 {
        self as i32
    }
}

/// Derive the [`CandidateScriptKind`] for a dictionary record. `hanji.is_none()`
/// is the only TAILO path; otherwise the hanji string is NFKD-normalized
/// (folding `ê` → `e` + combining circumflex and `Ａ` → `A`) and any
/// resulting ASCII alphabetic codepoint flips the candidate to MIXED.
/// Digits / punctuation / kana / private-use glyphs alone do NOT flip
/// MIXED — the intent is "Roman letters inside the hanji display",
/// matching MOE `VT_MIXED` for entries like `台BAR`.
///
/// **Single source of truth for `CandidateScriptKind`.** `record_to_candidate`,
/// `custom_entry_to_candidate`, and the v3.5.8 S2 whole-sentence
/// walker's slot-0 synthesis (`composing::continuous::fetch_walker_slot0_inner`)
/// all derive `script_kind` through this fn so `CandidateMessage.script_kind` is
/// classified identically for span-local, custom, and synthesized
/// full-buffer candidates (Codex PR #285 P2, 2026-05-16 — a hand-rolled
/// `hanji.is_some()` binary in the synth path mis-emitted HANT for
/// mixed-script paths like `…hip相`). `pub` so `composing` reuses the
/// wire-visible classification instead of duplicating the NFKD rule.
pub fn derive_script_kind(hanji: Option<&str>) -> CandidateScriptKind {
    match hanji {
        None => CandidateScriptKind::Tailo,
        Some(text) if text.nfkd().any(|c| c.is_ascii_alphabetic()) => CandidateScriptKind::Mixed,
        Some(_) => CandidateScriptKind::Hant,
    }
}

/// One span-local candidate. Mirrors the 5-field shape pinned by
/// `docs/releases/v3.5.8/plan.md` § Phase 5 — Each Candidate carries (5-field shape).
#[derive(Debug, Clone, PartialEq)]
pub struct RawCandidate {
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
    pub canonical_tl: String,
    /// Result of [`ranking::calculate_continuous_score`].
    pub score: f32,
    /// Always [`FORM_NOTONE`] in Phase 5.
    pub form: u8,
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
    pub walker_cost: u16,
    /// Dictionary source bitmask copied verbatim from
    /// [`DictionaryRecord::bitmask`](crate::dictionary_reader::DictionaryRecord::bitmask). Used by the v3.5.8 Phase 9.1
    /// `CandidateSortKey` to derive `source_tier_rank` at sort time per
    /// `docs/releases/v3.5.8/plan.md` § Phase 9. Carrying it on the candidate (vs.
    /// re-reading the dictionary record) lets the sort be a pure
    /// function of the returned `RawCandidate` vector.
    pub bitmask: u16,
    /// MOE-aligned candidate-type discriminator (HANT / TAILO / MIXED).
    /// Derived by [`derive_script_kind`] from `DictionaryRecord.hanji`.
    /// Metadata-only in Phase 9.2 — not consulted by [`CandidateSortKey`].
    pub script_kind: CandidateScriptKind,
    /// Time-decayed user-selection weight for this candidate's
    /// `(display_text, canonical_tl)` pair
    /// ([`ranking::FrequencyData::user_weight`]; `0.0` = never selected,
    /// no wall clock, or clock-skew). Leading user-preference dimension
    /// of [`CandidateSortKey`] and of the walker's per-edge pick
    /// ([`best_candidate_for_key_with_barriers`]) — any selected word
    /// outranks every never-selected homophone. Rationale:
    /// `docs/engine/continuous-input-ranking.md` §3.2. Internal — NOT
    /// emitted on `CandidateMessage`.
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
    /// solely by [`CandidateSortKey`] to push every partial-prefix candidate
    /// strictly below every full-syllable candidate in lexicographic
    /// order, irrespective of `tier`, user weight, score, dict freq, or
    /// source rank.
    pub coverage_kind: u8,
    /// v3.5.8 Phase 9 Item 12 — `true` for candidates synthesized from
    /// a `custom_dictionary.db` entry ([`custom_entry_to_candidate`](candidate::custom_entry_to_candidate)),
    /// `false` for `dict.bin` FST hits ([`record_to_candidate`]). Read
    /// by [`CandidateSortKey::new`] (→ `source_tier_rank(bitmask, is_custom)`
    /// returns rank `0` when `true`, ahead of every `dict.bin` source
    /// tier) and by the `(canonical_tl, hanji)` dedupe winner policy
    /// (`docs/engine/continuous-commit-and-display.md` §10.10): on a
    /// duplicate `(canonical_tl, hanji)` identity the lowest `source_tier_rank`
    /// survivor wins, so a custom entry always beats a `dict.bin`
    /// duplicate. Internal axis only — NOT emitted on
    /// `CandidateMessage` (same pattern as [`coverage_kind`] /
    /// `user_weight`).
    pub is_custom: bool,
}

impl RawCandidate {
    /// The fields [`CandidateSortKey`] orders by.
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
/// `DictionaryRecord.hanji` / `RawCandidate.hanji` `Option` semantics
/// — drives [`derive_script_kind`] → `CandidateScriptKind::Tailo`).
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
}

/// Post-lookup tone constraint on the readings one continuous lookup
/// returns. The FST only has two romanization key families per record —
/// the fused toneless one and the fully-toned one
/// (`dictionary/build/create_fst.py`) — so every tone the user typed
/// that the key cannot express is re-applied HERE, after the lookup, by
/// checking each candidate's canonical reading against what was typed.
/// Built by `composing::shadow::span_key` (per key) and
/// `composing::shadow::whole_buffer_tone_pin` (whole buffer); consumed
/// through [`TonePin::admits`] on every candidate path — span-local,
/// walker slot 0, partial-prefix, custom merge — so the layers cannot
/// disagree on which readings qualify.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TonePin {
    /// No tone constraint: a toneless TL/POJ span (the deliberate
    /// "show all tones" affordance, §17 case 2), any English span (a
    /// digit there is not a tone), or a TPS span not closed by the space.
    #[default]
    None,
    /// A3 (§41) — a TPS span whose TAIL syllable was closed by the
    /// keyboard's space with no tone mark of its own: only the unmarked
    /// tone (1 on an open rime, 4 on a stop coda) may surface. Carries
    /// the fused `tps_notone` body of the span / buffer, which is what
    /// the whole-buffer sources align against; the exact / walker paths
    /// align against the MATCHED key instead (a §35 substituted reading
    /// reconstructs to the matched key, never the typed one).
    TpsSpaceEnd(String),
    /// §17 case 3 — a TL/POJ span carrying at least one ASCII tone digit:
    /// the hyphenless typed body with its digits (`teng5sek`) and the
    /// mode whose numeric-tone face it is read against. Every syllable
    /// the user toned must match that tone; a syllable left untoned is
    /// unconstrained, so `teng5sek` keeps 程式 `têng-sek` (5 + 4 stop
    /// coda) and drops 等式 `téng-sek` / 中式 `teng-sek`. Also set for a
    /// fully-toned span (`teng5sek4`): the verbatim toned key already
    /// filters the dictionary hits there, but custom entries are matched
    /// toneless and need the pin. §52 — `boundaries` are the byte offsets
    /// into `typed` where the user typed a `-`: a reading must end a
    /// syllable on every one, so `khi|ah` keeps 去啊 `khì--ah` and drops
    /// 隙 `khiah`. A span with a boundary but no digit is pinned too.
    /// `one_syllable_stretches` are the `(start, end)` ranges of `typed` a
    /// typed `-` closed as one inventory syllable: a reading may end no
    /// syllable strictly inside one, so `ai-` keeps 愛 `ài` and drops 阿姨
    /// `a-î`, both under the key `ai`.
    TypedTones {
        mode: phonetics::InputMode,
        typed: String,
        boundaries: Vec<TypedBoundary>,
        one_syllable_stretches: Vec<(usize, usize)>,
    },
}

/// §52 — one `-` run the user typed inside a span: where it sits in the
/// typed body, and whether it was the khinsiann `--` (two or more) rather
/// than a plain hyphen `-`. A reading must end a syllable there AND, when
/// another of its syllables follows, separate the two the same way: a
/// typed `--` keeps 去啊 `khì--ah` and drops 忍氣 `jím-khì`-shaped words;
/// a typed `-` does the reverse (a dictionary space counts as plain).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedBoundary {
    pub at: usize,
    pub khinsiann: bool,
}

impl TonePin {
    /// Does `reading` (a canonical TL reading — `DictionaryRecord.tl`, or
    /// a custom roman folded through `phonetics::api::canonical_tl_form`)
    /// satisfy this pin? `matched_key` is the FST key the reading came
    /// back under on the exact / walker paths; the whole-buffer sources
    /// (custom merge, partial-prefix extensions) pass `None` and the pin
    /// aligns against its own carried body — for a strict-prefix
    /// extension the matched key runs past the pin point, and a custom
    /// entry has no key at all.
    pub fn admits(&self, matched_key: Option<&str>, reading: &str) -> bool {
        match self {
            TonePin::None => true,
            TonePin::TpsSpaceEnd(body) => {
                reading_passes_space_pin(matched_key.unwrap_or(body), reading)
            }
            TonePin::TypedTones {
                mode,
                typed,
                boundaries,
                one_syllable_stretches,
            } => reading_passes_typed_tones(
                *mode,
                typed,
                boundaries,
                one_syllable_stretches,
                reading,
            ),
        }
    }

    /// [`TonePin::admits`] for a `custom_dictionary.db` entry: custom
    /// entries are synthesized whole-buffer (no matched key), and their
    /// roman may be stored in the user's native POJ form, so it is folded
    /// to canonical TL first — the same fold `custom_entry_to_candidate`
    /// applies for the identity key. The fold is skipped when nothing is
    /// pinned, the common toneless-buffer case.
    pub fn admits_custom(&self, roman: &str, mode: phonetics::InputMode) -> bool {
        matches!(self, TonePin::None)
            || self.admits(None, &phonetics::api::canonical_tl_form(roman, mode))
    }
}

/// §17 case 3 + §52: [`phonetics::typed_syllable_walk`] must consume the typed body
/// (`teng5sek` against 程式 `têng-sek`: `teng` + `5` matches, `sek` is
/// free), and every typed boundary must be a syllable end of the reading
/// with the same separator kind — `--` vs `-` / space
/// (`phonetics::tl_syllable_khinsiann_flags`; a dictionary space counts
/// as plain). The last syllable's end carries no kind: a trailing typed
/// `--` constrains the NEXT word, not this one. A boundary at 0 is the
/// run typed right before the span: only a reading that itself opens with
/// `--` (a custom `--ah`) reads it, and then the run must be `--` too.
/// No syllable of the reading may end strictly inside a one-syllable
/// stretch.
fn reading_passes_typed_tones(
    mode: phonetics::InputMode,
    typed: &str,
    boundaries: &[TypedBoundary],
    one_syllable_stretches: &[(usize, usize)],
    reading: &str,
) -> bool {
    let Some(walk) =
        phonetics::typed_syllable_walk(mode, typed, reading).filter(|w| w.typed_consumed)
    else {
        return false;
    };
    let khinsiann_before = if boundaries.is_empty() {
        Vec::new()
    } else {
        phonetics::tl_syllable_khinsiann_flags(reading)
    };
    let opens_khinsiann = khinsiann_before.first().copied().unwrap_or(false);
    boundaries.iter().all(|b| {
        if b.at == 0 {
            return !opens_khinsiann || b.khinsiann;
        }
        walk.ends.iter().enumerate().any(|(index, &at)| {
            at == b.at
                && khinsiann_before
                    .get(index + 1)
                    .is_none_or(|&k| k == b.khinsiann)
        })
    }) && !walk.ends.iter().any(|&at| {
        one_syllable_stretches
            .iter()
            .any(|&(start, end)| start < at && at < end)
    })
}

/// Span aliases for [`fetch_candidates_for_keys_with_barriers`]: `(start_byte, end_byte)`
/// in the user-facing input buffer (TL ASCII or TPS Bopomofo bytes,
/// depending on caller). The engine only stores these verbatim in the
/// returned `RawCandidate.consumed_span`; FST lookup uses the paired key.
pub type ConsumedSpan = (u32, u32);

/// Resolve a continuous lookup key to its stored readings.
///
/// TPS keys go through the ambiguity-aware automaton
/// (`PrefixIndex::lookup_exact_tps_readings`, §35): one index walk
/// returns every reading of the pressed keys, substitution-count
/// ascending so the user's literal text always resolves first. TL /
/// POJ / hanji keys keep the plain exact lookup — byte-identical
/// behavior, zero automaton cost (`matched_key` = the query key,
/// `subst` = 0).
///
/// Every consumer MUST validate records against the returned
/// `matched_key`, never the query key: a substituted reading's
/// `tps_notone` reconstruction equals the MATCHED key, and the
/// literal-key guard would reject every recovered word (Codex
/// pre-impl 2026-08-19 BLOCK 3).
fn for_each_exact_reading(
    prefix_index: &PrefixIndex,
    key: &str,
    final_only_offsets: &[usize],
    mut visit: impl FnMut(&str, u32),
) {
    if key.starts_with(KeyFamily::Tps.prefix()) {
        for (matched_key, rowid, _subst) in
            prefix_index.lookup_exact_tps_readings(key, final_only_offsets)
        {
            visit(&matched_key, rowid);
        }
    } else {
        // TL / POJ / hanji: byte-identical to the pre-§35 exact lookup —
        // the matched key IS the query key, no per-rowid allocation.
        for rowid in prefix_index.lookup_exact(key) {
            visit(key, rowid);
        }
    }
}

/// A3 (§41) — the two tones TPS writes with no mark: 1 on an open rime,
/// 4 on a stop coda (ㆴ/ㆵ/ㆻ/ㆷ). Pressing the keyboard's space closes a
/// syllable, and an unmarked closed syllable can only be one of these
/// two, so they are exactly the tones a space-pinned candidate may carry.
/// Tone 8 shares tone 4's coda but writes a dot, so it is excluded here —
/// that is the 一 (`tsit8`) vs 這 (`tsit4`) split the bug report hit.
fn is_unmarked_tps_tone(tone: char) -> bool {
    matches!(tone, '1' | '4')
}

/// A3 (§41) — does `reading` satisfy the space pin implied by `tps_body`?
/// True when the reading has a syllable boundary exactly at the end of
/// `tps_body` AND the syllable ending there carries an unmarked tone
/// ([`is_unmarked_tps_tone`]).
///
/// `tps_body` is either a full `tps:`-family FST key — the MATCHED key on
/// the exact / walker dictionary paths, since §35 substitutions
/// reconstruct to the matched form and the literal query key would reject
/// legitimate ambiguity-family hits — or a bare body, which is the shape
/// the whole-buffer sources (custom entries, the walker's custom override)
/// carry. A key in any other family passes through: TL/POJ tones are
/// ASCII digits, already pinned by the verbatim toned key (§17).
///
/// A missing boundary is a REJECT, not a pass: the user closed a syllable
/// there, so a reading that runs through that point mid-syllable is not
/// the word they typed.
///
/// `reading` is a canonical TL reading — `DictionaryRecord.tl` for a
/// dictionary hit, `CustomEntry.roman` for a custom entry.
fn reading_passes_space_pin(tps_body: &str, reading: &str) -> bool {
    let body = match tps_body.strip_prefix(KeyFamily::Tps.prefix()) {
        Some(body) => body,
        // Bare body — the whole-buffer form, TPS by construction.
        None if !tps_body.contains(':') => tps_body,
        // Another FST family (`tl:` / `poj:` / `hanzi:`) — never pinned.
        None => return true,
    };
    if body.chars().any(phonetics::is_tps_tone_mark) {
        // Marked body — the verbatim toned key already filtered by tone.
        return true;
    }
    phonetics::tps_notone_prefix_boundary_tone(reading, body).is_some_and(is_unmarked_tps_tone)
}

/// Every dictionary candidate for one exact continuous key, in FST rowid
/// order, handed to `sink` one at a time (each carries its record's
/// `walker_cost`). The single visitor the span-local fetch
/// ([`fetch_candidates_for_keys_with_barriers`], collects all) and the
/// walker edge provider ([`best_candidate_for_key_with_barriers`], keeps
/// the min cost) share, so the two layers cannot drift on which rows qualify —
/// §35: an edge the expanded segmenter admitted must find its dictionary
/// payload, or the layers split authority (Codex pre-impl BLOCK 3).
///
/// Per row, validated against the MATCHED key (a substituted TPS reading
/// reconstructs to the matched key, never the query key — see
/// [`for_each_exact_reading`]): the source `filter`, the `tl_abbrev` /
/// `poj_abbrev` / `tps_abbrev` acronym-collision guard
/// ([`matches_continuous_toneless_key`] — continuous input is
/// phonetic-syllable, not acronym; normal-mode `lexicon::search` keeps
/// acronym matching), and the key's [`TonePin`] — the A3 (§41) space pin
/// (`ㄒㄧ`␣ keeps si1, drops si2/5/7; `ㄐㄧㆵ`␣ keeps tsit4, drops tsit8)
/// or the §17 typed-tone pin (`teng5sek` keeps têng-sek, drops téng-sek).
/// Without the pin on the walker side the span-local list and the
/// whole-sentence slot 0 would disagree and a wrong-tone word would
/// reappear at index 0.
///
/// Exact hits are always `COVERAGE_KIND_FULL`: the key came from a valid
/// syllable ending. Partial-prefix hits go through
/// [`fetch_partial_prefix_candidates_unbounded`] instead. `filter` is
/// built by the caller.
fn exact_candidates_for_key(
    key: &str,
    tps_final_only: &[usize],
    tone_pin: &TonePin,
    consumed_span: ConsumedSpan,
    filter: &Filter,
    ctx: &ContinuousFetchCtx<'_>,
    mut sink: impl FnMut(RawCandidate),
) {
    for_each_exact_reading(
        ctx.prefix_index,
        key,
        tps_final_only,
        |matched_key, rowid| {
            let Some(record) = ctx.dict.record(rowid) else {
                return;
            };
            if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, filter) {
                return;
            }
            if !matches_continuous_toneless_key(matched_key, &record.tl) {
                return;
            }
            if !tone_pin.admits(Some(matched_key), &record.tl) {
                return;
            }
            let effective = DictionaryReader::effective_source_bitmask(
                record.bitmask,
                record.kautian_subtag,
                filter,
            );
            sink(record_to_candidate(
                record,
                effective,
                consumed_span,
                ctx.freq_map,
                ctx.now_ms,
                ctx.context,
                COVERAGE_KIND_FULL,
            ));
        },
    );
}

/// Mode-agnostic span-local fetch entry. Each input pair is
/// `(consumed_span, fst_key)`: `consumed_span` is the user-facing
/// byte range that committing this candidate will eat, and `fst_key`
/// is the already-prefixed FST lookup key (e.g. `"tl:tsua"` for TL/English,
/// `"poj:chiah"` for POJ, `"tps:ㄉㄞ"` for TPS — v3.5.9 B-2 PR #309
/// promoted POJ and v3.5.9 D / C-3b promoted TPS to first-class FST
/// families). The production caller
/// (`composing::continuous::assemble_candidates`) selects the
/// prefix via `phonetics::KeyFamily::for_input_mode(mode)` and feeds
/// pairs in directly for all modes.
///
/// # v3.5.8 Phase 9.1 — lexicographic CandidateSortKey
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`); it is the predicate input
/// for Tier 1 (`consumed_span_end == raw_len`). Sorting follows
/// `docs/releases/v3.5.8/plan.md` § Phase 9 sort_key formula:
///
/// ```text
/// (coverage_kind, tier, -user_weight, context_rank, walker_cost,
///  -adjusted_score, -freq, -coverage_bytes, source_tier_rank, stable_idx)
/// ```
///
/// E1 P5b: `walker_cost` (the record's corpus cost) orders the list on the
/// walker's scale; `-adjusted_score` / `-freq` break an equal cost.
///
/// v3.5.8 whole-sentence lattice + walker S8: `-coverage_bytes` was relocated
/// from dim 3 to dim 6 (below `-adjusted_score` / `-freq`). With the
/// slot-0 whole-sentence walker owning phrase priority, a graded
/// longest-coverage-first rule inside a tier only buried the short
/// single-syllable first-segment candidate the user wants for
/// segment-by-segment selection. Coverage is now a weak tiebreak that
/// fires only when score AND freq are equal — matching librime's
/// per-segment menu, which keeps multi-length candidates but never lets
/// a longer code-length bury a shorter strict match
/// (`script_translator.cc` `kNumExactMatchOnTop`).
///
/// # v3.5.8 Phase 9.3a — user-frequency plumb
///
/// `freq_map` is the user-selection snapshot keyed by the
/// `(display_text, canonical_tl)` pair (Core Principle #7), built once
/// per fetch from the engine's `user_frequency.db` rows
/// (`composing::UserRows.frequency`). `now_ms` is the platform's
/// epoch-ms wall clock at fetch time. `record_to_candidate` looks
/// up each candidate by that pair, computes
/// [`ranking::user_freq_boost`] (saturated at
/// [`ranking::MAX_BOOST`]), and derives
/// `CandidateSortKey.neg_user_weight` via [`ranking::decayed_user_weight_delta`]
/// (which guards against `now_ms <= 0`, `last_used_ms <= 0`, and
/// clock skew). NaN scores (only reachable if the boost helper
/// produces a non-finite value — which it cannot under the
/// public contract) are coerced to `f64::MIN` at `CandidateSortKey`
/// construction so the descending-order invariant holds.
///
/// # Barriers
///
/// `tps_final_only[i]` = byte offsets into `keys[i].1` of glyphs
/// immediately before a stripped separator / hyphen barrier — those pattern
/// slots keep only Final-role readings (§31 boundary respect; §35).
/// Parallel-indexed rather than widening the key tuple so the many
/// existing `(span, key)` call sites and fixtures stay untouched; an
/// empty slice (or a short one) means "no barriers", which is also the
/// TL / POJ / English shape.
pub fn fetch_candidates_for_keys_with_barriers(
    keys: &[(ConsumedSpan, String)],
    tps_final_only: &[Vec<usize>],
    tone_pins: &[TonePin],
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    // Item 12: custom entries can still surface even when the
    // syllabifier produced no `keys` for the FST path (e.g. the
    // dispatcher only reaches `fetch_partial_prefix_candidates` when
    // `keys` is empty, so this fn's `keys.is_empty()` branch is dead
    // for production callers — but a future caller passing empty
    // `keys` + non-empty `custom` must still get the custom merge).
    if keys.is_empty() && ctx.custom.is_empty() {
        return Vec::new();
    }

    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let mut out: Vec<RawCandidate> = Vec::new();

    for (key_index, (span, key)) in keys.iter().enumerate() {
        let final_only: &[usize] = tps_final_only
            .get(key_index)
            .map(|offsets| offsets.as_slice())
            .unwrap_or(&[]);
        // `tone_pins[i]` is the tone constraint for `keys[i]` (§17 typed
        // digits / §41 space-closed TPS tail). A short or empty slice
        // means "no pin", the legacy all-tones shape.
        let tone_pin = tone_pins.get(key_index).unwrap_or(&TonePin::None);
        exact_candidates_for_key(key, final_only, tone_pin, *span, &filter, ctx, |cand| {
            out.push(cand)
        });
    }
    merge_custom_dedupe_sort(out, ctx, raw_len, COVERAGE_KIND_FULL)
}

/// v3.5.8 Phase 9 Item 10 — partial-prefix candidate fetch for the
/// continuous-input path. Called when the syllabifier failed to find a
/// single valid syllable ending inside `raw` (so
/// [`fetch_candidates_for_keys_with_barriers`] would return empty) and we want the
/// candidate strip to surface engine prefix-match hits below any
/// future full-syllable matches. See
/// `docs/engine/continuous-candidate-display.md` §15.3.D + §15.5.
///
/// Pipeline:
///
/// 1. `prefix_index.lookup_prefix(&key.1)` — FST byte-sorted rowid scan.
/// 2. Hydrate budget cap at [`PARTIAL_PREFIX_HYDRATE_CAP`] rowids —
///    bounds `dict.record` work for single-char prefixes (`tl:k`,
///    `tl:t`) that match hundreds of FST entries. Set well above the
///    output cap so high-frequency short candidates landing past the
///    legacy byte-sort first 30 still reach the sort.
/// 3. Hydrate via `dict.record(rowid)`; drop rows that fail the
///    `enabled_sources_bitmask` filter (D-12 invariant parity with
///    [`fetch_candidates_for_keys_with_barriers`]).
/// 4. Build candidates with `coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX`
///    and `consumed_span = key.0` (caller pins `(0, raw.len())` per
///    Q15.4 — partial-prefix candidates always final-commit).
/// 5. Sort via the same [`CandidateSortKey`]; the leading
///    `coverage_kind` dim is `1` here so the whole batch ranks below
///    any concurrent full-syllable hits if a caller ever merges them
///    (this fn produces partial-prefix candidates only).
/// 6. Truncate to [`PARTIAL_PREFIX_OUTPUT_CAP`] after the sort, so the
///    returned slice is the globally best-scoring subset (not the FST
///    byte-sort prefix). Custom entries participate in the sort and
///    contribute to the output count.
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`); kept here for the Tier 0/1
/// (`consumed_span_end == raw_len`) downstream dim even though every
/// partial-prefix candidate has `consumed_span_end == raw_len` today,
/// so its `tier` is always 0 within `coverage_kind == 1`. This keeps
/// the `raw_len` / ctx contract shared with [`fetch_candidates_for_keys_with_barriers`].
///
/// `freq_map` + `now_ms` propagate user-frequency boost and recency
/// rank to partial-prefix hits identically to the full-syllable
/// path. An empty map + `now_ms = 0` is the cold-start neutral.
///
/// **Caller obligation** — `key.1` MUST carry an FST namespace plus
/// a non-empty body (e.g. `"tl:gu"`). Passing the bare namespace
/// (`"tl:"`) is permitted by the empty-string guard but is treated
/// as a legitimate "match every entry under the namespace" query
/// — it hydrates up to [`PARTIAL_PREFIX_HYDRATE_CAP`] FST entries
/// under that prefix, which is a footgun when triggered by misuse
/// rather than design. Production callers go through
/// `composing::shadow::build_partial_prefix_key` (v3.5.9 B-2 renamed
/// from `build_partial_prefix_key_tl` since the emitter is now
/// mode-aware), which returns `None` when the toneless body would be
/// empty and therefore never emits a bare `"tl:"` / `"poj:"` alone.
pub fn fetch_partial_prefix_candidates(
    key: &(ConsumedSpan, String),
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let mut out = fetch_partial_prefix_candidates_unbounded(key, raw_len, ctx);
    out.truncate(PARTIAL_PREFIX_OUTPUT_CAP);
    out
}

/// Same hydrate-custom-dedupe-sort pipeline as
/// [`fetch_partial_prefix_candidates`] but **without** the
/// `PARTIAL_PREFIX_OUTPUT_CAP` truncate at the tail.
///
/// Exists to address the Codex PR #351 r3321758666 finding: for inputs
/// whose exact key has many homophones (Codex example: production
/// `tl_notone = hong` has 30+ exact rows; `hong` syllabifies so Step 4b
/// fires), the post-sort truncate inside the bounded wrapper consumes
/// the entire `PARTIAL_PREFIX_OUTPUT_CAP` budget on rows that the
/// caller is about to drop via a cross-batch FULL/PARTIAL dedupe. The
/// caller then sees zero strict-prefix extensions even though they
/// exist in the dictionary's prefix range.
///
/// The fix surfaces the un-truncated sorted pool so the caller can
/// apply its FULL-block exclude BEFORE truncating. The empty-keys
/// branch (which has no FULL block to exclude against) keeps using
/// the bounded wrapper unchanged.
///
/// Caller obligation: must apply its own truncate (typically
/// `PARTIAL_PREFIX_OUTPUT_CAP`) after the cross-batch filter; the
/// returned vec is bounded only by `PARTIAL_PREFIX_HYDRATE_CAP` +
/// custom count.
pub fn fetch_partial_prefix_candidates_unbounded(
    key: &(ConsumedSpan, String),
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let (span, fst_key) = key;
    if fst_key.is_empty() && ctx.custom.is_empty() {
        return Vec::new();
    }
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    // Pre-allocate to the worst-case pool size so the hydration loop
    // does not grow `out` through the 16/32/64/128/256/512 doubling
    // sequence. Saves 2-3 reallocs on single-char prefixes that
    // saturate `HYDRATE_CAP`.
    let mut out: Vec<RawCandidate> =
        Vec::with_capacity(PARTIAL_PREFIX_HYDRATE_CAP + ctx.custom.len());
    // `take(PARTIAL_PREFIX_HYDRATE_CAP)` bounds `dict.record` work for
    // single-char prefixes (`tl:k`, `tl:t`) that match hundreds of FST
    // entries. The cap is intentionally set far above the visible
    // output size: per-record hydration is a few mmap slice reads
    // plus 2-4 small String allocations, so the extra budget is cheap,
    // but it lets high-frequency short candidates landing past the
    // legacy byte-sort first 30 (e.g. `tl:ki` after a wall of
    // `tl:ka-*` phrases) reach the CandidateSortKey sort.
    //
    // This fn does NOT truncate — the bounded wrapper
    // `fetch_partial_prefix_candidates` applies `PARTIAL_PREFIX_OUTPUT_CAP`
    // for callers that don't need cross-batch exclude; Step 4b in
    // `composing::continuous` takes the un-truncated pool, applies its
    // FULL-block exclude, then truncates itself. Filter rejects still
    // consume hydration budget — goal is bounding worst-case work, not
    // maximizing hits.
    // Item 12: guard the unbounded `lookup_prefix("")` scan — with the
    // early-return now gated on `fst_key.is_empty() && custom.is_empty()`,
    // an empty `fst_key` + non-empty `custom` reaches here and must NOT
    // trigger a whole-FST scan.
    if !fst_key.is_empty() {
        // Spend the hydrate budget on the SHORTEST matched keys first (all
        // modes). The FST wire separator `0xFF` is greater than any UTF-8
        // byte, so a short exact key (`tps:ㄍㄚ` / `tl:ka`) byte-sorts AFTER
        // every longer extension of it; a plain `lookup_prefix(..).take(cap)`
        // therefore front-loads the deepest, longest (rarest) words and
        // buries the short high-frequency single-syllable readings past the
        // cap — user-reported: typing `ㄍ` surfaced only multi-syllable
        // phrases, the common single chars never reached the ranker. Length
        // bucketing is a hydration-budget policy only; the visible order is
        // still the downstream `CandidateSortKey` (recency / score / frequency).
        //
        // Acronym `*_abbrev` keys live in their own `*-abbrev:` family
        // (`create_fst.py`; `fetch_abbrev_candidates` is their only reader),
        // so this range holds phonetic readings only and the shortest bucket
        // is the single-syllable keys — no per-key exclusion (the pre-tag
        // `skip_abbrev` heuristic) is needed to keep 是/sī from being starved
        // by a wall of `tl:sb`-style acronym keys.
        // §35 — the TPS partial hydration resolves through the same
        // ambiguity pattern as the exact paths (single lookup authority):
        // bare `ㄇ` lists ㆬ… words alongside ㄇ… words. The matched key
        // travels with each rowid so the record guard below validates what
        // the pattern actually hit, not the literal prefix (Codex
        // post-impl 2026-08-19 BLOCK 1). TL/POJ keep the plain lookup.
        let hits: Vec<(String, u32)> = if fst_key.starts_with(KeyFamily::Tps.prefix()) {
            ctx.prefix_index
                .lookup_prefix_shortest_first_tps_readings(fst_key, PARTIAL_PREFIX_HYDRATE_CAP)
        } else {
            ctx.prefix_index
                .lookup_prefix_shortest_first(fst_key, PARTIAL_PREFIX_HYDRATE_CAP)
                .into_iter()
                .map(|rowid| (fst_key.to_string(), rowid))
                .collect()
        };
        // Loop-invariant: the family and the typed body's length are the
        // same for every hydrated row.
        let reach = SyllableReach::new(fst_key);
        for (matched_key, rowid) in hits {
            let Some(record) = ctx.dict.record(rowid) else {
                continue;
            };
            if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, &filter) {
                continue;
            }
            // Codex PR #351 r3319500948 — drop `tl_abbrev` / `poj_abbrev` /
            // `tps_abbrev` collisions whose FST key happens to share the
            // input prefix. Mirrors the span-local + walker guard at
            // `fetch_candidates_for_keys_with_barriers` /
            // `best_candidate_for_key_with_barriers` (`matches_continuous_toneless_key`)
            // but uses the prefix-aware `*_prefix_key` variant — the
            // partial-prefix path's key body is a STRICT PREFIX of the
            // toneless, so equality would reject every legitimate
            // extension hit.
            // §35 — prefix guard runs on the MATCHED key: a substituted
            // hit (`tps:ㆬㄒㄧ` under typed `tps:ㄇ`) reconstructs to the
            // matched form; the literal prefix would reject it.
            if !matches_continuous_toneless_prefix_key(&matched_key, &record.tl) {
                continue;
            }
            // Tone pin: a strict-prefix extension is only eligible when
            // the syllables typed so far carry the typed tones (§17) / the
            // space-closed unmarked tone (§41). Checked against the TYPED
            // body (the pin point), not the matched key — the matched key
            // runs past the pin into the extension's later syllables.
            if !ctx.tone_pin.admits(None, &record.tl) {
                continue;
            }
            // Syllable reach: a strict-prefix extension may not carry a
            // syllable the user never typed into (`tsuisi` must not surface
            // 水社寮 `tsuí-siā-liâu`). Dictionary rows only — the custom-entry
            // loop below is deliberately exempt (product owner 2026-08-21: a
            // word the user added themselves stays prefix-visible).
            if !reach
                .as_ref()
                .is_none_or(|reach| reach.admits(&matched_key, &record.tl))
            {
                continue;
            }
            let effective = DictionaryReader::effective_source_bitmask(
                record.bitmask,
                record.kautian_subtag,
                &filter,
            );
            out.push(record_to_candidate(
                record,
                effective,
                *span,
                ctx.freq_map,
                ctx.now_ms,
                ctx.context,
                COVERAGE_KIND_PARTIAL_PREFIX,
            ));
        }
    }
    merge_custom_dedupe_sort(out, ctx, raw_len, COVERAGE_KIND_PARTIAL_PREFIX)
}

/// v3.5.8 S2 — single best dictionary candidate for one exact FST
/// key. Returns the [`record_to_candidate`] over
/// `prefix_index.lookup_exact(key)` that [`pick_edge_word`] chooses
/// (user-selected word, then the corpus `walker_cost`, then the dictionary
/// score; NaN coerced low via `CandidateSortKey`; ties keep the first
/// inserted row for determinism) together with the key's
/// min walker cost ([`EdgeBest::span_walker_cost`]), or `None` when the
/// key has no dict hit. PR-9.6 — the walker reads the
/// SAME `ctx.enabled_sources_bitmask` filter the span-local path
/// applies (`composing::continuous::assemble_candidates` builds one
/// [`ContinuousFetchCtx`] for both), so a whole-sentence parse never
/// re-surfaces a word whose only source the user toggled off. Only the
/// lookup fields of `ctx` are read (`prefix_index` / `dict` /
/// `freq_map` / `now_ms` / `enabled_sources_bitmask`); `custom` /
/// `mode` / `tone_pin` belong to the whole-buffer merge — this edge's
/// own pin arrives as `tone_pin`.
///
/// The whole-sentence walker (`composing::lattice::walker`) calls
/// this once per lattice edge through a dispatch-injected edge
/// provider so the walker stays pure + shadow-space native and the
/// lexicon candidate construction is **reused, not duplicated**
/// (Codex pre-impl S2 Q1b, 2026-05-16). `consumed_span` is stamped
/// onto the returned candidate verbatim; the walker reads `roman` /
/// `hanji` / `syllable_count` / `user_weight` off the candidate and
/// prices the edge on [`EdgeBest::span_walker_cost`].
///
/// `tps_final_only` is the §35 barrier restriction for this edge's key
/// (byte offsets of Final-only pattern slots, family prefix included).
/// The walker resolves multi-syllable edges that may span a stripped
/// separator, so it needs the same restriction the span-local fetch
/// gets — without it a walker edge could re-read a separator-closed
/// coda as the next syllable's onset. Empty slice = no barriers.
pub fn best_candidate_for_key_with_barriers(
    key: &str,
    tps_final_only: &[usize],
    tone_pin: &TonePin,
    consumed_span: ConsumedSpan,
    learned: &[&LearnedEntry],
    ctx: &ContinuousFetchCtx<'_>,
) -> Option<EdgeBest> {
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    // `span_walker_cost` is the dictionary's own minimum — learned rows
    // join the pick below but carry no corpus probability.
    let mut span_walker_cost = WALKER_COST_UNPRICED;
    let mut homophones: Vec<RawCandidate> = Vec::new();
    exact_candidates_for_key(
        key,
        tps_final_only,
        tone_pin,
        consumed_span,
        &filter,
        ctx,
        |cand| {
            span_walker_cost = span_walker_cost.min(cand.walker_cost);
            homophones.push(cand);
        },
    );
    // Learned phrases (§50) — the caller matched these rows to this edge's
    // key; they join the SAME pick with no corpus cost
    // ([`WALKER_COST_UNPRICED`]) and `frequency = 0`, so a dictionary
    // homophone wins unless the user's `user_frequency` or the previous
    // word's context says otherwise, and the learned row is the edge when
    // the dictionary has nothing under the key. The caller caps the edge's
    // `span_walker_cost` for segmentation.
    for entry in learned {
        homophones.push(learned_entry_to_candidate(
            entry,
            consumed_span,
            ctx.freq_map,
            ctx.now_ms,
            ctx.context,
            COVERAGE_KIND_FULL,
        ));
    }
    let candidate = pick_edge_word(homophones, consumed_span.1, ctx.context.is_empty())?;
    Some(EdgeBest {
        candidate,
        span_walker_cost,
    })
}

/// The edge's word among its homophones: the [`CandidateSortKey`] the
/// candidate list sorts by (E1 P5a / P5b) — a selected word first, then a
/// context hit, then the word the corpus writes most (`walker_cost`), with the
/// dictionary score / freq / source dims only breaking an equal cost. Every homophone
/// shares `coverage_kind` / `consumed_span`, so `raw_len = consumed_span.1`
/// pins `tier` equal; the insertion index breaks a full tie.
///
/// With a previous-word context (§56) the pick may change the WORD but never
/// the edge's COST, so segmentation is frozen: the context-free winner is
/// found first, and the contextual winner is chosen only among homophones
/// with its `syllable_count`. `context_rank` sits below `user_weight` in the
/// key, so the winner keeps the context-free `user_weight` (the span max) —
/// and `EdgeBest::span_walker_cost` is the key min regardless of the pick —
/// which are the pick's only inputs to `lattice::edge_cost`.
fn pick_edge_word(
    homophones: Vec<RawCandidate>,
    raw_len: u32,
    context_free: bool,
) -> Option<RawCandidate> {
    let edge_key = |i: usize, cand: &RawCandidate, with_context: bool| {
        let facts = cand.rank_facts();
        if with_context {
            CandidateSortKey::new(&facts, raw_len, i as u32)
        } else {
            CandidateSortKey::without_context(&facts, raw_len, i as u32)
        }
    };
    let (baseline_index, syllable_count) = homophones
        .iter()
        .enumerate()
        .min_by_key(|(i, homophone)| edge_key(*i, homophone, false))
        .map(|(i, cand)| (i, cand.syllable_count))?;
    let index = if context_free {
        baseline_index
    } else {
        homophones
            .iter()
            .enumerate()
            .filter(|(_, cand)| cand.syllable_count == syllable_count)
            .min_by_key(|(i, homophone)| edge_key(*i, homophone, true))
            .map(|(i, _)| i)
            .unwrap_or(baseline_index)
    };
    homophones.into_iter().nth(index)
}

/// The words [`best_candidate_for_key_with_barriers`] picks the edge's word
/// among — every dictionary row under `key` that passes the edge's source,
/// tone-pin and barrier filters — as their `display_text`, the word half of
/// the `(display_text, canonical_tl)` pair user frequency is keyed on. A
/// Hanji conversion reads the user's counts for these before its walk.
pub fn homophone_words_for_key(
    key: &str,
    tps_final_only: &[usize],
    tone_pin: &TonePin,
    consumed_span: ConsumedSpan,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<String> {
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let mut words = Vec::new();
    exact_candidates_for_key(
        key,
        tps_final_only,
        tone_pin,
        consumed_span,
        &filter,
        ctx,
        |cand| words.push(cand.display_text),
    );
    words
}

/// One lattice edge's word, as picked by
/// [`best_candidate_for_key_with_barriers`], plus the evidence that the
/// edge's span IS a dictionary word.
#[derive(Debug)]
pub struct EdgeBest {
    /// The homophone the user sees for this edge — user-selected word
    /// first, then the previous word's context, then the corpus
    /// `walker_cost` ([`pick_edge_word`]).
    pub candidate: RawCandidate,
    /// Lowest `dict.bin` `walker_cost` among the key's homophones that
    /// pass the edge's source / tone-pin / barrier filters — not the
    /// picked word's own (E1 D3). The walker prices the edge's
    /// segmentation on this so a rarer preferred homophone does not price
    /// its span out of the best path: "is this span a word" is decoupled
    /// from "which word". (`candidate.user_weight` is likewise the span's
    /// max, since user weight is the leading pick dimension.)
    /// [`WALKER_COST_UNPRICED`] when only learned rows sit under the key.
    /// Rationale + numbers: `docs/engine/continuous-input-ranking.md` §3.2.
    pub span_walker_cost: u16,
}

/// Does the exact hanji `hanji` resolve to a dictionary entry of
/// **exactly `syllable_count` TL syllables**? Used by the composing
/// render/commit join ([`crate`] consumer `composing::api::nailed_prefix`)
/// to decide whether a contiguous run of manually-nailed single-syllable
/// segments reconstructs a known n-syllable compound (`紅尾冬` /
/// `âng-bóe-tang`, `查某` / `tsa-bóo`) and therefore render its internal
/// boundaries as hyphens instead of spaces.
///
/// `syllable_count < 2` returns `false` unconditionally (a single
/// syllable is not a "compound" by definition; the caller never queries
/// `n=1`).
///
/// Scans **all** `prefix_index.lookup_exact("hanzi:<hanji>")` rowids —
/// NOT [`best_candidate_for_key_with_barriers`], which returns a single ranking
/// winner and would make a presentation separator depend on score
/// (Codex pre-impl Q2 2026-05-18) — and returns `true` iff some record
/// has `syllable_count == <argument>` **and** `hanji == Some(hanji)`.
/// The `syllable_count` gate is load-bearing: many two-CJK-codepoint
/// dictionary entries are NOT two TL syllables (e.g. `先生 / sin-senn`,
/// `新婦 / sim-pū`); existence alone would over-hyphenate them (Codex
/// pre-impl N2 2026-05-18). The explicit `hanji` re-check guards
/// against wrong rowids / future index drift; it cannot bridge
/// byte-different but visually-equivalent variant forms (an accepted
/// data-level limitation, identical to the toneless-key path).
///
/// **v3.5.9 longest-match extension** — previously fixed at 2 syllables
/// (`§10.2 Option A`), now parameterized so the caller's longest-match
/// loop can ask "is `hanji` an n-syllable compound?" for `n >= 2`. Peer
/// IMEs (khiin-rs `autospace`, librime spelling-algebra DAG, McBopomofo
/// `ReadingGrid`) handle multi-syllable compounds at segmentation /
/// lattice-walk time instead of at commit-join time; this extension is
/// the manual-nail-flow-compatible analog and is intentionally narrower
/// in scope (an isolated UX heuristic, not a general best practice).
pub fn compound_hanji_exists(
    hanji: &str,
    syllable_count: u8,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> bool {
    if syllable_count < 2 {
        return false;
    }
    let key = format!("{HANJI_KEY_PREFIX}{hanji}");
    for rowid in prefix_index.lookup_exact(&key) {
        if let Some(record) = dict.record(rowid) {
            if record.syllable_count == syllable_count && record.hanji.as_deref() == Some(hanji) {
                return true;
            }
        }
    }
    false
}

/// Whole-buffer abbreviation lookup — every dictionary row indexed under
/// `fst_key` in the abbreviation family (`tl-abbrev:ss` → 鎖匙 `só-sî`,
/// `tps-abbrev:ㄙㄒ` → 鎖匙; `create_fst.py` emits one `*-abbrev:` key per
/// ≥ 2-syllable row, `dictionary/common/abbrev.py`). `behavioral-invariants.md`
/// §46.
///
/// The family tag is the authority: a `*-abbrev:` key can only be an
/// abbreviation face, so no per-record face check is needed (build-time
/// parity `tests/roman_num_face_parity.rs` / `tests/tps_abbrev_parity.rs`
/// pins the column the key came from). The phonetic families never hold
/// an abbreviation key, which is what lets the span-local / walker /
/// partial-prefix fetches stay acronym-free without any heuristic. Caller
/// gate: `composing::shadow::abbrev_query_key` (letters / glyphs only —
/// no digit, tone mark or separator, so no [`TonePin`] or barrier can
/// apply).
///
/// Per row: source `filter`, then [`record_to_candidate`] with
/// [`COVERAGE_KIND_ABBREV`] at `consumed_span = (0, raw_len)`;
/// `syllable_count` stays the record's (鎖匙 = 2). Sorted by the same
/// [`CandidateSortKey`]; **not truncated** — the caller excludes rows already on
/// screen first, then applies [`PARTIAL_PREFIX_OUTPUT_CAP`] (the Step 4b
/// lesson, Codex PR #351). Custom entries are not merged here; the fetch
/// that ran before already merged them for the same buffer. Hydration is
/// uncapped: `tl-abbrev:tt` resolves ~8.5k rowids and a rank-blind cap in
/// rowid order would drop the frequent words the sort exists to surface.
pub fn fetch_abbrev_candidates(
    fst_key: &str,
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let rowids = ctx.prefix_index.lookup_exact(fst_key);
    let mut out: Vec<RawCandidate> = Vec::with_capacity(rowids.len());
    for rowid in rowids {
        let Some(record) = ctx.dict.record(rowid) else {
            continue;
        };
        if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, &filter) {
            continue;
        }
        let effective = DictionaryReader::effective_source_bitmask(
            record.bitmask,
            record.kautian_subtag,
            &filter,
        );
        out.push(record_to_candidate(
            record,
            effective,
            (0, raw_len),
            ctx.freq_map,
            ctx.now_ms,
            ctx.context,
            COVERAGE_KIND_ABBREV,
        ));
    }
    dedupe_by_roman_hanji_span(&mut out);
    sort_by_candidate_key(out, raw_len, RawCandidate::rank_facts)
}

/// The typed input must reach INTO a record's FINAL syllable for a
/// strict-prefix extension hit to be offered as a continuous candidate.
///
/// This is the engine-wide rule "a candidate never carries more syllables than
/// the user has typed" (product owner, 2026-08-21, all three platforms).
/// Before it, `lookup_prefix` hydrated every row whose key merely STARTS with
/// what was typed, so `tsuisi` (2 syllables) surfaced 水社寮 `tsuí-siā-liâu`
/// (3) and `kesithau` (3) surfaced 家私頭仔 `ke-si-thâu-á` (4) — the whole last
/// syllable was never typed at all.
///
/// Stated per-syllable rather than as a syllable-count comparison because "how
/// many syllables did the user type" has no single answer: `aia` reads as 2
/// hops (`ai`+`a`) or 3 (`a`+`i`+`a`), and 阿姨仔 `a-î-á` — an EXACT key hit,
/// not an extension — must survive. Measuring how far the typed bytes reach
/// into the record's OWN syllable chain answers the product question directly
/// and leaves exact hits untouched (their key IS the typed body, so the head of
/// any multi-syllable reading is strictly shorter than it).
///
/// Built once per lookup from the typed key, then asked about each hydrated
/// row: the family and the typed body's length are the same for every row.
///
/// **Subsumes, deliberately does not replace, its siblings.** The prefix test
/// inside [`SyllableReach::syllable_ends`] is the same `starts_with` the three
/// `matches_continuous_*_toneless_prefix_key` guards run a few lines earlier,
/// and `phonetics::tps_notone_prefix_boundary_tone` (§41) walks the same
/// per-syllable accumulation to answer the adjacent question "does the typed
/// body land ON a boundary, and at what tone". Folding them into one walk is
/// the right end state, but it would tighten three shipped guards — they
/// fail-open on a tone-bearing body, this one measures it — so it belongs to a
/// refactor round with its own behaviour-freeze list, not here. Do not add a
/// fifth independent reconstruction.
struct SyllableReach<'a> {
    /// `<family>:`, including the colon.
    family: &'a str,
    typed_body_len: usize,
}

impl<'a> SyllableReach<'a> {
    /// `None` for a key with no `<family>:` prefix — nothing to measure
    /// against, so the caller leaves every hit alone.
    fn new(typed_key: &'a str) -> Option<Self> {
        let family_end = typed_key.find(':')? + 1;
        Some(Self {
            family: &typed_key[..family_end],
            typed_body_len: typed_key.len() - family_end,
        })
    }

    /// Whether `record_tl` may be offered for the hit that came back as
    /// `matched_key`.
    ///
    /// The reading is selected by `matched_key` — that is the face the row was
    /// found under, and for the §35 TPS ambiguity families it is a substituted,
    /// FULL stored key rather than the typed prefix
    /// (`lookup_prefix_shortest_first_tps_readings`). The reach is measured
    /// against the TYPED body, since substitution is charwise and cannot
    /// lengthen the typed prefix.
    ///
    /// Fail-open: a hit this cannot place on a reconstructable face keeps its
    /// pre-rule behaviour rather than being dropped on a derivation miss. Two
    /// ways that happens, both narrow: a family with no romanization face at
    /// all (`hanzi:`, or one added after this was written), and a body the
    /// reconstructed face does not cover — an acronym face, which the sibling
    /// guards in the same loop already reject on their own. Every production
    /// romanization face IS reconstructable: `tests/roman_num_face_parity.rs`
    /// and `tests/tps_notone_parity.rs` pin all five columns byte-for-byte
    /// against the shipped CSV, so a fail-open here means a real drift, not a
    /// tolerated gap.
    fn admits(&self, matched_key: &str, record_tl: &str) -> bool {
        let Some(matched_body) = matched_key.strip_prefix(self.family) else {
            return true;
        };
        let Some(ends) = self.syllable_ends(matched_body, record_tl) else {
            return true;
        };
        // A single-syllable reading is reached by any non-empty typed prefix.
        match ends.len() {
            0 | 1 => true,
            count => (ends[count - 2] as usize) < self.typed_body_len,
        }
    }

    /// Where each syllable of `record_tl` ends, on the key surface
    /// `matched_body` was found under — `None` when no reconstructable surface
    /// covers it.
    ///
    /// Surface is read off the body itself, the same way the sibling guards
    /// read it: an ASCII digit means the numeric-tone family (`tl:<tl_num>` /
    /// `poj:<poj_num>`), a Bopomofo tone mark means `tps:<tps_num>`, anything
    /// else is the fused toneless family. The C-3a `er`↔`or` dialect variant is
    /// a second face of the same reading, so it is tried when the primary does
    /// not cover the body; the substitution is one Bopomofo scalar for another
    /// of the same width, so the boundaries carry over unchanged.
    fn syllable_ends(&self, matched_body: &str, record_tl: &str) -> Option<Vec<u32>> {
        match KeyFace::of(self.family, matched_body)? {
            KeyFace::TpsNum | KeyFace::TpsNotone => {
                let (primary, ends) = if matched_body.chars().any(phonetics::is_tps_tone_mark) {
                    phonetics::tps_num_syllable_ends_from_tl(record_tl)
                } else {
                    phonetics::tps_notone_syllable_ends_from_tl(record_tl)
                };
                let faces = with_tps_or_variant(primary);
                debug_assert!(
                    faces
                        .1
                        .as_ref()
                        .is_none_or(|variant| variant.len() == faces.0.len()),
                    "or-variant substitution must preserve byte offsets",
                );
                tps_face_starts_with(&faces, matched_body).then_some(ends)
            }
            face @ (KeyFace::TlNum | KeyFace::TlNotone) => {
                let num = phonetics::tl_num_syllable_ends_from_tl(record_tl);
                Self::covering(matched_body, num, face == KeyFace::TlNotone)
            }
            face @ (KeyFace::PojNum | KeyFace::PojNotone) => {
                let num = phonetics::poj_num_syllable_ends_from_tl(record_tl);
                Self::covering(matched_body, num, face == KeyFace::PojNotone)
            }
        }
    }

    /// `ends` when `face` covers `matched_body`, dropping the tone digits first
    /// when the body came from the toneless surface.
    ///
    /// The toneless columns ARE the numeric ones with the digits removed
    /// (`dictionary/common/notone.py::remove_tone`), so one derivation answers
    /// both surfaces: dropping a digit shortens that syllable and every
    /// boundary after it by one. A syllable that is nothing but its tone digit
    /// disappears entirely, and must not leave a boundary behind — that would
    /// count a syllable the toneless face does not have.
    fn covering(
        matched_body: &str,
        (num, ends): (String, Vec<u32>),
        toneless: bool,
    ) -> Option<Vec<u32>> {
        if !toneless {
            return num.starts_with(matched_body).then_some(ends);
        }
        let mut face = String::with_capacity(num.len());
        let mut face_ends = Vec::with_capacity(ends.len());
        let mut cursor = 0usize;
        for end in ends {
            let syllable = num.get(cursor..end as usize)?;
            cursor = end as usize;
            let before = face.len();
            face.extend(syllable.chars().filter(|c| !c.is_ascii_digit()));
            if face.len() != before {
                face_ends.push(face.len() as u32);
            }
        }
        face.starts_with(matched_body).then_some(face_ends)
    }
}

#[cfg(test)]
mod mode_derive_tests {
    //! Hermetic unit tests for the v3.5.8 Phase 9.2 `CandidateScriptKind`
    //! derive (`derive_script_kind`). Covers the four classification axes
    //! Codex co-decided 2026-05-11:
    //!
    //! 1. `hanji.is_none()` → TAILO
    //! 2. Plain-ASCII Latin in hanji → MIXED
    //! 3. NFC-composed Latin (e.g. `ê`) in hanji → MIXED via NFKD
    //! 4. Fullwidth Latin (e.g. `Ａ`) in hanji → MIXED via NFKD
    //! 5. Pure CJK → HANT
    //! 6. Digits / punctuation alone do NOT flip MIXED
    use super::*;

    #[test]
    fn no_hanji_means_tailo() {
        // Roman-only entries (`hanji = None`, `display_text` falls back
        // to the TL field).
        assert_eq!(derive_script_kind(None), CandidateScriptKind::Tailo);
    }

    #[test]
    fn pure_cjk_is_hant() {
        // Canonical hanji-only display.
        assert_eq!(
            derive_script_kind(Some("臺灣台語")),
            CandidateScriptKind::Hant
        );
        assert_eq!(derive_script_kind(Some("珠仔")), CandidateScriptKind::Hant);
        assert_eq!(derive_script_kind(Some("台")), CandidateScriptKind::Hant);
    }

    #[test]
    fn plain_ascii_latin_in_hanji_is_mixed() {
        // Real dictionary entries: `hip相`, `iah是`, `ing暗`. The first
        // Latin codepoint is plain ASCII so it would flip MIXED even
        // without NFKD; this test pins the easy path.
        assert_eq!(
            derive_script_kind(Some("hip相")),
            CandidateScriptKind::Mixed
        );
        assert_eq!(
            derive_script_kind(Some("iah是")),
            CandidateScriptKind::Mixed
        );
        assert_eq!(
            derive_script_kind(Some("ing暗")),
            CandidateScriptKind::Mixed
        );
        // Hypothetical "台BAR" — Codex Q-F3 example.
        assert_eq!(
            derive_script_kind(Some("台BAR")),
            CandidateScriptKind::Mixed
        );
    }

    #[test]
    fn s2_synth_concatenated_hanji_is_mixed_when_any_edge_has_latin() {
        // v3.5.8 S2 (Codex PR #285 P2): the whole-sentence walker
        // synthesizes the slot-0 hanji by concatenating each edge's
        // hanji and classifies the WHOLE joined string through this
        // fn (`composing::continuous::fetch_walker_slot0_inner`). A Latin
        // letter in a NON-first segment (e.g. path `臺灣` + `hip相`)
        // must still flip MIXED — equivalent to the per-edge OR and
        // matching how a single multi-syllable record would classify.
        assert_eq!(
            derive_script_kind(Some("臺灣hip相")),
            CandidateScriptKind::Mixed
        );
        // All-CJK concatenation stays HANT (the common phrase path).
        assert_eq!(
            derive_script_kind(Some("臺灣台語")),
            CandidateScriptKind::Hant
        );
    }

    #[test]
    fn composed_latin_in_hanji_is_mixed_via_nfkd() {
        // Real entries `ê早` (line 22953 of dictionary.csv), `ē得`
        // (22960), `屎î` (42037). The Latin codepoint is NFC-composed
        // (e.g. `ê` = U+00EA, NOT `e` + combining circumflex), so
        // `is_ascii_alphabetic` on the original chars would miss it.
        // NFKD decomposes to base ASCII `e` / `i` + combining mark.
        assert_eq!(derive_script_kind(Some("ê早")), CandidateScriptKind::Mixed);
        assert_eq!(derive_script_kind(Some("ē得")), CandidateScriptKind::Mixed);
        assert_eq!(derive_script_kind(Some("屎î")), CandidateScriptKind::Mixed);
    }

    #[test]
    fn fullwidth_latin_in_hanji_is_mixed_via_nfkd() {
        // Theoretical: U+FF21..U+FF3A fullwidth Latin folds to ASCII
        // under NFKD (NOT NFD). Pins the "K/D normalization, not just
        // D" choice from Codex F3-c.
        assert_eq!(derive_script_kind(Some("Ａ字")), CandidateScriptKind::Mixed);
        assert_eq!(derive_script_kind(Some("字Ｚ")), CandidateScriptKind::Mixed);
    }

    #[test]
    fn digits_or_punctuation_alone_stay_hant() {
        // F3-d: `123` is HANT (no Latin LETTERS). `3Q` is MIXED via the
        // `Q`. Punctuation likewise does not flip MIXED.
        assert_eq!(derive_script_kind(Some("123")), CandidateScriptKind::Hant);
        assert_eq!(derive_script_kind(Some("3Q")), CandidateScriptKind::Mixed);
        assert_eq!(derive_script_kind(Some("。、")), CandidateScriptKind::Hant);
    }

    #[test]
    fn empty_hanji_string_stays_hant() {
        // Defensive: `hanji = Some("")` (shouldn't happen but the
        // contract is "any Some without Latin letters = HANT", and an
        // empty NFKD iterator finds no ASCII alphabetic codepoint).
        assert_eq!(derive_script_kind(Some("")), CandidateScriptKind::Hant);
    }

    #[test]
    fn proto_wire_value_matches_enum_discriminant() {
        // The proto enum (`CandidateScriptKind` in composing.proto) uses
        // exactly UNSPECIFIED=0, HANT=1, TAILO=2, MIXED=3. The cast
        // here pins the wire integers so a future reshuffle of the
        // Rust `repr(u8)` discriminants would fail this test.
        assert_eq!(CandidateScriptKind::Unspecified.to_proto_i32(), 0);
        assert_eq!(CandidateScriptKind::Hant.to_proto_i32(), 1);
        assert_eq!(CandidateScriptKind::Tailo.to_proto_i32(), 2);
        assert_eq!(CandidateScriptKind::Mixed.to_proto_i32(), 3);
    }

    #[test]
    fn local_enum_matches_prost_generated_proto_enum() {
        // Cross-pin: the local `CandidateScriptKind` (in this crate) must agree
        // byte-for-byte with `protos::engine::CandidateScriptKind` (prost-
        // generated from `engine/protos/proto/composing.proto`). If the
        // proto definition is reshuffled the cast in
        // `raw_to_proto_candidate` (which feeds prost via `i32`) would
        // silently misroute; this test fails first.
        //
        // Per Codex post-impl finding #2 (P3, 2026-05-11).
        use protos::engine::CandidateScriptKind as ProtoCandidateScriptKind;
        assert_eq!(
            CandidateScriptKind::Unspecified.to_proto_i32(),
            ProtoCandidateScriptKind::Unspecified as i32
        );
        assert_eq!(
            CandidateScriptKind::Hant.to_proto_i32(),
            ProtoCandidateScriptKind::Hant as i32
        );
        assert_eq!(
            CandidateScriptKind::Tailo.to_proto_i32(),
            ProtoCandidateScriptKind::Tailo as i32
        );
        assert_eq!(
            CandidateScriptKind::Mixed.to_proto_i32(),
            ProtoCandidateScriptKind::Mixed as i32
        );
    }
}

#[cfg(test)]
mod typed_tone_pin_tests {
    use super::{reading_passes_typed_tones, TonePin, TypedBoundary};
    use phonetics::InputMode::{self, Poj, Tl};

    fn passes(mode: InputMode, typed: &str, reading: &str) -> bool {
        reading_passes_typed_tones(mode, typed, &[], &[], reading)
    }

    // §17 case 3 — the reported shape. Faces via `poj_num_syllable_ends_from_tl`:
    // trace: 程式 tîng-sik → POJ têng-sek → `teng5sek4` ends [5, 9];
    //        等式 tíng-sik → `teng2sek4`; 中式 ting-sik → `teng1sek4`.
    // Typed `teng5sek`: `teng` + `5` == 5 ✓, `sek` + no digit → free.
    #[test]
    fn partial_tone_pins_the_toned_syllable_and_frees_the_rest() {
        assert!(passes(Poj, "teng5sek", "tîng-sik"));
        assert!(!passes(Poj, "teng5sek", "tíng-sik"));
        assert!(!passes(Poj, "teng5sek", "ting-sik"));
    }

    // Reverse partial (`tengsek4`): first syllable free, second pinned to 4.
    // 程式 sik4 ✓; a tone-8 second syllable is rejected.
    // trace: 色 sik → `sek4`; 熟 si̍k → `sek8`.
    #[test]
    fn untoned_leading_syllable_is_free_and_later_digit_still_pins() {
        assert!(passes(Poj, "tengsek4", "tîng-sik"));
        assert!(!passes(Poj, "tengsek4", "tîng-si̍k"));
    }

    // Fully toned: every syllable pinned (the verbatim key covers dictionary
    // hits; custom entries rely on this).
    #[test]
    fn fully_toned_pins_every_syllable() {
        assert!(passes(Tl, "ting5sik4", "tîng-sik"));
        assert!(!passes(Tl, "ting5sik8", "tîng-sik"));
        assert!(!passes(Tl, "ting2sik4", "tîng-sik"));
    }

    // Partial-prefix path: the typed text may end at a boundary or
    // mid-syllable; everything typed so far must still be honored.
    // trace: 中西 ting-se → `teng1se1`; 程世 tîng-sè → `teng5se3`.
    #[test]
    fn prefix_typed_text_ending_early_keeps_the_pins_so_far() {
        assert!(passes(Poj, "teng5se", "tîng-sè"));
        assert!(!passes(Poj, "teng5se", "ting-se"));
        assert!(passes(Poj, "teng5s", "tîng-sik"));
        assert!(!passes(Poj, "teng2s", "tîng-sik"));
        // Typed runs past a 2-syllable reading into a 3rd: 中西區 ting-se-khu.
        assert!(passes(Poj, "teng1sekhu", "ting-se-khu"));
        assert!(!passes(Poj, "teng5sekhu", "ting-se-khu"));
    }

    // Fail-closed: typed text the face cannot account for is a reject —
    // a 3-syllable typed body against a 2-syllable reading.
    // trace: 中西 ting-se → `teng1se1`, exhausted with `khu` unconsumed.
    #[test]
    fn typed_text_running_past_the_face_rejects() {
        assert!(!passes(Poj, "teng1sekhu", "ting-se"));
    }

    // Nasal-`oo` alias: the typed spelling `hoonn` reaches 好 `hònn` through
    // the alias key; the pin must still place the digit and reject a
    // wrong tone rather than fail open on the spelling difference.
    // trace: 好玄 hònn-hiân → tl_num `honn3hian5`; alias per syllable `hoonn`.
    #[test]
    fn nasal_oo_alias_spelling_still_pins_the_tone() {
        assert!(passes(Tl, "hoonn3hian", "hònn-hiân"));
        assert!(!passes(Tl, "hoonn5hian", "hònn-hiân"));
        assert!(passes(Tl, "honn3hian", "hònn-hiân"));
    }

    // Codex post-impl 2026-09-14: the typed text may end INSIDE an alias
    // spelling too. Production: `si7honn` offers 是乎 `sī-honnh`, so
    // `si7hoonn` (alias, one letter short of `hoonnh`) must as well.
    // trace: sī-honnh → tl_num `si7honnh4`; syllable 2 letters `honnh`,
    //        alias `hoonnh`; rest `hoonn` is a prefix of the alias.
    #[test]
    fn typed_text_ending_inside_an_alias_spelling_passes() {
        assert!(passes(Tl, "si7honn", "sī-honnh"));
        assert!(passes(Tl, "si7hoonn", "sī-honnh"));
        assert!(passes(Tl, "si7hoonnh", "sī-honnh"));
        assert!(!passes(Tl, "si2hoonn", "sī-honnh"));
    }

    // A syllable whose letters are not the next thing typed cannot place
    // its digit → reject (the toneless guards own spelling; this must not
    // fail open on a mismatch). `ho5onn` is `ho` + `onn`, not the alias.
    // trace: 好 hònn → `honn3`, alias `hoonn` — neither is a prefix of `ho5onn`.
    #[test]
    fn misaligned_letters_reject() {
        assert!(!passes(Tl, "ho5onn", "hònn"));
        // 和唔 hô-onn → `ho5onn1` aligns and passes.
        assert!(passes(Tl, "ho5onn", "hô-onn"));
    }

    // ---- §52 typed `-` boundaries (USER 2026-09-22) ----
    fn passes_boundaries(
        mode: InputMode,
        typed: &str,
        boundaries: &[usize],
        reading: &str,
    ) -> bool {
        let plain: Vec<(usize, bool)> = boundaries.iter().map(|&at| (at, false)).collect();
        passes_kinds(mode, typed, &plain, reading)
    }

    fn passes_kinds(
        mode: InputMode,
        typed: &str,
        boundaries: &[(usize, bool)],
        reading: &str,
    ) -> bool {
        let boundaries: Vec<TypedBoundary> = boundaries
            .iter()
            .map(|&(at, khinsiann)| TypedBoundary { at, khinsiann })
            .collect();
        reading_passes_typed_tones(mode, typed, &boundaries, &[], reading)
    }

    /// Plain `-` boundaries at the ends of `stretches`, each one syllable.
    fn passes_stretches(typed: &str, stretches: &[(usize, usize)], reading: &str) -> bool {
        let boundaries: Vec<TypedBoundary> = stretches
            .iter()
            .map(|&(_, at)| TypedBoundary {
                at,
                khinsiann: false,
            })
            .collect();
        reading_passes_typed_tones(Tl, typed, &boundaries, stretches, reading)
    }

    // trace: typed `ai-` → stretch 0..2.
    //   愛 ài → one syllable over `ai`, walk ends [2] ✓ nothing inside.
    //   阿姨 a-î → `a` + `i`, walk ends [1, 2]; 1 lies inside 0..2 ✗.
    //   `siamtioh`, stretch 0..4: 閃著 siám-tio̍h ends [4, 8] ✓;
    //   si-am-tio̍h ends [2, 4, 8]; 2 lies inside 0..4 ✗.
    #[test]
    fn a_reading_may_not_split_a_one_syllable_stretch() {
        assert!(passes_stretches("ai", &[(0, 2)], "ài"));
        assert!(!passes_stretches("ai", &[(0, 2)], "a-î"));
        assert!(passes_stretches("siamtioh", &[(0, 4)], "siám-tio̍h"));
        assert!(!passes_stretches("siamtioh", &[(0, 4)], "si-am-tio̍h"));
        assert!(passes_stretches("aiai", &[(0, 2), (2, 4)], "ài-ài"));
        assert!(!passes_stretches("aiai", &[(0, 2), (2, 4)], "ài-a-î"));
        // A boundary with no one-syllable stretch keeps the split (`taigi-`).
        assert!(passes_boundaries(Tl, "ai", &[2], "a-î"));
    }

    // The kind of the typed run must match the reading's own separator at
    // that boundary; a plain `-` and a dictionary space are the same kind.
    #[test]
    fn typed_boundary_kind_must_match_the_readings_separator() {
        assert!(passes_kinds(Tl, "khiah", &[(3, true)], "khì--ah"));
        assert!(!passes_kinds(Tl, "khiah", &[(3, false)], "khì--ah"));
        assert!(passes_kinds(Tl, "jimkhi", &[(3, false)], "jím-khì"));
        assert!(!passes_kinds(Tl, "jimkhi", &[(3, true)], "jím-khì"));
        assert!(passes_kinds(Tl, "iasi", &[(2, false)], "iā sī"));
        assert!(!passes_kinds(Tl, "iasi", &[(2, true)], "iā sī"));
        // A trailing `--` constrains the next word, not this reading.
        assert!(passes_kinds(Tl, "tai", &[(3, true)], "tâi"));
        assert!(passes_kinds(Tl, "tai", &[(3, false)], "tâi"));
        // …but a reading that continues past it must continue the same way.
        assert!(passes_kinds(Tl, "tai", &[(3, true)], "tâi--uân"));
        assert!(!passes_kinds(Tl, "tai", &[(3, true)], "tâi-uân"));
        // The run typed right before the span (`at: 0`): read only by a
        // reading that opens with `--` itself (a custom `--ah` row).
        assert!(passes_kinds(Tl, "ah", &[(0, true)], "--ah"));
        assert!(!passes_kinds(Tl, "ah", &[(0, false)], "--ah"));
        assert!(passes_kinds(Tl, "ah", &[(0, true)], "ah"));
        assert!(passes_kinds(Tl, "ah", &[(0, false)], "ah"));
    }

    // trace: typed `khiah` with `-` after `khi` → boundary 3.
    //   去啊 khì--ah → `khi3ah4` ends [4, 7]; `khi` consumed → cursor 3 ✓.
    //   隙 khiah → `khiah4` one syllable; letters `khiah` run past 3 ✗.
    //   齒仔 khí-á → `khi2a2`; `khi` ✓ then `a` leaves `h` unconsumed ✗.
    #[test]
    fn typed_boundary_must_land_on_a_syllable_end_of_the_reading() {
        assert!(passes_kinds(Tl, "khiah", &[(3, true)], "khì--ah"));
        assert!(passes_boundaries(Tl, "khiah", &[3], "khì-ah"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "khiah"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "khia̍h"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "khí-á"));
        // Same reading, no boundary typed: everything still passes.
        assert!(passes_boundaries(Tl, "khiah", &[], "khiah"));
        assert!(passes_kinds(Poj, "khiah", &[(3, true)], "khì--ah"));
    }

    // A typed digit sits before the boundary: `khi3` + `-` + `ah` → 4.
    #[test]
    fn typed_boundary_and_typed_tone_are_both_required() {
        assert!(passes_kinds(Tl, "khi3ah", &[(4, true)], "khì--ah"));
        assert!(!passes_kinds(Tl, "khi3ah", &[(4, true)], "khí--ah"));
        assert!(!passes_boundaries(Tl, "khi3ah", &[4], "khiah"));
    }

    // Trailing `-` (`tai-`): the boundary is the span end; a reading whose
    // first syllable is exactly `tai` may continue (custom / learned rows
    // are matched on the typed prefix), one that runs past it may not.
    #[test]
    fn trailing_boundary_closes_the_syllable_the_user_typed() {
        assert!(passes_boundaries(Tl, "tai", &[3], "tâi"));
        assert!(passes_boundaries(Tl, "tai", &[3], "tâi-uân"));
        assert!(!passes_boundaries(Tl, "tai", &[3], "tâin"));
    }

    // `typed` is the lowercased shadow; a custom reading keeps its capital.
    #[test]
    fn boundary_check_reads_case_blind() {
        assert!(passes_kinds(Tl, "khiah", &[(3, true)], "Khì--ah"));
        assert!(!passes_boundaries(Tl, "khiah", &[3], "Khiah"));
    }

    #[test]
    fn admits_dispatches_per_variant() {
        assert!(TonePin::None.admits(None, "tíng-sik"));
        let typed = TonePin::TypedTones {
            mode: Poj,
            typed: "teng5sek".to_owned(),
            boundaries: Vec::new(),
            one_syllable_stretches: Vec::new(),
        };
        assert!(typed.admits(Some("poj:tengsek"), "tîng-sik"));
        assert!(!typed.admits(Some("poj:tengsek"), "tíng-sik"));
        // Custom entries: folded to canonical TL first (POJ-form roman in
        // POJ mode), and never folded when nothing is pinned.
        assert!(typed.admits_custom("têng-sek", Poj));
        assert!(!typed.admits_custom("téng-sek", Poj));
        assert!(TonePin::None.admits_custom("téng-sek", Poj));
        // TPS: exact path aligns on the matched key, whole-buffer on the body.
        // trace: 詩 si → tps notone `ㄒㄧ`, tone 1 → passes; 是 sī tone 7 → rejected.
        let space = TonePin::TpsSpaceEnd("ㄒㄧ".to_owned());
        assert!(space.admits(None, "si"));
        assert!(!space.admits(None, "sī"));
        assert!(space.admits(Some("tps:ㄒㄧ"), "si"));
        assert!(!space.admits(Some("tps:ㄒㄧ"), "sī"));
    }
}

#[cfg(test)]
mod edge_word_pick_tests {
    use super::*;
    use ranking::{CONTEXT_RANK_BUNDLED, CONTEXT_RANK_NONE, CONTEXT_RANK_USER};

    fn homophone(
        hanji: &str,
        frequency: u32,
        syllable_count: u8,
        user_weight: f64,
        context_rank: u8,
    ) -> RawCandidate {
        RawCandidate {
            consumed_span: (0, 3),
            syllable_count,
            display_text: hanji.to_owned(),
            roman: "tse".to_owned(),
            hanji: Some(hanji.to_owned()),
            canonical_tl: "tse".to_owned(),
            score: frequency as f32,
            form: FORM_NOTONE,
            frequency,
            walker_cost: EQUAL_COST,
            bitmask: 0,
            script_kind: CandidateScriptKind::Hant,
            user_weight,
            context_rank,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        }
    }

    /// A typical seen word's corpus cost; the tests that do not exercise the
    /// corpus dimension give every homophone this one.
    const EQUAL_COST: u16 = 7_000;

    fn costed(candidate: RawCandidate, walker_cost: u16) -> RawCandidate {
        RawCandidate {
            walker_cost,
            ..candidate
        }
    }

    fn picked(homophones: Vec<RawCandidate>, context_free: bool) -> Option<String> {
        pick_edge_word(homophones, 3, context_free).map(|c| c.display_text)
    }

    // E1 P5a: the word the corpus writes most fills the edge, whatever the
    // dictionary frequency (production `koh`: 擱 and 閣 share frequency
    // 29,890; `kap`: 甲 outranks 佮 on source rank).
    #[test]
    fn corpus_cost_takes_the_word_over_frequency() {
        let key = vec![
            costed(homophone("擱", 29_890, 1, 0.0, CONTEXT_RANK_NONE), 9_000),
            costed(homophone("閣", 29_890, 1, 0.0, CONTEXT_RANK_NONE), 6_000),
            costed(homophone("故", 40_000, 1, 0.0, CONTEXT_RANK_NONE), 8_000),
        ];
        assert_eq!(picked(key, true).as_deref(), Some("閣"));
    }

    // E1 P5a: an equal cost (corpus-unseen words share one smoothed cost)
    // leaves the pick to the dictionary score, as before P5a.
    #[test]
    fn equal_cost_falls_to_the_dictionary_score() {
        let key = vec![
            costed(homophone("狗巢", 25, 2, 0.0, CONTEXT_RANK_NONE), 13_180),
            costed(homophone("狗岫", 50, 2, 0.0, CONTEXT_RANK_NONE), 13_180),
        ];
        assert_eq!(picked(key, true).as_deref(), Some("狗岫"));
    }

    // A selected word still leads the corpus: user weight is the first
    // dimension of the pick.
    #[test]
    fn selected_word_beats_the_corpus_choice() {
        let key = vec![
            costed(homophone("教授", 10, 2, 0.0, CONTEXT_RANK_NONE), 7_000),
            costed(homophone("狗岫", 25, 2, 0.2, CONTEXT_RANK_NONE), 13_180),
        ];
        assert_eq!(picked(key, true).as_deref(), Some("狗岫"));
    }

    // Learned phrases (§50) carry no corpus cost: a dictionary homophone
    // wins unless the user's weight or the previous word's context favours
    // the learned row, and a learned row alone is the edge.
    #[test]
    fn learned_row_competes_through_user_weight_and_context_only() {
        let dictionary = || costed(homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE), 7_000);
        let learned = |user_weight, context_rank| {
            costed(
                homophone("姊", 0, 1, user_weight, context_rank),
                WALKER_COST_UNPRICED,
            )
        };
        assert_eq!(
            picked(vec![learned(0.0, CONTEXT_RANK_NONE), dictionary()], true).as_deref(),
            Some("這")
        );
        assert_eq!(
            picked(vec![dictionary(), learned(0.3, CONTEXT_RANK_NONE)], true).as_deref(),
            Some("姊")
        );
        assert_eq!(
            picked(vec![dictionary(), learned(0.0, CONTEXT_RANK_USER)], false).as_deref(),
            Some("姊")
        );
        assert_eq!(
            picked(vec![learned(0.0, CONTEXT_RANK_NONE)], true).as_deref(),
            Some("姊")
        );
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK (§56): a context hit takes the
    // edge's word; with every corpus cost equal here, the context-free pick
    // is the frequency order.
    #[test]
    fn context_hit_takes_the_word() {
        let key = || {
            vec![
                homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE),
                homophone("濟", 100, 1, 0.0, CONTEXT_RANK_BUNDLED),
                homophone("姊", 10, 1, 0.0, CONTEXT_RANK_USER),
            ]
        };
        assert_eq!(picked(key(), true).as_deref(), Some("這"));
        assert_eq!(picked(key(), false).as_deref(), Some("姊"));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK: the pick never changes the edge's
    // cost inputs — a hit with another syllable count or a lower user weight
    // than the context-free winner cannot take the edge.
    #[test]
    fn context_hit_keeps_syllable_count_and_user_weight() {
        let two_syllables = vec![
            homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE),
            homophone("濟濟", 100, 2, 0.0, CONTEXT_RANK_USER),
        ];
        assert_eq!(picked(two_syllables, false).as_deref(), Some("這"));
        let selected_stranger = vec![
            homophone("這", 1000, 1, 0.5, CONTEXT_RANK_NONE),
            homophone("濟", 100, 1, 0.0, CONTEXT_RANK_USER),
        ];
        assert_eq!(picked(selected_stranger, false).as_deref(), Some("這"));
        let hit_among_equals = vec![
            homophone("這", 1000, 1, 0.5, CONTEXT_RANK_NONE),
            homophone("濟", 100, 1, 0.5, CONTEXT_RANK_USER),
        ];
        assert_eq!(picked(hit_among_equals, false).as_deref(), Some("濟"));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK with corpus costs: the context-free
    // winner is now the cheapest word, and the contextual pick keeps its
    // `syllable_count` and `user_weight` — with `span_walker_cost` taken
    // over the key regardless of the pick, every `lattice::edge_cost` input
    // is the same with and without the context.
    #[test]
    fn context_hit_keeps_the_corpus_winners_cost_inputs() {
        let key = || {
            vec![
                costed(homophone("這", 1000, 1, 0.0, CONTEXT_RANK_NONE), 7_000),
                costed(homophone("這兜", 10, 2, 0.0, CONTEXT_RANK_NONE), 5_000),
                costed(homophone("濟", 100, 1, 0.0, CONTEXT_RANK_USER), 9_000),
                costed(homophone("姊妹", 1, 2, 0.0, CONTEXT_RANK_BUNDLED), 12_000),
            ]
        };
        let context_free = pick_edge_word(key(), 3, true).expect("a word");
        let contextual = pick_edge_word(key(), 3, false).expect("a word");
        assert_eq!(context_free.display_text, "這兜");
        assert_eq!(contextual.display_text, "姊妹");
        assert_eq!(contextual.syllable_count, context_free.syllable_count);
        assert_eq!(contextual.user_weight, context_free.user_weight);
    }

    #[test]
    fn no_homophone_no_word() {
        assert_eq!(picked(Vec::new(), false), None);
    }
}
