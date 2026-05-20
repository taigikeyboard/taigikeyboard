//! `fst-builder` — build / query helper for the dictionary fst prefix index.
//!
//! Replaces the prior `marisa_trie` Python C-binding pipeline. Stdin protocol
//! defined in `docs/engine/lexicon-slice-plan.md` §2.3.
//!
//! Subcommands:
//!   build <output.fst>
//!     reads stdin TL\tkey\trowid pairs, writes fst
//!
//!   build-syllables <output.fst> --tl-input <path> --poj-input <path>
//!     reads canonical `tl_num` / `poj_num` lines from two separate
//!     paths, emits the v3.5.9 B-1 tagged-single-FST syllable
//!     inventory (`tl:` + `poj:` families in one fst::Set)
//!
//!   query <input.fst> <prefix>
//!     list keys + rowids whose key has prefix

// 中文: 字典 fst 前綴索引的建置 / 查詢命令列工具,取代舊版 marisa_trie Python 綁定。

mod builder;
mod query;
mod syllables;

use std::process::ExitCode;

const USAGE: &str = "\
fst-builder build <output.fst>
    (reads stdin: TL\\tkey\\trowid lines)
fst-builder build-syllables <output.fst> --tl-input <path> --poj-input <path>
    (reads tl_num lines from --tl-input, poj_num lines from --poj-input;
     emits tagged-single-FST with tl:/poj: key prefixes)
fst-builder query <input.fst> <prefix>
    (lists keys + rowids matching prefix)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    match cmd {
        "build" if args.len() == 2 => match builder::run_build(&args[1]) {
            Ok(stats) => {
                eprintln!(
                    "[fst-builder] built {} (entries={}, distinct_keys={})",
                    &args[1], stats.entries, stats.distinct_keys
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("[fst-builder] build failed: {}", e);
                ExitCode::FAILURE
            }
        },
        "build-syllables" => match parse_build_syllables(&args[1..]) {
            Ok((output, tl_input, poj_input)) => {
                match syllables::run_build(output, tl_input, poj_input) {
                    Ok(stats) => {
                        eprintln!(
                            "[fst-builder] built {} (tl_lines_in={}, poj_lines_in={}, \
                             syllables={}, valid={}, invalid_skipped={}, distinct_keys={})",
                            output,
                            stats.tl_lines_in,
                            stats.poj_lines_in,
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
                }
            }
            Err(e) => {
                eprintln!("[fst-builder] {}", e);
                eprintln!("usage:\n{}", USAGE);
                ExitCode::FAILURE
            }
        },
        "query" if args.len() == 3 => match query::run_query(&args[1], &args[2]) {
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

/// Parse the positional `<output.fst>` plus `--tl-input <path>` /
/// `--poj-input <path>` flags from the `build-syllables` argument tail.
/// Both flags are mandatory in v3.5.9 B-1 — the dictionary build
/// pipeline always passes both. Order-insensitive across the three
/// pieces (`output`, `--tl-input`, `--poj-input`); rejects duplicates,
/// unknown flags, and missing values.
// 中文: 解析 build-syllables 的 <output.fst> + 兩個必填 input flag。三段順序自由,
// 中文:   重複、未知 flag、缺值都會回錯。
fn parse_build_syllables(tail: &[String]) -> Result<(&str, &str, &str), String> {
    let mut output: Option<&str> = None;
    let mut tl_input: Option<&str> = None;
    let mut poj_input: Option<&str> = None;

    let mut iter = tail.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tl-input" => {
                let val = iter
                    .next()
                    .ok_or_else(|| "missing value for --tl-input".to_string())?;
                if tl_input.replace(val.as_str()).is_some() {
                    return Err("--tl-input passed more than once".to_string());
                }
            }
            "--poj-input" => {
                let val = iter
                    .next()
                    .ok_or_else(|| "missing value for --poj-input".to_string())?;
                if poj_input.replace(val.as_str()).is_some() {
                    return Err("--poj-input passed more than once".to_string());
                }
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown build-syllables flag `{}`", other));
            }
            other => {
                if output.replace(other).is_some() {
                    return Err("multiple positional <output.fst> arguments".to_string());
                }
            }
        }
    }

    let output = output.ok_or_else(|| "missing positional <output.fst>".to_string())?;
    let tl_input = tl_input.ok_or_else(|| "missing required --tl-input flag".to_string())?;
    let poj_input = poj_input.ok_or_else(|| "missing required --poj-input flag".to_string())?;
    Ok((output, tl_input, poj_input))
}
