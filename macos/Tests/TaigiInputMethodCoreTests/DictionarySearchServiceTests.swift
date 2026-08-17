// What a dictionary search returns, and in what order.

@testable import TaigiInputMethodCore
import XCTest

/// Real engine, real store: the pipeline is mostly ordering and routing, and
/// both are only true against the dictionary the user actually has.
final class DictionarySearchServiceTests: XCTestCase {
    private var stores: UserDataStores!

    override func setUpWithError() throws {
        try super.setUpWithError()
        InstalledLexicon.installOnce()
        stores = try TestFixtures.makeUserDataStores()
    }

    private func makeService(
        inputMode: InputMode = .tl,
        customDict: Bool = true,
        dictionarySources: DictionarySourceToggles = .defaults,
    ) -> DictionarySearchService {
        DictionarySearchService(
            customDictionaryStore: stores.customDictionary,
            settingsProvider: StubEngineSettingsProvider(
                inputMode: inputMode,
                customDict: customDict,
                dictionarySources: dictionarySources,
            ),
        )
    }

    // MARK: - Routing

    func testAnEmptyQuery_asksNothing() {
        XCTAssertTrue(makeService().search("").isEmpty)
    }

    func testARomanQuery_findsWords() {
        let results = makeService().search("taigi")

        XCTAssertFalse(results.isEmpty)
        XCTAssertTrue(
            results.contains { $0.hanzi == "台語" || $0.hanzi == "臺語" },
            "a romanization query found no 台語: \(results.map { $0.hanzi ?? $0.roman })",
        )
    }

    /// 漢字 go to the other engine entry point. Which one to use is asked of
    /// the engine, not decided by scanning code points here.
    func testAHanziQuery_findsWords() {
        let results = makeService().search("台語")

        XCTAssertFalse(results.isEmpty)
        XCTAssertTrue(results.allSatisfy { $0.hanzi != nil })
    }

    // MARK: - The user's own dictionary

    func testACustomEntry_leadsTheResults() async throws {
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "taigi", hanzi: "我的台語"),
        )

        let results = makeService().search("taigi")

        XCTAssertEqual(results.first?.hanzi, "我的台語", "the user's own word did not lead")
        XCTAssertEqual(results.first?.sources, [.custom])
    }

    func testACustomEntry_isNotFoundByAHanziQuery() async throws {
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "taigi", hanzi: "我的台語"),
        )

        let results = makeService().search("我的台語")

        XCTAssertFalse(
            results.contains { $0.sources == [.custom] },
            "the custom dictionary is keyed by romanization and has nothing to answer a 漢字 query with",
        )
    }

    func testWithTheCustomDictionaryOff_itsEntriesAreNotSearched() async throws {
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "taigi", hanzi: "我的台語"),
        )

        let results = makeService(customDict: false).search("taigi")

        XCTAssertFalse(results.contains { $0.sources == [.custom] })
    }

    // MARK: - Order

    /// 教育部 is the reference dictionary, so its rows lead the bundled ones
    /// whatever the engine scored them.
    func testKautianResultsComeFirst() {
        let results = makeService().search("taigi").filter { $0.sources != [.custom] }
        guard let lastKautian = results.lastIndex(where: { $0.sources.contains(.kautian) }),
              let firstOther = results.firstIndex(where: { !$0.sources.contains(.kautian) })
        else { return }

        XCTAssertLessThan(lastKautian, firstOther, "a non-教典 result got ahead of a 教典 one")
    }

    /// The same query twice gives the same list — the sort's final tie-break
    /// is the engine's own order, so rows the engine scored equally cannot
    /// swap places between runs.
    func testTheSameQueryTwice_givesTheSameOrder() {
        let service = makeService()

        XCTAssertEqual(service.search("tai").map(\.id), service.search("tai").map(\.id))
    }

    // MARK: - Rendering

    /// The engine answers in TL. POJ is a rendering of the same reading, and
    /// the raw TL is kept underneath it: the two dictionary sites index TL, so
    /// a lookup built from what the user is currently typing would query for a
    /// spelling neither of them has.
    func testInPojMode_theDisplayIsPojWhileTheLookupKeyStaysTl() throws {
        // Each mode is queried in its own spelling, because the engine
        // searches the key family the mode names — a TL query in POJ mode is
        // not the same lookup.
        let tlRow = try XCTUnwrap(
            makeService(inputMode: .tl).search("tsiah").first { $0.roman.hasPrefix("tsia") },
            "TL mode found nothing for a TL query",
        )
        let pojRow = try XCTUnwrap(
            makeService(inputMode: .poj).search("chiah")
                .first { ($0.lookupTl ?? "").hasPrefix("tsia") },
            "POJ mode found nothing for a POJ query",
        )

        XCTAssertEqual(
            tlRow.roman,
            tlRow.lookupTl,
            "TL mode rendered something other than the engine's TL",
        )
        XCTAssertTrue(
            pojRow.roman.hasPrefix("chia"),
            "POJ mode showed \(pojRow.roman), which is not the POJ spelling",
        )
        XCTAssertNotEqual(
            pojRow.roman,
            pojRow.lookupTl,
            "the POJ display and the TL lookup key are the same string",
        )
        XCTAssertEqual(
            try ExternalLookupURLBuilder.moeURL(forTl: XCTUnwrap(pojRow.lookupTl)),
            pojRow.moeURL,
            "the lookup was built from the display rather than the reading",
        )
    }

    /// A word the user added is one the bundled dictionaries did not have, so
    /// there is no page on either site to send them to — and the spelling they
    /// typed it under is whichever script they were in, which is not what
    /// those sites index.
    func testACustomRow_offersNoExternalLookup() async throws {
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "gua", hanzi: "我的字"),
        )

        let customRow = try XCTUnwrap(
            makeService().search("gua").first { $0.sources == [.custom] },
        )

        XCTAssertNil(customRow.lookupTl)
        XCTAssertNil(customRow.moeURL)
        XCTAssertNil(customRow.chhoeURL)
    }

    // MARK: - Source toggles

    /// Switching every dictionary off means every dictionary, in the search as
    /// much as in the keyboard — the wire's `0` would mean the opposite.
    func testWithEveryDictionaryOff_noBundledResultsAreReturned() {
        let results = makeService(dictionarySources: .allSourcesOff).search("taigi")

        XCTAssertTrue(
            results.allSatisfy { $0.sources == [.custom] },
            "a bundled result survived every dictionary being switched off",
        )
    }

    /// The user's own dictionary is not one of the bundled sources, so it
    /// still answers when they are all off.
    func testWithEveryDictionaryOff_theUsersOwnEntriesStillAnswer() async throws {
        try await stores.customDictionary.upsert(
            CustomDictionaryRow(roman: "taigi", hanzi: "我的台語"),
        )
        let results = makeService(dictionarySources: .allSourcesOff).search("taigi")

        XCTAssertEqual(results.map(\.hanzi), ["我的台語"])
    }

    /// A badge names a dictionary the user has on. A row can reach the list
    /// through one enabled source while also belonging to disabled ones, and
    /// labelling it with those would describe a dictionary they switched off.
    func testBadgesNameOnlyEnabledDictionaries() {
        let withoutKautian = DictionarySourceToggles(
            kautian: false, taigitv: true, itaigi: true, sitbut: true,
            taihoa: true, taijit: true, kungge: true, stti: true,
            khpoo: true, variant: false, khiin: true, lkk: true, dev: true,
            kautianSubcollections: .defaults,
        )

        let results = makeService(dictionarySources: withoutKautian).search("taigi")

        XCTAssertFalse(
            results.contains { $0.sources.contains(.kautian) },
            "a result was labelled 教典 while 教典 is switched off",
        )
    }

    // MARK: - Identity

    /// Rows are identified per list. A single sentinel for every custom row
    /// would make two of the user's own words look like one row to SwiftUI.
    func testEveryResultHasItsOwnIdentity() async throws {
        try await stores.customDictionary.upsert(CustomDictionaryRow(roman: "tai", hanzi: "一"))
        try await stores.customDictionary.upsert(CustomDictionaryRow(roman: "tai", hanzi: "二"))

        let results = makeService().search("tai")

        XCTAssertEqual(Set(results.map(\.id)).count, results.count)
    }
}
