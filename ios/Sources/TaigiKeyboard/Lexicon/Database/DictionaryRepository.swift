import Foundation
import OSLog
import SQLite3

/// 詞典資料庫 Repository
/// 負責詞典資料的查詢與存取
final class DictionaryRepository: @unchecked Sendable {

    // MARK: - Column Definition

    private enum Column: String {
        case hanzi
        case pojRoman = "poj"
        case pojNoTone = "poj_no_tone"
        case tlRoman = "tl"
        case tlNoTone = "tl_no_tone"
    }

    // MARK: - Properties

    static let shared = DictionaryRepository()

    private let connectionManager: SQLiteConnectionManager
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "DictionaryRepository"
    )

    // MARK: - Initialization

    init(connectionManager: SQLiteConnectionManager? = nil) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: Self.getDatabasePath,
            queueLabel: "com.taigikeyboard.dictionary",
            loggerCategory: "DictionaryRepository"
        )
    }

    // MARK: - Database Path

    private static func getDatabasePath() throws -> String {
        let bundle = Bundle(for: DictionaryRepository.self)
        guard let path = bundle.path(
            forResource: LexiconConstants.Database.fileName,
            ofType: LexiconConstants.Database.fileExtension
        ) else {
            throw DictionaryError.databaseNotFound
        }
        return path
    }

    // MARK: - Query Methods

    /// 查詢詞典
    func query(
        for input: String,
        inputType: InputType,
        inputMode: InputMode,
        limit: Int = LexiconConstants.Search.defaultLimit
    ) async throws -> [TaigiWord] {
        guard !input.isEmpty else {
            return []
        }

        try await connectionManager.ensureInitialized()

        let column = getColumn(for: inputType, inputMode: inputMode)
        let searchText = input.lowercased()

        return try await connectionManager.execute { db in
            try self.performQuery(
                db: db,
                column: column,
                input: searchText,
                inputMode: inputMode,
                limit: limit
            )
        }
    }

    // MARK: - Private Query Implementation

    private func performQuery(
        db: OpaquePointer,
        column: Column,
        input: String,
        inputMode: InputMode,
        limit: Int
    ) throws -> [TaigiWord] {
        // 讀取異用字搜尋設定
        let includeVariants = SharedSettings.shared.variantSearchEnabled

        let sql = buildSQL(column: column, inputMode: inputMode, includeVariants: includeVariants)
        let stmt = try prepareStatement(db: db, sql: sql)
        defer { sqlite3_finalize(stmt) }

        try bindParameters(stmt: stmt, column: column, input: input, limit: limit)
        return try extract(from: stmt, limit: limit)
    }

    /// 建構 SQL 查詢字串
    /// - Parameter includeVariants: 是否包含異用字（true: 搜尋全部, false: 只搜尋原始詞）
    private func buildSQL(column: Column, inputMode: InputMode, includeVariants: Bool) -> String {
        let romanColumn = inputMode == .poj ? "poj" : "tl"
        let maxSyllableCount = 3

        // 異用字過濾條件：關閉時只搜尋 is_variant = 0
        let variantCondition = includeVariants ? "" : "AND is_variant = 0"

        return """
            SELECT id, \(romanColumn), hanzi, syllable_count
            FROM dictionary
            WHERE (REPLACE(\(column.rawValue), '-', '') LIKE ?
               OR \(column.rawValue) LIKE ?)
               AND syllable_count <= \(maxSyllableCount)
               \(variantCondition)
            ORDER BY
                CASE
                    WHEN \(column.rawValue) = ? THEN 0
                    WHEN \(column.rawValue) LIKE ? THEN 1
                    ELSE 2
                END,
                LENGTH(\(romanColumn)) ASC,
                \(romanColumn) ASC
            LIMIT ?;
        """
    }

    private func prepareStatement(db: OpaquePointer, sql: String) throws -> OpaquePointer {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed("Query prep failed: \(errorMsg)")
        }

        guard let statement = stmt else {
            throw DictionaryError.queryPreparationFailed("Statement is nil")
        }
        return statement
    }

    private func bindParameters(
        stmt: OpaquePointer,
        column _: Column,
        input: String,
        limit: Int
    ) throws {
        let normalizedInput = input.replacingOccurrences(of: "-", with: "")
        let normalizedPattern = "\(normalizedInput)%"
        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

        sqlite3_bind_text(stmt, 1, normalizedPattern, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 2, normalizedPattern, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 3, normalizedInput, -1, TRANSIENT)
        sqlite3_bind_text(stmt, 4, normalizedPattern, -1, TRANSIENT)
        sqlite3_bind_int(stmt, 5, Int32(limit))
    }

    private func extract(from stmt: OpaquePointer, limit: Int) throws -> [TaigiWord] {
        var allResults: [TaigiWord] = []
        allResults.reserveCapacity(limit)

        while sqlite3_step(stmt) == SQLITE_ROW, allResults.count < limit {
            let id = Int(sqlite3_column_int(stmt, 0))
            let roman = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let hanziText = sqlite3_column_text(stmt, 2).map(String.init(cString:))
            let hanzi = hanziText?.isEmpty == false ? hanziText : nil
            let lengthScore = Int(sqlite3_column_int(stmt, 3))

            let word = TaigiWord(
                id: id,
                roman: roman,
                hanzi: hanzi,
                lengthScore: lengthScore
            )

            allResults.append(word)
        }

        return allResults
    }

    // MARK: - Helper Methods

    private func getColumn(for inputType: InputType, inputMode: InputMode) -> Column {
        switch inputType {
        case .hanzi:
            .hanzi
        case .romanWithTone:
            inputMode == .poj ? .pojRoman : .tlRoman
        case .romanWithoutTone:
            inputMode == .poj ? .pojNoTone : .tlNoTone
        }
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }
}
