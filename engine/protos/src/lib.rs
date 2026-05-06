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
