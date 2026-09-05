import Foundation

/// User-frequency service — a thin wrapper over `UserFrequencyRepository` for the candidate pipeline.
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

    /// Record a candidate commit. R5: the `(word, tl)` pair is the identity
    /// (Core Principle #7) — `tl` is the candidate's canonical-TL reading so
    /// 一字多音 keep separate counts. Pass `""` only when the candidate has
    /// no canonical TL (wire skew / TPS-OOV) → the legacy fallback bucket.
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

    func isConnected() -> Bool {
        repository.isConnected()
    }
}
