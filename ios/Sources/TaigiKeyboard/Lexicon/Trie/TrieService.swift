import Foundation

/// MARISA-trie 查詢服務
///
/// 提供前綴搜尋和完全匹配功能，用於快速查詢詞典索引。
/// 底層使用 C++ MARISA-trie library 透過 C 橋接層存取。
///
/// 支援多實例：每個 TrieService 實例管理一個獨立的 trie handle。
/// `.shared` 繼續管理 dictionary.trie（使用舊 global API 維持相容）。
final class TrieService: @unchecked Sendable {
    // MARK: - Constants

    private enum Constants {
        static let defaultSearchLimit = 1000
    }

    // MARK: - Properties

    static let shared = TrieService(
        fileName: "dictionary",
        fileExtension: "trie",
        logCategory: "TrieService",
    )

    private let logger: DebugLogger
    private let fileName: String
    private let fileExtension: String

    /// 用於保護初始化的序列佇列
    private let initQueue = DispatchQueue(label: "com.taigikeyboard.trie.init")
    private var isInitialized = false

    /// Handle-based trie（-1 = 未載入）
    private var handle: trie_handle_t = -1

    // MARK: - Initialization

    init(fileName: String, fileExtension: String, logCategory: String = "TrieService") {
        self.fileName = fileName
        self.fileExtension = fileExtension
        logger = DebugLogger(category: logCategory)
    }

    deinit {
        if isInitialized, handle >= 0 {
            trie_h_close(handle)
        }
    }

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
                logger.error("[INIT] Trie file not found in bundle: \(fileName).\(fileExtension)")
                return false
            }

            let h = trie_create(path)

            if h >= 0 {
                handle = h
                isInitialized = true
                let keyCount = trie_h_get_key_count(h)
                logger.info("[INIT] Trie loaded: \(fileName).\(fileExtension), keys=\(keyCount)")
            } else {
                logger.error("[INIT] Failed to load trie: \(fileName).\(fileExtension)")
            }

            return isInitialized
        }
    }

    /// 前綴搜尋
    /// - Parameters:
    ///   - prefix: 搜尋前綴
    ///   - limit: 最大結果數
    /// - Returns: 匹配的 rowid 列表
    func prefixSearch(_ prefix: String, limit: Int = Constants.defaultSearchLimit) -> [Int] {
        guard isInitialized, handle >= 0 else {
            logger.warning("[SEARCH] Trie not initialized")
            return []
        }

        guard !prefix.isEmpty else {
            return []
        }

        var results = [Int32](repeating: 0, count: limit)
        let count = trie_h_prefix_search(handle, prefix, &results, Int32(limit))

        if count > 0 {
            return results.prefix(Int(count)).map { Int($0) }
        }

        return []
    }

    /// 完全匹配查詢
    /// - Parameter key: 要查詢的 key
    /// - Returns: 匹配的 rowid 列表（一個 key 可能對應多個 rowid）
    func lookup(_ key: String) -> [Int] {
        guard isInitialized, handle >= 0 else {
            logger.warning("[LOOKUP] Trie not initialized")
            return []
        }

        guard !key.isEmpty else {
            return []
        }

        let maxResults = 1000
        var results = [Int32](repeating: 0, count: maxResults)
        let count = trie_h_lookup(handle, key, &results, Int32(maxResults))

        if count > 0 {
            return results.prefix(Int(count)).map { Int($0) }
        }

        return []
    }

    /// 檢查是否已初始化
    var isReady: Bool {
        isInitialized && trie_h_is_loaded(handle)
    }

    /// 釋放資源
    func close() {
        initQueue.sync {
            if isInitialized {
                trie_h_close(handle)
                handle = -1
                isInitialized = false
                logger.info("[CLOSE] Trie closed: \(fileName).\(fileExtension)")
            }
        }
    }

    // MARK: - Private Methods

    /// 取得 trie 檔案路徑
    private var triePath: String? {
        ResourceBundleResolver.dictionaryBundle.path(
            forResource: fileName,
            ofType: fileExtension,
        )
    }
}
