//! `fst-builder` — build / query helper for the dictionary fst prefix index.
//!
//! Replaces the prior `marisa_trie` Python C-binding pipeline. Stdin protocol
//! defined in `docs/engine/lexicon-slice-plan.md` §2.3.
//!
//! Subcommands:
//!   build <output.fst>            — read stdin TL\tkey\trowid pairs, write fst
//!   build-syllables <output.fst>  — read stdin tl_num lines, emit canonical
//!                                    TL syllable inventory FST (v3.5.8 Phase 2)
//!   query <input.fst> <prefix>    — list keys + rowids whose key has prefix

// 中文: 字典 fst 前綴索引的建置 / 查詢命令列工具,取代舊版 marisa_trie Python 綁定。

mod builder;
mod query;
mod syllables;

use std::process::ExitCode;

const USAGE: &str = "\
fst-builder build <output.fst>            (reads stdin: TL\\tkey\\trowid lines)
fst-builder build-syllables <output.fst>  (reads stdin: one tl_num per line)
fst-builder query <input.fst> <prefix>    (lists keys + rowids matching prefix)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, output] if cmd == "build" => match builder::run_build(output.as_str()) {
            Ok(stats) => {
                eprintln!(
                    "[fst-builder] built {} (entries={}, distinct_keys={})",
                    output, stats.entries, stats.distinct_keys
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("[fst-builder] build failed: {}", e);
                ExitCode::FAILURE
            }
        },
        [cmd, output] if cmd == "build-syllables" => match syllables::run_build(output.as_str()) {
            Ok(stats) => {
                eprintln!(
                    "[fst-builder] built {} (lines_in={}, syllables={}, valid={}, invalid_skipped={}, distinct_keys={})",
                    output,
                    stats.lines_in,
                    stats.syllables_extracted,
                    stats.valid_syllables,
                    stats.invalid_skipped,
                    stats.distinct_keys_out,
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("[fst-builder] build-syllables failed: {}", e);
                ExitCode::FAILURE
            }
        },
        [cmd, fst_path, prefix] if cmd == "query" => match query::run_query(fst_path, prefix) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("[fst-builder] query failed: {}", e);
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage:\n{}", USAGE);
            ExitCode::FAILURE
        }
    }
}
