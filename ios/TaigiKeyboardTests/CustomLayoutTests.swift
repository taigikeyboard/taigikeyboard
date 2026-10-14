@testable import TaigiKeyboard
import XCTest

/// Tests for `CustomLayout` (key slots, swap / spare tray, validation), `UserLayoutStore` and
/// the `SharedSettings` custom-layout selection contract.
final class CustomLayoutTests: XCTestCase {
    // trace: phahTaigi_iPhone rows 1–3 (`TaigiLayouts.swift`), bottom row has no `.char`.
    private static let phahTaigiKeys = [
        "!", "?", "e", "r", "t", "y", "u", "i", "o", "p",
        "a", "s", "d", "f", "g", "h", "j", "k", "l", "-",
        ",", ".", "c", "v", "b", "n", "m",
    ]

    private var suiteName: String!
    private var defaults: UserDefaults!

    override func setUp() {
        super.setUp()
        suiteName = "CustomLayoutTests.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)!
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        suiteName = nil
        super.tearDown()
    }

    // MARK: - Model

    func testCustomLayout_freshLayoutMatchesBaseTable() {
        let layout = CustomLayout(base: .phahTaigi, script: .tl)
        XCTAssertEqual(layout.rows(withGlobe: false), TaigiLayouts.Alphabetic.phahTaigi_iPhone)
        XCTAssertEqual(layout.rows(withGlobe: true), TaigiLayouts.Alphabetic.phahTaigi_withGlobe)
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_spareKeysAreLettersMissingFromBoard() {
        // trace: tray letters a–z + o͘ + "-" minus the board; PhahTaigi lacks q w x z and o͘.
        XCTAssertEqual(CustomLayout(base: .phahTaigi, script: .tl).spareKeys, ["q", "w", "x", "z", "o\u{0358}"])
        // trace: Lohankha POJ carries every letter and o͘ (row 2 end), so nothing is spare.
        XCTAssertEqual(CustomLayout(base: .qwerty, script: .poj).spareKeys, [])
    }

    func testCustomLayout_swapMovesWholeKeyIncludingFullWidthForm() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        // Slot 20 is "," (row 3, column 1), slot 22 is "c" (row 3, column 3).
        layout.swapKeys(20, 22)
        let rows = layout.rows(withGlobe: false)
        XCTAssertEqual(rows[3][1], .char("c"))
        XCTAssertEqual(rows[3][3], .char(",", fullWidth: "，"))
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_replaceTradesKeyWithSpare() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.replaceKey(at: 0, with: "q")
        XCTAssertEqual(layout.rows(withGlobe: false)[1][0], .char("q"))
        // trace: "!" is a base key now off the board, so it follows the tray letters.
        XCTAssertEqual(layout.spareKeys, ["w", "x", "z", "o\u{0358}", "!"])
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_replaceIgnoresKeyAlreadyOnBoardOrUnknown() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.replaceKey(at: 0, with: "e")
        layout.replaceKey(at: 0, with: "😀")
        XCTAssertEqual(layout, CustomLayout(id: layout.id, base: .phahTaigi, script: .tl))
    }

    func testCustomLayout_resetKeepsScriptAndTakesNewBase() {
        var layout = CustomLayout(base: .phahTaigi, script: .poj)
        layout.swapKeys(0, 1)
        layout.reset(to: .qwerty)
        XCTAssertEqual(layout, CustomLayout(id: layout.id, base: .qwerty, script: .poj))
        XCTAssertEqual(layout.rows(withGlobe: false), TaigiLayouts.Alphabetic.qwerty_POJ_iPhone)
    }

    func testCustomLayout_decodedRecordValidation() throws {
        let id = UUID().uuidString
        func decode(base: String, keys: [String]) throws -> CustomLayout {
            let json = try JSONSerialization.data(withJSONObject: [
                "id": id, "name": "x", "base": base, "script": "tl", "keys": keys,
            ])
            return try JSONDecoder().decode(CustomLayout.self, from: json)
        }
        XCTAssertTrue(try decode(base: "phahTaigi", keys: Self.phahTaigiKeys).isValid)
        XCTAssertFalse(try decode(base: "phahTaigi", keys: Array(Self.phahTaigiKeys.dropLast())).isValid, "slot count")
        XCTAssertFalse(try decode(base: "phahTaigi", keys: ["e"] + Self.phahTaigiKeys.dropFirst()).isValid, "duplicate")
        XCTAssertFalse(try decode(base: "phahTaigi", keys: ["😀"] + Self.phahTaigiKeys.dropFirst()).isValid, "unknown key")
        XCTAssertFalse(try decode(base: "tps", keys: Self.phahTaigiKeys).isValid, "TPS base")
    }

    // MARK: - Store

    func testUserLayoutStore_capAndUpdate() throws {
        let store = try UserLayoutStore(containerURL: makeDirectory(), onMutated: {})
        let layouts = (0 ..< UserLayoutStore.maxLayouts).map { _ in CustomLayout(base: .moe1, script: .tl) }
        for layout in layouts {
            XCTAssertTrue(store.save(layout))
        }
        XCTAssertFalse(store.save(CustomLayout(base: .moe1, script: .tl)), "past the cap")
        var edited = layouts[0]
        edited.name = "edited"
        XCTAssertTrue(store.save(edited), "an update never hits the cap")
        XCTAssertEqual(store.load().map(\.name).first, "edited")
        XCTAssertTrue(store.delete(id: edited.id))
        XCTAssertEqual(store.load().count, UserLayoutStore.maxLayouts - 1)
    }

    func testUserLayoutStore_dropsInvalidRecords() throws {
        let directory = try makeDirectory()
        let valid = CustomLayout(base: .phahTaigi, script: .tl)
        let invalid: [String: Any] = [
            "id": UUID().uuidString, "name": "bad", "base": "phahTaigi", "script": "tl", "keys": ["e"],
        ]
        let malformed: [String: Any] = ["id": "not-a-uuid", "base": "unknownLayout"]
        let validObject = try JSONSerialization.jsonObject(with: JSONEncoder().encode(valid))
        let data = try JSONSerialization.data(withJSONObject: [validObject, invalid, malformed])
        try data.write(to: directory.appendingPathComponent(UserLayoutStore.fileName))
        XCTAssertEqual(UserLayoutStore(containerURL: directory, onMutated: {}).load(), [valid])
    }

    func testUserLayoutStore_neverOverwritesUnreadableFile() throws {
        let directory = try makeDirectory()
        let fileURL = directory.appendingPathComponent(UserLayoutStore.fileName)
        let garbage = Data("not json".utf8)
        try garbage.write(to: fileURL)
        let store = UserLayoutStore(containerURL: directory, onMutated: {})
        XCTAssertEqual(store.load(), [])
        XCTAssertFalse(store.save(CustomLayout(base: .moe1, script: .tl)))
        XCTAssertEqual(try Data(contentsOf: fileURL), garbage)
    }

    // MARK: - Selection contract

    func testSharedSettings_saveAppliesLayout() throws {
        let settings = try makeSettings()
        let layout = CustomLayout(base: .moe2, script: .poj)
        XCTAssertTrue(settings.saveCustomLayout(layout))
        XCTAssertEqual(settings.keyboardLayoutType, .moe2)
        XCTAssertEqual(settings.activeCustomLayout, layout)
    }

    func testSharedSettings_builtInSelectionClearsCustomEvenForSameBase() throws {
        let settings = try makeSettings()
        XCTAssertTrue(settings.saveCustomLayout(CustomLayout(base: .moe1, script: .tl)))
        settings.keyboardLayoutType = .moe1
        XCTAssertNil(settings.selectedCustomLayoutId)
        XCTAssertNil(settings.activeCustomLayout)
    }

    func testSharedSettings_tpsFromInputModeSuspendsAndResumes() throws {
        let settings = try makeSettings()
        let layout = CustomLayout(base: .qwerty, script: .tl)
        XCTAssertTrue(settings.saveCustomLayout(layout))
        settings.inputMode = .tps
        XCTAssertEqual(settings.keyboardLayoutType, .tps)
        XCTAssertNil(settings.activeCustomLayout)
        settings.inputMode = .tl
        XCTAssertEqual(settings.keyboardLayoutType, .qwerty)
        XCTAssertEqual(settings.activeCustomLayout, layout)
    }

    func testSharedSettings_tpsCardEndsCustomSelection() throws {
        let settings = try makeSettings()
        XCTAssertTrue(settings.saveCustomLayout(CustomLayout(base: .qwerty, script: .tl)))
        settings.keyboardLayoutType = .tps
        settings.keyboardLayoutType = .qwerty
        XCTAssertNil(settings.activeCustomLayout)
    }

    func testSharedSettings_deleteActiveKeepsBase() throws {
        let settings = try makeSettings()
        let layout = CustomLayout(base: .moe1, script: .tl)
        XCTAssertTrue(settings.saveCustomLayout(layout))
        XCTAssertTrue(settings.deleteCustomLayout(id: layout.id))
        XCTAssertNil(settings.selectedCustomLayoutId)
        XCTAssertEqual(settings.keyboardLayoutType, .moe1)
    }

    func testSharedSettings_resetClearsSelectionAndKeepsLayouts() throws {
        let settings = try makeSettings()
        XCTAssertTrue(settings.saveCustomLayout(CustomLayout(base: .moe1, script: .tl)))
        settings.resetToDefaults()
        XCTAssertNil(settings.selectedCustomLayoutId)
        XCTAssertEqual(settings.loadCustomLayouts().count, 1)
    }

    // MARK: - Helpers

    private func makeDirectory() throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return directory
    }

    private func makeSettings() throws -> SharedSettings {
        try SharedSettings(userDefaults: defaults, themesContainerURL: makeDirectory())
    }
}
