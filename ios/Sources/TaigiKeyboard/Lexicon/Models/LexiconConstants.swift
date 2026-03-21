/// 詞典相關常數配置
enum LexiconConstants {
    enum Database {
        static let fileName = "dictionary"
        static let fileExtension = "db"
    }

    enum Logging {
        static let subsystem = "com.siansiansu.taigikeyboard"
    }

    enum Search {
        static let defaultLimit = 100
    }

    enum TriePrefix {
        static let tl = "tl:"
        static let poj = "poj:"

        static func prefix(for mode: InputMode) -> String {
            switch mode {
            case .poj: return poj
            case .tl, .english, .tps: return tl
            }
        }
    }
}
