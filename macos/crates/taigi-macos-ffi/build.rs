//! Generates the Swift bridge (`RustTaigi`) for `src/lib.rs` and the prost
//! types for `proto/desktop_shell.proto`. The Swift bridge is staged into
//! `macos/RustEngine/` by `engine/scripts/build-macos-xcframework.sh`.

use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=proto");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    swift_bridge_build::parse_bridges(vec!["src/lib.rs"])
        .write_all_concatenated(&out_dir, "RustTaigi");

    prost_build::Config::new().compile_protos(&["proto/desktop_shell.proto"], &["proto"])
}
