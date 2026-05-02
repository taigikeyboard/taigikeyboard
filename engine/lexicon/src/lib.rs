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
pub mod dictionary_reader;
pub mod dispatch;
pub mod error;
pub mod handle;
pub mod key_normalizer;
pub mod paths;
pub mod prefix_index;
pub mod search;

const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<handle::EngineHandle>();
};

pub use error::LexiconError;
pub use handle::EngineHandle;
pub use paths::LexiconPaths;
