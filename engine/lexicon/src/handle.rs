//! `EngineHandle` — process-singleton lifecycle for the lexicon engine.
//!
//! Holds `Mutex<Option<EngineState>>`. `install` builds a fresh
//! `EngineState` from validated paths and atomically swaps it in
//! ON SUCCESS — if any step fails, the previous state stays intact.
//!
//! `with_state` borrows the active state under the mutex, runs the
//! caller's closure, and returns. Concurrent `search` calls serialize
//! with each other and with `install`. No read/write split until
//! profiling proves contention (audit § scope D8 deferred).

// 進程級單例 — 持有 Mutex<Option<EngineState>>,install 成功才原子置換,失敗保留舊狀態。
// 並發 search 與 install 透過同一把 mutex 序列化,目前無讀寫分離。

use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::association_reader::AssociationReader;
use crate::dictionary_reader::DictionaryReader;
use crate::error::LexiconError;
use crate::paths::LexiconPaths;
use crate::prefix_index::PrefixIndex;
use crate::syllable_inventory::SyllableInventory;

/// Active lexicon state. The mandatory readers (`prefix_index`,
/// `dictionary`, `association`) are populated on every successful install;
/// `syllable_inventory` is opt-in (Phase 6 adds the wiring; the platform
/// only supplies the path once Phase 7 / 8 bundles `syllables.fst`).
/// `with_state` callers verify presence defensively.
// 安裝成功後的引擎狀態快照;install 失敗時整個 EngineState 不會替換。
// syllable_inventory 是 Phase 6 新增的選擇性欄位,平台未提供路徑時為 None。
pub struct EngineState {
    // 平台告知的字典版本號 (用於與 association.bin / dictionary.bin 對齊驗證)。
    pub dictionary_version: u32,
    // FST 前綴索引 (供羅馬字 / 漢字前綴查詢)。
    pub prefix_index: Option<PrefixIndex>,
    // TKDB 字典讀取器 (rowid → 詞條)。
    pub dictionary: Option<DictionaryReader>,
    // TKWA bigram 讀取器 (前一詞 → 後續候選詞)。
    pub association: Option<AssociationReader>,
    /// v3.5.8 Phase 6 — TL syllable inventory backed by `syllables.fst`.
    /// `None` when the platform did not pass a path; the composing
    /// continuous-input dispatcher returns an empty candidate list in
    /// that case.
    // Phase 6 新增 — TL 音節合法集合;平台未提供路徑時為 None。
    pub syllable_inventory: Option<SyllableInventory>,
}

// 引擎控制句柄 — 純 zero-sized type,所有 API 為靜態方法。
pub struct EngineHandle;

static STATE: Lazy<Mutex<Option<EngineState>>> = Lazy::new(|| Mutex::new(None));

// install 完成後回傳的統計數據,供平台 UI 顯示與健康檢查。
#[derive(Debug, Clone, Copy)]
pub struct InstallStats {
    // dictionary.bin 紀錄總數。
    pub dictionary_record_count: u64,
    // FST 前綴索引條目數。
    pub prefix_index_entry_count: u64,
}

impl EngineHandle {
    /// Install (or reinstall) the lexicon state. Idempotent: on success the
    /// new state atomically replaces any previous; on failure (open / mmap
    /// / format error) the previous state stays intact.
    // 安裝 (或重裝) 引擎狀態 — 成功時原子置換,失敗時保留舊狀態。
    pub fn install(paths: LexiconPaths) -> Result<InstallStats, LexiconError> {
        let prefix_index = PrefixIndex::open(&paths.fst)?;
        let dictionary = DictionaryReader::open(&paths.dictionary_bin)?;
        let association = AssociationReader::open(&paths.association_bin)?;
        let syllable_inventory = paths
            .syllables_fst
            .as_deref()
            .map(SyllableInventory::open)
            .transpose()?;

        let stats = InstallStats {
            dictionary_record_count: dictionary.record_count() as u64,
            prefix_index_entry_count: prefix_index.entry_count(),
        };

        let new_state = EngineState {
            dictionary_version: paths.dictionary_version,
            prefix_index: Some(prefix_index),
            dictionary: Some(dictionary),
            association: Some(association),
            syllable_inventory,
        };

        let mut guard = STATE
            .lock()
            .map_err(|_| LexiconError::Internal("install: state mutex poisoned".into()))?;
        *guard = Some(new_state);

        Ok(stats)
    }

    // 在 mutex 保護下借用目前的 EngineState 執行 closure;尚未 install 時回傳 NotInitialized。
    pub fn with_state<F, R>(f: F) -> Result<R, LexiconError>
    where
        F: FnOnce(&EngineState) -> Result<R, LexiconError>,
    {
        let guard = STATE
            .lock()
            .map_err(|_| LexiconError::Internal("with_state: state mutex poisoned".into()))?;
        match guard.as_ref() {
            Some(state) => f(state),
            None => Err(LexiconError::NotInitialized),
        }
    }
}
