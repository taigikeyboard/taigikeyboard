//! `DictionaryReader` — TKDB mmap reader.
//!
//! Binary format (little-endian; canonical spec: `docs/engine/binary-format.md`):
//!     Header: "TKDB" (4) || version u32 || count u32 || build_ts u32   (16 bytes)
//!     Offset table: count × u32 (absolute byte offset to each record)
//!     Records: bitmask u16 || frequency u32 || hanzi_len u8 || tl_len u8
//!              || syllable_count u8 || hanzi || tl
//!
//! `syllable_count` is the number of TL syllables in the entry, range 1..=4
//! (capped by `MAX_SYLLABLES` at builder side).
//!
//! `lookup` accesses by 1-based rowid. `passes_filter` is the 3-layer
//! (variant excl → khiin excl → source-OR with-dev) filter.

// 中文: DictionaryReader — TKDB 詞庫二進位 mmap 讀取器。
// 中文: 以 1-based rowid 索引;passes_filter 為 3 層過濾 (variant → khiin → 來源 OR + dev)。

use mmap_host::MmapHandle;

use crate::error::LexiconError;

const MAGIC: &[u8; 4] = b"TKDB";
const HEADER_SIZE: usize = 16;
const SUPPORTED_VERSION: u32 = 2;
/// bitmask(2) + frequency(4) + hanzi_len(1) + tl_len(1) + syllable_count(1).
const RECORD_FIXED_PREFIX: usize = 9;

/// Bit positions for the 12-source bitmask. Mirrors
/// `dictionary/common/source_bits.py::SOURCE_BITS` (positions 0-11) +
/// `IS_VARIANT_BIT` at bit 12. Drift causes silent filter divergence.
// 中文: khiin 來源位元 (bit 9)。
pub const KHIIN_BIT: u16 = 1 << 9;
// 中文: dev 來源位元 (bit 10),永遠視為啟用。
pub const DEV_BIT: u16 = 1 << 10;
// 中文: 異體字標記位元 (bit 12),由 variant 過濾邏輯使用。
pub const VARIANT_BIT: u16 = 1 << 12;

// 中文: 字典紀錄 — 來源 bitmask、出現頻率、漢字 (可選)、TL 羅馬字、音節數。
#[derive(Debug, Clone)]
pub struct DictionaryRecord {
    // 中文: 來源 + 異體字標記的 13 位元 bitmask。
    pub bitmask: u16,
    // 中文: 詞頻,用於候選詞排序 (DESC)。
    pub frequency: u32,
    // 中文: 漢字寫法 (可選,部分音節無對應漢字)。
    pub hanzi: Option<String>,
    // 中文: TL 羅馬字寫法 (必填)。
    pub tl: String,
    // 中文: TL key 的音節數 (1..=4)。
    pub syllable_count: u8,
}

// 中文: TKDB mmap 讀取器,持有 mmap handle 與紀錄數等 header 資訊。
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

// 中文: 3 層過濾條件 — variant、khiin、來源 mask 三段獨立控制。
#[derive(Debug, Clone, Copy)]
pub struct Filter {
    /// All variant entries excluded when false.
    // 中文: false 時排除所有異體字紀錄 (bit 12)。
    pub variant: bool,
    /// Khiin source excluded when false.
    // 中文: false 時排除 khiin 來源紀錄 (bit 9)。
    pub khiin: bool,
    /// All sources enabled when true (skips per-source mask check).
    // 中文: true 時略過 enabled_mask 比對,直接通過。
    pub all_enabled: bool,
    /// Per-source enable bitmask (12 bits, low-order = source bit).
    // 中文: 啟用來源的 12 位元 bitmask。
    pub enabled_mask: u16,
}

impl Filter {
    /// Decode a single 32-bit `enabled_sources_bitmask` (the wire format
    /// both platforms send) into the 3-axis filter. Variant + khiin gates
    /// ride bits 12 + 9 of the same mask; bits 0..=11 carry per-source
    /// enables. `u32::MAX` is the "all sources enabled" sentinel that
    /// short-circuits the per-source mask check inside
    /// [`DictionaryReader::passes_filter`].
    /// Pinned by `INVARIANT_LEX_FILTER_BITMASK` (audit §4); the layout
    /// must stay byte-identical to the platform encoder.
    // 中文: 把平台送過來的 32-bit enabled_sources_bitmask 解成 3 軸 Filter;bit 12 / 9 是 variant / khiin gate。
    pub fn from_enabled_bitmask(enabled_sources_bitmask: u32) -> Self {
        Self {
            variant: (enabled_sources_bitmask & (1 << 12)) != 0,
            khiin: (enabled_sources_bitmask & (1 << 9)) != 0,
            all_enabled: enabled_sources_bitmask == u32::MAX,
            enabled_mask: (enabled_sources_bitmask & 0x0FFF) as u16,
        }
    }
}

impl DictionaryReader {
    // 中文: 開啟並驗證 dictionary.bin — 檢查 magic、版本與 offset 表大小。
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
            let detail = if version == 1 {
                format!(
                    "dictionary.bin: unsupported version 1 (expected {SUPPORTED_VERSION}; \
                     v1→v2 binary layouts are not compatible — rebuild dictionary.bin)"
                )
            } else {
                format!(
                    "dictionary.bin: unsupported version {version} (expected {SUPPORTED_VERSION})"
                )
            };
            return Err(LexiconError::InvalidBinary(detail));
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

    // 中文: 回傳 dictionary.bin 中紀錄總數。
    pub fn record_count(&self) -> u32 {
        self.record_count
    }

    // 中文: 回傳 dictionary.bin 的 build timestamp (供版本對齊驗證使用)。
    pub fn build_timestamp(&self) -> u32 {
        self.build_timestamp
    }

    /// Read a record by 1-based rowid. Returns `None` if rowid is out of
    /// range or the record bytes are malformed.
    // 中文: 以 1-based rowid 讀取單筆字典紀錄;rowid 越界或格式錯誤回傳 None。
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
        if record_end <= record_offset
            || record_end > bytes.len()
            || record_offset + RECORD_FIXED_PREFIX > record_end
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
        let syllable_count = bytes[pos];
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
            syllable_count,
        })
    }

    /// 3-layer filter: variant exclusion → khiin exclusion →
    /// source-OR-with-dev. Mirrors iOS / Android `passesFilter`.
    // 中文: 3 層過濾 — variant 排除 → khiin 排除 → 來源 OR 比對 (dev 永遠通過)。
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
