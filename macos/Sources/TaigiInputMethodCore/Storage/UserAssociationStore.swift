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
    /// One past the iOS / Android number, because the shape is one column wider
    /// in its key — see `applySchema`.
    private static let schemaVersion: Int32 = 6

    private let database: LearningDatabase

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/NextWord/Services/NextWordService.swift:31-33.
    private let capacity = LearningCapacity(table: tableName, maxRows: 50000, deleteBatch: 5000)

    init(directory: @escaping @Sendable () throws -> URL) {
        database = LearningDatabase(
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
            try connection.execute("BEGIN IMMEDIATE;")
            do {
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
                // Inside the `do`: a failing COMMIT leaves the transaction open,
                // and every later `BEGIN IMMEDIATE` would then fail against it —
                // one bad commit would take the store out for the rest of the
                // process's life.
                try connection.execute("COMMIT;")
            } catch {
                // Harmless when no transaction is active; the alternative is
                // asking SQLite whether one is, which answers the same question
                // twice.
                try? connection.execute("ROLLBACK;")
                throw error
            }
        }
    }

    /// Every learned row with its count, most-used first. Nothing in the
    /// shipped input method calls this — it exists so the store's writes are
    /// provable by test rather than only by inspecting the file by hand.
    func allRows() -> [AssociationRow]? {
        database.read { connection in
            try connection.query(
                """
                SELECT prev_word, prev_tl, next_word, next_tl, count FROM \(Self.tableName)
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
    }

    // MARK: - Schema

    /// NAMED CROSS-PLATFORM DIVERGENCE (`cross-platform-alignment.md` §3,
    /// intentional): the unique key carries `prev_tl`, which the iOS and
    /// Android v5 tables leave out
    /// (`ios/…/NextWord/Repository/NextWordSchema.swift:37-50`).
    ///
    /// Without it, 重/tîng → 複 and 重/tāng → 複 collapse into one row whose
    /// `prev_tl` is whichever was written last, which contradicts the rule that
    /// a Taiwanese word is the `(漢字, canonical TL)` pair (`CLAUDE.md` Core
    /// Principle #7) — and the read side ranks on `prev_tl`, so the merged row
    /// answers for a reading it was not learned under.
    ///
    /// macOS is a fresh install with no rows to migrate, and the only reason
    /// this table is being written at all is to hand a future prediction
    /// surface real history; recording that history under a key that conflates
    /// two morphemes would spend the cost and lose the benefit. The same gap on
    /// iOS and Android is real and is NOT fixed here: correcting it there means
    /// a schema migration over data users already have, which is a decision of
    /// its own.
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
            CREATE INDEX IF NOT EXISTS idx_user_prev
                ON \(tableName)(prev_word, prev_tl);
            """,
        )
        connection.userVersion = schemaVersion
    }
}
