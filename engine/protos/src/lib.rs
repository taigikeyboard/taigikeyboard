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

// 中文: Taigi 引擎共用的 protobuf wire 型別 crate,內容全部由 prost-build 在編譯期產生並 include 進來。
// 中文: 平台端 .pb.swift / .java 綁定不在此產生,改由 gen-platform-protos.sh 處理。

#![allow(clippy::all, clippy::pedantic)]

pub mod engine {
    include!(concat!(env!("OUT_DIR"), "/taigi.engine.rs"));
}

// 中文: 候選詞顯示 wire 值的唯一正規化點 — 0 / 未知 / SIDE_BY_SIDE 都是漢羅並排。
impl engine::AppConfig {
    /// Whether candidate cells render romanization only (候選詞顯示 = 羅馬字).
    ///
    /// The single normalisation point for `candidate_display_mode`: the
    /// proto3 default `0`, an unknown value from a newer platform, and
    /// `SIDE_BY_SIDE` all answer `false` (legacy behaviour), so the two
    /// engine readers can never drift on the fallback.
    pub fn is_roman_only_display(&self) -> bool {
        engine::CandidateDisplayMode::try_from(self.candidate_display_mode)
            .is_ok_and(|mode| mode == engine::CandidateDisplayMode::RomanOnly)
    }
}
