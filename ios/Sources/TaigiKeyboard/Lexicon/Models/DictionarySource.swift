import Foundation

/// Dictionary source enum matching DB column names.
///
/// The ordering of `allCases` is authoritative for the bitmask layout used by
/// `DictionaryBinaryReader` — do not reorder without updating the binary format.
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
