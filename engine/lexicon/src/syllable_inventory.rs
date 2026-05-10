//! `SyllableInventory` — fst::Set wrapper backed by mmap'd `syllables.fst`.
//!
//! Built by `dictionary/build/create_syllables_fst.py` (which pipes raw
//! `tl_num` strings to `engine/build-helpers/fst-builder build-syllables`).
//! Each phonotactically valid TL syllable appears in two canonical forms:
//! numeric (`tsua7`) and toneless (`tsua`). POJ has no separate inventory
//! per `docs/roadmap.md` §Phase 2 line 193 — POJ input is normalized to TL
//! at runtime via `phonetics::poj::to_tl`.
//!
//! Phase 2 deliberately exposes only `contains` membership; the Phase 3
//! syllabifier (composing crate, future slice) will add prefix walking
//! and any metadata it needs.

// 中文: SyllableInventory — mmap 過的 syllables.fst 包裝,提供 v3.5.8 Phase 2 的 TL 音節合法集合查詢。
// 中文: 每個合法音節同時存 numeric (tsua7) 與 toneless (tsua) 兩種 key;POJ 走 to_tl 正規化後查 TL 表。

use fst::Set;
use mmap_host::MmapHandle;

use crate::error::LexiconError;

// 中文: SyllableInventory — 持有 fst::Set;條目 = numeric + toneless key 的去重總和。
pub struct SyllableInventory {
    set: Set<MmappedSetData>,
}

/// Wrapper that lets `fst::Set` own the `MmapHandle` while exposing
/// `AsRef<[u8]>`. Mirrors the same-named wrapper in
/// `prefix_index::MmappedSetData`; both stay module-local because the
/// duplication is small and crate-internal — extract if a third caller
/// appears.
struct MmappedSetData {
    handle: MmapHandle,
}

impl AsRef<[u8]> for MmappedSetData {
    fn as_ref(&self) -> &[u8] {
        self.handle.as_slice()
    }
}

impl SyllableInventory {
    /// Open `syllables.fst` mmap'd readonly. Validates that fst can parse
    /// the bytes; deeper format checks happen on first lookup.
    // 中文: 以唯讀 mmap 開啟 syllables.fst,並驗證 fst crate 能解析。
    pub fn open(path: &std::path::Path) -> Result<Self, LexiconError> {
        let handle = MmapHandle::open_readonly(path).map_err(|source| LexiconError::Mmap {
            path: path.display().to_string(),
            source,
        })?;
        let data = MmappedSetData { handle };
        let set = Set::new(data).map_err(|e| {
            LexiconError::InvalidBinary(format!("fst load `{}`: {}", path.display(), e))
        })?;
        Ok(Self { set })
    }

    /// Membership test for a single canonical TL syllable key (numeric or
    /// toneless). Callers MUST canonicalize POJ-shaped input via
    /// `phonetics::canonicalize_syllable` first; this loader does no
    /// normalization.
    // 中文: 查詢單一 canonical TL 音節 key (numeric 或 toneless) 是否存在;POJ 輸入請先用 canonicalize_syllable 正規化。
    pub fn contains(&self, syllable: &str) -> bool {
        self.set.contains(syllable.as_bytes())
    }

    // 中文: 回傳 FST 內的 key 總數 (numeric + toneless 去重後)。
    pub fn entry_count(&self) -> u64 {
        self.set.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}
