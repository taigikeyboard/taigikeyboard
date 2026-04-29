import Foundation

// 台語詞彙資料模型

// MARK: - Shared-Core Candidate

/// Pure logic, Foundation-only. Eligible for cross-platform extraction.
public struct TaigiWord: Equatable {
    let id: Int
    let roman: String
    let hanzi: String?
    let lengthScore: Int?
    let sourceBitmask: UInt16?

    init(
        id: Int,
        roman: String,
        hanzi: String?,
        lengthScore: Int?,
        sourceBitmask: UInt16? = nil,
    ) {
        self.id = id
        self.roman = roman
        self.hanzi = hanzi
        self.lengthScore = lengthScore
        self.sourceBitmask = sourceBitmask
    }

    var displayText: String {
        if let hanzi, !hanzi.isEmpty {
            return hanzi
        }
        return roman
    }
}
