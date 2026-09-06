//! `engine/lexicon` — Rust read-path crate for the v3.5.6 lexicon slice.
//!
//! Owns the prefix index (fst), the bundled binary readers
//! (`dictionary.bin` + `association.bin`), and the search orchestration
//! that powers IME autocomplete, Tab3 dictionary lookup, and the
//! NextWord crate's bundled bigram lookup (called via the platform
//! bridges, NOT via direct cross-crate import — see
//! `engine/protos/proto/lexicon.proto::AssocLookupRequest`).
//!
//! Crate is `unsafe_code = "forbid"`. The mmap unsafe carve-out lives
//! exclusively in `engine/mmap-host`.

pub mod api;
pub mod association_reader;
pub mod classification;
pub mod continuous;
mod dictionary_filters;
pub mod dictionary_reader;
pub mod dispatch;
pub mod error;
pub mod handle;
pub mod key_normalizer;
pub mod paths;
pub mod prefix_index;
pub mod search;
pub mod syllable_inventory;
pub mod tps_pattern;

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<handle::EngineHandle>();
};

pub use continuous::{
    best_candidate_for_key, best_candidate_for_key_with_barriers, compound_hanji_exists,
    derive_mode, fetch_candidates_for_keys_with_barriers, fetch_partial_prefix_candidates,
    fetch_partial_prefix_candidates_unbounded, reading_passes_space_pin, CandidateMode,
    ConsumedSpan, ContinuousFetchCtx, CustomEntry, RawCandidate, COVERAGE_KIND_FULL,
    COVERAGE_KIND_PARTIAL_PREFIX, FORM_NOTONE, PARTIAL_PREFIX_HYDRATE_CAP,
    PARTIAL_PREFIX_OUTPUT_CAP,
};
pub use error::LexiconError;
pub use handle::EngineHandle;
pub use paths::LexiconPaths;
pub use syllable_inventory::SyllableInventory;
