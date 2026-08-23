// The two guarantees left on the learning tables: they stay bounded, and they can be emptied.

@testable import TaigiInputMethodCore
import XCTest

/// With the 詞頻紀錄 and 詞關聯紀錄 panes gone, these are the only promises the
/// product still makes about learned data: it cannot grow without bound
/// (`LearningCapacity`), and the store call behind the 一般 pane's one
/// destructive button empties it (`deleteAll`).
///
/// The bound is asserted through each store's real `record` path rather than
/// against the helper alone — a cap the writer never invokes is not a cap. The
/// button's own orchestration is not covered here; this is the store layer.
///
/// The shipped caps (20000 / 50000 rows) are out of reach of a test, so the
/// stores take an injected `LearningCapacity` and these use a small one.
final class LearningCapacityTests: XCTestCase {
    private var directory: URL!

    override func setUpWithError() throws {
        try super.setUpWithError()
        directory = try TestFixtures.scratchDirectory()
    }

    // MARK: - Bounded

    /// The writer must reach the cap on its own. `recordsBetweenChecks: 1` so
    /// every write checks — the shipped throttle of 100 exists to keep the
    /// COUNT off the keystroke path, not to change what the cap means.
    func testRecordingPastTheCap_prunesDownBelowIt() throws {
        let store = try makeFrequencyStore(
            capacity: LearningCapacity(
                table: "user_frequency", maxRows: 10, deleteBatch: 4, recordsBetweenChecks: 1,
            ),
        )

        for index in 0 ..< 20 {
            store.record(word: "字\(index)", tl: "tsi\(index)")
        }

        let rows = try countRows(in: store)
        XCTAssertLessThanOrEqual(rows, 10, "the table grew past its cap")
        XCTAssertGreaterThan(rows, 0, "pruning emptied the table instead of trimming it")
    }

    /// "Least useful" is low count first, then stale — the order iOS and
    /// Android delete in, so the three platforms forget the same rows.
    ///
    /// Asserted from both ends: the often-used row survives AND the table has
    /// actually shed once-used rows, so a prune that deleted nothing (or
    /// deleted by insertion order) fails here.
    func testPruning_dropsTheLeastUsedRowsFirst() throws {
        let store = try makeFrequencyStore(
            capacity: LearningCapacity(
                table: "user_frequency", maxRows: 3, deleteBatch: 2, recordsBetweenChecks: 1,
            ),
        )

        for _ in 0 ..< 5 {
            store.record(word: "常", tl: "siông")
        }
        let onceUsed = (0 ..< 8).map { "罕\($0)" }
        for word in onceUsed {
            store.record(word: word, tl: "hán")
        }

        let survivors = try words(in: store)
        XCTAssertTrue(survivors.contains("常"), "the most-used row was pruned: \(survivors)")
        XCTAssertLessThan(
            Set(onceUsed).intersection(survivors).count, onceUsed.count,
            "nothing was pruned: \(survivors)",
        )
    }

    /// The association store's own writer must reach its own cap. Not covered
    /// by the frequency cases above: a compound commit counts several rows
    /// towards one throttle tick, and the table name the capacity interpolates
    /// is per store.
    func testRecordingAssociationsPastTheCap_prunesDownBelowIt() throws {
        let directory = directory!
        let store = UserAssociationStore(
            directory: { directory },
            capacity: LearningCapacity(
                table: "user_association", maxRows: 10, deleteBatch: 4, recordsBetweenChecks: 1,
            ),
        )
        store.open()
        XCTAssertTrue(
            TestFixtures.spinRunLoop(until: { store.isReady }, timeout: 5),
            "the association store did not open",
        )

        for index in 0 ..< 20 {
            store.record([
                AssociationPair(
                    previous: "頭\(index)", previousTl: "thâu\(index)",
                    next: "尾\(index)", nextTl: "bué\(index)",
                ),
            ])
        }

        let rows = try XCTUnwrap(store.allRows())
        XCTAssertLessThanOrEqual(rows.count, 10, "the table grew past its cap")
        XCTAssertGreaterThan(rows.count, 0, "pruning emptied the table instead of trimming it")
    }

    /// Under the cap nothing is dropped — the prune is a ceiling, not a
    /// rolling window.
    func testRecordingUnderTheCap_keepsEveryRow() throws {
        let store = try makeFrequencyStore(
            capacity: LearningCapacity(
                table: "user_frequency", maxRows: 50, deleteBatch: 10, recordsBetweenChecks: 1,
            ),
        )

        for index in 0 ..< 20 {
            store.record(word: "字\(index)", tl: "tsi\(index)")
        }

        let rows = try countRows(in: store)
        XCTAssertEqual(rows, 20)
    }

    /// The throttle counts rows written, so a table can sit over its cap until
    /// the next check. Documents the cap as a ROW-COUNT ceiling enforced on a
    /// schedule, not an invariant true after every single write.
    func testTheThrottle_letsTheTableOvershootUntilTheNextCheck() {
        let capacity = LearningCapacity(
            table: "user_frequency", maxRows: 10, deleteBatch: 4, recordsBetweenChecks: 100,
        )

        XCTAssertFalse(capacity.shouldEnforce(after: 99))
        XCTAssertTrue(capacity.shouldEnforce(after: 1))
        XCTAssertFalse(capacity.shouldEnforce(after: 1), "the counter did not reset")
    }

    // MARK: - Emptied

    func testClearingFrequencies_emptiesTheStoreAndReportsTheCount() async throws {
        let stores = try TestFixtures.makeUserDataStores()
        stores.frequency.record(word: "我", tl: "guá")
        stores.frequency.record(word: "你", tl: "lí")

        let removed = try await stores.frequency.deleteAll()

        XCTAssertEqual(removed, 2)
        XCTAssertEqual(try countRows(in: stores.frequency), 0)
    }

    func testClearingAssociations_emptiesTheStoreAndReportsTheCount() async throws {
        let stores = try TestFixtures.makeUserDataStores()
        stores.association.record([
            AssociationPair(previous: "我", previousTl: "guá", next: "是", nextTl: "sī"),
            AssociationPair(previous: "你", previousTl: "lí", next: "好", nextTl: "hó"),
        ])

        let removed = try await stores.association.deleteAll()

        XCTAssertEqual(removed, 2)
        XCTAssertEqual(stores.association.allRows(), [])
    }

    /// Clearing one learning table must leave the others alone. The 一般 pane
    /// clears both learning stores, but it does so with two separate calls, and
    /// the custom dictionary is never in scope of either.
    func testClearingOneStore_leavesTheOthersAlone() async throws {
        let stores = try TestFixtures.makeUserDataStores()
        stores.frequency.record(word: "我", tl: "guá")
        stores.association.record([
            AssociationPair(previous: "我", previousTl: "guá", next: "是", nextTl: "sī"),
        ])
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "gua", hanzi: "我"),
        )

        _ = try await stores.frequency.deleteAll()

        let customRows = try await stores.customDictionary.allRows()
        XCTAssertEqual(stores.association.allRows()?.count, 1)
        XCTAssertEqual(customRows.count, 1)
    }

    // MARK: - Fixture

    private func makeFrequencyStore(capacity: LearningCapacity) throws -> UserFrequencyStore {
        let directory = directory!
        let store = UserFrequencyStore(directory: { directory }, capacity: capacity)
        store.open()
        XCTAssertTrue(
            TestFixtures.spinRunLoop(until: { store.isReady }, timeout: 5),
            "the frequency store did not open",
        )
        return store
    }

    private func countRows(in store: UserFrequencyStore) throws -> Int {
        try words(in: store).count
    }

    /// Most-used first, so the first element is the row a prune must spare.
    private func words(in store: UserFrequencyStore) throws -> [String] {
        try XCTUnwrap(store.allRows()).map(\.word)
    }
}
