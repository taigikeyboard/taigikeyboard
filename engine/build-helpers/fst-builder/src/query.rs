//! Query subcommand: list entries matching a prefix.
//!
//! Used by `dictionary/tools/query_fst.py` for dev debug; not on the
//! IME hot path. Replaces the old `query_trie.py` MARISA-binding helper.

use fst::{IntoStreamer, Set, Streamer};
use std::fs;

const SEPARATOR: u8 = 0xFF;

/// Returns the next lex sibling — smallest byte sequence strictly greater
/// than `prefix` such that no string starting with `prefix` is `>= sibling`.
/// Mirrors `engine/lexicon::prefix_index::next_lex_sibling`.
fn next_lex_sibling(prefix: &[u8]) -> Option<Vec<u8>> {
    let mut out = prefix.to_vec();
    while let Some(last) = out.last_mut() {
        if *last < 0xFF {
            *last += 1;
            return Some(out);
        }
        out.pop();
    }
    None
}

pub fn run_query(fst_path: &str, prefix: &str) -> Result<(), String> {
    let bytes = fs::read(fst_path).map_err(|e| format!("read `{}`: {}", fst_path, e))?;
    let set = Set::new(bytes).map_err(|e| format!("fst load: {}", e))?;

    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.is_empty() {
        eprintln!("[fst-builder] empty prefix");
        return Ok(());
    }
    let lo = prefix_bytes.to_vec();
    let hi = next_lex_sibling(prefix_bytes)
        .ok_or_else(|| "prefix is all 0xFF; no successor".to_string())?;

    let mut stream = set.range().ge(&lo).lt(&hi).into_stream();
    let mut hits: usize = 0;
    while let Some(entry) = stream.next() {
        if entry.len() < prefix_bytes.len() + 1 + 4 {
            continue;
        }
        // Find the SEPARATOR byte to slice key from rowid (entries have
        // form: key + 0xFF + rowid_le_4; rowid_le_4 may itself contain
        // 0xFF bytes, so scan from the end-of-key boundary which is
        // `entry.len() - 5`).
        let key_end = entry.len() - 5;
        let key = std::str::from_utf8(&entry[..key_end])
            .map_err(|e| format!("entry utf8: {}", e))?;
        let mut rowid_buf = [0u8; 4];
        rowid_buf.copy_from_slice(&entry[entry.len() - 4..]);
        let rowid = u32::from_le_bytes(rowid_buf);
        println!("{}\t{}", key, rowid);
        hits += 1;
    }

    eprintln!("[fst-builder] {} hits for prefix `{}`", hits, prefix);
    Ok(())
}
