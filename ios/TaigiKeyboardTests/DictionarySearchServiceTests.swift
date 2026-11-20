@testable import TaigiKeyboard
import XCTest

/// Dictionary search pins, mirrored by Android `DictionarySearchServiceTest`.
/// The engine's `DictionarySearch` op owns the search (golden:
/// `engine/dispatch/tests/dictionary_search.rs`); the platform side is the
/// settings snapshot the service sends and the row decoding.
final class DictionarySearchServiceTests: XCTestCase {
    private struct Sent: Equatable {
        let query: String
        let inputMode: InputMode
        let toggles: RustEngineBridge.DictionaryToggles
        let customDictionaryEnabled: Bool
        let limit: Int
    }

    private final class FakeLexicon: LexiconClient, @unchecked Sendable {
        private let rows: [DictionarySearchResult]
        private(set) var sent: [Sent] = []

        init(rows: [DictionarySearchResult] = []) {
            self.rows = rows
        }

        func dictionarySearch(
            query: String,
            inputMode: InputMode,
            toggles: RustEngineBridge.DictionaryToggles,
            customDictionaryEnabled: Bool,
            limit: Int,
        ) async -> [DictionarySearchResult] {
            sent.append(Sent(
                query: query,
                inputMode: inputMode,
                toggles: toggles,
                customDictionaryEnabled: customDictionaryEnabled,
                limit: limit,
            ))
            return rows
        }
    }

    private func makeService(settings: StubEngineSettings = StubEngineSettings(), lexicon: FakeLexicon) -> DictionarySearchService {
        DictionarySearchService(lexicon: lexicon, settingsProvider: StubEngineSettingsProvider(settings))
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `an empty query asks the engine nothing`.
    func testAnEmptyQuery_asksTheEngineNothing() async {
        let lexicon = FakeLexicon()

        let results = await makeService(lexicon: lexicon).search(query: "")

        XCTAssertTrue(results.isEmpty)
        XCTAssertTrue(lexicon.sent.isEmpty)
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `one request carries the settings snapshot`.
    func testOneRequest_carriesTheSettingsSnapshot() async {
        var settings = StubEngineSettings()
        settings.inputMode = .tps
        settings.isCustomDictEnabled = false
        settings.isTaiHuaDictEnabled = false
        let lexicon = FakeLexicon()

        _ = await makeService(settings: settings, lexicon: lexicon).search(query: "ㄉㄞ")

        XCTAssertEqual(lexicon.sent, [Sent(
            query: "ㄉㄞ",
            inputMode: .tps,
            toggles: RustEngineBridge.DictionaryToggles(from: settings),
            customDictionaryEnabled: false,
            limit: 20,
        )])
        XCTAssertFalse(lexicon.sent[0].toggles.taihoa)
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `rows keep the engine order`:
    // the engine puts custom entries first and owns the dictionary order.
    func testRows_keepTheEngineOrder() async {
        let rows = [
            DictionarySearchResult(roman: "taigi", tl: "taigi", hanji: "我的台語", sources: [.custom]),
            DictionarySearchResult(roman: "tâi-gí", tl: "tâi-gí", hanji: "台語", sources: [.kautian]),
        ]

        let results = await makeService(lexicon: FakeLexicon(rows: rows)).search(query: "taigi")

        XCTAssertEqual(results, rows)
    }

    // CROSS-PLATFORM INVARIANT — mirrors Android `a row decodes lookup TL, absent Hanji and known sources`.
    func testARow_decodesLookupTlAbsentHanjiAndKnownSources() {
        var row = Taigi_Engine_DictionarySearchRow()
        row.customID = "c1"
        row.roman = "Tâi-gí"
        row.lookupTl = "Tâi-gí"
        row.sources = [.dictSourceCustom, .dictSourceUnspecified]

        XCTAssertEqual(
            DictionarySearchResult(row),
            DictionarySearchResult(roman: "Tâi-gí", tl: "Tâi-gí", hanji: nil, sources: [.custom]),
        )

        var system = Taigi_Engine_DictionarySearchRow()
        system.systemID = 7
        system.roman = "tâi-gí"
        system.lookupTl = "tâi-gí"
        system.hanji = "台語"
        system.sources = [.dictSourceKautian, .dictSourceTaigitv]

        XCTAssertEqual(
            DictionarySearchResult(system),
            DictionarySearchResult(roman: "tâi-gí", tl: "tâi-gí", hanji: "台語", sources: [.kautian, .taigitv]),
        )
    }
}
