import Foundation
import OSLog

/// 使用者詞頻服務
/// 管理使用者選擇詞彙的頻率
final class UserFrequencyService: @unchecked Sendable {

    // MARK: - Properties

    static let shared = UserFrequencyService()

    private let repository: UserFrequencyRepository
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "UserFrequencyService"
    )

    // MARK: - Initialization

    init(repository: UserFrequencyRepository = .shared) {
        self.repository = repository
    }

    // MARK: - Type Alias

    typealias FrequencyData = UserFrequencyRepository.FrequencyData

    // MARK: - Public API

    /// 記錄詞彙使用
    static func recordUsage(for word: String) {
        shared.recordUsage(for: word)
    }

    /// 取得詞彙使用頻率
    static func frequency(for word: String) -> Int {
        shared.frequency(for: word)
    }

    /// 取得詞彙使用頻率資料（包含頻率和最後使用時間）
    static func frequencyData(for word: String) -> FrequencyData {
        shared.frequencyData(for: word)
    }

    /// 批次取得多個詞彙的頻率資料
    static func frequencyDataBatch(for words: [String]) -> [String: FrequencyData] {
        shared.frequencyDataBatch(for: words)
    }

    /// 檢查是否已連接
    static func isConnected() -> Bool {
        shared.isConnected()
    }

    /// 刪除使用者資料庫
    static func deleteUserDatabase() throws {
        try shared.deleteDatabase()
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

    func deleteDatabase() throws {
        try repository.deleteDatabase()
    }

    // MARK: - Debug Methods

    #if DEBUG
    static func deleteDatabase() {
        shared.deleteDB()
    }

    private func deleteDB() {
        do {
            try repository.deleteDatabase()
        } catch {
            logger.error("[TEST] Failed to delete database: \(error.localizedDescription, privacy: .public)")
        }
    }
    #endif
}
