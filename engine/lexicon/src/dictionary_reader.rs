//! `DictionaryReader` — TKDB mmap reader.
//!
//! Binary format (little-endian; canonical spec: `docs/engine/binary-format.md`):
//!     Header: "TKDB" (4) || version u32 || count u32 || build_ts u32   (16 bytes)
//!     Offset table: count × u32 (absolute byte offset to each record)
//!     Records: bitmask u16 || frequency u32 || hanzi_len u8 || tl_len u8
//!              || syllable_count u8 || kautian_subtag u16 || hanzi || tl
//!
//! `syllable_count` is the number of TL syllables in the entry, range 1..=4
//! (capped by `MAX_SYLLABLES` at builder side). `kautian_subtag` (v3) is the
//! per-row kautian subcollection provenance (0 for non-kautian rows).
//!
//! `lookup` accesses by 1-based rowid. `passes_filter` is the 4-layer
//! (variant excl → khiin excl → kautian-subcollection gate → source-OR
//! with-dev) filter.

// 中文: DictionaryReader — TKDB 詞庫二進位 mmap 讀取器。
// 中文: 以 1-based rowid 索引;passes_filter 為 4 層過濾 (variant → khiin → kautian subcollection gate → 來源 OR + dev)。

use mmap_host::MmapHandle;

use crate::error::LexiconError;

const MAGIC: &[u8; 4] = b"TKDB";
const HEADER_SIZE: usize = 16;
const SUPPORTED_VERSION: u32 = 3;
/// bitmask(2) + frequency(4) + hanzi_len(1) + tl_len(1) + syllable_count(1)
/// + kautian_subtag(2).
const RECORD_FIXED_PREFIX: usize = 11;

/// Bit positions for the 12-source bitmask. Mirrors
/// `dictionary/common/source_bits.py::SOURCE_BITS` (positions 0-11) +
/// `IS_VARIANT_BIT` at bit 12. Drift causes silent filter divergence.
// 中文: kautian 來源位元 (bit 0),subcollection gate 會在 OR 前清掉此位元。
pub const KAUTIAN_BIT: u16 = 1 << 0;
// 中文: khiin 來源位元 (bit 9)。
pub const KHIIN_BIT: u16 = 1 << 9;
// 中文: dev 來源位元 (bit 10),永遠視為啟用。
pub const DEV_BIT: u16 = 1 << 10;
// 中文: 異體字標記位元 (bit 12),由 variant 過濾邏輯使用。
pub const VARIANT_BIT: u16 = 1 << 12;

/// kautian subcollection subtag (dictionary.bin v3) — a SEPARATE u16 per
/// record (NOT part of `bitmask`) recording which subcollections a kautian
/// row belongs to. Mirrors
/// `dictionary/common/source_bits.py::encode_kautian_subtag`: bit 0 = main,
/// bits 1..=10 = accent_mask (10 dialect columns), bit 11 = name; bits 12-15
/// reserved. Reserved bits are masked off on read so a future writer cannot
/// corrupt the filter AND.
// 中文: kautian subcollection subtag (v3) — 與 bitmask 分離的 u16;bit0=主條目, bit1-10=腔調, bit11=姓名附錄, 12-15 保留。
pub const KAUTIAN_SUBTAG_USED_MASK: u16 = 0x0FFF;
/// Subtag bit positions (mirror `source_bits.py::KAUTIAN_SUBTAG_*`). The
/// ENCODE side (`dictionary_filters::compute_filters`) packs the wire enable
/// mask from these so the subcollection bit layout lives in Rust only.
// 中文: subtag 位元位置 (鏡射 source_bits.py);ENCODE 端用這些打包 wire 啟用遮罩,佈局只存在 Rust。
pub const KAUTIAN_SUBTAG_MAIN_BIT: u16 = 0;
pub const KAUTIAN_SUBTAG_ACCENT_SHIFT: u16 = 1;
pub const KAUTIAN_SUBTAG_ACCENT_COUNT: usize = 10;
pub const KAUTIAN_SUBTAG_NAME_BIT: u16 = 11;

/// Wire layout: the user's kautian subcollection ENABLE bits ride the high
/// region of `enabled_sources_bitmask` (u32). bit 13 = active sentinel — when
/// 0 the engine SKIPS subcollection gating entirely (legacy / pre-UI default
/// = all subcollections on, zero behaviour change). bits 14..=25 = enable mask
/// in the SAME 12-bit layout as the subtag, so the filter test is one AND.
/// `u32::MAX` (the all-enabled sentinel) also carries bit 13 set; a real
/// platform mask (Phase 3 `compute_filters`) sets bit 13 + the 12 bits
/// explicitly and MUST never equal `u32::MAX`.
// 中文: 線上格式 — 使用者的 kautian subcollection 啟用位元放在 enabled_sources_bitmask 高位;bit13=啟用旗標 (0=略過過濾=全開), bit14-25=啟用遮罩 (與 subtag 同佈局)。
pub const WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT: u32 = 1 << 13;
pub const WIRE_KAUTIAN_SUBCOLL_SHIFT: u32 = 14;
pub const WIRE_KAUTIAN_SUBCOLL_MASK: u32 = 0x0FFF;

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
    // 中文: kautian subcollection subtag (v3);非 kautian 列為 0。保留位元已遮除。
    pub kautian_subtag: u16,
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
    /// kautian subcollection filtering active (wire bit 13). False ⇒ skip the
    /// subcollection gate entirely (legacy / all-on, zero behaviour change).
    // 中文: kautian subcollection 過濾旗標 (wire bit 13);false ⇒ 略過過濾 (全開)。
    pub kautian_subcoll_active: bool,
    /// Enabled kautian subcollections — 12-bit mask, SAME layout as the record
    /// subtag (main | accent[10] | name).
    // 中文: 啟用的 kautian subcollection 12 位元遮罩,佈局與 record subtag 相同。
    pub kautian_subcoll_mask: u16,
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
            kautian_subcoll_active: (enabled_sources_bitmask & WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT)
                != 0,
            kautian_subcoll_mask: ((enabled_sources_bitmask >> WIRE_KAUTIAN_SUBCOLL_SHIFT)
                & WIRE_KAUTIAN_SUBCOLL_MASK) as u16,
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
            let detail = if version == 1 || version == 2 {
                format!(
                    "dictionary.bin: unsupported version {version} (expected {SUPPORTED_VERSION}; \
                     v1/v2→v3 binary layouts are not compatible — rebuild dictionary.bin via \
                     `make dict` then redeploy artifacts in lockstep)"
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
        // v3: kautian subcollection subtag (2 bytes). The RECORD_FIXED_PREFIX
        // bound checked above guarantees these 2 bytes are in range. Reserved
        // bits (12-15) are masked off so they can never affect the filter AND.
        let kautian_subtag =
            u16::from_le_bytes(bytes[pos..pos + 2].try_into().ok()?) & KAUTIAN_SUBTAG_USED_MASK;
        pos += 2;
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
            kautian_subtag,
        })
    }

    /// kautian subcollection gate (DD6) — the SINGLE source of truth for both
    /// the filter (below) and the emitted `source_bitmask` (ranking tier) at
    /// every call site. When subcollection filtering is active AND this is a
    /// kautian-source row, the kautian bit (bit 0) is CLEARED unless at least
    /// one of the row's subcollections (main / accent / name) is enabled. Other
    /// source bits are never touched, so a multi-source row stays visible via
    /// its other sources and inherits the other source's ranking tier. Rows
    /// without the kautian bit, and the legacy/all-on case (`!active`), pass
    /// through unchanged.
    // 中文: kautian subcollection gate (DD6) — 過濾與排序 tier 共用的單一真實來源。
    // 中文: 過濾啟用且為 kautian 列時,若無任何 subcollection 啟用則清掉 kautian 位元 (其他來源不動,保留 DD6 多來源)。
    pub fn effective_source_bitmask(
        record_bitmask: u16,
        record_subtag: u16,
        filter: &Filter,
    ) -> u16 {
        if !filter.kautian_subcoll_active || (record_bitmask & KAUTIAN_BIT) == 0 {
            return record_bitmask;
        }
        if (record_subtag & filter.kautian_subcoll_mask) != 0 {
            record_bitmask
        } else {
            record_bitmask & !KAUTIAN_BIT
        }
    }

    /// Filter: variant exclusion → khiin exclusion → kautian subcollection gate
    /// → source-OR-with-dev. `record_subtag` is `DictionaryRecord.kautian_subtag`
    /// (0 for non-kautian rows). The source-OR runs on the
    /// [`Self::effective_source_bitmask`] so a fully-disabled kautian row drops
    /// only its kautian contribution.
    // 中文: 過濾 — variant 排除 → khiin 排除 → kautian subcollection gate → 來源 OR (dev 永遠通過)。
    pub fn passes_filter(record_bitmask: u16, record_subtag: u16, filter: &Filter) -> bool {
        if !filter.variant && (record_bitmask & VARIANT_BIT) != 0 {
            return false;
        }
        if !filter.khiin && (record_bitmask & KHIIN_BIT) != 0 {
            return false;
        }
        if filter.all_enabled {
            return true;
        }
        let effective = Self::effective_source_bitmask(record_bitmask, record_subtag, filter);
        (effective & filter.enabled_mask) != 0 || (effective & DEV_BIT) != 0
    }
}

#[cfg(test)]
mod tests {
    //! kautian subcollection filter (v3) — pure-logic acceptance matrix
    //! driving `from_enabled_bitmask` / `effective_source_bitmask` /
    //! `passes_filter` via hand-built wire masks. Binary-layout round-trip
    //! lives in `tests/dictionary_reader_v3.rs`.
    use super::*;

    const TAIGITV: u16 = 1 << 1;

    // subtag / wire-subcoll shared 12-bit layout: bit 0 = main,
    // bits 1..=10 = accent, bit 11 = name.
    const SUB_MAIN: u16 = 1 << 0;
    const SUB_NAME: u16 = 1 << 11;
    fn sub_accent(i: u16) -> u16 {
        1u16 << (1 + i)
    }

    // Low source bits: kautian (0) + dev (10, always-on).
    const SRC_KAUTIAN_DEV: u32 = (1 << 0) | (1 << 10);

    /// Build a wire `enabled_sources_bitmask`: low source bits plus an
    /// optionally-active subcollection mask in the high region.
    fn wire(source_bits: u32, subcoll_active: bool, subcoll_mask: u16) -> u32 {
        let mut m = source_bits;
        if subcoll_active {
            m |= WIRE_KAUTIAN_SUBCOLL_ACTIVE_BIT;
        }
        m |= (subcoll_mask as u32 & WIRE_KAUTIAN_SUBCOLL_MASK) << WIRE_KAUTIAN_SUBCOLL_SHIFT;
        m
    }

    #[test]
    fn from_enabled_bitmask_decodes_subcoll_high_bits() {
        // Legacy (no high bits): inactive, mask 0, low-bit decode unaffected.
        let f = Filter::from_enabled_bitmask(SRC_KAUTIAN_DEV);
        assert!(!f.kautian_subcoll_active);
        assert_eq!(f.kautian_subcoll_mask, 0);
        assert_eq!(f.enabled_mask, (1 << 0) | (1 << 10));

        // Active + main|name enabled.
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, SUB_MAIN | SUB_NAME));
        assert!(f.kautian_subcoll_active);
        assert_eq!(f.kautian_subcoll_mask, SUB_MAIN | SUB_NAME);

        // u32::MAX → all_enabled + active + every subcollection bit.
        let f = Filter::from_enabled_bitmask(u32::MAX);
        assert!(f.all_enabled);
        assert!(f.kautian_subcoll_active);
        assert_eq!(f.kautian_subcoll_mask, WIRE_KAUTIAN_SUBCOLL_MASK as u16);
    }

    #[test]
    fn effective_bitmask_legacy_passthrough() {
        let f = Filter::from_enabled_bitmask(SRC_KAUTIAN_DEV);
        assert_eq!(
            DictionaryReader::effective_source_bitmask(KAUTIAN_BIT, sub_accent(0), &f),
            KAUTIAN_BIT
        );
    }

    #[test]
    fn effective_bitmask_drops_kautian_when_subcoll_disabled() {
        // Active, only main enabled; accent-only kautian row → kautian cleared.
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, SUB_MAIN));
        assert_eq!(
            DictionaryReader::effective_source_bitmask(KAUTIAN_BIT, sub_accent(0), &f),
            0
        );
        // Headword row (main set) survives.
        assert_eq!(
            DictionaryReader::effective_source_bitmask(KAUTIAN_BIT, SUB_MAIN, &f),
            KAUTIAN_BIT
        );
    }

    #[test]
    fn effective_bitmask_multi_source_keeps_other_source() {
        // DD6: kautian dropped, taigitv preserved.
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, SUB_MAIN));
        assert_eq!(
            DictionaryReader::effective_source_bitmask(KAUTIAN_BIT | TAIGITV, sub_accent(0), &f),
            TAIGITV
        );
    }

    #[test]
    fn effective_bitmask_non_kautian_untouched() {
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, SUB_MAIN));
        assert_eq!(
            DictionaryReader::effective_source_bitmask(TAIGITV, 0, &f),
            TAIGITV
        );
    }

    #[test]
    fn passes_filter_legacy_unchanged_behaviour() {
        // Legacy mask: accent-only kautian row passes (no gating) — zero
        // behaviour change until the platform sends the active sentinel.
        let f = Filter::from_enabled_bitmask(SRC_KAUTIAN_DEV);
        assert!(DictionaryReader::passes_filter(KAUTIAN_BIT, sub_accent(0), &f));
    }

    #[test]
    fn passes_filter_main_only_drops_accent_keeps_headword() {
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, SUB_MAIN));
        assert!(!DictionaryReader::passes_filter(KAUTIAN_BIT, sub_accent(0), &f));
        assert!(DictionaryReader::passes_filter(KAUTIAN_BIT, SUB_MAIN, &f));
        // DD6: word that is BOTH headword AND accent stays visible via main.
        assert!(DictionaryReader::passes_filter(
            KAUTIAN_BIT,
            SUB_MAIN | sub_accent(0),
            &f
        ));
    }

    #[test]
    fn passes_filter_accent_enabled_matches_accent_row() {
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, sub_accent(3)));
        assert!(DictionaryReader::passes_filter(KAUTIAN_BIT, sub_accent(3), &f));
        assert!(!DictionaryReader::passes_filter(KAUTIAN_BIT, sub_accent(0), &f));
    }

    #[test]
    fn passes_filter_name_enabled_matches_name_row() {
        let f = Filter::from_enabled_bitmask(wire(SRC_KAUTIAN_DEV, true, SUB_NAME));
        assert!(DictionaryReader::passes_filter(KAUTIAN_BIT, SUB_NAME, &f));
        assert!(!DictionaryReader::passes_filter(KAUTIAN_BIT, SUB_MAIN, &f));
    }

    #[test]
    fn passes_filter_multi_source_disabled_kautian_survives_via_other() {
        // kautian + taigitv + dev enabled; only main subcollection on; row is
        // an accent-only kautian|taigitv entry → survives via taigitv (DD6).
        let src = (1 << 0) | (1 << 1) | (1 << 10);
        let f = Filter::from_enabled_bitmask(wire(src, true, SUB_MAIN));
        assert!(DictionaryReader::passes_filter(
            KAUTIAN_BIT | TAIGITV,
            sub_accent(0),
            &f
        ));
    }

    #[test]
    fn passes_filter_all_enabled_short_circuits() {
        let f = Filter::from_enabled_bitmask(u32::MAX);
        assert!(DictionaryReader::passes_filter(KAUTIAN_BIT, sub_accent(0), &f));
    }
}
