//! The engine bridge: the protobuf envelope to `dispatch::process_request`
//! and the typed per-slice surfaces over it (composing, lexicon, next-word).
//!
//! In-process on Windows — no FFI seam — but the SAME envelope contract the
//! iOS/Android/macOS bridges speak, so the engine is reused unchanged and
//! every `file:line` in `docs/engine/` applies here too. Port of
//! `macos/Sources/TaigiInputMethodCore/Engine/RustEngineBridge*.swift`.

mod bridge;
mod composing;
mod external_lookup;
mod lexicon;
mod nextword;
mod phonetics;
mod transition;

pub use composing::{
    append, commit_continuous, commit_preedit_then_insert_external, commit_raw, delete_backward,
    enter_continuous, fetch_at_pos, reset, CommitContinuousArgs, CustomEntry, FetchArgs,
    FrequencyRow,
};
pub use external_lookup::{chhoe_url, digit_tone_form, moe_url};
pub use lexicon::{
    dictionary_filters, enabled_sources_bitmask, install as lexicon_install, is_hanzi,
    search_by_hanzi, search_with_sources, DictionaryFilters, DictionarySource, LexiconInstallStats,
    LexiconRow, ALL_SOURCES_ENABLED_SEARCH_BITMASK, NO_SOURCES_ENABLED_BITMASK,
};
pub use nextword::{
    reset_full as nextword_reset_full,
    update_last_selected_word as nextword_update_last_selected_word,
    word_selected as nextword_word_selected, AssociationPair, NextWordEffect, NextWordOutcome,
};
pub use phonetics::{
    derive_custom_query_key, derive_custom_search_keys, nfd_preprocess_for_lookup, poj_to_tl,
    strip_tone, tl_to_poj, CustomSearchKey,
};
pub use transition::{
    CandidateMode, ComposingTransition, ContinuousCandidate, ContinuousFetchResult, Effect,
};

#[cfg(test)]
pub(crate) use transition::test_support;
