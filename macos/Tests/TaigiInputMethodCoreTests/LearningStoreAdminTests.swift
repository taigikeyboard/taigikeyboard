// Listing, forgetting and importing what the input method has learned.

@testable import TaigiInputMethodCore
import XCTest

/// The operations behind the 詞頻 and 詞關聯 management pages. Unlike the
/// keystroke path, every one of these is something the user pressed a button
/// for, so a failure has to surface rather than be swallowed.
final class LearningStoreAdminTests: XCTestCase {
    private var stores: UserDataStores!

    override func setUpWithError() throws {
        try super.setUpWithError()
        stores = try TestFixtures.makeUserDataStores()
    }

    // MARK: - 詞頻

    func testFrequencyRows_areListedMostUsedFirst() async throws {
        stores.frequency.record(word: "我", tl: "guá")
        stores.frequency.record(word: "你", tl: "lí")
        stores.frequency.record(word: "你", tl: "lí")

        let rows = try await stores.frequency.allRows()

        XCTAssertEqual(rows.map(\.word), ["你", "我"])
        XCTAssertEqual(rows.map(\.count), [2, 1])
    }

    /// Identity is the pair: forgetting 重/tāng must leave 重/tîng alone.
    func testForgettingOneReading_leavesTheOtherAlone() async throws {
        stores.frequency.record(word: "重", tl: "tāng")
        stores.frequency.record(word: "重", tl: "tîng")

        let deleted = try await stores.frequency.delete(word: "重", tl: "tāng")

        XCTAssertTrue(deleted)
        let rows = try await stores.frequency.allRows()
        XCTAssertEqual(rows.map(\.tl), ["tîng"])
    }

    func testForgettingSomethingUnlearned_reportsThatNothingWent() async throws {
        let deleted = try await stores.frequency.delete(word: "我", tl: "guá")

        XCTAssertFalse(deleted)
    }

    func testClearingFrequencies_emptiesTheStoreAndReportsTheCount() async throws {
        stores.frequency.record(word: "我", tl: "guá")
        stores.frequency.record(word: "你", tl: "lí")

        let removed = try await stores.frequency.deleteAll()

        let remaining = try await stores.frequency.allRows()
        XCTAssertEqual(removed, 2)
        XCTAssertTrue(remaining.isEmpty)
    }

    /// Merge-by-max: restoring an older backup must never walk a count the
    /// user has since built up backwards, and must not sum either — the same
    /// backup restored twice would otherwise double everything.
    func testImportingFrequencies_keepsWhicheverCountIsHigher() async throws {
        for _ in 0 ..< 5 {
            stores.frequency.record(word: "我", tl: "guá")
        }

        _ = try await stores.frequency.batchImportMerge([
            FrequencyRow(word: "我", tl: "guá", count: 2, lastUsedMillis: 0),
            FrequencyRow(word: "你", tl: "lí", count: 9, lastUsedMillis: 0),
        ])

        let rows = try await stores.frequency.allRows()
        XCTAssertEqual(rows.first(where: { $0.word == "我" })?.count, 5, "an older backup lost")
        XCTAssertEqual(rows.first(where: { $0.word == "你" })?.count, 9, "a newer backup won")
    }

    /// A row from a backup written before the pair key carries no reading, and
    /// lands in the tolerant empty-TL bucket rather than merging into a real
    /// reading it was never learned under.
    func testImportingALegacyRow_landsInTheEmptyReadingBucket() async throws {
        stores.frequency.record(word: "我", tl: "guá")

        _ = try await stores.frequency.batchImportMerge([
            FrequencyRow(word: "我", tl: "", count: 3, lastUsedMillis: 0),
        ])

        let rows = try await stores.frequency.allRows().filter { $0.word == "我" }
        XCTAssertEqual(Set(rows.map(\.tl)), ["guá", ""])
    }

    // MARK: - 詞關聯

    func testAssociationRows_areListedAndForgottenByTheFullPair() async throws {
        let pair = AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó")
        let other = AssociationPair(previous: "台語", previousTl: "tâi-gú", next: "好", nextTl: "hó")
        stores.association.record([pair, other])

        let before = try await stores.association.rows()
        let deleted = try await stores.association.delete(pair)
        let after = try await stores.association.rows()

        XCTAssertEqual(before.count, 2)
        XCTAssertTrue(deleted)
        XCTAssertEqual(after.map(\.pair), [other], "the other reading of the same 漢字 went with it")
    }

    func testClearingAssociations_emptiesTheStoreAndReportsTheCount() async throws {
        stores.association.record([
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó"),
        ])

        let removed = try await stores.association.deleteAll()

        let remaining = try await stores.association.rows()
        XCTAssertEqual(removed, 1)
        XCTAssertTrue(remaining.isEmpty)
    }

    func testImportingAssociations_keepsWhicheverCountIsHigher() async throws {
        let pair = AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó")
        for _ in 0 ..< 4 {
            stores.association.record([pair])
        }

        _ = try await stores.association.batchImportMerge([
            AssociationRow(pair: pair, count: 1),
        ])

        let rows = try await stores.association.rows()
        XCTAssertEqual(rows.first?.count, 4)
    }
}
