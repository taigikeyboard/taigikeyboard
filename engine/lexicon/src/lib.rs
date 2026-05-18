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

// 中文: lexicon crate — 詞庫讀取路徑的 Rust 實作。
// 中文: 包含 FST 前綴索引、TKDB/TKWA 二進位讀取器與搜尋協調邏輯,服務 IME 候選詞、Tab3 字典查詢與 NextWord bigram 查詢。

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

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<handle::EngineHandle>();
};

pub use continuous::{
    best_candidate_for_key, compound_hanji_exists, derive_mode, fetch_candidates_for_endings,
    fetch_candidates_for_keys, fetch_partial_prefix_candidates, CandidateMode, ConsumedSpan,
    CustomEntry, RawCandidate, COVERAGE_KIND_FULL, COVERAGE_KIND_PARTIAL_PREFIX, FORM_NOTONE,
    PARTIAL_PREFIX_CAP,
};
pub use error::LexiconError;
pub use handle::EngineHandle;
pub use paths::LexiconPaths;
pub use syllable_inventory::SyllableInventory;
