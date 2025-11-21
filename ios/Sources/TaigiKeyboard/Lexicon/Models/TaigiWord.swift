import Foundation

/// 台語詞彙資料模型
public struct TaigiWord {
    let id: Int
    let roman: String
    let hanzi: String?
    let lengthScore: Int?

    var displayText: String {
        if let hanzi, !hanzi.isEmpty {
            return hanzi
        }
        return roman
    }
}
