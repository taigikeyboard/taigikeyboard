//! Search orchestration.
//!
//! Pipeline (per `docs/engine/lexicon-slice-plan.md` §5.4):
//! 1. **D-8 hard guard** — `input_type == Hanzi` short-circuits to `[]`
//!    BEFORE any reader is touched. Pinned by INVARIANT_LEX_HANZI_GUARD
//!    (Rust + iOS + Android per Codex Mod 1).
//! 2. Build trie key via `key_normalizer`.
//! 3. `prefix_index.lookup_prefix` returns insertion-ordered rowids
//!    (D-12 parity correction toward Android).
//! 4. Resolve each rowid through `dictionary_reader.record` + filter,
//!    take `limit`, return `LexiconRowOut`.
//!
//! TPS dialect er↔or recall: C-3a moved the runtime expansion into the
//! build pipeline (dual-emit `tps:` keys for the ㄜ and ㄛ glyphs at the
//! same rowid). The lexicon search path is now mode-blind for that axis.

// 中文: 詞庫搜尋協調層 — Hanzi guard、key 規範化、前綴+完全比對合併、過濾與排序。TPS 方言 er↔or 已於 C-3a 移到 build pipeline 雙 emit,不再有 runtime 分支。

use indexmap::IndexSet;

use crate::association_reader::{AssocFilter, AssociationReader};
use crate::dictionary_reader::{DictionaryReader, DictionaryRecord, Filter};
use crate::error::LexiconError;
use crate::key_normalizer::{self, KeyMode, KeyType};
use crate::prefix_index::PrefixIndex;

/// Public per-row output. Mirrors proto `TaigiWord` but kept Rust-native to
/// avoid coupling search internals to prost types.
// 中文: 搜尋結果單筆 — 對應 proto TaigiWord,刻意維持 Rust-native 結構以隔離 prost 型別。
#[derive(Debug, Clone)]
pub struct LexiconRowOut {
    // 中文: 字典 rowid (1-based)。
    pub id: i64,
    // 中文: TL 羅馬字。
    pub roman: String,
    // 中文: 漢字寫法 (可選)。
    pub hanji: Option<String>,
    // 中文: 排序用的長度/頻率分數 (沿用 frequency 數值)。
    pub length_score: Option<i32>,
    // 中文: 來源 bitmask,供平台貼來源標籤。
    pub source_bitmask: Option<u32>,
}

/// Public per-bigram output. Mirrors proto `LexiconAssocEntry`.
// 中文: bigram 查詢結果單筆 — 對應 proto LexiconAssocEntry。
#[derive(Debug, Clone)]
pub struct LexiconAssocOut {
    // 中文: 前一個詞 (查詢的 key)。
    pub previous_word: String,
    // 中文: 後續候選詞 (漢字)。
    pub candidate_word: String,
    // 中文: 後續候選詞的 TL 羅馬字。
    pub candidate_tl: String,
    // 中文: bigram 出現次數。
    pub count: u32,
}

// 中文: 搜尋輸入類型;由 dispatch 將 proto InputType 對應到此 enum。
#[derive(Debug, Clone, Copy)]
pub enum SearchInputType {
    // 中文: 無聲調的羅馬字輸入。
    RomanNoTone,
    // 中文: 含聲調的羅馬字輸入 (聲調符號或數字皆可)。
    RomanWithTone,
    // 中文: 漢字輸入,在 search() 中會直接短路返回 []。
    Hanzi,
}

// 中文: 搜尋輸入模式 (羅馬字方案);由 dispatch 將 proto InputMode 對應到此 enum。
#[derive(Debug, Clone, Copy)]
pub enum SearchInputMode {
    // 中文: TL 羅馬字。
    Tl,
    // 中文: POJ 羅馬字。
    Poj,
    // 中文: TPS Bopomofo (查 `tps:` 族群,C-1 起獨立 family;er↔or 方言以 build-time 雙 emit 處理)。
    Tps,
}

// 中文: search() / search_with_sources() 的輸入參數打包。
#[derive(Debug, Clone)]
pub struct SearchParams {
    // 中文: 使用者輸入字串 (尚未經 normalize)。
    pub input: String,
    // 中文: 輸入類型 (Hanzi / RomanWithTone / RomanNoTone)。
    pub input_type: SearchInputType,
    // 中文: 輸入模式 (TL / POJ / TPS)。
    pub input_mode: SearchInputMode,
    // 中文: 結果筆數上限。
    pub limit: u32,
    // 中文: 啟用字典來源的 bitmask (含 variant + khiin 控制位元)。
    pub enabled_sources_bitmask: u32,
}

// 中文: IME 候選詞主搜尋 — Hanzi 短路、組查詢 key、合併 exact + prefix rowid、可選 TPS er↔or 擴展、過濾排序後回傳。
pub fn search(
    params: &SearchParams,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<LexiconRowOut>, LexiconError> {
    // D-8 hard guard.
    if matches!(params.input_type, SearchInputType::Hanzi) {
        return Ok(Vec::new());
    }
    if params.limit == 0 {
        return Ok(Vec::new());
    }

    let key = key_normalizer::build(
        &params.input,
        KeyType::Romanization,
        match params.input_mode {
            SearchInputMode::Tl => KeyMode::Tl,
            SearchInputMode::Poj => KeyMode::Poj,
            SearchInputMode::Tps => KeyMode::Tps,
        },
    );

    // Exact-then-prefix concatenation matches Android's
    // `(exactRowIds + prefixRowIds).distinct()` semantics. IndexSet
    // dedup preserves insertion order — D-12 parity correction toward
    // Android pinned by INVARIANT_LEX_LOOKUP_ROWIDS_ORDER.
    let mut rowids: IndexSet<u32> = IndexSet::new();
    for id in prefix_index.lookup_exact(&key) {
        rowids.insert(id);
    }
    for id in prefix_index.lookup_prefix(&key) {
        rowids.insert(id);
    }

    Ok(collect_filtered_sorted(
        rowids,
        dict,
        params.enabled_sources_bitmask,
        params.limit,
    ))
}

// 中文: Tab3 多來源羅馬字搜尋 — 與 search() 共用流程,僅由 api 層強制 RomanWithTone 模式。
pub fn search_with_sources(
    params: &SearchParams,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<LexiconRowOut>, LexiconError> {
    // Tab3 multi-source — matches the romanization path; hanzi inputs are
    // dispatched to search_by_hanzi instead.
    search(params, prefix_index, dict)
}

// 中文: Tab3 漢字查詢 — 以 hanzi: 前綴掃描 FST,過濾排序後回傳。
pub fn search_by_hanzi(
    query: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Result<Vec<LexiconRowOut>, LexiconError> {
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let key = format!("hanzi:{query}");
    let mut rowids: IndexSet<u32> = IndexSet::new();
    for id in prefix_index.lookup_exact(&key) {
        rowids.insert(id);
    }
    for id in prefix_index.lookup_prefix(&key) {
        rowids.insert(id);
    }
    Ok(collect_filtered_sorted(
        rowids,
        dict,
        enabled_sources_bitmask,
        limit,
    ))
}

/// Filter rowids through `passesFilter`, then sort by `frequency` descending,
/// then take `limit`. Mirrors iOS `DictionaryRepository.lookupRowIds` +
/// platform sort step (see audit §3 — frequency-desc sort happens INSIDE
/// the repository today; this slice consolidates both into the engine).
/// Pinned by `INVARIANT_LEX_FREQUENCY_SORT` (parity test added in commit 7
/// follow-up).
// 中文: 將 rowid 解碼為紀錄、套 3 層過濾、依 frequency 由大到小穩定排序、截斷至 limit。
fn collect_filtered_sorted(
    rowids: IndexSet<u32>,
    dict: &DictionaryReader,
    enabled_sources_bitmask: u32,
    limit: u32,
) -> Vec<LexiconRowOut> {
    let filter = Filter::from_enabled_bitmask(enabled_sources_bitmask);
    let mut staged: Vec<(u32, DictionaryRecord)> = Vec::with_capacity(rowids.len());
    for rowid in rowids {
        if let Some(record) = dict.record(rowid) {
            if !DictionaryReader::passes_filter(record.bitmask, &filter) {
                continue;
            }
            staged.push((rowid, record));
        }
    }
    // Stable sort by frequency descending. Tied scores fall back to
    // insertion order (IndexSet rowid order preserved by `sort_by_key`).
    staged.sort_by_key(|entry| std::cmp::Reverse(entry.1.frequency));
    let limit_usize = limit as usize;
    staged.truncate(limit_usize);
    staged
        .into_iter()
        .map(|(rowid, record)| record_to_row(rowid, record))
        .collect()
}

// 中文: NextWord bigram 查詢 — 從 association reader 取得至多 limit 筆 bigram 紀錄,套用 9-bit 來源過濾後回傳。
pub fn assoc_lookup(
    previous_word: &str,
    limit: u32,
    enabled_sources_bitmask: u32,
    assoc: &AssociationReader,
) -> Result<Vec<LexiconAssocOut>, LexiconError> {
    if previous_word.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let filter = AssocFilter {
        all_enabled: enabled_sources_bitmask == u32::MAX,
        enabled_mask: (enabled_sources_bitmask & 0x1FF) as u16, // 9 association bits
    };
    let raw = assoc.lookup(previous_word, limit as usize);
    let mut out: Vec<LexiconAssocOut> = Vec::with_capacity(raw.len());
    for entry in raw {
        if !AssociationReader::passes_filter(entry.bitmask, &filter) {
            continue;
        }
        out.push(LexiconAssocOut {
            previous_word: previous_word.to_string(),
            candidate_word: entry.next_word,
            candidate_tl: entry.next_tl,
            count: entry.count,
        });
    }
    Ok(out)
}

fn record_to_row(rowid: u32, record: DictionaryRecord) -> LexiconRowOut {
    LexiconRowOut {
        id: rowid as i64,
        roman: record.tl,
        hanji: record.hanzi,
        length_score: Some(record.frequency as i32),
        source_bitmask: Some(record.bitmask as u32),
    }
}
