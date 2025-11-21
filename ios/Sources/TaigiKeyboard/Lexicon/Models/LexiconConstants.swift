import Foundation

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
}
