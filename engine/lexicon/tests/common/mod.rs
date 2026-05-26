//! Shared fixture builders for lexicon integration tests.
//!
//! Cargo compiles each `tests/*.rs` as a separate crate, so a `tests/common/`
//! module declared via `mod common;` from each test file is the standard
//! way to share helpers without leaking them into the production crate.

#![allow(dead_code)] // Different test files use different subsets.

use std::path::PathBuf;

const HEADER_SIZE: usize = 16;

/// Single TKDB record fixture. `syllable_count = None` emits a v1 layout
/// (no syllable_count byte); `Some(n)` emits the v2 layout with that count.
pub struct DictRow<'a> {
    pub bitmask: u16,
    pub frequency: u32,
    pub syllable_count: Option<u8>,
    pub hanzi: &'a str,
    pub tl: &'a str,
}

/// Build a TKDB byte sequence with the given `magic`, `version`, and rows.
/// The caller is responsible for picking a `version` that matches the
/// row layout (v2 → all rows have `syllable_count = Some(_)`).
pub fn build_tkdb_bin(magic: &[u8; 4], version: u32, rows: &[DictRow<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(magic);
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // build_ts

    let offset_table_size = rows.len() * 4;
    let mut offsets = Vec::<u32>::with_capacity(rows.len());
    let mut payload = Vec::<u8>::new();
    for row in rows {
        offsets.push((HEADER_SIZE + offset_table_size + payload.len()) as u32);
        payload.extend_from_slice(&row.bitmask.to_le_bytes());
        payload.extend_from_slice(&row.frequency.to_le_bytes());
        payload.push(row.hanzi.len() as u8);
        payload.push(row.tl.len() as u8);
        if let Some(syll) = row.syllable_count {
            payload.push(syll);
        }
        payload.extend_from_slice(row.hanzi.as_bytes());
        payload.extend_from_slice(row.tl.as_bytes());
    }
    for off in &offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&payload);
    out
}

/// 4-tuple convenience for v2 fixtures: `(bitmask, frequency, syllable_count, hanzi, tl)`.
pub fn build_tkdb_v2(magic: &[u8; 4], rows: &[(u16, u32, u8, &str, &str)]) -> Vec<u8> {
    let dict_rows: Vec<DictRow<'_>> = rows
        .iter()
        .map(|(bm, freq, syll, hanzi, tl)| DictRow {
            bitmask: *bm,
            frequency: *freq,
            syllable_count: Some(*syll),
            hanzi,
            tl,
        })
        .collect();
    build_tkdb_bin(magic, 2, &dict_rows)
}

/// Write `bytes` to a unique tmpdir path and return it.
///
/// `name` is a human-readable suffix for debugging; the actual path is
/// namespaced with `process::id()` plus a per-process atomic counter so
/// parallel tests within the same `cargo test` binary never collide
/// (cargo runs `tests/*.rs` test fns in parallel by default; without
/// the counter, two tests passing the same `name` — or two callers of
/// `synth_dictionary_reader` with the same row count — would race on
/// `File::create` and produce a flaky "file too small" panic). Mirrors
/// the safe pattern at `tests/span_local_fetch.rs:155-156`.
pub fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("lexicon-test-{name}-{pid}-{n}"));
    std::fs::write(&path, bytes).expect("write temp");
    path
}
