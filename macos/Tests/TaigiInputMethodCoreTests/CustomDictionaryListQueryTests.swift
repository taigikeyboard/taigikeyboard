// What the 自訂詞庫 list actually asks the database for.

@testable import TaigiInputMethodCore
import XCTest

/// The filter and the limit are SQL, not a pass over an array the page already
/// read: a 30000-row dictionary read whole to show a screenful is a cost paid
/// on every keystroke in the filter box.
///
/// The 詞頻 and 詞關聯 halves of this file went with their pages — those stores
/// no longer answer list queries at all.
final class CustomDictionaryListQueryTests: XCTestCase {
    private var stores: UserDataStores!

    override func setUpWithError() throws {
        try super.setUpWithError()
        stores = try TestFixtures.makeUserDataStores()
    }

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

    /// Paging is SQL too, for the reason the limit is: the page the user is on
    /// is the only page read.
    func testCustomDictionary_offsetsIntoTheList() async throws {
        try await seedCustomDictionary()

        let all = try await stores.customDictionary.rows(filter: "", limit: 3)
        let second = try await stores.customDictionary.rows(filter: "", limit: 1, offset: 1)

        XCTAssertEqual(second.map(\.id), [all[1].id])
    }

    /// The pager divides THIS number, so it has to count what the list query
    /// filters — a count taken under a different WHERE would page past the end
    /// of the results, or stop short of them.
    func testCustomDictionaryCount_matchesWhatTheFilterReturns() async throws {
        try await seedCustomDictionary()

        let filtered = try await stores.customDictionary.count(filter: "i")
        let rows = try await stores.customDictionary.rows(filter: "i", limit: 100)

        let unfiltered = try await stores.customDictionary.count(filter: "")

        XCTAssertEqual(filtered, rows.count)
        XCTAssertEqual(unfiltered, 3)
    }

    /// A `%` typed into the filter box is a character the user is looking for,
    /// not a wildcard that matches the whole dictionary.
    func testCustomDictionary_treatsWildcardCharactersAsText() async throws {
        try await seedCustomDictionary()
        try await stores.customDictionary.upsert(CustomDictionaryRow(roman: "pah", hanzi: "100%"))

        let rows = try await stores.customDictionary.rows(filter: "%", limit: 100)

        XCTAssertEqual(rows.map(\.hanzi), ["100%"])
    }
}
