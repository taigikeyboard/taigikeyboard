//! Dev CLI mirroring `taigi-converter`'s `tai` command. Used to spot-check
//! phonetics output during development. NOT shipped to iOS/Android.

use anyhow::{anyhow, Result};
use clap::Parser;
use phonetics::{convert, to_tone_marks, to_tone_number, InputMode, System};
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(name = "cli", about = "Taigi phonetics dev tool")]
struct Args {
    /// Source system: tl, poj, tps  (or `mark` / `number` for tone-format conversion)
    from: String,
    /// Target system: tl, poj, tps, mark, number
    to: String,
    /// Input text. Reads stdin if omitted.
    text: Vec<String>,
}

fn parse_system(s: &str) -> Result<System> {
    match s.to_lowercase().as_str() {
        "tl" | "tailo" | "tai-lo" => Ok(System::Tl),
        "poj" => Ok(System::Poj),
        "tps" | "zhuyin" => Ok(System::Tps),
        other => Err(anyhow!("unknown system: {other}")),
    }
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let text = if args.text.is_empty() {
        read_stdin()?
    } else {
        args.text.join(" ")
    };

    let result = match args.to.to_lowercase().as_str() {
        "mark" => to_tone_marks(&text, InputMode::Tl),
        "number" | "num" => to_tone_number(&text),
        _ => convert(&text, parse_system(&args.from)?, parse_system(&args.to)?)?,
    };
    println!("{result}");
    Ok(())
}
