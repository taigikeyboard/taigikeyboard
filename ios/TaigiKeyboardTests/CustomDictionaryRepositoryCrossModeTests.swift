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
        try seedLegacyRow(id: "legacy-1", roman: "chiah", hanzi: "食", userVersion: 1)

        // Opening the repository runs ensureTables + the v1→v2 migrator backfill.
        try await repository.ensureInitialized()

        try await assertFinds(input: "tsiah", mode: .tl, expectedHanzi: "食")
        try await assertFinds(input: "chiah", mode: .poj, expectedHanzi: "食")
    }


    /// INVARIANT_CUSTOM_DICT_CAPACITY — the 30000-row cap constant is pinned on
    /// both platforms (Android `CustomDictionaryCapacityPolicy.MAX_ENTRIES`), and
    /// the capacity policy tracks row count + bypasses updates of an existing id.
    /// The over-cap throw + grandfather-on-import behavior is dogfood-pinned (S14)
    /// — a 30000-row insert test is impractical; Android exercises the pure
    /// decision arithmetic directly in `CustomDictionaryCapacityPolicyTest`.
    func test_INVARIANT_CUSTOM_DICT_CAPACITY_constAndExistingRowBypass() async throws {
        XCTAssertEqual(
            CustomDictionaryCapacityPolicy.maxEntries, 30000,
            "row cap must stay aligned with Android MAX_ENTRIES",
        )

        try await repository.ensureInitialized()
        let initialCount = try await repository.count()
        XCTAssertEqual(initialCount, 0, "fresh DB starts empty")

        let entry = CustomDictionaryEntry(roman: "chiah", hanzi: "食")
        try await repository.upsert(entry)
        let afterInsert = try await repository.count()
        XCTAssertEqual(afterInsert, 1, "count tracks inserted rows")

        // Re-upserting the SAME id is an update, not a new insert — the count
        // stays 1, exercising the guard's existing-row bypass path.
        try await repository.upsert(entry)
        let afterUpdate = try await repository.count()
        XCTAssertEqual(afterUpdate, 1, "update of an existing id is not a new insert (guard bypass)")
    }

    /// User report 2026-08-20 (backlog B1): the custom entry
    /// 「絆創膏 `băng-só͘-khó͘`」 was findable in TL but vanished in POJ at the
    /// SECOND `o`. POJ writes /ɔ/ as `o` + U+0358 while the raw keyboard
    /// buffer holds the ASCII `oo` the user typed — the oo double-tap rewrite
    /// is display-only — so the stored key must fold to the ASCII spelling.
    func test_pojEntryWithOoDot_isFoundByAsciiOoInput() async throws {
        try await repository.upsert(CustomDictionaryEntry(roman: "băng-só͘-khó͘", hanzi: "絆創膏"))

        // The keystroke the report died on, and the full romanization after it.
        try await assertFinds(input: "bangsoo", mode: .poj, expectedHanzi: "絆創膏")
        try await assertFinds(input: "bangsookhoo", mode: .poj, expectedHanzi: "絆創膏")
        // The dedicated POJ `o͘` key types U+0358 into the buffer instead.
        try await assertFinds(input: "bangso\u{0358}kho\u{0358}", mode: .poj, expectedHanzi: "絆創膏")
        // TL control — this path was never broken.
        try await assertFinds(input: "bangsookhoo", mode: .tl, expectedHanzi: "絆創膏")
    }

    /// An entry stored by a pre-v3 build carries the keys THAT build derived,
    /// and a `o͘` entry's POJ key had the dot dropped as if it were a tone
    /// diacritic. The v3 migration re-derives every key, so the entry becomes
    /// reachable from the keyboard again without the user re-adding it.
    func test_migrationV3_rederivesStaleOoDotKeys() async throws {
        try seedLegacyRow(
            id: "legacy-oo-dot",
            roman: "băng-só͘-khó͘",
            hanzi: "絆創膏",
            userVersion: 2,
            staleKeys: [
                // What the pre-fix derivation wrote: the dot dropped from the
                // toneless key, kept as a display glyph in the tone-aware one.
                (family: "poj", form: "notone", key: "bangsokho"),
                (family: "poj", form: "num", key: "bang9so\u{0358}2kho\u{0358}2"),
            ],
        )

        try await repository.ensureInitialized()

        try await assertFinds(input: "bangsookhoo", mode: .poj, expectedHanzi: "絆創膏")
        try await assertFinds(input: "bang9soo2khoo2", mode: .poj, expectedHanzi: "絆創膏")
    }

    /// Create a pre-migration DB at `dbPath`: the main table, one entry, the
    /// side-table rows as the build of that era derived them (none before v2,
    /// when the side table did not exist yet), and its `PRAGMA user_version`.
    /// The DDL is written out rather than taken from `CustomDictionarySchema`
    /// because the point is the shape an OLD build left behind.
    private func seedLegacyRow(
        id: String,
        roman: String,
        hanzi: String,
        userVersion: Int,
        staleKeys: [(family: String, form: String, key: String)] = [],
    ) throws {
        var db: OpaquePointer?
        guard sqlite3_open(dbPath, &db) == SQLITE_OK else {
            throw XCTSkip("could not open temp sqlite for legacy seed")
        }
        defer { sqlite3_close(db) }

        var ddl = """
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
        if userVersion >= 2 {
            ddl += """
                CREATE TABLE custom_search_key (
                    entry_id TEXT NOT NULL,
                    family   TEXT NOT NULL,
                    form     TEXT NOT NULL,
                    key      TEXT NOT NULL
                );
            """
        }
        XCTAssertEqual(sqlite3_exec(db, ddl, nil, nil, nil), SQLITE_OK)

        var stmt: OpaquePointer?
        XCTAssertEqual(
            sqlite3_prepare_v2(db, "INSERT INTO custom_dictionary (id, roman, hanzi) VALUES (?, ?, ?);", -1, &stmt, nil),
            SQLITE_OK,
        )
        stmt.bindText(1, id)
        stmt.bindText(2, roman)
        stmt.bindText(3, hanzi)
        XCTAssertEqual(sqlite3_step(stmt), SQLITE_DONE)
        sqlite3_finalize(stmt)

        var keyStmt: OpaquePointer?
        if !staleKeys.isEmpty {
            XCTAssertEqual(
                sqlite3_prepare_v2(
                    db,
                    "INSERT INTO custom_search_key (entry_id, family, form, key) VALUES (?, ?, ?, ?);",
                    -1, &keyStmt, nil,
                ),
                SQLITE_OK,
            )
        }
        defer { sqlite3_finalize(keyStmt) }
        for staleKey in staleKeys {
            sqlite3_reset(keyStmt)
            sqlite3_clear_bindings(keyStmt)
            keyStmt.bindText(1, id)
            keyStmt.bindText(2, staleKey.family)
            keyStmt.bindText(3, staleKey.form)
            keyStmt.bindText(4, staleKey.key)
            XCTAssertEqual(sqlite3_step(keyStmt), SQLITE_DONE)
        }

        XCTAssertEqual(sqlite3_exec(db, "PRAGMA user_version = \(userVersion);", nil, nil, nil), SQLITE_OK)
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
