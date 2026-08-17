// The user's own dictionary: what it stores, what it refuses, and what stays
// findable afterwards.

@testable import TaigiInputMethodCore
import XCTest

/// A stand-in for the engine's key derivation, so the store's transaction and
/// capacity behaviour can be driven without an FFI round-trip — and so a failing
/// derivation is reachable at all.
private func stubSearchKeys(for roman: String) -> [CustomSearchKey]? {
    guard !roman.isEmpty else { return nil }
    return [
        CustomSearchKey(family: "tl", form: "notone", key: roman.lowercased()),
        CustomSearchKey(family: "poj", form: "notone", key: roman.lowercased()),
    ]
}

final class CustomDictionaryStoreTests: XCTestCase {
    private func makeStore(
        deriveSearchKeys: @escaping @Sendable (String) -> [CustomSearchKey]? = {
            stubSearchKeys(for: $0)
        },
        maxEntries: Int = CustomDictionaryStore.maxEntries,
    ) throws -> CustomDictionaryStore {
        let directory = try TestFixtures.scratchDirectory()
        let store = CustomDictionaryStore(
            directory: { directory },
            deriveSearchKeys: deriveSearchKeys,
            entryLimit: maxEntries,
        )
        store.open()
        XCTAssertTrue(
            TestFixtures.spinRunLoop(until: { store.isReady }, timeout: 5),
            "the store never opened",
        )
        return store
    }

    private func row(_ roman: String, _ hanzi: String, id: String = UUID().uuidString) -> CustomDictionaryRow {
        CustomDictionaryRow(id: id, roman: roman, hanzi: hanzi)
    }

    private func queryKey(_ key: String, family: String = "tl") -> CustomSearchKey {
        CustomSearchKey(family: family, form: "notone", key: key)
    }

    // MARK: - CRUD

    func testAnAddedEntry_isFoundByItsKey() async throws {
        let store = try makeStore()

        try await store.upsert(row("gua", "我"))

        XCTAssertEqual(store.rows(matching: queryKey("gua")).map(\.hanzi), ["我"])
    }

    /// The keys are derived per family, so an entry added while typing TL is
    /// still found by someone typing POJ.
    func testAnAddedEntry_isFoundFromAnotherRomanization() async throws {
        let store = try makeStore()

        try await store.upsert(row("gua", "我"))

        XCTAssertEqual(store.rows(matching: queryKey("gua", family: "poj")).count, 1)
    }

    /// A prefix is enough — the keyboard looks the entry up while the word is
    /// still being typed.
    func testAPrefix_matches() async throws {
        let store = try makeStore()

        try await store.upsert(row("taigi", "台語"))

        XCTAssertEqual(store.rows(matching: queryKey("tai")).map(\.hanzi), ["台語"])
    }

    /// Editing a romanization must not leave the entry findable under the one
    /// it replaced — the side keys are replaced, not accumulated.
    func testEditingTheRomanization_dropsTheOldKeys() async throws {
        let store = try makeStore()
        var entry = row("gua", "我")
        try await store.upsert(entry)

        entry.roman = "goa"
        try await store.upsert(entry)

        XCTAssertEqual(store.rows(matching: queryKey("goa")).count, 1)
        XCTAssertTrue(
            store.rows(matching: queryKey("gua")).isEmpty,
            "the entry is still answering to the romanization it no longer has",
        )
    }

    func testDeletingAnEntry_removesItAndItsKeys() async throws {
        let store = try makeStore()
        let entry = row("gua", "我")
        try await store.upsert(entry)

        let deleted = try await store.delete(id: entry.id)

        XCTAssertTrue(deleted)
        XCTAssertTrue(store.rows(matching: queryKey("gua")).isEmpty)
        let deletedAgain = try await store.delete(id: entry.id)
        XCTAssertFalse(deletedAgain, "deleting what is not there is not a deletion")
    }

    func testDeleteAll_emptiesTheDictionaryAndReportsWhatWent() async throws {
        let store = try makeStore()
        try await store.upsert(row("gua", "我"))
        try await store.upsert(row("li", "你"))

        let removed = try await store.deleteAll()

        let remaining = try await store.count()
        XCTAssertEqual(removed, 2)
        XCTAssertEqual(remaining, 0)
        XCTAssertTrue(store.rows(matching: queryKey("gua")).isEmpty)
    }

    /// A romanization-only entry is legitimate: the engine reads the absent
    /// 漢字 as "render the romanization".
    func testAnEntryWithNoHanzi_isStored() async throws {
        let store = try makeStore()

        try await store.upsert(row("gua", ""))

        XCTAssertEqual(store.rows(matching: queryKey("gua")).map(\.hanzi), [""])
    }

    // MARK: - Atomicity

    /// A row whose search keys could not be derived is not written at all: it
    /// would show up in the settings list and never appear while typing, which
    /// reads as a broken dictionary rather than a refused entry.
    func testADerivationFailure_writesNothing() async throws {
        let store = try makeStore(deriveSearchKeys: { _ in nil })

        do {
            try await store.upsert(row("gua", "我"))
            XCTFail("a derivation failure has to surface")
        } catch let error as CustomDictionaryError {
            guard case .searchKeyDerivationFailed = error else {
                return XCTFail("wrong error: \(error)")
            }
        }

        let stored = try await store.count()
        XCTAssertEqual(stored, 0)
    }

    // MARK: - Capacity

    func testTheCap_isTheCrossPlatformThirtyThousand() {
        XCTAssertEqual(CustomDictionaryStore.maxEntries, 30000)
    }

    /// A file larger than the cap is refused before anything is written —
    /// a half-imported dictionary is worse than a rejected one.
    func testAnImportLargerThanTheCap_isRefusedUpFront() async throws {
        let store = try makeStore()
        let oversized = (0 ... CustomDictionaryStore.maxEntries).map { row("r\($0)", "字\($0)") }

        do {
            _ = try await store.batchImport(oversized)
            XCTFail("an oversized import has to be refused")
        } catch let error as CustomDictionaryError {
            guard case .capacityReached = error else { return XCTFail("wrong error: \(error)") }
        }

        let stored = try await store.count()
        XCTAssertEqual(stored, 0)
    }

    // MARK: - Import

    func testImport_skipsRowsAlreadyStored() async throws {
        let store = try makeStore()
        try await store.upsert(row("gua", "我"))

        let result = try await store.batchImport([row("gua", "我"), row("li", "你")])

        let storedCount = try await store.count()
        XCTAssertEqual(result, CustomDictionaryImportResult(imported: 1, skipped: 1))
        XCTAssertEqual(storedCount, 2)
    }

    /// Two rows sharing a romanization but not a 漢字 are two words, and both
    /// are imported — the raw pair is what an import dedupes on, matching iOS
    /// and Android.
    func testImport_keepsTwoHanziUnderOneRomanization() async throws {
        let store = try makeStore()

        let result = try await store.batchImport([row("tai", "台"), row("tai", "臺")])

        XCTAssertEqual(result.imported, 2)
        XCTAssertEqual(Set(store.rows(matching: queryKey("tai")).map(\.hanzi)), ["台", "臺"])
    }

    func testImport_isFindableImmediately() async throws {
        let store = try makeStore()

        _ = try await store.batchImport([row("taigi", "台語")])

        XCTAssertEqual(store.rows(matching: queryKey("taigi")).map(\.hanzi), ["台語"])
    }

    /// More rows than one transaction covers, so the chunking is exercised
    /// rather than only described.
    func testImport_writesEveryChunk() async throws {
        let store = try makeStore()
        let rows = (0 ..< 1200).map { row("r\($0)", "字\($0)") }

        let result = try await store.batchImport(rows)

        let storedCount = try await store.count()
        XCTAssertEqual(result.imported, 1200)
        XCTAssertEqual(storedCount, 1200)
    }

    /// The cap is enforced against the row count at the moment of the write,
    /// not one read before the import started: every `await` hands the queue
    /// back, so an entry added by hand mid-import would otherwise be spent
    /// out of headroom this import had already claimed.
    func testImport_stopsAtTheCapCountedAtWriteTime() async throws {
        let store = try makeStore(maxEntries: 3)
        try await store.upsert(row("existing", "有"))

        let result = try await store.batchImport([
            row("a", "一"), row("b", "二"), row("c", "三"),
        ])

        let stored = try await store.count()
        XCTAssertEqual(stored, 3, "the cap held")
        XCTAssertEqual(result, CustomDictionaryImportResult(imported: 2, skipped: 1))
    }

    /// A romanization is derived once however many 漢字 share it, and a
    /// derivation that fails does so before the first row is written.
    func testImport_derivesEachRomanizationOnce() async throws {
        let derivedRomans = DerivationRecorder()
        let store = try makeStore(deriveSearchKeys: { roman in
            derivedRomans.record(roman)
            return stubSearchKeys(for: roman)
        })

        _ = try await store.batchImport([
            row("tai", "台"), row("tai", "臺"), row("gi", "語"),
        ])

        XCTAssertEqual(derivedRomans.romans.sorted(), ["gi", "tai"])
    }

    func testImport_withAFailingDerivation_writesNothing() async throws {
        let store = try makeStore(deriveSearchKeys: { roman in
            roman == "bad" ? nil : stubSearchKeys(for: roman)
        })

        do {
            _ = try await store.batchImport([row("ok", "好"), row("bad", "壞")])
            XCTFail("a derivation failure has to surface")
        } catch let error as CustomDictionaryError {
            guard case .searchKeyDerivationFailed = error else {
                return XCTFail("wrong error: \(error)")
            }
        }

        let stored = try await store.count()
        XCTAssertEqual(stored, 0, "the rows before the failing one were written anyway")
    }

    // MARK: - Seed

    func testSeed_addsTheTwoDefaultEntriesToAnEmptyDictionary() async throws {
        let store = try makeStore()

        try await store.seedIfEmpty()

        let seeded = try await store.count()
        let hanzi = try await Set(store.allRows().map(\.hanzi))
        XCTAssertEqual(seeded, 2)
        XCTAssertEqual(hanzi, ["𠢕早", "食飽未"])
    }

    /// Deleting a seeded entry and relaunching must not bring it back — the
    /// guard is on the dictionary being empty, not on each entry being present.
    func testSeed_doesNothingOnceTheUserHasTouchedTheDictionary() async throws {
        let store = try makeStore()
        try await store.seedIfEmpty()
        _ = try await store.delete(id: CustomDictionaryStore.seedEntries[0].id)

        try await store.seedIfEmpty()

        let remaining = try await store.count()
        XCTAssertEqual(remaining, 1)
    }

    // MARK: - Not open

    /// The keystroke path answers empty rather than waiting, exactly as the
    /// frequency store does — a composition must never block on a file.
    func testAStoreThatIsNotOpen_answersNoRowsRatherThanWaiting() throws {
        let directory = try TestFixtures.scratchDirectory()
        let store = CustomDictionaryStore(directory: { directory })

        XCTAssertTrue(store.rows(matching: queryKey("gua")).isEmpty)
    }

    /// What the user asked for is a different matter: a write that did not
    /// happen has to say so rather than report a silent success.
    func testAWriteToAStoreThatIsNotOpen_throws() async throws {
        let directory = try TestFixtures.scratchDirectory()
        let store = CustomDictionaryStore(directory: { directory })

        do {
            try await store.upsert(row("gua", "我"))
            XCTFail("a write to a closed store has to surface")
        } catch let error as UserDataDatabaseError {
            guard case .notOpen = error else { return XCTFail("wrong error: \(error)") }
        }
    }
}

/// Records which romanizations the store asked to derive. A class so the
/// `@Sendable` derivation closure can write to it.
private final class DerivationRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var stored: [String] = []

    var romans: [String] {
        lock.withLock { stored }
    }

    func record(_ roman: String) {
        lock.withLock { stored.append(roman) }
    }
}
