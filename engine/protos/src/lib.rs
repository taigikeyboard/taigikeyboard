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
