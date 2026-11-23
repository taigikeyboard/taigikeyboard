//! Integration tests over the production lexicon (`assets/dictionaries/`),
//! installed once per process — never next to a hermetic fixture install.

mod common;

mod candidate_dump;
mod continuous_context_rerank;
mod cross_mode_parity;
mod explain_slot0_cut_prod;
mod hyphen_toned_span_prod;
mod one_syllable_segment_prod;
mod oov_syllable_synth_prod;
mod telex_prod;
mod tps_glyph_alias;
mod tps_hanji_conversion_prod;
mod walker_fixed_inputs_prod;
mod walker_gold;
