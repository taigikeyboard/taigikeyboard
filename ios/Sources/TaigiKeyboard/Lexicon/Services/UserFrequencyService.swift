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
    func ensureInitialized() async throws {
        try await repository.ensureInitialized()
    }

    // MARK: - Instance Methods

    func recordUsage(for word: String) {
        Task { [weak self] in
            await self?.repository.recordWord(word)
        }
    }

    func frequency(for word: String) -> Int {
        repository.count(for: word)
    }

    func frequencyData(for word: String) -> FrequencyData {
        repository.frequencyData(for: word)
    }

    func frequencyDataBatch(for words: [String]) -> [String: FrequencyData] {
        repository.frequencyDataBatch(for: words)
    }

    func topWords(limit: Int = 100) -> [(word: String, count: Int)] {
        repository.topWords(limit: limit)
    }

    func isConnected() -> Bool {
        repository.isConnected()
    }
}
