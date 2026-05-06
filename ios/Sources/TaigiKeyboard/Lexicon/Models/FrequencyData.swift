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
