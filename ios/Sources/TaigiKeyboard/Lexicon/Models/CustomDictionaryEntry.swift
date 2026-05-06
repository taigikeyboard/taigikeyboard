// 中文: 使用者自訂詞庫的單筆紀錄模型 — id / roman / hanzi + 建立與更新時間。

import Foundation

/// Custom dictionary entry model

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
// 中文: 自訂詞庫項目 — Identifiable + Equatable,id 為 UUID 字串。
struct CustomDictionaryEntry: Identifiable, Equatable {
    let id: String
    var roman: String
    var hanzi: String
    let createdAt: Date
    var updatedAt: Date

    init(
        id: String = UUID().uuidString,
        roman: String,
        hanzi: String,
        createdAt: Date = Date(),
        updatedAt: Date = Date(),
    ) {
        self.id = id
        self.roman = roman
        self.hanzi = hanzi
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}
