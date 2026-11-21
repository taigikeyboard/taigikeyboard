//! Document-text policies the controller applies around a commit:
//! auto-space and full-width punctuation. The decisions are the one copy the
//! three desktops run; the auto-space attaching set is the engine's, and
//! macOS keeps a Swift twin of the full-width map for the TPS punctuation
//! chord and the swap check its controller makes itself.

mod auto_space;
mod full_width;

pub use auto_space::{is_gate_active, spaces_candidate_commits};
pub use full_width::{full_width_mapped, is_punctuation_chord_key};
