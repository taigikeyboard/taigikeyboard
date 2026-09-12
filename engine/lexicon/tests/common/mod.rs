//! Shared fixture builders for lexicon integration tests.
//!
//! Cargo compiles each `tests/*.rs` as a separate crate, so a `tests/common/`
//! module declared via `mod common;` from each test file is the standard
//! way to share helpers without leaking them into the production crate.

#![allow(dead_code)] // Different test files use different subsets.

use std::path::PathBuf;

use lexicon::{
    fetch_candidates_for_keys_with_barriers, ConsumedSpan, ContinuousFetchCtx, RawCandidate,
};
use phonetics::InputMode;

const HEADER_SIZE: usize = 16;

/// Single TKDB record fixture. The optional fields drive the on-disk record
/// layout (independent of the header `version` int):
/// - `syllable_count = None` → v1 layout (no syllable_count byte);
///   `Some(n)` → v2+ layout with that count.
/// - `kautian_subtag = None` → v1/v2 layout (no subtag bytes);
///   `Some(s)` → v3 layout with the 2-byte subtag after syllable_count.
pub struct DictRow<'a> {
    pub bitmask: u16,
    pub frequency: u32,
    pub syllable_count: Option<u8>,
    pub kautian_subtag: Option<u16>,
    pub hanzi: &'a str,
    pub tl: &'a str,
}

/// Build a TKDB byte sequence with the given `magic`, `version`, and rows.
/// The caller is responsible for picking a `version` that matches the
/// row layout (v2 → `syllable_count = Some(_)`; v3 → also
/// `kautian_subtag = Some(_)`).
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
        if let Some(subtag) = row.kautian_subtag {
            payload.extend_from_slice(&subtag.to_le_bytes());
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

/// 4-tuple convenience for v2 fixtures: `(bitmask, frequency, syllable_count,
/// hanzi, tl)`. Emits a VERSION-2 binary (no subtag) — used only by the
/// v2-loud-reject test now that the reader requires v3.
pub fn build_tkdb_v2(magic: &[u8; 4], rows: &[(u16, u32, u8, &str, &str)]) -> Vec<u8> {
    let dict_rows: Vec<DictRow<'_>> = rows
        .iter()
        .map(|(bm, freq, syll, hanzi, tl)| DictRow {
            bitmask: *bm,
            frequency: *freq,
            syllable_count: Some(*syll),
            kautian_subtag: None,
            hanzi,
            tl,
        })
        .collect();
    build_tkdb_bin(magic, 2, &dict_rows)
}

/// 5-tuple convenience for v3 fixtures: `(bitmask, frequency, syllable_count,
/// hanzi, tl)` with `kautian_subtag = 0` on every row. The default for tests
/// that don't exercise subcollection provenance. Delegates to
/// `build_tkdb_v3_subtag` (mirrors the `build_tkdb_v2` → `build_tkdb_bin`
/// thin-wrapper pattern).
pub fn build_tkdb_v3(magic: &[u8; 4], rows: &[(u16, u32, u8, &str, &str)]) -> Vec<u8> {
    let with_subtag: Vec<(u16, u32, u8, u16, &str, &str)> = rows
        .iter()
        .map(|(bm, freq, syll, hanzi, tl)| (*bm, *freq, *syll, 0u16, *hanzi, *tl))
        .collect();
    build_tkdb_v3_subtag(magic, &with_subtag)
}

/// 6-tuple convenience for v3 fixtures with explicit kautian subtags:
/// `(bitmask, frequency, syllable_count, kautian_subtag, hanzi, tl)`.
pub fn build_tkdb_v3_subtag(magic: &[u8; 4], rows: &[(u16, u32, u8, u16, &str, &str)]) -> Vec<u8> {
    let dict_rows: Vec<DictRow<'_>> = rows
        .iter()
        .map(|(bm, freq, syll, subtag, hanzi, tl)| DictRow {
            bitmask: *bm,
            frequency: *freq,
            syllable_count: Some(*syll),
            kautian_subtag: Some(*subtag),
            hanzi,
            tl,
        })
        .collect();
    build_tkdb_bin(magic, 3, &dict_rows)
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

/// Test-only span-local fetch: every dictionary candidate whose toneless
/// key matches `input[pos..end]` for some `end` in `endings`. Production
/// never uses this shape — `composing::continuous::assemble_candidates`
/// builds its `(consumed_span, "<prefix>:<toneless>")` pairs itself and
/// calls `lexicon::fetch_candidates_for_keys_with_barriers` directly. This
/// wrapper lets the tests here hand over pre-computed syllabifier endings
/// without rebuilding the pairs inline.
///
/// `prefix ∈ {tl, poj, tps}` follows `mode` (English shares `tl:`). ASCII
/// digits are stripped from the span — the digit half of the upstream
/// `dictionary/common/notone.py::remove_tone` regex `[\d\-]` — so numeric-
/// tone input (`tai1bak4`) still hits the fused toneless FST key
/// (`tl:taipak`). Hyphens are NOT stripped: production folds them upstream
/// (`composing::shadow::build_hyphen_shadow`), so a hyphen reaching this fn
/// is an upstream contract violation and the FST lookup correctly misses.
///
/// Always forces `custom = &[]` (the lexicon integration tests never carry
/// custom-dict matches; the Item 12 tests in `span_local_fetch.rs` call the
/// production entry directly for that). Pass `&FrequencyMap::new()` +
/// `now_ms = 0` for cold-start neutral ranking (boost = 1.0,
/// recency_rank = 1 everywhere).
pub fn fetch_candidates_for_endings(
    input: &str,
    pos: usize,
    endings: &[usize],
    mode: InputMode,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    if endings.is_empty() || pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }

    let lower = input.to_ascii_lowercase();
    let mut keys: Vec<(ConsumedSpan, String)> = Vec::with_capacity(endings.len());
    let prefix = match mode {
        InputMode::Poj => "poj",
        InputMode::Tps => "tps",
        InputMode::Tl | InputMode::English => "tl",
    };
    for &end in endings {
        if end <= pos || end > lower.len() || !lower.is_char_boundary(end) {
            continue;
        }
        let segment = &lower[pos..end];
        let toneless: String = segment.chars().filter(|c| !c.is_ascii_digit()).collect();
        if toneless.is_empty() {
            continue;
        }
        keys.push(((pos as u32, end as u32), format!("{prefix}:{toneless}")));
    }

    // `raw_len` is the full `input.len()` even when `pos != 0`: Tier 1 is
    // full-buffer coverage, not `input.len() - pos`. Every ctx field is
    // listed explicitly (not `..*ctx`) so the forced-empty-custom contract
    // stays loud if a non-`Copy` field is ever added.
    let inner = ContinuousFetchCtx {
        enabled_sources_bitmask: ctx.enabled_sources_bitmask,
        freq_map: ctx.freq_map,
        now_ms: ctx.now_ms,
        custom: &[],
        prefix_index: ctx.prefix_index,
        dict: ctx.dict,
        mode: ctx.mode,
        tps_space_pinned_body: ctx.tps_space_pinned_body,
    };
    fetch_candidates_for_keys_with_barriers(&keys, &[], &[], input.len() as u32, &inner)
}

/// One `dictionary.fst` wire entry: `key || 0xFF || rowid_le_4`.
pub fn wire_entry(key: &str, rowid: u32) -> Vec<u8> {
    let mut wire = key.as_bytes().to_vec();
    wire.push(0xFF);
    wire.extend_from_slice(&rowid.to_le_bytes());
    wire
}

/// Build a wire-format `dictionary.fst` from `(key, rowid)` pairs and
/// open it as a `PrefixIndex`.
pub fn build_wire_index(name: &str, entries: &[(&str, u32)]) -> lexicon::prefix_index::PrefixIndex {
    let mut wires: Vec<Vec<u8>> = entries
        .iter()
        .map(|(key, rowid)| wire_entry(key, *rowid))
        .collect();
    wires.sort();
    let path = write_temp(name, &[]);
    let file = std::fs::File::create(&path).expect("create fst");
    let mut builder = fst::SetBuilder::new(std::io::BufWriter::new(file)).expect("builder");
    for wire in &wires {
        builder.insert(wire).expect("insert");
    }
    builder.finish().expect("finish");
    lexicon::prefix_index::PrefixIndex::open(&path).expect("open index")
}
