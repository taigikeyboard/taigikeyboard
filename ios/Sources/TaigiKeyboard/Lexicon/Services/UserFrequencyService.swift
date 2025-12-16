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
    static func getFrequency(for word: String) -> Int {
        shared.getFrequency(for: word)
    }

    /// 取得詞彙使用頻率資料（包含頻率和最後使用時間）
    static func getFrequencyData(for word: String) -> FrequencyData {
        shared.getFrequencyData(for: word)
    }

    /// 批次取得多個詞彙的頻率資料
    static func getFrequencyDataBatch(for words: [String]) -> [String: FrequencyData] {
        shared.getFrequencyDataBatch(for: words)
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

    func getFrequency(for word: String) -> Int {
        repository.getCount(for: word)
    }

    func getFrequencyData(for word: String) -> FrequencyData {
        repository.getFrequencyData(for: word)
    }

    func getFrequencyDataBatch(for words: [String]) -> [String: FrequencyData] {
        repository.getFrequencyDataBatch(for: words)
    }

    func getTopWords(limit: Int = 100) -> [(word: String, count: Int)] {
        repository.getTopWords(limit: limit)
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
            logger.error("[TEST] Failed to delete database: \(error.localizedDescription)")
        }
    }
    #endif
}
