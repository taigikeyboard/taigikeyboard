import Foundation

// Dictionary source enum matching dictionary.csv column names.
//
// Used for search-result attribution (which dictionaries a word came from).
// The bitmask layout is owned by `dictionary/common/source_bits.py` and decoded
// explicitly by `LexiconBitmask` (which maps each bit position to its
// `DictionarySource` value), so `allCases` ordering is NOT load-bearing — the
// `.custom` case in particular has no `dictionary.bin` bit (UI-only marker for
// user-added entries).
//
// The localized badge label for each source is resolved at the UI call site
// (`DictionaryTab.tagKey(for:)`), not stored here — keeps this shared-core enum
// free of App-layer i18n types.

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
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
}
