//! Phonetics integration tests — one binary, one link (`Cargo.toml`
//! `autotests = false`); each `tests/<file>.rs` stays in place as a module
//! of this root.

mod canonical_tl_form;
mod case_transform_golden;
mod empty;
mod fixtures;
mod invariants;
mod normalize_tone_nasal_case;
mod op_coverage;
mod roman_only_spelling_parity;
mod roundtrip;

/// Every `tests/*.rs` is a `[[test]]` root in `Cargo.toml` or a `mod` of one,
/// so a new test file cannot silently stop running under `autotests = false`.
#[test]
fn every_test_file_belongs_to_a_test_binary() {
    test_support::assert_every_test_file_is_declared(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )));
}
