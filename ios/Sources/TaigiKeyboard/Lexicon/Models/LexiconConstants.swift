// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// 詞典相關常數配置
enum LexiconConstants {
    enum Logging {
        static let subsystem = "com.siansiansu.taigikeyboard"
    }

    enum Search {
        static let defaultLimit = 200
    }

    enum TriePrefix {
        static let tl = "tl:"
        static let poj = "poj:"
        static let hanzi = "hanzi:"

        static func prefix(for mode: InputMode) -> String {
            switch mode {
            case .poj: poj
            case .tl, .english, .tps: tl
            }
        }
    }
}
