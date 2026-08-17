// Which word the user tends to type after which.

import Foundation

/// A learned bigram. Both halves carry their canonical TL because a Taiwanese
/// word is the `(漢字, canonical TL)` pair (`CLAUDE.md` Core Principle #7) —
/// 重/tîng followed by 複 is not the same observation as 重/tāng followed by 複.
struct AssociationPair: Equatable, Sendable {
    let previous: String
    let previousTl: String
    let next: String
    let nextTl: String
}

/// A stored bigram and how many times it has been seen.
struct AssociationRow: Equatable, Sendable {
    let pair: AssociationPair
    let count: Int
}

/// Records the bigrams the engine decides are worth learning.
///
/// Write-only for now, and deliberately so: macOS shows no next-word
/// predictions, because the only surface that could carry them is the candidate
/// panel, and putting it on screen while the user is NOT composing would mean
/// intercepting keys in a state where this input method currently forwards
/// everything to the host — the one change least safe to make on a platform
/// whose key handling has not yet been exercised on a real keyboard. What this
/// store buys today is that when that surface does land, it starts with the
/// user's real history instead of an empty table.
///
/// User-writable SQLite stays platform-native by policy
/// (`.claude/rules/rust-migration-policy.md` §6).
final class UserAssociationStore: @unchecked Sendable {
    private static let tableName = "user_association"
    /// CROSS-PLATFORM INVARIANT — mirrors iOS `NextWordSchema.schemaVersion`
    /// and Android `NextWordService.DATABASE_VERSION`. Drift causes silent
    /// divergence.
    private static let schemaVersion: Int32 = 6

    private let database: UserDataDatabase

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/NextWord/Services/NextWordService.swift:31-33.
    private let capacity = LearningCapacity(table: tableName, maxRows: 50000, deleteBatch: 5000)

    init(directory: @escaping @Sendable () throws -> URL) {
        database = UserDataDatabase(
            fileName: "user_association.db",
            name: "UserAssociationStore",
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

    /// Records `pairs` in order, in one transaction.
    ///
    /// Order matters because the engine emits a compound word's bigrams in
    /// reading order and they collide on the same UNIQUE key when a word
    /// repeats; writing them sequentially is what makes the counts land in a
    /// defined state. One transaction rather than one per pair because a
    /// compound commit is a single user action, and paying a separate durable
    /// write per bigram is what a commit of a long phrase would feel like.
    func record(_ pairs: [AssociationPair]) {
        let recordable = pairs.filter { !$0.previous.isEmpty && !$0.next.isEmpty }
        guard !recordable.isEmpty else { return }
        let shouldCheckCapacity = capacity.shouldEnforce(after: recordable.count)

        database.write { connection in
            try connection.withImmediateTransaction {
                for pair in recordable {
                    try connection.run(
                        """
                        INSERT INTO \(Self.tableName)
                            (prev_word, prev_tl, next_word, next_tl, count, last_used)
                        VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
                        ON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET
                            count = count + 1,
                            last_used = CURRENT_TIMESTAMP;
                        """,
                        [
                            .text(pair.previous),
                            .text(pair.previousTl),
                            .text(pair.next),
                            .text(pair.nextTl),
                        ],
                    )
                }
                if shouldCheckCapacity {
                    try self.capacity.enforce(connection)
                }
            }
        }
    }

    /// Every learned row with its count, most-used first. Synchronous, and the
    /// answer is `nil` rather than `[]` when the store could not be read.
    ///
    /// Kept alongside the async `rows()` because the tests that prove this
    /// store's writes run without an `await`, and because a caller inside the
    /// queue would deadlock on the async one.
    func allRows() -> [AssociationRow]? {
        database.read { connection in
            try Self.selectAllRows(connection)
        }
    }

    // MARK: - User-driven administration

    /// Every learned row, for the viewer and the export.
    func rows() async throws -> [AssociationRow] {
        try await database.perform { connection in
            try Self.selectAllRows(connection)
        }
    }

    /// Forgets one bigram. All four identity columns are matched: a bigram is
    /// keyed on both words AND both readings, so deleting by the two 漢字 alone
    /// would take out readings the user never asked about.
    @discardableResult
    func delete(_ pair: AssociationPair) async throws -> Bool {
        try await database.perform { connection in
            let bindings: [SQLiteBinding] = [
                .text(pair.previous),
                .text(pair.previousTl),
                .text(pair.next),
                .text(pair.nextTl),
            ]
            let existed = try connection.scalar(
                """
                SELECT 1 FROM \(Self.tableName)
                WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?
                LIMIT 1;
                """,
                bindings,
            ) != nil
            try connection.run(
                """
                DELETE FROM \(Self.tableName)
                WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?;
                """,
                bindings,
            )
            return existed
        }
    }

    /// Forgets every bigram, and reports how many went.
    @discardableResult
    func deleteAll() async throws -> Int {
        try await database.perform { connection in
            let existing = try connection.scalar("SELECT COUNT(*) FROM \(Self.tableName);") ?? 0
            try connection.execute("DELETE FROM \(Self.tableName);")
            try? connection.execute("VACUUM;")
            return existing
        }
    }

    /// Merges imported counts in, keeping whichever is higher — the same
    /// merge-by-max rule the frequency store uses, and for the same reason:
    /// restoring a backup must not walk a learned count backwards.
    @discardableResult
    func batchImportMerge(_ rows: [AssociationRow]) async throws -> Int {
        guard !rows.isEmpty else { return 0 }
        return try await database.perform { connection in
            try connection.withImmediateTransaction {
                for row in rows {
                    try connection.run(
                        """
                        INSERT INTO \(Self.tableName)
                            (prev_word, prev_tl, next_word, next_tl, count, last_used)
                        VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                        ON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET
                            count = MAX(count, excluded.count),
                            last_used = CURRENT_TIMESTAMP;
                        """,
                        [
                            .text(row.pair.previous),
                            .text(row.pair.previousTl),
                            .text(row.pair.next),
                            .text(row.pair.nextTl),
                            .integer(row.count),
                        ],
                    )
                }
                return rows.count
            }
        }
    }

    private static func selectAllRows(_ connection: SQLiteConnection) throws -> [AssociationRow] {
        try connection.query(
            """
            SELECT prev_word, prev_tl, next_word, next_tl, count FROM \(tableName)
            ORDER BY count DESC, last_used DESC, prev_word ASC, next_word ASC;
            """,
        ) { row in
            AssociationRow(
                pair: AssociationPair(
                    previous: row.text(0),
                    previousTl: row.text(1),
                    next: row.text(2),
                    nextTl: row.text(3),
                ),
                count: row.integer(4),
            )
        }
    }

    // MARK: - Schema

    /// The unique key carries `prev_tl` because a Taiwanese word is the
    /// `(漢字, canonical TL)` pair (`CLAUDE.md` Core Principle #7) on the
    /// bigram's PREVIOUS side as well as its next: without it, 重/tîng → 複 and
    /// 重/tāng → 複 collapse into one row whose `prev_tl` is whichever was
    /// written last, and the read side ranks on `prev_tl`, so the merged row
    /// answers for a reading it was not learned under.
    ///
    /// This was a NAMED CROSS-PLATFORM DIVERGENCE while iOS and Android sat at
    /// v5; both now ship the same v6 key, so the three schemas agree. See
    /// `docs/architecture/behavioral-invariants.md` §24 and
    /// `ios/…/NextWord/Repository/NextWordSchema.swift`.
    ///
    /// The `DROP INDEX` is the one migration this store does: builds before
    /// that convergence named the same `(prev_word, prev_tl)` index
    /// `idx_user_prev`, so a database written by one carries the old name until
    /// its next open. Nothing else here needs migrating — the table shape has
    /// never changed on macOS.
    private static let applySchema: @Sendable (SQLiteConnection) throws -> Void = { connection in
        try connection.execute(
            """
            CREATE TABLE IF NOT EXISTS \(tableName) (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                prev_tl TEXT DEFAULT '',
                next_word TEXT NOT NULL,
                next_tl TEXT DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, prev_tl, next_word, next_tl)
            );
            DROP INDEX IF EXISTS idx_user_prev;
            CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl
                ON \(tableName)(prev_word, prev_tl);
            """,
        )
        connection.userVersion = schemaVersion
    }
}
