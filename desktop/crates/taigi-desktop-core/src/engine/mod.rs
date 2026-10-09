//! The engine bridge: the protobuf envelope to `dispatch::process_request`
//! and the typed per-slice surfaces over it (composing, lexicon, next-word).
//!
//! In-process on Windows and Linux, and on macOS behind `taigi-macos-ffi` —
//! no engine FFI seam — but the SAME envelope contract the iOS/Android
//! bridges speak, so the engine is reused unchanged and every `file:line` in
//! `docs/engine/` applies here too.

mod bridge;
mod composing;
pub mod dictionary_search;
mod external_lookup;
mod lexicon;
mod nextword;
mod phonetics;
mod transition;
pub mod user_data;

pub use composing::{
    append, commit_as_shown, commit_as_typed, commit_continuous,
    commit_preedit_then_insert_external, commit_raw, delete_backward, fetch_at_pos, move_caret,
    reset, telex_key, tps_key, CommitContinuousArgs, CommitScript,
};
pub use external_lookup::{chhoe_url, moe_url};
pub use lexicon::{
    dictionary_filters, install as lexicon_install, is_hanji, search_by_hanji, search_with_sources,
    DictionaryFilters, DictionarySource, LexiconInstallStats, LexiconRow,
    ALL_SOURCES_ENABLED_SEARCH_BITMASK,
};
pub use nextword::{
    reset_all as nextword_reset_all,
    update_last_selected_word as nextword_update_last_selected_word,
    word_selected as nextword_word_selected,
};
pub use phonetics::{is_attaching_punctuation, tl_display_to_tps, tl_to_poj, TPS_OR_MAPS_TO_ER};
pub use transition::{
    ComposingTransition, ContinuousCandidate, ContinuousCommitResult, ContinuousFetchResult, Effect,
};

#[cfg(test)]
pub(crate) use transition::test_support;
