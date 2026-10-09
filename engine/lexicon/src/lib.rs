//! `engine/lexicon` — Rust read-path crate for the v3.5.6 lexicon slice.
//!
//! Owns the prefix index (fst), the bundled binary readers
//! (`dictionary.bin` + `association.bin`), and the search orchestration
//! that powers IME autocomplete, Tab3 dictionary lookup, and the
//! bundled bigram lookup (`api::lookup_associations`). engine/dispatch calls it for
//! nextword `PredictNext`; the `nextword` crate never imports `lexicon`.
//!
//! Crate is `unsafe_code = "forbid"`. The mmap unsafe carve-out lives
//! exclusively in `engine/mmap-host`.

pub mod api;
pub mod association_reader;
pub mod classification;
mod continuous;
mod dictionary_filters;
pub mod dictionary_reader;
mod error;
mod handle;
mod paths;
pub mod prefix_index;
pub mod requests;
pub mod search;
mod syllable_inventory;
mod tps_pattern;

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<handle::EngineHandle>();
};

pub use continuous::{
    best_candidate_for_key_with_barriers, compound_hanji_exists, fetch_abbrev_candidates,
    fetch_candidates_for_keys_with_barriers, fetch_partial_prefix_candidates,
    fetch_partial_prefix_candidates_unbounded, homophone_words_for_key, ConsumedSpan,
    ContinuousFetchCtx, CustomEntry, EdgeBest, LearnedEntry, RawCandidate, TonePin, TypedBoundary,
    COVERAGE_KIND_ABBREV, COVERAGE_KIND_FULL, COVERAGE_KIND_PARTIAL_PREFIX,
    PARTIAL_PREFIX_HYDRATE_CAP, PARTIAL_PREFIX_OUTPUT_CAP,
};
pub use error::LexiconError;
pub use handle::EngineHandle;
pub use paths::LexiconPaths;
pub use syllable_inventory::SyllableInventory;
