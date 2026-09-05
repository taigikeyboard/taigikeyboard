//! `AssociationReader` — TKWA bundled-bigram mmap reader.
//!
//! Binary format (little-endian; mirrors iOS / Android readers byte-for-byte):
//!     Header: "TKWA" (4) || version u32 || key_count u32 || entry_count u32
//!             || build_ts u32   (20 bytes)
//!     Key offset table: key_count × u32 (absolute byte offset to each key)
//!     Key section (sorted by prev_word UTF-8):
//!         each: prev_word_len u8 || prev_word || entry_offset u32 ||
//!               entry_count u16
//!     Entry section (sorted by count DESC per group):
//!         each: bitmask u16 || count u32 || nw_len u8 || nt_len u8 ||
//!               next_word || next_tl
//!
//! Lookup is binary search by raw UTF-8 prev_word bytes (key section is
//! UTF-8-sorted). 1-layer source filter, deliberately different from
//! DictionaryReader's 3-layer filter (no variant/khiin/dev bits in
//! association entries) — see `docs/engine/binary-format.md` §4.3.

use std::cmp::Ordering;

use mmap_host::MmapHandle;

use crate::error::LexiconError;

const MAGIC: &[u8; 4] = b"TKWA";
const HEADER_SIZE: usize = 20;
const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct AssociationEntry {
    pub next_word: String,
    pub next_tl: String,
    pub count: u32,
    // Dictionary-source bitmask; only the low 9 bits are used.
    pub bitmask: u16,
}

pub struct AssociationReader {
    handle: MmapHandle,
    key_count: u32,
    build_timestamp: u32,
}

// Source filter; all_enabled short-circuits the mask comparison.
#[derive(Debug, Clone, Copy)]
pub struct AssocFilter {
    // True when every source is enabled (the u32::MAX sentinel).
    pub all_enabled: bool,
    pub enabled_mask: u16,
}

impl AssociationReader {
    // Opens and validates association.bin: magic, version, offset-table size.
    pub fn open(path: &std::path::Path) -> Result<Self, LexiconError> {
        let handle = MmapHandle::open_readonly(path).map_err(|source| LexiconError::Mmap {
            path: path.display().to_string(),
            source,
        })?;
        let bytes = handle.as_slice();
        if bytes.len() < HEADER_SIZE {
            return Err(LexiconError::InvalidBinary(format!(
                "association.bin: file too small ({} < {})",
                bytes.len(),
                HEADER_SIZE
            )));
        }
        if &bytes[..4] != MAGIC {
            return Err(LexiconError::InvalidBinary(
                "association.bin: TKWA magic mismatch".into(),
            ));
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().expect("4 bytes"));
        if version != SUPPORTED_VERSION {
            return Err(LexiconError::InvalidBinary(format!(
                "association.bin: unsupported version {version}"
            )));
        }
        let key_count = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
        let build_timestamp = u32::from_le_bytes(bytes[16..20].try_into().expect("4 bytes"));
        let min_size = HEADER_SIZE + (key_count as usize) * 4;
        if bytes.len() < min_size {
            return Err(LexiconError::InvalidBinary(format!(
                "association.bin: offset table truncated ({} < {})",
                bytes.len(),
                min_size
            )));
        }
        Ok(Self {
            handle,
            key_count,
            build_timestamp,
        })
    }

    pub fn build_timestamp(&self) -> u32 {
        self.build_timestamp
    }

    /// Binary search by prev_word; returns up to `limit` entries in their
    /// stored order (sorted by count DESC per build).
    pub fn lookup(&self, prev_word: &str, limit: usize) -> Vec<AssociationEntry> {
        if prev_word.is_empty() || self.key_count == 0 {
            return Vec::new();
        }
        let target = prev_word.as_bytes();
        let bytes = self.handle.as_slice();
        let mut lo: i64 = 0;
        let mut hi: i64 = (self.key_count as i64) - 1;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            let cmp = self.compare_key_at(mid as u32, target, bytes);
            match cmp {
                Ordering::Equal => return self.read_entries(mid as u32, limit, bytes),
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid - 1,
            }
        }
        Vec::new()
    }

    pub fn passes_filter(entry_bitmask: u16, filter: &AssocFilter) -> bool {
        if filter.all_enabled {
            return true;
        }
        if filter.enabled_mask == 0 {
            return false;
        }
        (entry_bitmask & filter.enabled_mask) != 0
    }

    fn compare_key_at(&self, index: u32, target: &[u8], bytes: &[u8]) -> Ordering {
        let key_offset = self.key_offset_at(index, bytes);
        if key_offset >= bytes.len() {
            return Ordering::Less;
        }
        let key_len = bytes[key_offset] as usize;
        let key_start = key_offset + 1;
        if key_start + key_len > bytes.len() {
            return Ordering::Less;
        }
        let cmp_len = key_len.min(target.len());
        for i in 0..cmp_len {
            let a = bytes[key_start + i];
            let b = target[i];
            if a != b {
                return if a < b {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
        }
        key_len.cmp(&target.len())
    }

    fn key_offset_at(&self, index: u32, bytes: &[u8]) -> usize {
        let pos = HEADER_SIZE + (index as usize) * 4;
        u32::from_le_bytes(bytes[pos..pos + 4].try_into().expect("4 bytes")) as usize
    }

    fn read_entries(&self, index: u32, limit: usize, bytes: &[u8]) -> Vec<AssociationEntry> {
        let key_offset = self.key_offset_at(index, bytes);
        if key_offset >= bytes.len() {
            return Vec::new();
        }
        let key_len = bytes[key_offset] as usize;
        let meta_pos = key_offset + 1 + key_len;
        if meta_pos + 6 > bytes.len() {
            return Vec::new();
        }
        let entry_offset =
            u32::from_le_bytes(bytes[meta_pos..meta_pos + 4].try_into().expect("4 bytes")) as usize;
        let entry_count = u16::from_le_bytes(
            bytes[meta_pos + 4..meta_pos + 6]
                .try_into()
                .expect("2 bytes"),
        ) as usize;
        if entry_offset > bytes.len() {
            return Vec::new();
        }
        let read_count = entry_count.min(limit);
        let mut out = Vec::with_capacity(read_count);
        let mut pos = entry_offset;
        for _ in 0..read_count {
            if pos + 8 > bytes.len() {
                break;
            }
            let bitmask = u16::from_le_bytes(bytes[pos..pos + 2].try_into().expect("2 bytes"));
            pos += 2;
            let count = u32::from_le_bytes(bytes[pos..pos + 4].try_into().expect("4 bytes"));
            pos += 4;
            let nw_len = bytes[pos] as usize;
            pos += 1;
            let nt_len = bytes[pos] as usize;
            pos += 1;
            if pos + nw_len + nt_len > bytes.len() {
                break;
            }
            let next_word = match std::str::from_utf8(&bytes[pos..pos + nw_len]) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    pos += nw_len + nt_len;
                    continue;
                }
            };
            pos += nw_len;
            let next_tl = match std::str::from_utf8(&bytes[pos..pos + nt_len]) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    pos += nt_len;
                    continue;
                }
            };
            pos += nt_len;
            out.push(AssociationEntry {
                next_word,
                next_tl,
                count,
                bitmask,
            });
        }
        out
    }
}
