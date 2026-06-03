// 中文: 使用者詞頻 service — 對 UserFrequencyRepository 做薄包裝,提供候選詞流程使用。

import Foundation

/// 使用者詞頻服務
/// 管理使用者選擇詞彙的頻率
final class UserFrequencyService: @unchecked Sendable {
    // MARK: - Properties

    private let repository: UserFrequencyRepository
    private let logger = DebugLogger(category: "UserFrequencyService")

    // MARK: - Initialization

    init(repository: UserFrequencyRepository = CompositionRoot.userFrequencyRepository) {
        self.repository = repository
    }

    // MARK: - Lifecycle

    /// Ensure the underlying frequency DB is open and schema is applied.
    /// LexiconService calls this before a search arrives so candidate ranking
    /// can read frequencies without a cold-connect penalty on the first query.
    // 中文: 確保底層頻率 DB 已打開且 schema 套用 — 第一次查詢前先呼叫,避免 cold-connect 延遲。
    func ensureInitialized() async throws {
        try await repository.ensureInitialized()
    }

    // MARK: - Instance Methods

    /// Record a candidate commit. R5: the `(word, tl)` pair is the identity
    /// (Core Principle #7) — `tl` is the candidate's canonical-TL reading so
    /// 一字多音 keep separate counts. Pass `""` only when the candidate has
    /// no canonical TL (wire skew / TPS-OOV) → the legacy fallback bucket.
    // 中文: 紀錄候選 commit;R5 身分 = (word, tl) pair(#7),tl 為 canonical TL 讀音。
    func recordUsage(for word: String, tl: String) {
        Task { [weak self] in
            await self?.repository.recordWord(word, tl: tl)
        }
    }

    func frequency(for word: String) -> Int {
        repository.count(for: word)
    }

    func frequencyData(for word: String) -> FrequencyData {
        repository.frequencyData(for: word)
    }

    func frequencyDataBatch(for words: [String]) -> [FrequencyRow] {
        repository.frequencyDataBatch(for: words)
    }

    func topWords(limit: Int = 100) -> [(word: String, count: Int)] {
        repository.topWords(limit: limit)
    }

    func isConnected() -> Bool {
        repository.isConnected()
    }
}
