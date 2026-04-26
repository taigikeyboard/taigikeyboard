//! Generates the Swift bridge code from the `#[swift_bridge::bridge]` block in
//! `src/lib.rs`. The output is written to `OUT_DIR` and copied into the
//! xcframework headers + sibling `RustTaigi.swift` by
//! `engine/scripts/build-xcframework.sh`.

fn main() {
    let bridges = vec!["src/lib.rs"];
    for path in &bridges {
        println!("cargo:rerun-if-changed={path}");
    }
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR set by cargo");
    swift_bridge_build::parse_bridges(bridges).write_all_concatenated(out_dir, "RustTaigi");
}
