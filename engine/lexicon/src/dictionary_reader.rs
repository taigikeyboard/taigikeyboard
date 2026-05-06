//! `DictionaryReader` — TKDB mmap reader.
//!
//! Binary format (little-endian; mirrors iOS / Android readers byte-for-byte):
//!     Header: "TKDB" (4) || version u32 || count u32 || build_ts u32   (16 bytes)
//!     Offset table: count × u32 (absolute byte offset to each record)
//!     Records: bitmask u16 || frequency u32 || hanzi_len u8 || tl_len u8
//!              || hanzi || tl
//!
//! `lookup` accesses by 1-based rowid (matches MARISA RecordTrie payload).
//! `passes_filter` is the 3-layer (variant excl → khiin excl → source-OR
//! with-dev) filter; mirrors iOS `DictionaryBinaryReader.passesFilter` and
//! Android equivalent.

use mmap_host::MmapHandle;

use crate::error::LexiconError;

const MAGIC: &[u8; 4] = b"TKDB";
const HEADER_SIZE: usize = 16;
const SUPPORTED_VERSION: u32 = 1;

/// Bit positions for the 12-source bitmask. Mirrors
/// `dictionary/common/source_bits.py::SOURCE_BITS` (positions 0-11) +
/// `IS_VARIANT_BIT` at bit 12. Drift causes silent filter divergence.
pub const KHIIN_BIT: u16 = 1 << 9;
pub const DEV_BIT: u16 = 1 << 10;
pub const VARIANT_BIT: u16 = 1 << 12;

#[derive(Debug, Clone)]
pub struct DictionaryRecord {
    pub bitmask: u16,
    pub frequency: u32,
    pub hanzi: Option<String>,
    pub tl: String,
}

pub struct DictionaryReader {
    handle: MmapHandle,
    record_count: u32,
    build_timestamp: u32,
}

impl std::fmt::Debug for DictionaryReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DictionaryReader")
            .field("record_count", &self.record_count)
            .field("build_timestamp", &self.build_timestamp)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Filter {
    /// All variant entries excluded when false.
    pub variant: bool,
    /// Khiin source excluded when false.
    pub khiin: bool,
    /// All sources enabled when true (skips per-source mask check).
    pub all_enabled: bool,
    /// Per-source enable bitmask (12 bits, low-order = source bit).
    pub enabled_mask: u16,
}

impl DictionaryReader {
    pub fn open(path: &std::path::Path) -> Result<Self, LexiconError> {
        let handle = MmapHandle::open_readonly(path).map_err(|source| LexiconError::Mmap {
            path: path.display().to_string(),
            source,
        })?;
        let bytes = handle.as_slice();
        if bytes.len() < HEADER_SIZE {
            return Err(LexiconError::InvalidBinary(format!(
                "dictionary.bin: file too small ({} < {})",
                bytes.len(),
                HEADER_SIZE
            )));
        }
        if &bytes[..4] != MAGIC {
            return Err(LexiconError::InvalidBinary(
                "dictionary.bin: TKDB magic mismatch".into(),
            ));
        }
        let version = u32::from_le_bytes(bytes[4..8].try_into().expect("4 bytes"));
        if version != SUPPORTED_VERSION {
            return Err(LexiconError::InvalidBinary(format!(
                "dictionary.bin: unsupported version {version}"
            )));
        }
        let count = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes"));
        let build_timestamp = u32::from_le_bytes(bytes[12..16].try_into().expect("4 bytes"));

        let min_size = HEADER_SIZE + (count as usize) * 4;
        if bytes.len() < min_size {
            return Err(LexiconError::InvalidBinary(format!(
                "dictionary.bin: offset table truncated ({} < {})",
                bytes.len(),
                min_size
            )));
        }
        Ok(Self {
            handle,
            record_count: count,
            build_timestamp,
        })
    }

    pub fn record_count(&self) -> u32 {
        self.record_count
    }

    pub fn build_timestamp(&self) -> u32 {
        self.build_timestamp
    }

    /// Read a record by 1-based rowid. Returns `None` if rowid is out of
    /// range or the record bytes are malformed.
    pub fn record(&self, rowid: u32) -> Option<DictionaryRecord> {
        if rowid == 0 || rowid > self.record_count {
            return None;
        }
        let bytes = self.handle.as_slice();
        let idx = (rowid - 1) as usize;
        let offset_pos = HEADER_SIZE + idx * 4;
        let record_offset =
            u32::from_le_bytes(bytes[offset_pos..offset_pos + 4].try_into().ok()?) as usize;
        let record_end = if (idx + 1) < (self.record_count as usize) {
            let next_pos = HEADER_SIZE + (idx + 1) * 4;
            u32::from_le_bytes(bytes[next_pos..next_pos + 4].try_into().ok()?) as usize
        } else {
            bytes.len()
        };
        if record_end <= record_offset || record_end > bytes.len() || record_offset + 8 > record_end
        {
            return None;
        }
        let mut pos = record_offset;
        let bitmask = u16::from_le_bytes(bytes[pos..pos + 2].try_into().ok()?);
        pos += 2;
        let frequency = u32::from_le_bytes(bytes[pos..pos + 4].try_into().ok()?);
        pos += 4;
        let hanzi_len = bytes[pos] as usize;
        pos += 1;
        let tl_len = bytes[pos] as usize;
        pos += 1;
        if pos + hanzi_len + tl_len > record_end {
            return None;
        }
        let hanzi = if hanzi_len == 0 {
            None
        } else {
            std::str::from_utf8(&bytes[pos..pos + hanzi_len])
                .ok()
                .map(str::to_string)
        };
        if hanzi_len > 0 && hanzi.is_none() {
            return None;
        }
        pos += hanzi_len;
        let tl = std::str::from_utf8(&bytes[pos..pos + tl_len])
            .ok()?
            .to_string();

        Some(DictionaryRecord {
            bitmask,
            frequency,
            hanzi,
            tl,
        })
    }

    /// 3-layer filter: variant exclusion → khiin exclusion →
    /// source-OR-with-dev. Mirrors iOS / Android `passesFilter`.
    pub fn passes_filter(record_bitmask: u16, filter: &Filter) -> bool {
        if !filter.variant && (record_bitmask & VARIANT_BIT) != 0 {
            return false;
        }
        if !filter.khiin && (record_bitmask & KHIIN_BIT) != 0 {
            return false;
        }
        if filter.all_enabled {
            return true;
        }
        (record_bitmask & filter.enabled_mask) != 0 || (record_bitmask & DEV_BIT) != 0
    }
}
