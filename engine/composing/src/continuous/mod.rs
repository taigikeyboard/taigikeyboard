//! v3.5.9 A2 — `composing::continuous` seam.
//!
//! `assemble_candidates` is the named seam between `requests::handle_fetch_at_pos`
//! and the lexicon resolution primitives. It owns the 6-step continuous-input
//! candidate-vector contract (post-A2 reframing; line refs are `main` after
//! the seam extract — re-grep before relying on numbers):
//!
//! 1. Build the FST keys for this fetch: ALL modes (Tl / Poj / Tps / English)
//!    run `shadow::build_shadow_lattice_with_barriers` ONCE + `shadow::
//!    left_anchored_keys_and_restrictions` projection (single `LexiconHandle::
//!    with_state` scope around steps 1–4, **D1 fold**). v3.5.9 D / C-3b
//!    retired the legacy TPS-specific `build_keys_tps` short-circuit (which
//!    folded TPS into `tl:` keys via `phonetics::tps_to_tl`); TPS now walks
//!    the same shadow → lattice path as TL/POJ with the only difference
//!    being the family the syllabifier + key emitter resolve through
//!    `mode` (`tps:` against the C-0 emit of `dictionary.fst`).
//! 2. `keys.is_empty()` → [`fetch_via_lexicon_partial_inner`] (Item 10
//!    fallthrough) for ALL modes (TL/POJ/English/TPS). Each mode emits its
//!    own family prefix via [`phonetics::KeyFamily::for_input_mode`], so a TPS
//!    leading initial like `ㄉ` scans `tps:ㄉ` byte-range exactly as TL's
//!    `g` scans `tl:g`.
//!    Else → `lexicon::fetch_candidates_for_keys_with_barriers` (span-local
//!    fetch).
//! 3. Per-candidate recase loop (`recase_roman` over each
//!    `consumed_span`; presentation `roman` only —`display_text` / `hanji`
//!    untouched).
//! 4. `fetch_walker_slot0_inner(..) → Some(slot0)`: span-aware
//!    retain-dedupe on `(hanji, consumed_span)` + identity or recased roman, then
//!    `insert(0, ..)`. The walker returns a [`WalkerSlot0`] and the seam
//!    converts it to a `RawCandidate` (slot 0 is an explicit prepend, not
//!    sort-compared, so `score` / `frequency` / `bitmask` stay `0`).
//!
//!    **Step 4b — whole-input prefix-extension scan (else-branch only).**
//!    After the walker slot-0 prepend, run
//!    [`fetch_via_lexicon_partial_inner`] against the whole-buffer key
//!    from [`crate::shadow::build_partial_prefix_key`] so dict rows
//!    whose key extends BEYOND any lattice edge (e.g. `tl:taigir` /
//!    `tâi-gír` for input `taigi`) surface as
//!    `COVERAGE_KIND_PARTIAL_PREFIX` candidates. Gated by
//!    `synth_consumed_span` (skipped when raw carries a pending
//!    trailing hyphen, same suppression as walker slot-0). Mainstream
//!    IME parity (librime `LookupWords(predictive=true)`, khiin-rs
//!    `candidates_for_splittable`, our own `lexicon::search`
//!    exact++prefix merge); the empty-keys branch above already
//!    covers this for inputs with no valid syllable, Step 4b extends
//!    it to already-syllabified inputs. Cross-batch
//!    `(canonical_tl | roman, hanji, consumed_span)` dedupe drops extension hits
//!    already present in the FULL block. Step 4d then re-sorts the merged
//!    blocks by `CandidateSortKey` (tier first, USER 2026-10-12), so an
//!    extension covering the whole input leads front-segment rows. A picked
//!    multi-syllable extension then swaps above a walker sentence
//!    (`assemble::picked_extension_outranks_sentence`); every other slot 0 stays.
//! 5. POJ presentation pass: `mode == Poj` → [`recase_tl_as_poj_display`]
//!    over every candidate's `roman` then [`dedupe_rendered_continuous`].
//!    Skipped (not identity-called) for TL / English / TPS — the gate sits
//!    at the caller so the helper itself is POJ-only by signature. Runs
//!    AFTER the step-4 retain-dedupe so the prepended slot-0 row never
//!    moves; both dedupes preserved.
//! 6. The dispatch caller wraps the resulting `Vec<RawCandidate>` into
//!    `ContinuousResponse` via `raw_to_proto_candidate` + `with_continuous`.
//!
//! **D1 fold (B2 lifecycle invariant).** Before A2 the shadow + lattice was
//! built twice per non-TPS `FetchAtPos` — once in the key-building pass,
//! once at the head of `fetch_walker_slot0`. A2 hoists ONE `LexiconHandle::
//! with_state` scope into [`assemble_candidates`] and calls
//! `build_shadow_lattice_with_barriers(raw, inv, mode)` exactly once. Byte-identical
//! under the mobile-IME lifecycle invariant — `EngineHandle::install` is
//! reachable only from `lexicon::api::install_engine` (mobile bridge
//! startup) plus the two test fixtures (`lexicon/tests/parity.rs`,
//! `composing/tests/golden_fetch_at_pos.rs`), never from a composing
//! `FetchAtPos`. Re-grep `EngineHandle::install` reachability whenever
//! touching this seam.
//!
//! **B1 step ordering** is load-bearing — step 5 (POJ render + post-render
//! dedupe) MUST run after step 4 (slot-0 prepend). Reordering changes the
//! visible candidate vector.
//!
//! [`fetch_via_lexicon_partial_inner`]: partial_prefix::fetch_via_lexicon_partial_inner
//! [`WalkerSlot0`]: walk::WalkerSlot0
//! [`recase_tl_as_poj_display`]: recase::recase_tl_as_poj_display
//! [`dedupe_rendered_continuous`]: dedupe::dedupe_rendered_continuous
//! [`assemble_candidates`]: assemble::assemble_candidates

mod assemble;
mod dedupe;
mod edge;
mod partial_prefix;
mod recase;
mod segmentation;
mod walk;

pub(crate) use assemble::{assemble_candidates, ListShape};
pub(crate) use dedupe::retain_first_by_key;
pub(crate) use edge::{edge_homophone_words, edge_toneless_bodies};
pub(crate) use segmentation::{explain_segmentation, BufferCut, Segmentation, WordCut};
pub(crate) use walk::{synth_consumed_span, walk_buffer};
