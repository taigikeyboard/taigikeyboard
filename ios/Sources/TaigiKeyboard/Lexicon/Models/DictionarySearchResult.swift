// Dictionary tab 用的搜尋結果模型 — 含 source 資訊與外部辭典連結。

import Foundation

/// Search result with source information for dictionary exploration (Tab 3).
// Tab 3 詞典探索的搜尋結果。除了顯示用 roman,另存純 TL 給外部辭典 URL 使用。
struct DictionarySearchResult {
    /// Sentinel id for results synthesised from the user's custom dictionary.
    // 自訂詞庫合成結果的 sentinel id。
    static let customDictMarkerId = -2

    let id: Int
    let roman: String // Display form (POJ or TL based on user setting)
    let tl: String // Raw TL from database (for external lookup URLs)
    let hanzi: String?
    let frequency: Int
    let sources: [DictionarySource]

    /// Chhoe Taigi dictionary lookup URL for this result's TL form.
    // 對應 Chhoe Taigi 辭典的查詢 URL。
    var chhoeURL: URL? {
        ExternalLookupURLBuilder.chhoeURL(forTL: tl)
    }

    /// MOE Sutian dictionary lookup URL for this result's TL form.
    // 對應教育部臺語辭典(萌典)的查詢 URL。
    var moeURL: URL? {
        ExternalLookupURLBuilder.moeURL(forTL: tl)
    }
}
