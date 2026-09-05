// Compile-time: calls prost-build to compile every .proto under proto/ into Rust types in OUT_DIR.

use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=proto");
    prost_build::Config::new().compile_protos(
        &[
            "proto/envelope.proto",
            "proto/phonetics.proto",
            "proto/composing.proto",
            "proto/lexicon.proto",
            "proto/nextword.proto",
            "proto/case.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}
