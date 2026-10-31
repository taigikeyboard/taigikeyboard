import KeyboardKit
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

    // trace: moe2_TL_iPhone row 1 ph ts tsh oo, row 2 th, row 3 kh ng nn; moe2_POJ_iPhone ch chh.
    private static let multiLetterKeys = ["ph", "ts", "tsh", "oo", "th", "kh", "ng", "nn", "ch", "chh"]

    // trace: CustomLayout.trayFinals, most frequent first; they close the letters.
    private static let finals = [
        "an", "ia", "in", "au", "ong", "ian", "un", "ai", "iu", "ann", "ang", "ui", "inn", "iau", "er", "ir",
    ]

    // trace: phahTaigi function slots 0 shift, 1 backspace, 2 numeric, 3 emoji, 4 space, …
    private static let phahTaigiEmoji = CustomLayout.Place.function(3)

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

    func testCustomLayout_baseKeepsItsOwnKeyForms() {
        // trace: MOE1 `(` `)` have no full-width form; the symbol page's do, and must not leak in.
        XCTAssertEqual(CustomLayout(base: .moe1, script: .tl).rows(withGlobe: false), TaigiLayouts.Alphabetic.moe1_TL_iPhone)
    }

    func testCustomLayout_spareLettersAndSymbolsMissingFromBoard() {
        let phahTaigi = CustomLayout(base: .phahTaigi, script: .tl)
        // trace: tray letters a–z + o͘ + ⁿ + ee + "-", then the romanization tables' multi-letter keys in
        // table order (MOE2 TL rows 1–3, then MOE2 POJ `ch` `chh`), then the finals, minus the board. PhahTaigi
        // lacks q w x z, o͘, ⁿ, ee and every multi-letter key.
        XCTAssertEqual(phahTaigi.spareLetters, ["q", "w", "x", "z", "o\u{0358}", "\u{207F}", "ee"] + Self.multiLetterKeys + Self.finals)
        // trace: symbol page = 37 keys, "-" counts as a letter → 36; PhahTaigi holds . , ? ! → 32.
        XCTAssertEqual(phahTaigi.spareSymbols.count, 32)
        XCTAssertEqual(Array(phahTaigi.spareSymbols.prefix(3)), ["~", "/", "="])
        XCTAssertFalse(phahTaigi.spareSymbols.contains(","))
        // trace: Lohankha POJ carries every letter and o͘ (row 2 end); its bottom row holds ",".
        let lohankha = CustomLayout(base: .qwerty, script: .poj)
        XCTAssertEqual(lohankha.spareLetters, ["\u{207F}", "ee"] + Self.multiLetterKeys + Self.finals)
        XCTAssertEqual(lohankha.spareSymbols.count, 35)
    }

    func testCustomLayout_swapMovesWholeKeyIncludingFullWidthForm() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        // Slot 20 is "," (row 3, column 1), slot 22 is "c" (row 3, column 3).
        layout.swap(.key(20), .key(22))
        let rows = layout.rows(withGlobe: false)
        XCTAssertEqual(rows[3][1], .char("c"))
        XCTAssertEqual(rows[3][3], .char(",", fullWidth: "，"))
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_replaceTradesKeyWithSpare() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.replace(.key(0), with: "q")
        XCTAssertEqual(layout.rows(withGlobe: false)[1][0], .char("q"))
        XCTAssertEqual(layout.spareLetters, ["w", "x", "z", "o\u{0358}", "\u{207F}", "ee"] + Self.multiLetterKeys + Self.finals)
        XCTAssertTrue(layout.spareSymbols.contains("!"))
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_trayFinalIsPlainKey() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        // Slot 2 is "e" (row 1, column 2); no built-in table has "ang".
        layout.replace(.key(2), with: "ang")
        XCTAssertEqual(layout.rows(withGlobe: false)[1][2], .char("ang"))
        XCTAssertFalse(layout.spareLetters.contains("ang"))
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_trayNasalMarkerIsPlainKey() {
        var layout = CustomLayout(base: .qwerty, script: .poj)
        // Slot 0 is "q" (row 1, column 0); no built-in table has ⁿ on the board.
        layout.replace(.key(0), with: "\u{207F}")
        XCTAssertEqual(layout.rows(withGlobe: false)[1][0], .char("\u{207F}"))
        XCTAssertFalse(layout.spareLetters.contains("\u{207F}"))
        XCTAssertTrue(layout.isValid)
    }

    func testCustomLayout_traySymbolKeepsFullWidthForm() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        // Slot 2 is "e" (row 1, column 2); the symbol page's "(" carries "（".
        layout.replace(.key(2), with: "(")
        XCTAssertEqual(layout.rows(withGlobe: false)[1][2], .char("(", fullWidth: "（"))
    }

    func testCustomLayout_removedKeyLeavesKeyboardRowButStaysInEditor() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.remove(.key(0))
        // trace: row 1 without "!" → ? e r t y u i o p.
        XCTAssertEqual(layout.rows(withGlobe: false)[1], Array(TaigiLayouts.Alphabetic.phahTaigi_iPhone[1].dropFirst()))
        XCTAssertEqual(layout.editorRows[1][0], CustomLayout.removedSlot)
        XCTAssertTrue(layout.spareSymbols.contains("!"))
        XCTAssertTrue(layout.isValid)
        layout.replace(.key(0), with: "q")
        XCTAssertEqual(layout.rows(withGlobe: false)[1][0], .char("q"))
    }

    func testCustomLayout_rowLeftEmptyLeavesKeyboard() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        // Slots 0–9 are row 1, which has no function key.
        for slot in 0 ..< 10 {
            layout.remove(.key(slot))
        }
        XCTAssertEqual(layout.rows(withGlobe: false).count, TaigiLayouts.Alphabetic.phahTaigi_iPhone.count - 1)
        XCTAssertEqual(layout.editorRows.count, TaigiLayouts.Alphabetic.phahTaigi_iPhone.count)
        // Later slots keep their numbers: slot 10 is still "a" (editor row 2, column 0).
        XCTAssertEqual(CustomLayout.places(of: layout.editorRows)[2][0], .key(10))
        XCTAssertEqual(layout.editorRows[2][0], .char("a"))
        // Swapping a key into an empty place moves it there.
        layout.swap(.key(0), .key(10))
        XCTAssertEqual(layout.rows(withGlobe: false)[1], [.char("a")])
        XCTAssertEqual(layout.editorRows[2][0], CustomLayout.removedSlot)
    }

    func testCustomLayout_inputTypeKeyFollowsRemovedComma() {
        var layout = CustomLayout(base: .qwerty, script: .tl)
        // trace: Lohankha TL slots = row 1 (10) + row 2 (9) + row 3 (7); slot 26 is the bottom-row ",".
        layout.remove(.key(26))
        let rows = CustomLayoutService.applyInputTypeKey(layout.rows(withGlobe: false), inputType: .emailAddress)
        // No "," left on the bottom row, so "@" goes right after the emoji key.
        XCTAssertEqual(rows.last, [.numeric, .emoji, .char("@"), .space, .char("-"), .translate, .return])
    }

    func testCustomLayout_inputTypeKeyWithoutEmojiKey() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.remove(Self.phahTaigiEmoji)
        let plain = CustomLayoutService.applyInputTypeKey(layout.rows(withGlobe: false), inputType: .url)
        XCTAssertEqual(plain.last, [.numeric, .char("/"), .space, .translate, .return])
        let globe = CustomLayoutService.applyInputTypeKey(layout.rows(withGlobe: true), inputType: .emailAddress)
        XCTAssertEqual(globe.last, [.numeric, .globe, .char("@"), .space, .translate, .return])
    }

    func testCustomLayout_emojiKeyRemovesAndComesBackOnlyToItsPlace() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        XCTAssertEqual(CustomLayout.places(of: layout.editorRows)[4][1], Self.phahTaigiEmoji)
        XCTAssertEqual(layout.emojiPlace, Self.phahTaigiEmoji)
        layout.remove(Self.phahTaigiEmoji)
        XCTAssertEqual(layout.rows(withGlobe: false).last, [.numeric, .space, .translate, .return])
        XCTAssertEqual(layout.editorRows.last, TaigiLayouts.Alphabetic.phahTaigi_iPhone.last)
        XCTAssertEqual(layout.removedPlaces, [Self.phahTaigiEmoji])
        XCTAssertFalse(layout.isUnchanged)
        // A letter never fills the emoji place, and other function keys never leave.
        layout.replace(Self.phahTaigiEmoji, with: "q")
        XCTAssertFalse(layout.canRemove(.function(0)))
        layout.restoreEmojiKey()
        XCTAssertTrue(layout.isUnchanged)
    }

    func testCustomLayout_functionKeysSwapOnlyWithinTheirRow() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        // trace: function slots 0 shift, 1 backspace (row 3); 2 numeric … 6 return (row 4).
        XCTAssertFalse(layout.canSwap(.function(0), .function(2)), "shift and 123 sit in different rows")
        XCTAssertFalse(layout.canSwap(.function(2), .key(0)), "a function key never trades with a letter")
        layout.swap(.function(2), .function(6))
        XCTAssertEqual(layout.rows(withGlobe: false).last, [.return, .emoji, .space, .translate, .numeric])
        // The globe key follows 123 wherever it went.
        XCTAssertEqual(layout.rows(withGlobe: true).last, [.return, .emoji, .space, .translate, .numeric, .globe])
        // The email key goes before space whatever the order.
        let email = CustomLayoutService.applyInputTypeKey(layout.rows(withGlobe: false), inputType: .emailAddress)
        XCTAssertEqual(email.last, [.return, .emoji, .char("@"), .space, .translate, .numeric])
        XCTAssertTrue(layout.isValid)
        XCTAssertFalse(layout.isUnchanged)
    }

    func testCustomLayout_recordMovingFunctionKeyAcrossRowsIsInvalid() throws {
        let json = try JSONSerialization.data(withJSONObject: [
            "id": UUID().uuidString, "name": "x", "base": "phahTaigi", "script": "tl", "keys": Self.phahTaigiKeys,
            "functionKeys": ["numeric", "backspace", "shift", "emoji", "space", "translate", "return"],
        ])
        XCTAssertFalse(try JSONDecoder().decode(CustomLayout.self, from: json).isValid)
    }

    func testCustomLayout_recordWithoutNewerFieldsKeepsBaseFunctionKeys() throws {
        let json = try JSONSerialization.data(withJSONObject: [
            "id": UUID().uuidString, "name": "x", "base": "phahTaigi", "script": "tl", "keys": Self.phahTaigiKeys,
        ])
        let layout = try JSONDecoder().decode(CustomLayout.self, from: json)
        XCTAssertTrue(layout.hasEmojiKey)
        XCTAssertTrue(layout.isUnchanged)
    }

    func testCustomLayout_codableRoundTripKeepsRemovedKeys() throws {
        var layout = CustomLayout(name: "x", base: .moe2, script: .poj)
        layout.remove(.key(0))
        layout.remove(.key(5))
        layout.swap(.function(0), .function(1))
        // trace: moe2 bottom row function keys numeric, emoji, space, translate, return → emoji = 3.
        layout.remove(.function(3))
        let decoded = try JSONDecoder().decode(CustomLayout.self, from: JSONEncoder().encode(layout))
        XCTAssertEqual(decoded, layout)
        XCTAssertTrue(decoded.isValid)
    }

    func testCustomLayout_replaceIgnoresKeyAlreadyOnBoardOrUnknown() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.replace(.key(0), with: "e")
        layout.replace(.key(0), with: "😀")
        XCTAssertEqual(layout, CustomLayout(id: layout.id, base: .phahTaigi, script: .tl))
    }

    func testCustomLayout_resetKeepsScriptAndTakesNewBase() {
        var layout = CustomLayout(base: .phahTaigi, script: .poj)
        layout.swap(.key(0), .key(1))
        layout.reset(to: .qwerty)
        XCTAssertEqual(layout, CustomLayout(id: layout.id, base: .qwerty, script: .poj))
        XCTAssertEqual(layout.rows(withGlobe: false), TaigiLayouts.Alphabetic.qwerty_POJ_iPhone)
    }

    // MARK: - Long-press lists

    func testCallouts_defaultIsBuiltInList() {
        let layout = CustomLayout(base: .phahTaigi, script: .tl)
        // trace: PhahTaigi has no layout-specific map → SymbolCallouts[","] = ，、
        XCTAssertEqual(layout.callouts(for: ","), ["，", "、"])
        XCTAssertEqual(layout.callouts(for: "a"), TaigiCallouts.calloutChars(for: "a", inputMode: .tl, layoutType: .phahTaigi))
        XCTAssertNil(layout.calloutOverride(for: ","))
    }

    func testCallouts_setNormalisesAndReplacesBuiltInList() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.setCallouts([" ; ", "", ":", ";", "1", "2", "3", "4", "5", "6", "7"], for: ",")
        // trace: trimmed, blank and repeated ";" dropped, capped at 8.
        XCTAssertEqual(layout.callouts(for: ","), [";", ":", "1", "2", "3", "4", "5", "6"])
        XCTAssertFalse(layout.isUnchanged)
        layout.setCallouts([], for: ".")
        XCTAssertEqual(layout.calloutOverride(for: "."), [], "an empty list leaves the key without a long-press")
    }

    func testCallouts_listEqualToBuiltInIsNotStored() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.setCallouts([";"], for: ",")
        layout.setCallouts(["，", "、"], for: ",")
        XCTAssertNil(layout.callouts[","])
        XCTAssertTrue(layout.isUnchanged)
    }

    func testCallouts_overrideFollowsFullWidthFormAndShift() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.setCallouts([";", ":"], for: ",")
        layout.setCallouts(["á", "à"], for: "a")
        // trace: PhahTaigi "," is .char(",", fullWidth: "，"); full-width mode sends "，".
        XCTAssertEqual(layout.calloutOverride(for: "，"), [";", ":"])
        XCTAssertEqual(layout.calloutOverride(for: "A"), ["Á", "À"])
        XCTAssertNil(layout.calloutOverride(for: "e"))
    }

    func testCallouts_followKeyThroughSwapAndClearOnReset() throws {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.setCallouts(["{"], for: ",")
        // Slot 20 is ",", slot 22 is "c".
        layout.swap(.key(20), .key(22))
        XCTAssertEqual(layout.rows(withGlobe: false)[3][3], .char(",", fullWidth: "，"))
        XCTAssertEqual(layout.calloutOverride(for: ","), ["{"])
        XCTAssertEqual(layout.calloutOverride(for: "，"), ["{"])
        let decoded = try JSONDecoder().decode(CustomLayout.self, from: JSONEncoder().encode(layout))
        XCTAssertEqual(decoded, layout)
        layout.reset(to: .phahTaigi)
        XCTAssertTrue(layout.callouts.isEmpty)
    }

    func testCallouts_keyLeavingBoardDropsItsList() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.setCallouts(["{"], for: ",")
        layout.setCallouts(["1"], for: "e")
        // Slot 20 is ",", slot 2 is "e"; "q" is a spare letter.
        layout.remove(.key(20))
        layout.replace(.key(2), with: "q")
        XCTAssertTrue(layout.callouts.isEmpty)
        XCTAssertNil(layout.calloutOverride(for: ","))
        layout.replace(.key(20), with: ",")
        XCTAssertNil(layout.calloutOverride(for: ","), "a key back from the tray has the built-in list")
    }

    func testCallouts_customListOnlyOnLetterPageOfTaigiMode() {
        var layout = CustomLayout(base: .phahTaigi, script: .tl)
        layout.setCallouts([";"], for: ",")
        func chars(_ inputMode: InputMode, _ keyboardType: Keyboard.KeyboardType) -> [String]? {
            TaigiCallouts.customCalloutChars(for: ",", inputMode: inputMode, keyboardType: keyboardType, customLayout: layout)
        }
        XCTAssertEqual(chars(.tl, .alphabetic), [";"])
        XCTAssertEqual(chars(.poj, .alphabetic), [";"])
        XCTAssertNil(chars(.english, .alphabetic))
        XCTAssertNil(chars(.tl, .numeric))
        XCTAssertNil(chars(.tl, .symbolic))
    }

    func testCustomLayout_decodedRecordValidation() throws {
        let id = UUID().uuidString
        func decode(base: String, keys: [Any]) throws -> CustomLayout {
            let json = try JSONSerialization.data(withJSONObject: [
                "id": id, "name": "x", "base": base, "script": "tl", "keys": keys,
            ])
            return try JSONDecoder().decode(CustomLayout.self, from: json)
        }
        XCTAssertTrue(try decode(base: "phahTaigi", keys: Self.phahTaigiKeys).isValid)
        XCTAssertTrue(try decode(base: "phahTaigi", keys: [NSNull()] + Self.phahTaigiKeys.dropFirst().map { $0 as Any }).isValid, "removed key")
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
