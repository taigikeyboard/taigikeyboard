//! Hermetic integration tests — one binary, one link (`Cargo.toml`
//! `autotests = false`); each `tests/<file>.rs` stays in place as a module
//! of this root. A module that installs a fixture lexicon holds
//! `test_support::engine_install_lock` from install through its last assertion;
//! the others never depend on what the process-wide lexicon holds. A test
//! on the production lexicon goes to `prod.rs`; one that needs no lexicon
//! installed at all goes to `no_lexicon.rs`.

mod common;

mod composing_caret;
mod continuous_abbrev;
mod continuous_custom_priority;
mod continuous_custom_reach;
mod continuous_custom_unicode_form;
mod continuous_explicit_tone;
mod continuous_keys_tl_hyphen;
mod continuous_keys_tl_lattice;
mod continuous_keys_tl_poj_diacritic;
mod continuous_keys_tps;
mod continuous_learned_phrase;
mod continuous_one_syllable_segment;
mod continuous_partial_tone;
mod continuous_phase;
mod continuous_slot0_dict_roman;
mod continuous_slot0_user_selection;
mod continuous_typed_boundary;
mod continuous_typed_separator;
mod golden_fetch_at_pos;
mod intent_coverage;
mod invariants;
mod nasal_marker_case;
mod prediction_commit_resolution;
mod proptest_sequences;
mod raw_input_pending_tail;
mod roman_only_display_dedup;
mod syllabifier_tl;
mod syllable_separator_display;
mod tps_display_dedup;
mod tps_hanji_conversion;
mod tps_hanji_conversion_caret;
mod tps_hanji_conversion_pick;
mod tps_key;
mod tps_space_pinned_tone;
mod tps_typed_tone;

/// Every `tests/*.rs` is a `[[test]]` root in `Cargo.toml` or a `mod` of one,
/// so a new test file cannot silently stop running under `autotests = false`.
#[test]
fn every_test_file_belongs_to_a_test_binary() {
    test_support::assert_every_test_file_is_declared(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )));
}
