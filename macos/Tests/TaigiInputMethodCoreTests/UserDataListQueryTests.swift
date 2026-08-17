// What the management lists actually ask the database for.

@testable import TaigiInputMethodCore
import XCTest

/// The filter and the limit are SQL, not a pass over an array the page already
/// read: a 30000-row dictionary read whole to show a screenful is a cost paid
/// on every keystroke in the filter box.
final class UserDataListQueryTests: XCTestCase {
    private var stores: UserDataStores!

    override func setUpWithError() throws {
        try super.setUpWithError()
        stores = try TestFixtures.makeUserDataStores()
    }

    // MARK: - 自訂詞庫

    private func seedCustomDictionary() async throws {
        for (roman, hanzi) in [("gua", "我"), ("li", "你"), ("taigi", "台語")] {
            try await stores.customDictionary.upsert(
                CustomDictionaryRow(roman: roman, hanzi: hanzi),
            )
        }
    }

    func testCustomDictionary_filtersOnEitherColumn() async throws {
        try await seedCustomDictionary()

        let byRoman = try await stores.customDictionary.rows(filter: "tai", limit: 100)
        let byHanzi = try await stores.customDictionary.rows(filter: "你", limit: 100)

        XCTAssertEqual(byRoman.map(\.hanzi), ["台語"])
        XCTAssertEqual(byHanzi.map(\.roman), ["li"])
    }

    func testCustomDictionary_honoursTheLimit() async throws {
        try await seedCustomDictionary()

        let rows = try await stores.customDictionary.rows(filter: "", limit: 2)

        XCTAssertEqual(rows.count, 2)
    }

    /// A `%` typed into the filter box is a character the user is looking for,
    /// not a wildcard that matches the whole dictionary.
    func testCustomDictionary_treatsWildcardCharactersAsText() async throws {
        try await seedCustomDictionary()
        try await stores.customDictionary.upsert(CustomDictionaryRow(roman: "pah", hanzi: "100%"))

        let rows = try await stores.customDictionary.rows(filter: "%", limit: 100)

        XCTAssertEqual(rows.map(\.hanzi), ["100%"])
    }

    // MARK: - 詞頻

    func testFrequency_filtersOnWordAndReading() async throws {
        stores.frequency.record(word: "重", tl: "tāng")
        stores.frequency.record(word: "我", tl: "guá")

        let byWord = try await stores.frequency.rows(filter: "重", limit: 100)
        let byReading = try await stores.frequency.rows(filter: "guá", limit: 100)

        XCTAssertEqual(byWord.map(\.tl), ["tāng"])
        XCTAssertEqual(byReading.map(\.word), ["我"])
    }

    func testFrequency_listsMostUsedFirstWithinTheLimit() async throws {
        stores.frequency.record(word: "少", tl: "tsió")
        for _ in 0 ..< 3 {
            stores.frequency.record(word: "濟", tl: "tsē")
        }

        let rows = try await stores.frequency.rows(filter: "", limit: 1)

        XCTAssertEqual(rows.map(\.word), ["濟"])
    }

    // MARK: - 詞關聯

    func testAssociation_filtersOnAnyOfTheFourIdentityColumns() async throws {
        stores.association.record([
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "真好", nextTl: "tsin-hó"),
        ])

        for filter in ["台語", "tâi-gí", "真好", "tsin-hó"] {
            let rows = try await stores.association.rows(filter: filter, limit: 100)
            XCTAssertEqual(rows.count, 1, "filtering by \(filter) found nothing")
        }
    }

    func testAssociation_honoursTheLimit() async throws {
        stores.association.record([
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "好", nextTl: "hó"),
            AssociationPair(previous: "台語", previousTl: "tâi-gí", next: "䆀", nextTl: "bái"),
        ])

        let rows = try await stores.association.rows(filter: "", limit: 1)

        XCTAssertEqual(rows.count, 1)
    }
}
