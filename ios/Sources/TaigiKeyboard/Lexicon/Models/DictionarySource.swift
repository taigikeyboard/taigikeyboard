// 中文: 辭典來源 enum — 對應 dictionary.csv 欄位,用於搜尋結果的 source 標籤與 badge 顯示。
// 中文: bit 位置由 dictionary/common/source_bits.py 與 LexiconBitmask 共同擁有,allCases 順序非載入相依。

import Foundation

/// Dictionary source enum matching dictionary.csv column names.
///
/// Used for search-result attribution and badge display. The bitmask layout
/// is owned by `dictionary/common/source_bits.py` and decoded explicitly by
/// `LexiconBitmask` (which maps each bit position to its `DictionarySource`
/// value), so `allCases` ordering is NOT load-bearing — the `.custom` case
/// in particular has no `dictionary.bin` bit (UI-only marker for user-added
/// entries).

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
// 中文: 對應 dictionary.csv 欄位的辭典來源,custom 為 UI 標記用,沒有 dictionary.bin bit。
enum DictionarySource: String, CaseIterable {
    case kautian // 教育部臺灣台語常用詞辭典
    case taigitv // 台語新詞辭庫
    case itaigi // iTaigi 華台對照典
    case sitbut // 台灣植物名彙
    case taihoa // 台華線頂對照典
    case taijit // 台日大辭典
    case kungge // 台語工藝詞庫
    case stti // 學科術語辭典
    case khpoo // 腔口補充資料
    case khiin // 在來字
    case lkk // LKK漢羅合用建議用字
    case dev // 開發補充資料
    case custom // 自訂詞庫

    /// Short display name for badge.
    // 中文: badge 顯示用的短名稱(中文)。
    var displayName: String {
        switch self {
        case .kautian: "教典"
        case .taigitv: "台語新詞"
        case .itaigi: "iTaigi"
        case .sitbut: "植物名彙"
        case .taihoa: "台華對照"
        case .taijit: "臺日"
        case .kungge: "工藝辭典"
        case .stti: "學科術語"
        case .khpoo: "補充資料"
        case .khiin: "補充資料"
        case .lkk: "漢羅合用"
        case .dev: "補充資料"
        case .custom: "補充資料"
        }
    }
}
