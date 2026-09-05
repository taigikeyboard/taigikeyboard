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

// SyllableInventory — mmap 過的 syllables.fst 包裝。v3.5.9 B-1 起改 tagged-single-FST 格式,
//   每個音節以 `tl:` / `poj:` 前綴 + numeric/toneless 兩種 key 存在;contains_in(mode, …) 依模式選家族。

use fst::Set;
use mmap_host::MmapHandle;
use phonetics::InputMode;

use crate::error::LexiconError;

// SyllableInventory — 持有 fst::Set;條目 = `tl:` + `poj:` 兩家族下 numeric + toneless key 的去重總和。
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
    // 以唯讀 mmap 開啟 syllables.fst,並驗證 fst crate 能解析。
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
    /// `phonetics::canonicalize_poj_syllable`; TPS via
    /// `phonetics::canonicalize_tps_syllable`); this loader does no
    /// normalization. `mode == InputMode::English` routes through the
    /// TL family — English buffers do not have their own syllable
    /// inventory and rely on TL phonotactic gating where syllabification
    /// is invoked at all.
    ///
    /// v3.5.9 D / C-3b — `InputMode::Tps` routes to the `tps:` family
    /// emitted by `dictionary/build/create_syllables_fst.py` (one
    /// Bopomofo syllable per line, in both numeric-tone-marked and
    /// toneless forms).
    // 在 mode 指定的家族裡查詢一個 canonical 音節 key (numeric 或 toneless)。
    //   呼叫端必須已用對應家族的 canonicalize_*_syllable 正規化過。
    //   InputMode::English 不單設家族,沿用 `tl:` 家族查詢 (English 路徑不會走音節切分時為 no-op)。
    // v3.5.9 D / C-3b — InputMode::Tps 走 `tps:` 家族 (C-0 build pipeline 已 emit
    //   syllables.fst 內 Bopomofo per-syllable + numeric/toneless)。
    pub fn contains_in(&self, mode: InputMode, syllable: &str) -> bool {
        let prefix = match mode {
            InputMode::Poj => "poj:",
            InputMode::Tps => "tps:",
            InputMode::Tl | InputMode::English => "tl:",
        };
        self.contains_prefixed(prefix, syllable)
    }

    /// TPS ambiguity-aware membership probe: true when ANY reading of
    /// `syllable` under the TPS ambiguity families is in the `tps:`
    /// inventory (`INVARIANT_TPS_DEFOLD_ENUMERATE` §35). The literal
    /// probe (`contains_in(Tps, …)`) is a strict subset, so every edge
    /// the segmenter produced before this round still exists.
    ///
    /// `final_only_offsets` = byte offsets into `syllable` of glyphs
    /// immediately before a hard close (span end at a stripped separator
    /// barrier); tone-mark restriction is derived inside the pattern.
    // TPS 歧義感知 inventory 探測 — syllable 任一讀法在 tps: 家族即 true;
    //   字面探測為嚴格子集 → 既有 edge 全數保留。final_only_offsets = barrier 前一格偏移。
    pub fn contains_in_tps_readings(&self, syllable: &str, final_only_offsets: &[usize]) -> bool {
        use fst::{IntoStreamer, Streamer};
        if syllable.is_empty() {
            return false;
        }
        // Fast path: the literal reading needs no automaton — and when the
        // span holds no ambiguity-family glyph at all, the pattern could
        // only ever match the literal, so a literal miss is a miss. This
        // keeps the per-keystroke BFS (O(n × 24) probes, most of which
        // miss) from building an automaton per probe.
        // 字面命中免 automaton;span 無歧義 glyph 時 pattern 等同字面,
        //   字面 miss 即 miss — BFS 熱路徑(多數 probe 為 miss)不必逐 probe 建自動機。
        if self.contains_prefixed("tps:", syllable) {
            return true;
        }
        if !crate::tps_pattern::has_ambiguous_glyph(syllable) {
            return false;
        }
        let mut key = String::with_capacity(4 + syllable.len());
        key.push_str("tps:");
        key.push_str(syllable);
        // Pattern offsets are relative to the full key (prefix included).
        let shifted: Vec<usize> = final_only_offsets.iter().map(|o| o + 4).collect();
        let pattern = crate::tps_pattern::TpsKeyPattern::new(
            &key,
            crate::tps_pattern::WireMode::Exact,
            &shifted,
        );
        self.set.search(&pattern).into_stream().next().is_some()
    }

    /// Deprecated single-family membership test, kept as an alias of
    /// `contains_in(InputMode::Tl, …)` for the v3.5.9 B-1 grace period.
    /// All in-tree callers should migrate to `contains_in`; once those
    /// migrations land (B-1c plumbs `mode` through the syllabifier),
    /// this alias retires in B-7.
    // B-1 grace-period 兼容入口 — 等同 contains_in(Tl, syllable);B-7 退役。
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
    // 內部 helper — 把 prefix 與 syllable 拼成 fst key 再查 set。
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

    // 回傳 FST 內的 key 總數 (numeric + toneless 去重,含 tl: 與 poj: 兩家族)。
    pub fn entry_count(&self) -> u64 {
        self.set.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }
}
