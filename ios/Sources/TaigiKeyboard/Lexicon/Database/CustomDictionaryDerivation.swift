// 中文: 自訂詞庫的搜尋鍵衍生 — 從 roman 字串純函式產生 notone / abbrev / roman_num
// 中文: 三種索引欄位值,寫入端與查詢端共用同一份規則。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Pure-function derivation of search-key variants from a romanization string.
///
/// These keys back the indexed columns (`notone`, `abbrev`, `roman_num`) of
/// the custom-dictionary table. The same logic runs at write time (inside
/// `CustomDictionaryRepository.bindEntry` and the backfill migrator) and at
/// read time (when `LexiconService` / `DictionarySearchViewModel` build the
/// query key), so both sides must stay exactly in sync — hence a single
/// canonical implementation here rather than any callback into the service.
// 中文: 寫入端與查詢端共用同一份衍生規則,所以集中放在這裡而不是 callback 回 service。
enum CustomDictionaryDerivation {
    /// Toneless form — Rust `Method::DeriveNotone` strips tone diacritics + digits
    /// + hyphens + spaces after lowercase + nasal-marker conversion.
    // 中文: 無調符形式 — 走 Rust DeriveNotone,小寫 + 鼻音轉換後拔掉調符 / 數字 / 分隔符。
    static func generateNotone(_ roman: String) -> String {
        RustEngineBridge.deriveNotone(roman)
    }

    /// Abbreviation form — Rust `Method::DeriveAbbrev` returns first char per
    /// syllable (split by ASCII whitespace + hyphen), diacritics stripped.
    /// Returns "" when fewer than 2 syllables. Whitespace canonical
    /// `[ \t\n\x0B\f\r-]+` matches Android JVM `Regex("[\\s-]+")` (Codex v3 §1).
    // 中文: 縮寫形式 — 走 Rust DeriveAbbrev,每個音節取首字母。少於 2 音節回空字串。
    static func generateAbbrev(_ roman: String) -> String {
        RustEngineBridge.deriveAbbrev(roman)
    }

    /// Numeric-toned form for tone-aware search — same as `Method::NormalizeInput`.
    // 中文: 數字調形式 — 給 tone-aware 搜尋用,等同 NormalizeInput。
    static func generateRomanNum(_ roman: String) -> String {
        RustEngineBridge.normalizeInput(roman)
    }

    /// Build the custom-dictionary prefix-search key for `roman`.
    ///
    /// Selects the column strategy by input shape:
    /// - tone-aware (contains a digit) → lowercase, strip `-` / spaces, match `roman_num`.
    /// - toneless → strip diacritics/digits/separators via `generateNotone`, match `notone`.
    // 中文: 建構自訂詞庫 prefix 搜尋鍵 — 含數字 → 走 roman_num 欄位,
    // 中文: 不含數字 → 用 generateNotone 走 notone 欄位。
    static func searchPrefix(for roman: String) -> (key: String, isToneAware: Bool) {
        let isToneAware = roman.contains { $0.isNumber }
        let key: String = if isToneAware {
            roman.lowercased()
                .replacingOccurrences(of: "-", with: "")
                .replacingOccurrences(of: " ", with: "")
        } else {
            generateNotone(roman)
        }
        return (key, isToneAware)
    }
}
