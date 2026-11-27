//! Generates the Swift bridge for `src/lib.rs` and the prost types for
//! `proto/desktop_shell.proto`. The Swift bridge is staged into `macos/RustEngine/`
//! by `engine/scripts/build-macos-xcframework.sh`, beside the engine's.

use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=proto");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    // `RustTaigiDesktop`, not `RustTaigi`: swift-bridge writes each bridge to
    // `<OUT_DIR>/<name>/<name>.{h,swift}`, and the engine's already owns the
    // `RustTaigi` pair. Both land in the one `RustTaigi` Clang module.
    swift_bridge_build::parse_bridges(vec!["src/lib.rs"])
        .write_all_concatenated(&out_dir, "RustTaigiDesktop");

    prost_build::Config::new().compile_protos(&["proto/desktop_shell.proto"], &["proto"])
}
