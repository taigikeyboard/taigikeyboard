@testable import TaigiKeyboard
import XCTest

/// Dictionary search policy pins, mirrored by Android
/// `DictionarySearchServiceTest`. The lexicon and the custom dictionary are
/// fakes, so only the platform-side policy is observed: the custom-dictionary
/// toggle gates search as it gates the keyboard, system rows keep the engine's
/// order, and the TPS layout searches the `tps:` family.
final class DictionarySearchServiceTests: XCTestCase {
    private final class FakeLexicon: LexiconClient, @unchecked Sendable {
        private let rows: [RustEngineBridge.LexiconRow]
        private(set) var romanModes: [RustEngineBridge.LexiconInputMode] = []
        private(set) var hanjiModes: [RustEngineBridge.LexiconInputMode] = []

        init(rows: [RustEngineBridge.LexiconRow] = []) {
            self.rows = rows
        }

        /// CJK by the BMP range only — enough for the fixtures; production asks the engine.
        func isHanji(_ text: String) -> Bool {
            text.unicodeScalars.contains { (0x4E00 ... 0x9FFF).contains($0.value) }
        }

        func dictionaryFilters(toggles _: RustEngineBridge.DictionaryToggles) -> RustEngineBridge.DictionaryFilters {
            .allSourcesEnabled
        }

        func searchWithSources(
            input _: String,
            inputMode: RustEngineBridge.LexiconInputMode,
            limit _: UInt32,
            enabledSourcesBitmask _: UInt32,
        ) -> [RustEngineBridge.LexiconRow] {
            romanModes.append(inputMode)
            return rows
        }

        func searchByHanji(
            query _: String,
            inputMode: RustEngineBridge.LexiconInputMode,
            limit _: UInt32,
            enabledSourcesBitmask _: UInt32,
        ) -> [RustEngineBridge.LexiconRow] {
            hanjiModes.append(inputMode)
            return rows
        }
    }

    private final class FakeUserData: UserDataClientStub, @unchecked Sendable {
        private let entries: [CustomDictionaryEntry]
        private(set) var searchCalls = 0

        init(entries: [CustomDictionaryEntry]) {
            self.entries = entries
        }

        override func search(query: String, mode _: InputMode, limit _: Int) async -> [CustomDictionaryEntry] {
            searchCalls += 1
            return entries.filter { $0.roman.hasPrefix(query) }
        }

        override func listAll() async throws -> [CustomDictionaryEntry] {
            entries
        }
    }

    private let myWord = CustomDictionaryEntry(roman: "taigi", hanji: "我的台語")
    private let kautianRow = RustEngineBridge.LexiconRow(id: 1, roman: "tâi-gí", hanji: "台語", sources: [.kautian])

    private func makeService(
        settings: StubEngineSettings = StubEngineSettings(),
        lexicon: FakeLexicon = FakeLexicon(),
        userData: FakeUserData,
    ) -> DictionarySearchService {
        DictionarySearchService(lexicon: lexicon, userData: userData, settingsProvider: StubEngineSettingsProvider(settings))
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android DictionarySearchServiceTest
    // `custom dictionary off - its entries are neither searched nor listed`.
    func testCustomDictionaryOff_itsEntriesAreNotSearched() async throws {
        var settings = StubEngineSettings()
        settings.isCustomDictEnabled = false
        let userData = FakeUserData(entries: [myWord])

        let results = try await makeService(settings: settings, userData: userData).search(query: "taigi")

        XCTAssertEqual(userData.searchCalls, 0)
        XCTAssertFalse(results.contains { $0.sources == [.custom] })
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `custom dictionary on - its entries lead the system rows`.
    func testCustomDictionaryOn_itsEntriesLeadTheSystemRows() async throws {
        let userData = FakeUserData(entries: [myWord])

        let results = try await makeService(lexicon: FakeLexicon(rows: [kautianRow]), userData: userData).search(query: "taigi")

        XCTAssertEqual(userData.searchCalls, 1)
        XCTAssertEqual(results.first?.sources, [.custom])
        XCTAssertEqual(results.first?.id, DictionarySearchResult.customDictMarkerId)
        XCTAssertEqual(results.last?.sources, [.kautian])
        XCTAssertEqual(results.count, 2)
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `system rows keep the engine order`:
    // the engine owns the search order (E1 P5b), so a MOE row the engine listed last stays last.
    func testSystemRows_keepTheEngineOrder() async throws {
        let rows = [
            RustEngineBridge.LexiconRow(id: 2, roman: "tâi-gí", hanji: "台語", sources: [.taigitv]),
            RustEngineBridge.LexiconRow(id: 3, roman: "tāi-ki", hanji: "代記", sources: [.taigitv]),
            kautianRow,
        ]

        let results = try await makeService(lexicon: FakeLexicon(rows: rows), userData: FakeUserData(entries: [])).search(query: "taigi")

        XCTAssertEqual(results.map(\.id), [2, 3, 1])
    }

    func testAHanjiQuery_takesTheHanjiPathAndNeverConsultsTheCustomDictionary() async throws {
        let lexicon = FakeLexicon()
        let userData = FakeUserData(entries: [CustomDictionaryEntry(roman: "taigi", hanji: "台語")])

        _ = try await makeService(lexicon: lexicon, userData: userData).search(query: "台語")

        XCTAssertEqual(lexicon.hanjiModes.count, 1)
        XCTAssertTrue(lexicon.romanModes.isEmpty)
        XCTAssertEqual(userData.searchCalls, 0)
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `the TPS layout searches the TPS family`.
    func testTpsLayout_searchesTheTpsFamily() async throws {
        var settings = StubEngineSettings()
        settings.inputMode = .tps
        let lexicon = FakeLexicon()

        _ = try await makeService(settings: settings, lexicon: lexicon, userData: FakeUserData(entries: [])).search(query: "ㄉㄞ")

        XCTAssertEqual(lexicon.romanModes, [.tps])
    }

    func testLexiconMode_mapsEveryInputMode() {
        XCTAssertEqual(DictionarySearchService.lexiconMode(.tl), .tl)
        XCTAssertEqual(DictionarySearchService.lexiconMode(.poj), .poj)
        XCTAssertEqual(DictionarySearchService.lexiconMode(.tps), .tps)
        XCTAssertEqual(DictionarySearchService.lexiconMode(.english), .tl)
    }
}
