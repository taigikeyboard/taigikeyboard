// How often the user has committed each word, and how recently.

import Foundation

/// One learned row: a word, the reading it was learned under, and its usage.
///
/// A word is identified by the `(漢字, canonical TL)` PAIR (`CLAUDE.md` Core
/// Principle #7), which is why `tl` is part of the row rather than a label on
/// it: 重/tîng (重複) and 重/tāng (重量) are different words, and committing one
/// must not promote the other.
struct FrequencyRow: Equatable, Sendable {
    let word: String
    let tl: String
    let count: Int
    let lastUsedMillis: Int64
}

/// Records candidate commits and answers what the ranker should boost.
///
/// The engine does the ranking: this store only hands it counts through
/// `FetchAtPos.frequency_entries`, and the boost curve lives in
/// `engine/ranking/src/score.rs`. Keeping the policy there is what stops macOS
/// from ranking a word differently than iOS and Android do from the same data.
///
/// User-writable SQLite stays platform-native by policy
/// (`.claude/rules/rust-migration-policy.md` §6).
final class UserFrequencyStore: @unchecked Sendable {
    private static let tableName = "user_frequency"
    private static let schemaVersion: Int32 = 2

    private let database: UserDataDatabase

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Lexicon/Database/UserFrequencyPruner.swift:26-38.
    /// Drift changes how much history a long-running install keeps.
    /// A function, not a stored constant: `LearningCapacity` is a reference
    /// type holding a mutable throttle counter, and a default argument is
    /// re-evaluated per call. A stored constant would hand every instance of
    /// this store the same counter, so two of them — a test fixture beside the
    /// live one — would push each other towards a capacity check.
    static func shippedCapacity() -> LearningCapacity {
        LearningCapacity(table: tableName, maxRows: 20000, deleteBatch: 2000)
    }

    private let capacity: LearningCapacity

    /// `capacity` is a parameter so a test can hand in a small cap and watch a
    /// real `record` call prune — the shipped 20000 is not reachable in a test,
    /// and the pane that used to make the cap observable is gone.
    init(
        directory: @escaping @Sendable () throws -> URL,
        capacity: LearningCapacity = UserFrequencyStore.shippedCapacity(),
    ) {
        self.capacity = capacity
        database = UserDataDatabase(
            fileName: "user_frequency.db",
            name: "UserFrequencyStore",
            directory: directory,
            schema: Self.applySchema,
        )
    }

    var isReady: Bool {
        database.isReady
    }

    func open() {
        database.openInBackground()
    }

    /// Counts one commit of `word` under reading `tl`.
    ///
    /// `tl` may legitimately be empty — a candidate with no canonical reading —
    /// and lands in its own row rather than being merged into a real reading's,
    /// matching the tolerant lookup the engine performs.
    func record(word: String, tl: String) {
        guard !word.isEmpty else { return }
        let shouldCheckCapacity = capacity.shouldEnforce(after: 1)

        database.write { connection in
            try connection.run(
                """
                INSERT INTO \(Self.tableName) (word, tl, count, last_used)
                VALUES (?, ?, 1, CURRENT_TIMESTAMP)
                ON CONFLICT(word, tl) DO UPDATE SET
                    count = count + 1,
                    last_used = CURRENT_TIMESTAMP;
                """,
                [.text(word), .text(tl)],
            )
            guard shouldCheckCapacity else { return }
            try self.capacity.enforce(connection)
        }
    }

    /// Every learned row for any of `words`, or `nil` when the store could not
    /// be read. A word with several learned readings yields several rows; the
    /// engine keys them by `(display text, canonical TL)` and resolves them
    /// itself.
    func rows(forWords words: [String]) -> [FrequencyRow]? {
        guard !words.isEmpty else { return [] }
        let placeholders = Array(repeating: "?", count: words.count).joined(separator: ",")
        return database.read { connection in
            try connection.query(
                """
                SELECT \(Self.rowColumns) FROM \(Self.tableName)
                WHERE word IN (\(placeholders));
                """,
                words.map { .text($0) },
                decoding: Self.decodeRow,
            )
        }
    }

    /// Every learned row, most-used first, or `nil` when the store could not
    /// be read. Mirrors `UserAssociationStore.allRows()`, and like it exists
    /// for the tests: nothing in the shipped UI lists learned rows any more,
    /// but the capacity ceiling and `deleteAll` are only assertable against the
    /// table's actual contents.
    func allRows() -> [FrequencyRow]? {
        database.read { connection in
            try connection.query(
                """
                SELECT \(Self.rowColumns) FROM \(Self.tableName)
                ORDER BY \(Self.listOrder);
                """,
                decoding: Self.decodeRow,
            )
        }
    }

    /// The columns every `FrequencyRow` read selects, next to the decoder that
    /// reads them: the two agree by position, and only stay agreed while they
    /// are edited together.
    private static let rowColumns =
        "word, tl, count, CAST(strftime('%s', last_used) AS INTEGER) * 1000"

    /// One order, most-used first, with `(word, tl)` as the final tie-break so
    /// equal counts keep a stable order between two reads.
    private static let listOrder = "count DESC, last_used DESC, word ASC, tl ASC"

    private static func decodeRow(_ row: SQLiteRowReader) -> FrequencyRow {
        FrequencyRow(
            word: row.text(0),
            tl: row.text(1),
            count: row.integer(2),
            lastUsedMillis: row.integer64(3),
        )
    }

    // MARK: - User-driven administration

    /// Forgets everything, and reports how many rows went.
    @discardableResult
    func deleteAll() async throws -> Int {
        try await database.perform { connection in
            let existing = try connection.scalar("SELECT COUNT(*) FROM \(Self.tableName);") ?? 0
            try connection.execute("DELETE FROM \(Self.tableName);")
            try? connection.execute("VACUUM;")
            return existing
        }
    }

    // MARK: - Schema

    /// The current shape, created directly.
    ///
    /// iOS carries a migrator from a single-`word` unique key to this pair key
    /// (`UserFrequencySchema.swift:56-101`); macOS has never shipped the older
    /// shape, so there is nothing to migrate and porting the rebuild would be
    /// dead code guarding a state this platform cannot be in.
    private static let applySchema: @Sendable (SQLiteConnection) throws -> Void = { connection in
        try connection.execute(
            """
            CREATE TABLE IF NOT EXISTS \(tableName) (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL,
                tl TEXT NOT NULL DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(word, tl)
            );
            CREATE INDEX IF NOT EXISTS idx_count ON \(tableName)(count DESC);
            CREATE INDEX IF NOT EXISTS idx_last_used ON \(tableName)(last_used DESC);
            """,
        )
        // No index on `word` alone: the UNIQUE constraint's automatic index
        // already has it as the leftmost column, so it serves `WHERE word IN`.
        connection.userVersion = schemaVersion
    }
}
