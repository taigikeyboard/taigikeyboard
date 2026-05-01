//! `PrefixIndex` — fst::Set wrapper backed by mmap'd `dictionary.fst`.
//!
//! Wire format (mirrors `dictionary/build/create_fst.py` / fst-builder):
//!     key_bytes (UTF-8) || 0xFF || rowid_le_4
//!
//! `lookup_prefix` does a byte-range scan on `[prefix+0xFF, prefix+0x100)`
//! and decodes the trailing 4-byte rowid per hit. Insertion order is
//! preserved through fst's deterministic byte-sorted iteration. Replaces
//! both platforms' MARISA-trie + native-bridge stack.

use fst::{IntoStreamer, Set, Streamer};
use mmap_host::MmapHandle;

use crate::error::LexiconError;

const SEPARATOR: u8 = 0xFF;

pub struct PrefixIndex {
    set: Set<MmappedSetData>,
    entry_count: u64,
}

/// Wrapper allowing `fst::Set` to take ownership of the mmap handle while
/// exposing `AsRef<[u8]>`.
struct MmappedSetData {
    handle: MmapHandle,
}

impl AsRef<[u8]> for MmappedSetData {
    fn as_ref(&self) -> &[u8] {
        self.handle.as_slice()
    }
}

impl PrefixIndex {
    /// Open `dictionary.fst` mmap'd readonly. Validates that fst can parse
    /// the bytes; deeper format checks happen on first use.
    pub fn open(path: &std::path::Path) -> Result<Self, LexiconError> {
        let handle = MmapHandle::open_readonly(path).map_err(|source| LexiconError::Mmap {
            path: path.display().to_string(),
            source,
        })?;
        let data = MmappedSetData { handle };
        let set = Set::new(data).map_err(|e| {
            LexiconError::InvalidBinary(format!(
                "fst load `{}`: {}",
                path.display(),
                e
            ))
        })?;
        let entry_count = set.len() as u64;
        Ok(Self { set, entry_count })
    }

    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    /// Return rowids whose stored key has the given prefix. Insertion order
    /// preserved (deterministic per-build, matches Android `distinct()`
    /// behavior — D-12 parity correction).
    ///
    /// Range: half-open `[prefix, prefix_succ)` where `prefix_succ` is the
    /// next sibling prefix in lex order (last byte +1). All wire entries
    /// of the form `key + 0xFF + rowid_le_4` whose `key` starts with
    /// `prefix` fall in this range.
    pub fn lookup_prefix(&self, prefix: &str) -> Vec<u32> {
        let prefix_bytes = prefix.as_bytes();
        if prefix_bytes.is_empty() {
            return Vec::new();
        }
        let lo: Vec<u8> = prefix_bytes.to_vec();
        let Some(hi) = next_lex_sibling(prefix_bytes) else {
            // prefix is all 0xFF — no successor; fall back to a half-open
            // upper bound that includes everything (effectively scans
            // tail). Edge case for keys built from non-UTF-8 bytes; in
            // practice never reached for `tl:` / `poj:` / `hanzi:` prefixes.
            return self.scan_from(&lo);
        };

        let mut stream = self.set.range().ge(&lo).lt(&hi).into_stream();
        decode_rowids(&mut stream, prefix_bytes.len())
    }

    fn scan_from(&self, lo: &[u8]) -> Vec<u32> {
        let mut stream = self.set.range().ge(lo).into_stream();
        decode_rowids(&mut stream, lo.len())
    }

    /// Exact-match lookup — returns rowids whose key equals `key` exactly.
    /// Filters the prefix-scan output by entry-length parity.
    pub fn lookup_exact(&self, key: &str) -> Vec<u32> {
        let key_bytes = key.as_bytes();
        if key_bytes.is_empty() {
            return Vec::new();
        }
        // Range `[key + 0xFF, next_lex_sibling(key)]`. Because the
        // separator is 0xFF, the upper bound is the next sibling of the
        // raw `key` (incrementing key's last byte) — that strictly
        // excludes longer prefix-matching entries.
        let mut lo = Vec::with_capacity(key_bytes.len() + 1);
        lo.extend_from_slice(key_bytes);
        lo.push(SEPARATOR);
        let Some(hi) = next_lex_sibling(key_bytes) else {
            return Vec::new();
        };
        let mut stream = self.set.range().ge(&lo).lt(&hi).into_stream();
        let mut out: Vec<u32> = Vec::new();
        while let Some(entry) = stream.next() {
            if entry.len() != key_bytes.len() + 1 + 4 {
                continue;
            }
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&entry[entry.len() - 4..]);
            out.push(u32::from_le_bytes(buf));
        }
        out
    }
}

/// Returns the next lex sibling prefix — i.e. the smallest byte sequence
/// strictly greater than `prefix` such that no string starting with
/// `prefix` is `>= sibling`. Computed by incrementing the last non-0xFF
/// byte and truncating; returns `None` when prefix is all 0xFF (no
/// successor in u8 space).
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

fn decode_rowids(
    stream: &mut fst::set::Stream<'_, fst::automaton::AlwaysMatch>,
    min_key_len: usize,
) -> Vec<u32> {
    use fst::Streamer;
    let mut out: Vec<u32> = Vec::new();
    while let Some(entry) = stream.next() {
        if entry.len() < min_key_len + 1 + 4 {
            continue;
        }
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&entry[entry.len() - 4..]);
        out.push(u32::from_le_bytes(buf));
    }
    out
}
