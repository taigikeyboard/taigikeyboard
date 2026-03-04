import Foundation
import OSLog

/// MARISA-trie 查詢服務
///
/// 提供前綴搜尋和完全匹配功能，用於快速查詢詞典索引。
/// 底層使用 C++ MARISA-trie library 透過 C 橋接層存取。
final class TrieService: @unchecked Sendable {

    // MARK: - Constants

    private enum Constants {
        static let trieFileName = "dictionary"
        static let trieFileExtension = "trie"
        static let defaultSearchLimit = 1000
    }

    // MARK: - Properties

    static let shared = TrieService()

    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "TrieService"
    )

    /// 用於保護初始化的序列佇列
    private let initQueue = DispatchQueue(label: "com.taigikeyboard.trie.init")
    private var isInitialized = false

    // MARK: - Initialization

    private init() {}

    // MARK: - Public API

    /// 初始化 trie（從 Bundle 載入）
    /// - Returns: 是否成功載入
    @discardableResult
    func initialize() -> Bool {
        initQueue.sync {
            if isInitialized {
                return true
            }

            guard let path = triePath else {
                logger.error("[INIT] Trie file not found in bundle")
                return false
            }

            let success = trie_load(path)

            if success {
                isInitialized = true
                let keyCount = trie_get_key_count()
                logger.info("[INIT] Trie loaded, keys=\(keyCount)")
            } else {
                logger.error("[INIT] Failed to load trie")
            }

            return success
        }
    }

    /// 前綴搜尋
    /// - Parameters:
    ///   - prefix: 搜尋前綴
    ///   - limit: 最大結果數
    /// - Returns: 匹配的 rowid 列表
    func prefixSearch(_ prefix: String, limit: Int = Constants.defaultSearchLimit) -> [Int] {
        guard isInitialized else {
            logger.warning("[SEARCH] Trie not initialized")
            return []
        }

        guard !prefix.isEmpty else {
            return []
        }

        // 配置結果緩衝區
        var results = [Int32](repeating: 0, count: limit)
        let count = trie_prefix_search(prefix, &results, Int32(limit))

        if count > 0 {
            return results.prefix(Int(count)).map { Int($0) }
        }

        return []
    }

    /// 完全匹配查詢
    /// - Parameter key: 要查詢的 key
    /// - Returns: 匹配的 rowid 列表（一個 key 可能對應多個 rowid）
    func lookup(_ key: String) -> [Int] {
        guard isInitialized else {
            logger.warning("[LOOKUP] Trie not initialized")
            return []
        }

        guard !key.isEmpty else {
            return []
        }

        // 配置結果緩衝區（完全匹配通常結果較少）
        let maxResults = 100
        var results = [Int32](repeating: 0, count: maxResults)
        let count = trie_lookup(key, &results, Int32(maxResults))

        if count > 0 {
            return results.prefix(Int(count)).map { Int($0) }
        }

        return []
    }

    /// 檢查是否已初始化
    var isReady: Bool {
        isInitialized && trie_is_loaded()
    }

    /// 釋放資源
    func close() {
        initQueue.sync {
            if isInitialized {
                trie_close()
                isInitialized = false
                logger.info("[CLOSE] Trie closed")
            }
        }
    }

    // MARK: - Private Methods

    /// 取得 trie 檔案路徑
    private var triePath: String? {
        Bundle(for: type(of: self)).path(
            forResource: Constants.trieFileName,
            ofType: Constants.trieFileExtension
        )
    }
}
