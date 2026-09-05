// 詞典模組的常數設定 — log subsystem。

import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// 詞典相關常數配置
enum LexiconConstants {
    enum Logging {
        static let subsystem = "com.siansiansu.taigikeyboard"
    }
}
