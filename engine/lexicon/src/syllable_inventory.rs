//! `SyllableInventory` — fst::Set wrapper backed by mmap'd `syllables.fst`.
//!
//! Built by `dictionary/build/create_syllables_fst.py` (which stages
//! `tl_num` / `poj_num` lines to two temp files and invokes
//! `engine/build-helpers/fst-builder build-syllables --tl-input ...
//! --poj-input ...`). The output `syllables.fst` is the v3.5.9 B-1
//! tagged-single-FST format: each phonotactically valid syllable
//! appears in two canonical forms (numeric `tsua7` + toneless `tsua`)
//! tagged with one of two family prefixes:
//!   - `tl:<canonical>` — TL syllable inventory (POJ source rows fold
//!     to TL spelling via `phonetics::canonicalize_syllable`)
//!   - `poj:<canonical>` — POJ syllable inventory (POJ source rows
//!     keep their POJ ASCII shape via
//!     `phonetics::canonicalize_poj_syllable`)
//!
//! Why the family split (v3.5.9 B-1, from v3.5.8 single-family): about
//! half the dictionary's `poj_num` strings diverge from `tl_num`
//! (`chit8`/`tsit8`, `goa2`/`gua2`, `toa7`/`tua7`). The continuous
//! POJ buffer needs to recognise `chiah` as one valid syllable, which
//! a single TL-folded inventory cannot do without conflating boundaries.
//! Design + alternatives in `docs/reports/2026-05-20-v359-b-plan.md`
//! §B-1.

// 中文: SyllableInventory — mmap 過的 syllables.fst 包裝。v3.5.9 B-1 起改 tagged-single-FST 格式,
// 中文:   每個音節以 `tl:` / `poj:` 前綴 + numeric/toneless 兩種 key 存在;contains_in(mode, …) 依模式選家族。

use fst::Set;
use mmap_host::MmapHandle;
use phonetics::InputMode;

use crate::error::LexiconError;

// 中文: SyllableInventory — 持有 fst::Set;條目 = `tl:` + `poj:` 兩家族下 numeric + toneless key 的去重總和。
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

    /// Membership test for a single canonical syllable key in the family
    /// implied by `mode`. Callers MUST have canonicalized the syllable
    /// for the correct family ahead of time (TL via
    /// `phonetics::canonicalize_syllable`; POJ via
    /// `phonetics::canonicalize_poj_syllable`); this loader does no
    /// normalization. `mode == InputMode::English` routes through the
    /// TL family — English buffers do not have their own syllable
    /// inventory and rely on TL phonotactic gating where syllabification
    /// is invoked at all.
    // 中文: 在 mode 指定的家族裡查詢一個 canonical 音節 key (numeric 或 toneless)。
    // 中文:   呼叫端必須已用對應家族的 canonicalize_*_syllable 正規化過。
    // 中文:   InputMode::English 不單設家族,沿用 `tl:` 家族查詢 (English 路徑不會走音節切分時為 no-op)。
    pub fn contains_in(&self, mode: InputMode, syllable: &str) -> bool {
        let prefix = match mode {
            InputMode::Poj => "poj:",
            InputMode::Tl | InputMode::English => "tl:",
        };
        self.contains_prefixed(prefix, syllable)
    }

    /// Deprecated single-family membership test, kept as an alias of
    /// `contains_in(InputMode::Tl, …)` for the v3.5.9 B-1 grace period.
    /// All in-tree callers should migrate to `contains_in`; once those
    /// migrations land (B-1c plumbs `mode` through the syllabifier),
    /// this alias retires in B-7.
    // 中文: B-1 grace-period 兼容入口 — 等同 contains_in(Tl, syllable);B-7 退役。
    #[deprecated(
        since = "0.1.0",
        note = "use contains_in(InputMode::Tl, syllable) — v3.5.9 B-1 mode-aware inventory"
    )]
    pub fn contains(&self, syllable: &str) -> bool {
        self.contains_prefixed("tl:", syllable)
    }

    /// Internal helper: probe the underlying fst::Set for `<prefix><syllable>`
    /// without allocating a new String when the syllable is short enough
    /// to land on the stack. We currently always allocate; callers stay
    /// off the hot path (BFS syllabifier loops do ≤ MAX_SYLLABLE_BYTES
    /// probes per start, well bounded by inventory size).
    // 中文: 內部 helper — 把 prefix 與 syllable 拼成 fst key 再查 set。
    fn contains_prefixed(&self, prefix: &str, syllable: &str) -> bool {
        // Allocate a single String to avoid the cost of two .as_bytes()
        // calls and a manual byte concat. The fst::Set lookup itself
        // dominates this branch; the allocation stays as one fresh
        // String per probe, matching the prior implementation's cost
        // shape.
        let mut key = String::with_capacity(prefix.len() + syllable.len());
        key.push_str(prefix);
        key.push_str(syllable);
        self.set.contains(key.as_bytes())
    }

    // 中文: 回傳 FST 內的 key 總數 (numeric + toneless 去重,含 tl: 與 poj: 兩家族)。
    pub fn entry_count(&self) -> u64 {
        self.set.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}
