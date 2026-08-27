// The words the user added themselves, and the keys that make them findable
// from any romanization.

import Foundation

/// Why a custom-dictionary write did not happen.
enum CustomDictionaryError: Error, CustomStringConvertible {
    /// The dictionary already holds `maxEntries` words and this would be one
    /// more. Editing an entry that is already there is never refused.
    case capacityReached(limit: Int)
    /// The engine could not derive the search keys for a romanization. The row
    /// is not written: an entry with no side keys is visible in the settings
    /// list and unreachable from the keyboard, which is worse than a refusal.
    case searchKeyDerivationFailed(roman: String)

    var description: String {
        switch self {
        case let .capacityReached(limit): "custom dictionary is full (max \(limit) entries)"
        case let .searchKeyDerivationFailed(roman): "could not derive search keys for \(roman)"
        }
    }
}

/// What a CSV import did.
///
/// `skipped` does not say why — a duplicate, the capacity cap, and a row the
/// parser rejected all land in the same bucket, which is what the iOS and
/// Android import reports say too.
struct CustomDictionaryImportResult: Equatable, Sendable {
    let imported: Int
    let skipped: Int
}

/// The user's own dictionary: reads it on the keystroke path, and writes it
/// when the user asks.
///
/// Stores and imports the user-authored raw `(roman, hanzi)` pair, matching the
/// existing iOS and Android custom-dictionary persistence contract
/// (`docs/engine/custom-dictionary.md:23,85`). The raw roman stays the
/// lattice and presentation payload; the engine derives canonical TL separately
/// when it learns from a commit. Whether that raw pair should become a
/// canonical-TL identity is a question for all three platforms at once — a
/// macOS-only answer would make the same CSV import to a different row count
/// here than on the phones.
///
/// User-writable SQLite stays platform-native by policy
/// (`.claude/rules/rust-migration-policy.md` §6).
final class CustomDictionaryStore: @unchecked Sendable {
    private static let tableName = "custom_dictionary"
    private static let searchKeyTableName = "custom_search_key"
    private static let schemaVersion: Int32 = 3

    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryCapacityPolicy.swift:18
    /// and the Android `CustomDictionaryCapacityPolicy.MAX_ENTRIES`.
    /// Drift changes how many words the same `.taigi` backup restores.
    static let maxEntries = 30000

    /// One transaction per this many accepted rows, so a large import never
    /// holds the write lock for its whole run.
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryRepository.swift:180.
    private static let importChunkSize = 500

    /// What a fresh install can find before the user has added anything. Both
    /// entries mirror iOS (`CustomDictionaryService.swift:22-25`), ids
    /// included, so the same word is the same row on both platforms.
    static let seedEntries: [CustomDictionaryRow] = [
        CustomDictionaryRow(id: "default-gau-tsa", roman: "gâu-tsá", hanzi: "𠢕早"),
        CustomDictionaryRow(id: "default-tsiah-pa-bue", roman: "tsia̍h-pá--buē", hanzi: "食飽未"),
    ]

    private let database: UserDataDatabase
    private let deriveSearchKeys: @Sendable (String) -> [CustomSearchKey]?

    /// The cap this instance enforces. Injectable ONLY so a test can reach the
    /// limit without writing 30000 rows; the shipped value is the
    /// cross-platform constant above.
    private let entryLimit: Int

    /// - Parameter deriveSearchKeys: how a stored roman becomes the keys it is
    ///   findable under. Injected so a test can drive the store without the
    ///   engine, and because the shipped implementation is an FFI round-trip
    ///   that must happen OUTSIDE the write transaction.
    init(
        directory: @escaping @Sendable () throws -> URL,
        deriveSearchKeys: @escaping @Sendable (String) -> [CustomSearchKey]? = {
            RustEngineBridge.deriveCustomSearchKeys(roman: $0)
        },
        entryLimit: Int = CustomDictionaryStore.maxEntries,
    ) {
        database = UserDataDatabase(
            fileName: "custom_dictionary.db",
            name: "CustomDictionaryStore",
            directory: directory,
            schema: Self.applySchema,
        )
        self.deriveSearchKeys = deriveSearchKeys
        self.entryLimit = entryLimit
    }

    var isReady: Bool {
        database.isReady
    }

    func open() {
        database.openInBackground()
    }

    // MARK: - Keystroke path

    /// The entries matching `queryKey`, for the composition being typed.
    ///
    /// Synchronous and best-effort: a store that is not open yet answers `[]`
    /// rather than making the keystroke wait, exactly as the frequency store
    /// does. `limit` matches the iOS call site rather than its repository
    /// default — 20 is what the engine is handed.
    func rows(matching queryKey: CustomSearchKey, limit: Int = 20) -> [CustomDictionaryRow] {
        database.read { connection in
            // `form IN (?, 'abbrev')` lets an abbreviation row satisfy a query
            // in the same family; `DISTINCT` because one entry can own several
            // side rows that all match. `key` is bound verbatim — the engine
            // has already normalised and lowercased it.
            // CROSS-PLATFORM INVARIANT — mirrors
            // ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryRepository.swift:332-367.
            try connection.query(
                """
                SELECT DISTINCT entry.id, entry.roman, entry.hanzi, entry.created_at, entry.updated_at
                FROM \(Self.tableName) AS entry
                JOIN \(Self.searchKeyTableName) AS search_key ON search_key.entry_id = entry.id
                WHERE search_key.family = ?
                  AND search_key.form IN (?, 'abbrev')
                  AND search_key.key LIKE ? || '%'
                ORDER BY entry.roman
                LIMIT ?;
                """,
                [
                    .text(queryKey.family),
                    .text(queryKey.form),
                    .text(queryKey.key),
                    .integer(limit),
                ],
                decoding: Self.decodeRow,
            )
        } ?? []
    }

    // MARK: - User-driven writes

    /// Every entry, newest edit first — the order the settings list shows them
    /// in. Unbounded, for the export that has to write all of them.
    func allRows() async throws -> [CustomDictionaryRow] {
        try await database.perform { connection in
            try connection.query(
                """
                SELECT id, roman, hanzi, created_at, updated_at
                FROM \(Self.tableName)
                ORDER BY updated_at DESC;
                """,
                decoding: Self.decodeRow,
            )
        }
    }

    /// A page of entries for the settings list, newest edit first.
    ///
    /// The filter and the limit are both SQL: a list that read every row and
    /// then dropped all but a hundred would carry a 30000-row dictionary
    /// through memory to show a screenful.
    func rows(filter: String, limit: Int, offset: Int = 0) async throws -> [CustomDictionaryRow] {
        let trimmed = filter.trimmingCharacters(in: .whitespacesAndNewlines)
        return try await database.perform { connection in
            guard !trimmed.isEmpty else {
                return try connection.query(
                    """
                    SELECT id, roman, hanzi, created_at, updated_at
                    FROM \(Self.tableName)
                    ORDER BY updated_at DESC
                    LIMIT ? OFFSET ?;
                    """,
                    [.integer(limit), .integer(offset)],
                    decoding: Self.decodeRow,
                )
            }
            // Both columns, because the user filters by whichever half of the
            // entry they remember. `LIKE` is case-insensitive for ASCII in
            // SQLite, which is the romanization; 漢字 have no case to fold.
            let pattern = "%\(SQLiteConnection.escapedForLike(trimmed))%"
            return try connection.query(
                """
                SELECT id, roman, hanzi, created_at, updated_at
                FROM \(Self.tableName)
                WHERE roman LIKE ? ESCAPE '\\' OR hanzi LIKE ? ESCAPE '\\'
                ORDER BY updated_at DESC
                LIMIT ? OFFSET ?;
                """,
                [.text(pattern), .text(pattern), .integer(limit), .integer(offset)],
                decoding: Self.decodeRow,
            )
        }
    }

    func count() async throws -> Int {
        try await database.perform { connection in
            try Self.entryCount(connection)
        }
    }

    /// How many entries `filter` matches — the number the pager divides into
    /// pages.
    ///
    /// Its own query rather than the length of a fetch: a paged list only ever
    /// holds one page, so it cannot count what it does not hold, and counting
    /// by fetching 17000 rows to measure them would be the very thing paging
    /// exists to avoid.
    ///
    /// The same two predicates `rows(filter:limit:offset:)` filters on, and
    /// they have to stay that way: a count taken under a different WHERE would
    /// page past the end of the list, or stop short of it.
    func count(filter: String) async throws -> Int {
        let trimmed = filter.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return try await count() }
        let pattern = "%\(SQLiteConnection.escapedForLike(trimmed))%"
        return try await database.perform { connection in
            try connection.scalar(
                """
                SELECT COUNT(*)
                FROM \(Self.tableName)
                WHERE roman LIKE ? ESCAPE '\\' OR hanzi LIKE ? ESCAPE '\\';
                """,
                [.text(pattern), .text(pattern)],
            ) ?? 0
        }
    }

    /// Adds `row`, or replaces the one that already carries its id.
    ///
    /// The search keys are derived first, outside the transaction: the FFI
    /// round-trip has no business holding a write lock, and a derivation that
    /// fails must leave the database untouched rather than roll a transaction
    /// back.
    func upsert(_ row: CustomDictionaryRow) async throws {
        let searchKeys = try derivedKeys(for: row.roman)
        let limit = entryLimit
        try await database.perform { connection in
            try connection.withImmediateTransaction {
                // Inside the transaction, so the count cannot go stale between
                // the check and the insert.
                if try !Self.entryExists(connection, id: row.id),
                   try Self.entryCount(connection) >= limit
                {
                    throw CustomDictionaryError.capacityReached(limit: limit)
                }
                try Self.writeRow(connection, row: row, searchKeys: searchKeys)
            }
        }
    }

    /// Removes one entry. `false` means there was nothing with that id.
    @discardableResult
    func delete(id: String) async throws -> Bool {
        try await database.perform { connection in
            try connection.withImmediateTransaction {
                let existed = try Self.entryExists(connection, id: id)
                try connection.run(
                    "DELETE FROM \(Self.tableName) WHERE id = ?;",
                    [.text(id)],
                )
                try Self.deleteSearchKeys(connection, entryID: id)
                return existed
            }
        }
    }

    /// Empties the dictionary and reports how many entries went.
    @discardableResult
    func deleteAll() async throws -> Int {
        try await database.perform { connection in
            let removed = try connection.withImmediateTransaction { () -> Int in
                let existing = try Self.entryCount(connection)
                try connection.execute("DELETE FROM \(Self.tableName);")
                try connection.execute("DELETE FROM \(Self.searchKeyTableName);")
                return existing
            }
            // Outside the transaction, and best-effort: VACUUM needs exclusive
            // access, and a file that stays large is not a failed clear.
            try? connection.execute("VACUUM;")
            return removed
        }
    }

    /// Rebuilds every entry's search keys when the stored database predates the
    /// current key derivation, then records the new shape.
    ///
    /// Search keys are DERIVED: a correction to how a roman becomes a key
    /// leaves rows already on disk findable only under the old, wrong key. The
    /// v2 → v3 case is the POJ spelling fix — `o͘` (U+0358) used to be dropped
    /// from a key as if it were a tone diacritic, and the nasal ⁿ survived into
    /// the tone-aware key as a display glyph, while the query key carries the
    /// ASCII `oo` / `nn` a keyboard types. Every POJ entry containing either
    /// was unreachable until it is re-derived.
    ///
    /// Mirrors iOS `CustomDictionaryMigrator` (v3) and Android's v6 → v7. An
    /// entry whose roman will not derive keeps the keys it has rather than
    /// losing them: one unparseable row must not decide the fate of the rest.
    func rederiveSearchKeysIfNeeded() async throws {
        // The version and the rows it describes are read together: a second hop
        // for the guard would queue behind whatever else the launch is doing.
        let stored = try await database.perform { connection -> [(id: String, roman: String)]? in
            guard connection.userVersion < Self.schemaVersion else { return nil }
            return try Self.entryRomans(connection)
        }
        guard let stored else { return }

        // Derived before the transaction opens, exactly as `batchImport` does:
        // an FFI round-trip has no business holding the write lock.
        var derivedByRoman: [String: [CustomSearchKey]] = [:]
        for entry in stored where derivedByRoman[entry.roman] == nil {
            guard let keys = deriveSearchKeys(entry.roman), !keys.isEmpty else { continue }
            derivedByRoman[entry.roman] = keys
        }
        let rederived = stored.compactMap { entry in
            derivedByRoman[entry.roman].map { (id: entry.id, roman: entry.roman, searchKeys: $0) }
        }

        try await database.perform { connection in
            try connection.withImmediateTransaction {
                // Re-read inside the transaction: between the snapshot above and
                // this write the user may have edited or deleted an entry, and
                // that write already left current keys behind. Writing the
                // snapshot's keys over those would re-file the word under a
                // romanization it no longer has.
                let current = Dictionary(
                    try Self.entryRomans(connection).map { ($0.id, $0.roman) },
                    uniquingKeysWith: { first, _ in first },
                )
                for entry in rederived where current[entry.id] == entry.roman {
                    try Self.replaceSearchKeys(
                        connection,
                        entryID: entry.id,
                        searchKeys: entry.searchKeys,
                    )
                }
                connection.userVersion = Self.schemaVersion
            }
        }
    }

    /// Writes the seed entries, but only into a dictionary nobody has touched.
    ///
    /// All-or-nothing on emptiness, like iOS: deleting one seed and relaunching
    /// must not bring it back. The emptiness check and both writes share one
    /// transaction, so a word the user adds while this is running cannot end
    /// up alongside half a seed.
    func seedIfEmpty() async throws {
        let seeds = try Self.seedEntries.map { entry in
            try (row: entry, searchKeys: derivedKeys(for: entry.roman))
        }
        try await database.perform { connection in
            try connection.withImmediateTransaction {
                guard try Self.entryCount(connection) == 0 else { return }
                for seed in seeds {
                    try Self.writeRow(connection, row: seed.row, searchKeys: seed.searchKeys)
                }
            }
        }
    }

    /// Imports parsed rows, skipping the ones already stored and stopping at
    /// the capacity cap.
    ///
    /// Reaching the cap partway through is not an error: what fitted is kept,
    /// and the rest is reported as skipped. A file whose own row count is over
    /// the cap is refused before anything is written.
    ///
    /// Both the cap and the duplicate check happen INSIDE each chunk's
    /// transaction, against the row count and the rows that are there at that
    /// moment. Deciding either up front would be deciding it against a
    /// database that a later chunk no longer describes: every `await` hands
    /// the queue back, and an entry the user adds by hand in between would
    /// make the headroom this import is spending a number that was true
    /// before, not now.
    func batchImport(_ rows: [CustomDictionaryRow]) async throws -> CustomDictionaryImportResult {
        let limit = entryLimit
        guard rows.count <= limit else {
            throw CustomDictionaryError.capacityReached(limit: limit)
        }
        guard !rows.isEmpty else { return CustomDictionaryImportResult(imported: 0, skipped: 0) }

        // Deduping the FILE against itself needs no database read, so it costs
        // nothing to do here — and it keeps the derivation below from running
        // twice for a row the import would drop anyway.
        var seenInFile = Set<CustomDictionaryIdentity>()
        let candidates = rows.filter { seenInFile.insert($0.identity).inserted }

        // Derived before any transaction opens: an FFI round-trip has no
        // business holding a write lock, and a failure has to happen before
        // the first row lands rather than halfway through the file.
        var derivedByRoman: [String: [CustomSearchKey]] = [:]
        for row in candidates where derivedByRoman[row.roman] == nil {
            derivedByRoman[row.roman] = try derivedKeys(for: row.roman)
        }

        var imported = 0
        for chunkStart in stride(from: 0, to: candidates.count, by: Self.importChunkSize) {
            let chunkEnd = min(chunkStart + Self.importChunkSize, candidates.count)
            // Paired here rather than looked up inside the queued block: the
            // cache is a `var` this loop keeps mutating, and a closure that
            // captured it would be reading it from another thread.
            let chunk = candidates[chunkStart ..< chunkEnd].map { row in
                (row: row, searchKeys: derivedByRoman[row.roman] ?? [])
            }
            imported += try await database.perform { connection in
                try connection.withImmediateTransaction {
                    var storedCount = try Self.entryCount(connection)
                    var written = 0
                    for entry in chunk {
                        guard storedCount < limit else { break }
                        guard try !Self.rowExists(connection, identity: entry.row.identity) else {
                            continue
                        }
                        try Self.writeRow(
                            connection,
                            row: entry.row,
                            searchKeys: entry.searchKeys,
                        )
                        storedCount += 1
                        written += 1
                    }
                    return written
                }
            }
        }
        return CustomDictionaryImportResult(imported: imported, skipped: rows.count - imported)
    }

    // MARK: - Private

    private func derivedKeys(for roman: String) throws -> [CustomSearchKey] {
        guard let keys = deriveSearchKeys(roman), !keys.isEmpty else {
            throw CustomDictionaryError.searchKeyDerivationFailed(roman: roman)
        }
        return keys
    }

    private static func rowExists(
        _ connection: SQLiteConnection,
        identity: CustomDictionaryIdentity,
    ) throws -> Bool {
        try connection.scalar(
            "SELECT 1 FROM \(tableName) WHERE roman = ? AND hanzi = ? LIMIT 1;",
            [.text(identity.roman), .text(identity.hanzi)],
        ) != nil
    }

    /// The entry and its search keys, written together. Must be called inside a
    /// transaction: a row whose side keys did not land is invisible to the
    /// keyboard while looking present in the settings list.
    private static func writeRow(
        _ connection: SQLiteConnection,
        row: CustomDictionaryRow,
        searchKeys: [CustomSearchKey],
    ) throws {
        try connection.run(
            """
            INSERT INTO \(tableName) (id, roman, hanzi, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                roman = excluded.roman,
                hanzi = excluded.hanzi,
                updated_at = excluded.updated_at;
            """,
            [
                .text(row.id),
                .text(row.roman),
                .text(row.hanzi),
                .text(timestampFormatter.string(from: row.createdAt)),
                .text(timestampFormatter.string(from: row.updatedAt)),
            ],
        )
        // Replace rather than add: an edited roman must not stay findable
        // under the keys of the roman it replaced.
        try replaceSearchKeys(connection, entryID: row.id, searchKeys: searchKeys)
    }

    /// One entry's side-table rows, swapped for the ones passed in. Must be
    /// called inside a transaction — between the delete and the inserts the
    /// entry is findable under nothing.
    private static func replaceSearchKeys(
        _ connection: SQLiteConnection,
        entryID: String,
        searchKeys: [CustomSearchKey],
    ) throws {
        try deleteSearchKeys(connection, entryID: entryID)
        for searchKey in searchKeys {
            try connection.run(
                """
                INSERT INTO \(searchKeyTableName) (entry_id, family, form, key)
                VALUES (?, ?, ?, ?);
                """,
                [
                    .text(entryID),
                    .text(searchKey.family),
                    .text(searchKey.form),
                    .text(searchKey.key),
                ],
            )
        }
    }

    private static func deleteSearchKeys(_ connection: SQLiteConnection, entryID: String) throws {
        try connection.run(
            "DELETE FROM \(searchKeyTableName) WHERE entry_id = ?;",
            [.text(entryID)],
        )
    }

    /// Every entry as `(id, roman)`, unordered — what the key re-derivation
    /// needs, without the columns and the sort `allRows` owes the settings list.
    private static func entryRomans(
        _ connection: SQLiteConnection,
    ) throws -> [(id: String, roman: String)] {
        try connection.query("SELECT id, roman FROM \(tableName);") {
            (id: $0.text(0), roman: $0.text(1))
        }
    }

    private static func entryCount(_ connection: SQLiteConnection) throws -> Int {
        try connection.scalar("SELECT COUNT(*) FROM \(tableName);") ?? 0
    }

    private static func entryExists(_ connection: SQLiteConnection, id: String) throws -> Bool {
        try connection.scalar(
            "SELECT 1 FROM \(tableName) WHERE id = ? LIMIT 1;",
            [.text(id)],
        ) != nil
    }

    private static func decodeRow(_ row: SQLiteRowReader) -> CustomDictionaryRow {
        CustomDictionaryRow(
            id: row.text(0),
            roman: row.text(1),
            hanzi: row.text(2),
            createdAt: timestampFormatter.date(from: row.text(3)) ?? Date(),
            updatedAt: timestampFormatter.date(from: row.text(4)) ?? Date(),
        )
    }

    /// UTC and POSIX-fixed, so the stored text sorts the way the query orders
    /// by it regardless of the user's locale or time zone.
    private static let timestampFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd HH:mm:ss"
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.locale = Locale(identifier: "en_US_POSIX")
        return formatter
    }()

    // MARK: - Schema

    /// The current shape, created directly.
    ///
    /// `user_version` is 2 to match the iOS schema this mirrors, even though
    /// macOS has never shipped a version 1 and carries no migrator: the number
    /// names the SHAPE, and a store that claimed version 1 would be claiming a
    /// shape it does not have.
    private static let applySchema: @Sendable (SQLiteConnection) throws -> Void = { connection in
        try connection.execute(
            """
            CREATE TABLE IF NOT EXISTS \(tableName) (
                id TEXT PRIMARY KEY,
                roman TEXT NOT NULL,
                hanzi TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_custom_roman ON \(tableName)(roman);
            CREATE TABLE IF NOT EXISTS \(searchKeyTableName) (
                entry_id TEXT NOT NULL,
                family   TEXT NOT NULL,
                form     TEXT NOT NULL,
                key      TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_csk_lookup ON \(searchKeyTableName)(family, form, key);
            CREATE INDEX IF NOT EXISTS idx_csk_entry ON \(searchKeyTableName)(entry_id);
            """,
        )
        // The iOS table also carries `notone` / `abbrev` / `roman_num`
        // columns. They are write-only there — the query path has used the
        // side table since schema 2 — so macOS does not create them rather
        // than create columns nothing will ever read.
        //
        // Deliberately does NOT stamp `user_version`: creating tables says
        // nothing about whether the ROWS in them were derived by the current
        // logic. `rederiveSearchKeysIfNeeded` is what records the shape, once
        // it has made the data match it — a stamp here would tell that check
        // a database it has never looked at is already up to date.
    }
}
