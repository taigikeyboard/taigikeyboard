// 中文: 候選詞核心模型 — id / roman / hanzi / lengthScore / sourceBitmask。

import Foundation

// 台語詞彙資料模型

// MARK: - Shared-Core Candidate

/// Pure logic, Foundation-only. Eligible for cross-platform extraction.
// 中文: 台語詞彙 value type,排序 / 顯示 / FFI marshalling 都用這個型別。
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

    // 中文: 顯示文字 — hanzi 優先,沒有就 fallback 到 roman。
    var displayText: String {
        if let hanzi, !hanzi.isEmpty {
            return hanzi
        }
        return roman
    }
}
