//! Generated protobuf bindings for the Taigi engine wire types.
//!
//! Hand-written shell crate: every type is `include!`'d from the prost-build
//! output (`$OUT_DIR/taigi.engine.rs`), regenerated each build by `build.rs`
//! from `engine/protos/proto/{envelope,phonetics,composing,lexicon,nextword,
//! case}.proto`. Platform-side bindings (iOS `.pb.swift`, Android `.java`)
//! are NOT generated here — see `engine/scripts/gen-platform-protos.sh`.
//!
//! `clippy::all` + `clippy::pedantic` are silenced because the included
//! prost output lints noisily and is regenerated on every build.

#![allow(clippy::all, clippy::pedantic)]

pub mod engine {
    include!(concat!(env!("OUT_DIR"), "/taigi.engine.rs"));
}

impl engine::AppConfig {
    /// Whether candidate cells render romanization only (候選詞顯示 = 羅馬字).
    ///
    /// The single normalisation point for `candidate_display_mode`: the
    /// proto3 default `0`, an unknown value from a newer platform, and
    /// `SIDE_BY_SIDE` all answer `false` (legacy behaviour), so the two
    /// engine readers can never drift on the fallback.
    pub fn is_roman_only_display(&self) -> bool {
        // prost's accessor already maps an unknown value to `Unspecified`.
        self.candidate_display_mode() == engine::CandidateDisplayMode::RomanOnly
    }

    /// Whether every candidate cell shows ONE script (羅馬字, or 漢羅濫's
    /// split cells) — the displays under which a cell that reads like an
    /// earlier one is collapsed into it (§42 / §44), so the collapsed row's
    /// identity has to survive on the survivor. For 漢羅濫 the collapse is the
    /// platform's (§42 split) and the engine relies on it keeping the FIRST
    /// roman cell — the slot-0 §34 literal — as the survivor. 並排 (and the
    /// same fallbacks as [`Self::is_roman_only_display`]) answer `false`.
    pub fn is_single_script_display(&self) -> bool {
        matches!(
            self.candidate_display_mode(),
            engine::CandidateDisplayMode::RomanOnly | engine::CandidateDisplayMode::Combined
        )
    }
}
