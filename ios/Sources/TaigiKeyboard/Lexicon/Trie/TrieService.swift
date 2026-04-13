import Foundation

/// MARISA-trie 查詢服務
///
/// 提供前綴搜尋和完全匹配功能，用於快速查詢詞典索引。
/// 底層使用 C++ MARISA-trie library 透過 C 橋接層存取。
///
/// 支援多實例：每個 TrieService 實例管理一個獨立的 trie handle。
/// `.shared` 繼續管理 dictionary.trie。
final class TrieService: @unchecked Sendable {
    // MARK: - Properties

    static let shared = TrieService(
        fileName: "dictionary",
        fileExtension: "trie",
        logCategory: "TrieService",
    )

    private let logger: DebugLogger
    private let fileName: String
    private let fileExtension: String

    /// 用於保護 isInitialized / handle 的序列佇列
    private let stateQueue = DispatchQueue(label: "com.taigikeyboard.trie.state")
    private var _isInitialized = false
    private var _handle: trie_handle_t = -1

    // MARK: - Initialization

    init(fileName: String, fileExtension: String, logCategory: String = "TrieService") {
        self.fileName = fileName
        self.fileExtension = fileExtension
        logger = DebugLogger(category: logCategory)
    }

    deinit {
        stateQueue.sync {
            if _isInitialized, _handle >= 0 {
                trie_h_close(_handle)
            }
        }
    }

    // MARK: - State Access

    /// Read current handle atomically; returns -1 if not initialized
    private var currentHandle: trie_handle_t {
        stateQueue.sync { _isInitialized ? _handle : -1 }
    }

    // MARK: - Public API

    /// 初始化 trie（從 Bundle 載入）
    /// - Returns: 是否成功載入
    @discardableResult
    func initialize() -> Bool {
        stateQueue.sync {
            if _isInitialized {
                return true
            }

            guard let path = triePath else {
                logger.error("[INIT] Trie file not found in bundle: \(fileName).\(fileExtension)")
                return false
            }

            let h = trie_create(path)

            if h >= 0 {
                _handle = h
                _isInitialized = true
                let keyCount = trie_h_get_key_count(h)
                logger.info("[INIT] Trie loaded: \(fileName).\(fileExtension), keys=\(keyCount)")
            } else {
                logger.error("[INIT] Failed to load trie: \(fileName).\(fileExtension)")
            }

            return _isInitialized
        }
    }

    /// 前綴搜尋（回傳所有符合結果）
    /// - Parameter prefix: 搜尋前綴
    /// - Returns: 匹配的 rowid 列表
    func prefixSearch(_ prefix: String) -> [Int] {
        let h = currentHandle
        guard h >= 0 else {
            logger.warning("[SEARCH] Trie not initialized")
            return []
        }

        guard !prefix.isEmpty else {
            return []
        }

        let bufferSize = max(Int(trie_h_get_key_count(h)), 1000)
        var results = [Int32](repeating: 0, count: bufferSize)
        let count = trie_h_prefix_search(h, prefix, &results, Int32(bufferSize))

        if count > 0 {
            return results.prefix(Int(count)).map { Int($0) }
        }

        return []
    }

    /// 完全匹配查詢
    /// - Parameter key: 要查詢的 key
    /// - Returns: 匹配的 rowid 列表（一個 key 可能對應多個 rowid）
    func lookup(_ key: String) -> [Int] {
        let h = currentHandle
        guard h >= 0 else {
            logger.warning("[LOOKUP] Trie not initialized")
            return []
        }

        guard !key.isEmpty else {
            return []
        }

        let bufferSize = max(Int(trie_h_get_key_count(h)), 1000)
        var results = [Int32](repeating: 0, count: bufferSize)
        let count = trie_h_lookup(h, key, &results, Int32(bufferSize))

        if count > 0 {
            return results.prefix(Int(count)).map { Int($0) }
        }

        return []
    }

    /// 檢查是否已初始化
    var isReady: Bool {
        let h = currentHandle
        return h >= 0 && trie_h_is_loaded(h)
    }

    /// 釋放資源
    func close() {
        stateQueue.sync {
            if _isInitialized {
                trie_h_close(_handle)
                _handle = -1
                _isInitialized = false
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
