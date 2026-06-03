// 中文: 排序管線用的單字頻率快照 — 會被 RustEngineBridge.processCandidates 編成 proto 送進 Rust ranker。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Per-word frequency snapshot consumed by ranking. Marshalled to
/// `Taigi_Engine_FrequencyEntry` inside `RustEngineBridge.processCandidates`
/// for `engine/ranking/src/score.rs::calculate_score`.
///
/// Hoisted out of `UserFrequencyRepository` so the bridge marshalling code
/// depends on this Foundation-only value type rather than an iOS-only
/// SQLite repository.
// 中文: 單字頻率快照 — count + 上次使用毫秒時戳。從 SQLite repository 抽出來,
// 中文: 讓 bridge marshalling 只依賴 Foundation,不綁 iOS 的儲存層。
public struct FrequencyData {
    public let count: Int
    public let lastUsedMillis: Int64 // Unix timestamp in milliseconds

    public init(count: Int, lastUsedMillis: Int64) {
        self.count = count
        self.lastUsedMillis = lastUsedMillis
    }

    public static let empty = FrequencyData(count: 0, lastUsedMillis: 0)
}

/// One `user_frequency.db` row in R5 `(word, tl)` pair-key form: the
/// display-text key, its canonical-TL reading, and the snapshot. Returned
/// by `UserFrequencyRepository.frequencyDataBatch` so the engine can build
/// a `(display_text, canonical_tl)`-keyed `FrequencyMap` (Core Principle
/// #7). `tl == ""` is the legacy fallback bucket.
// 中文: R5 (word, tl) pair-key 的一列 — 顯示鍵 + canonical TL 讀音 + 快照;tl='' 為 legacy fallback 桶。
public struct FrequencyRow {
    public let word: String
    public let tl: String
    public let data: FrequencyData

    public init(word: String, tl: String, data: FrequencyData) {
        self.word = word
        self.tl = tl
        self.data = data
    }
}
