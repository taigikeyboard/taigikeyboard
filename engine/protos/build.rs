// 中文: 編譯期呼叫 prost-build 把 proto/ 下所有 .proto 編譯成 Rust 型別,輸出至 OUT_DIR。

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
