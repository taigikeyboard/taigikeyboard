//! Lexicon integration tests over hermetic fixtures — one binary, one link
//! (`Cargo.toml` `autotests = false`); each `tests/<file>.rs` stays in place
//! as a module of this root. Only `parity` installs the process-wide
//! lexicon, behind its own lock. The CSV parity suites are `prod.rs`.

mod common;

mod association_v2;
mod compound_hanji;
mod dictionary_reader_v4;
mod fused_toneless_key;
mod parity;
mod partial_prefix_syllable_reach;
mod prefix_index_length_bands;
mod rejects_v1;
mod span_local_fetch;
mod syllables_fst;
mod tlpoj_partial_prefix_abbrev;
mod tps_partial_prefix_abbrev;
mod tps_readings;
mod user_freq_plumb;
mod walker_model_metadata;
mod whole_buffer_abbrev;

/// Every `tests/*.rs` is a `[[test]]` root in `Cargo.toml` or a `mod` of one,
/// so a new test file cannot silently stop running under `autotests = false`.
#[test]
fn every_test_file_belongs_to_a_test_binary() {
    test_support::assert_every_test_file_is_declared(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )));
}
