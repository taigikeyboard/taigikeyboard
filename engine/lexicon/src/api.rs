//! Public API for the bridge layer. Each method takes proto-shaped inputs
//! and returns proto-shaped outputs; the dispatch module wraps these into
//! envelope responses.

// 對外 API 層 — 每個方法以 proto 訊息為輸入/輸出,由 dispatch 模組包成 envelope 回應。

use protos::engine::{
    AssocLookupRequest, AssocLookupResponse, ClassifyInputRequest, ClassifyInputResponse,
    DictionaryFiltersRequest, DictionaryFiltersResponse, InstallRequest, InstallResponse,
    IsHanziRequest, IsHanziResponse, LexiconAssocEntry, SearchByHanziRequest,
    SearchByHanziResponse, SearchRequest, SearchResponse, SearchWithSourcesRequest,
    SearchWithSourcesResponse, TaigiWord,
};

use crate::classification;
use crate::dictionary_filters::compute_filters;
use crate::error::LexiconError;
use crate::handle::EngineHandle;
use crate::paths::LexiconPaths;
use crate::search::{
    self, LexiconAssocOut, LexiconRowOut, SearchInputMode, SearchInputType, SearchParams,
};

// 安裝 (或重裝) 詞庫狀態 — 驗證路徑、開啟 FST/TKDB/TKWA (+選擇性 syllables.fst),並原子性置換到 EngineHandle。
pub fn install(req: InstallRequest) -> Result<InstallResponse, LexiconError> {
    let paths = LexiconPaths::validated(
        &req.trie_path,
        &req.dictionary_bin_path,
        &req.association_bin_path,
        &req.syllable_inventory_path,
        req.dictionary_version,
    )?;
    let stats = EngineHandle::install(paths)?;
    Ok(InstallResponse {
        dictionary_record_count: stats.dictionary_record_count,
        prefix_index_entry_count: stats.prefix_index_entry_count,
    })
}

// IME 候選詞查詢主入口 — 依 SearchRequest 建立參數,經前綴索引與 TKDB 過濾後回傳候選詞。
pub fn search(req: SearchRequest) -> Result<SearchResponse, LexiconError> {
    let params = build_search_params(&req)?;
    EngineHandle::with_state(|state| {
        let prefix_index = state
            .prefix_index
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("prefix_index unavailable".into()))?;
        let dict = state
            .dictionary
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("dictionary reader unavailable".into()))?;
        let rows = search::search(&params, prefix_index, dict)?;
        Ok(SearchResponse {
            rows: rows.into_iter().map(row_out_to_taigi_word).collect(),
        })
    })
}

// Tab3 多字典來源羅馬字查詢 — 強制以 RomanWithTone 模式進行,並套用使用者啟用的字典來源 bitmask。
pub fn search_with_sources(
    req: SearchWithSourcesRequest,
) -> Result<SearchWithSourcesResponse, LexiconError> {
    let params = SearchParams {
        input: req.input,
        input_type: SearchInputType::RomanWithTone,
        input_mode: proto_input_mode(req.input_mode),
        limit: req.limit,
        enabled_sources_bitmask: req.enabled_sources_bitmask,
    };
    EngineHandle::with_state(|state| {
        let prefix_index = state
            .prefix_index
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("prefix_index unavailable".into()))?;
        let dict = state
            .dictionary
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("dictionary reader unavailable".into()))?;
        let rows = search::search_with_sources(&params, prefix_index, dict)?;
        Ok(SearchWithSourcesResponse {
            rows: rows.into_iter().map(row_out_to_taigi_word).collect(),
        })
    })
}

// Tab3 漢字查詢路徑 — 以 hanzi: 前綴掃描索引,套用字典來源 bitmask 過濾。
pub fn search_by_hanzi(req: SearchByHanziRequest) -> Result<SearchByHanziResponse, LexiconError> {
    EngineHandle::with_state(|state| {
        let prefix_index = state
            .prefix_index
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("prefix_index unavailable".into()))?;
        let dict = state
            .dictionary
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("dictionary reader unavailable".into()))?;
        let rows = search::search_by_hanzi(
            &req.query,
            req.limit,
            req.enabled_sources_bitmask,
            prefix_index,
            dict,
        )?;
        Ok(SearchByHanziResponse {
            rows: rows.into_iter().map(row_out_to_taigi_word).collect(),
        })
    })
}

// NextWord bigram 查詢 — 以前一個詞為 key 進行二分搜尋,套用 9-bit 來源過濾後回傳排序後的後續候選詞。
pub fn assoc_lookup(req: AssocLookupRequest) -> Result<AssocLookupResponse, LexiconError> {
    EngineHandle::with_state(|state| {
        let assoc = state
            .association
            .as_ref()
            .ok_or_else(|| LexiconError::Internal("association reader unavailable".into()))?;
        let entries = search::assoc_lookup(
            &req.previous_word,
            req.limit,
            req.enabled_sources_bitmask,
            assoc,
        )?;
        Ok(AssocLookupResponse {
            entries: entries.into_iter().map(assoc_out_to_proto).collect(),
        })
    })
}

// IME 自動完成輸入分類器 — 判斷輸入屬於漢字、有聲調羅馬字或無聲調羅馬字,並回傳轉換後的查詢 key。
pub fn classify_input(req: ClassifyInputRequest) -> Result<ClassifyInputResponse, LexiconError> {
    let result = classification::classify_input(&req.raw);
    Ok(ClassifyInputResponse {
        input_type: result.input_type as i32,
        search_key: result.search_key,
    })
}

// Tab3 漢字判斷捷徑 — 給定文字內含任一 CJK 漢字 (含 Extensions A-E) 即回傳 true。
pub fn is_hanzi(req: IsHanziRequest) -> Result<IsHanziResponse, LexiconError> {
    Ok(IsHanziResponse {
        is_hanzi: classification::is_hanzi(&req.text),
    })
}

// 將使用者的字典開關偏好換算為 dictionary.bin / association.bin 的 bitmask 與啟用來源代碼清單。
pub fn dictionary_filters(
    req: DictionaryFiltersRequest,
) -> Result<DictionaryFiltersResponse, LexiconError> {
    let toggles = req.toggles.unwrap_or_default();
    Ok(compute_filters(&toggles))
}

fn build_search_params(req: &SearchRequest) -> Result<SearchParams, LexiconError> {
    // C-3a: `req.tps_or_mapped_to_er` is intentionally ignored — the
    // er↔or dialect axis is handled at build time via dual-emit `tps:`
    // keys in `dictionary.fst`. The proto field stays on the wire for
    // backward compatibility with platforms still setting it.
    Ok(SearchParams {
        input: req.input.clone(),
        input_type: proto_input_type(req.input_type),
        input_mode: proto_input_mode(req.input_mode),
        limit: req.limit,
        enabled_sources_bitmask: req.enabled_sources_bitmask,
    })
}

fn proto_input_type(value: i32) -> SearchInputType {
    use protos::engine::InputType;
    match InputType::try_from(value).unwrap_or(InputType::Unspecified) {
        InputType::RomanNoTone => SearchInputType::RomanNoTone,
        InputType::Hanzi => SearchInputType::Hanzi,
        // Unspecified + RomanWithTone all map to RomanWithTone (default
        // romanization path).
        _ => SearchInputType::RomanWithTone,
    }
}

fn proto_input_mode(value: i32) -> SearchInputMode {
    use protos::engine::InputMode;
    match InputMode::try_from(value).unwrap_or(InputMode::Unspecified) {
        InputMode::Poj => SearchInputMode::Poj,
        InputMode::Tps => SearchInputMode::Tps,
        // Unspecified + Tl default to TL.
        _ => SearchInputMode::Tl,
    }
}

fn row_out_to_taigi_word(row: LexiconRowOut) -> TaigiWord {
    TaigiWord {
        id: row.id,
        roman: row.roman,
        hanji: row.hanji,
        length_score: row.length_score,
        source_bitmask: row.source_bitmask,
    }
}

fn assoc_out_to_proto(entry: LexiconAssocOut) -> LexiconAssocEntry {
    LexiconAssocEntry {
        previous_word: entry.previous_word,
        candidate_word: entry.candidate_word,
        count: entry.count,
        candidate_tl: entry.candidate_tl,
    }
}
