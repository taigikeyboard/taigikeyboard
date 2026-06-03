@testable import TaigiKeyboard
import SQLite3
import XCTest

/// v3.6.1 R3 — `custom_dictionary.db` cross-input-mode search via the
/// `custom_search_key` side table.
///
/// Pins `INVARIANT_CUSTOM_DICT_CROSS_MODE`: an entry stored in ONE input mode
/// is findable from ANY input mode. The repository writes the full
/// {family × form} key bundle (`RustEngineBridge.deriveCustomSearchKeys`) on
/// upsert; the query derives a single family-native key
/// (`RustEngineBridge.deriveCustomQueryKey`) and joins it against the side
/// table.
///
/// Exercises the real repository write→side-table→query path against a
/// temp-file `SQLiteConnectionManager`. The Android counterpart cannot run a
/// JVM SQLite/`.so` test, so Android pins this via the shared SQL string +
/// S-series dogfood.
final class CustomDictionaryRepositoryCrossModeTests: XCTestCase {
    private var dbPath: String!
    private var repository: CustomDictionaryRepository!

    override func setUpWithError() throws {
        try super.setUpWithError()
        dbPath = NSTemporaryDirectory()
            .appending("custom_dict_crossmode_\(UUID().uuidString).db")
        let path = dbPath!
        let manager = SQLiteConnectionManager(
            databasePath: { path },
            queueLabel: "test.customdict.crossmode.\(UUID().uuidString)",
            loggerCategory: "CustomDictionaryRepositoryCrossModeTests",
        )
        repository = CustomDictionaryRepository(connectionManager: manager)
    }

    override func tearDownWithError() throws {
        try? repository.deleteDatabase()
        repository = nil
        for suffix in ["", "-wal", "-shm"] where dbPath != nil {
            let path = dbPath! + suffix
            if FileManager.default.fileExists(atPath: path) {
                try? FileManager.default.removeItem(atPath: path)
            }
        }
        dbPath = nil
        try super.tearDownWithError()
    }

    /// INVARIANT_CUSTOM_DICT_CROSS_MODE — POJ-stored 食 (`chiah`) is found by a
    /// TL query, a POJ query, AND a TPS-Bopomofo query.
    func test_INVARIANT_CUSTOM_DICT_CROSS_MODE_pojStored_foundFromAllModes() async throws {
        let entry = CustomDictionaryEntry(roman: "chiah", hanzi: "食")
        try await repository.upsert(entry)

        // TL input for the same word (canonical TL "tsiah").
        try await assertFinds(input: "tsiah", mode: .tl, expectedHanzi: "食")
        // POJ input matches the stored POJ spelling.
        try await assertFinds(input: "chiah", mode: .poj, expectedHanzi: "食")

        // TPS Bopomofo: the glyphs a TPS user types ARE the stored tps:notone
        // key. Pull it from the entry's own bundle and feed it back as input.
        let bopomofo = try XCTUnwrap(
            CustomDictionaryDerivation.searchKeys(for: "chiah")
                .first { $0.family == "tps" && $0.form == "notone" }?.key,
            "engine must emit a tps:notone key for chiah",
        )
        // mode .tl on purpose — the engine upgrades to the tps family because
        // the raw input carries Bopomofo, not because settings say tps.
        try await assertFinds(input: bopomofo, mode: .tl, expectedHanzi: "食")
    }

    /// Deleting an entry also drops its side-table rows, so a cross-mode query
    /// no longer finds it.
    func test_crossModeQuery_afterDelete_returnsEmpty() async throws {
        let entry = CustomDictionaryEntry(roman: "chiah", hanzi: "食")
        try await repository.upsert(entry)
        try await repository.delete(id: entry.id)

        let q = try XCTUnwrap(CustomDictionaryDerivation.queryKey(for: "tsiah", mode: .tl))
        let rows = try await repository.search(family: q.family, form: q.form, key: q.key, limit: 20)
        XCTAssertTrue(rows.isEmpty, "deleted entry must not surface via the side table")
    }

    /// INVARIANT_CUSTOM_DICT_CROSS_MODE — a pre-v2 entry (a row that exists
    /// with NO side-table rows, at `user_version = 1`) is backfilled by the
    /// migrator so a cross-mode query finds it after the repository opens.
    func test_INVARIANT_CUSTOM_DICT_CROSS_MODE_migrationBackfillsExistingRow() async throws {
        try seedLegacyV1Row(id: "legacy-1", roman: "chiah", hanzi: "食")

        // Opening the repository runs ensureTables + the v1→v2 migrator backfill.
        try await repository.ensureInitialized()

        try await assertFinds(input: "tsiah", mode: .tl, expectedHanzi: "食")
        try await assertFinds(input: "chiah", mode: .poj, expectedHanzi: "食")
    }

    /// Create a v1-shaped DB at `dbPath`: just `custom_dictionary` (no side
    /// table), one inserted row, `PRAGMA user_version = 1`. Mirrors the
    /// pre-R3 on-disk shape so the migrator's `< 2` branch fires.
    private func seedLegacyV1Row(id: String, roman: String, hanzi: String) throws {
        var db: OpaquePointer?
        guard sqlite3_open(dbPath, &db) == SQLITE_OK else {
            throw XCTSkip("could not open temp sqlite for legacy seed")
        }
        defer { sqlite3_close(db) }

        let ddl = """
            CREATE TABLE custom_dictionary (
                id TEXT PRIMARY KEY,
                roman TEXT NOT NULL,
                hanzi TEXT NOT NULL,
                notone TEXT DEFAULT '',
                abbrev TEXT DEFAULT '',
                roman_num TEXT DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        """
        XCTAssertEqual(sqlite3_exec(db, ddl, nil, nil, nil), SQLITE_OK)

        let insert = "INSERT INTO custom_dictionary (id, roman, hanzi) VALUES (?, ?, ?);"
        var stmt: OpaquePointer?
        XCTAssertEqual(sqlite3_prepare_v2(db, insert, -1, &stmt, nil), SQLITE_OK)
        stmt.bindText(1, id)
        stmt.bindText(2, roman)
        stmt.bindText(3, hanzi)
        XCTAssertEqual(sqlite3_step(stmt), SQLITE_DONE)
        sqlite3_finalize(stmt)

        XCTAssertEqual(sqlite3_exec(db, "PRAGMA user_version = 1;", nil, nil, nil), SQLITE_OK)
    }

    // MARK: - Helpers

    private func assertFinds(
        input: String,
        mode: InputMode,
        expectedHanzi: String,
        file: StaticString = #filePath,
        line: UInt = #line,
    ) async throws {
        let q = try XCTUnwrap(
            CustomDictionaryDerivation.queryKey(for: input, mode: mode),
            "query key nil for \(input) in \(mode)", file: file, line: line,
        )
        let rows = try await repository.search(family: q.family, form: q.form, key: q.key, limit: 20)
        XCTAssertTrue(
            rows.contains { $0.hanzi == expectedHanzi },
            "input '\(input)' (mode \(mode), family \(q.family)/\(q.form)/\(q.key)) must find \(expectedHanzi); got \(rows.map(\.hanzi))",
            file: file, line: line,
        )
    }
}
