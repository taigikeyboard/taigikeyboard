import SwiftUI
@testable import TaigiKeyboard
import XCTest

/// Tests for `UserThemeStore` — the App Group JSON persistence for
/// user-created themes (v3.6.2 PR-2a). Each test uses a fresh temp directory
/// instead of the real App Group container.
///
/// Coverage:
/// - add / load round-trip; mutation bumps the revision callback.
/// - cap of `maxUserThemes` (USER 2026-06-07): the 6th add no-ops.
/// - update replaces by id; delete removes by id.
/// - corrupt file and nil container degrade to empty, never crash.
final class UserThemeStoreTests: XCTestCase {
    private var tempDir: URL!

    override func setUp() {
        super.setUp()
        tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("UserThemeStoreTests.\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: tempDir)
        tempDir = nil
        super.tearDown()
    }

    private func makeUserTheme(name: String = "T", shadow: Double = 0) -> UserTheme {
        var colors = KeyboardColorSettings()
        colors.backgroundColor = CodableColor(.blue)
        var appearance = ThemeAppearance.default
        appearance.colors = colors
        appearance.keyShadowIntensity = shadow
        return UserTheme(
            id: UUID(),
            name: name,
            appearance: appearance,
            createdAt: Date(timeIntervalSince1970: 0),
            updatedAt: Date(timeIntervalSince1970: 0),
        )
    }

    func testLoad_emptyStore_returnsEmpty() {
        let store = UserThemeStore(containerURL: tempDir, onMutated: {})
        XCTAssertEqual(store.load(), [])
    }

    func testAddAndLoad_roundTrip_bumpsRevisionOnce() {
        var mutationCount = 0
        let store = UserThemeStore(containerURL: tempDir, onMutated: { mutationCount += 1 })
        let theme = makeUserTheme(name: "Mine", shadow: 0.3)

        XCTAssertTrue(store.add(theme))

        XCTAssertEqual(store.load(), [theme])
        XCTAssertEqual(mutationCount, 1)
    }

    // trace: cap = 5 → add 5 succeed, 6th rejected, list stays 5
    func testAdd_atCap_rejectsSixth() {
        let store = UserThemeStore(containerURL: tempDir, onMutated: {})
        for index in 0 ..< UserThemeStore.maxUserThemes {
            XCTAssertTrue(store.add(makeUserTheme(name: "T\(index)")))
        }
        XCTAssertFalse(store.add(makeUserTheme(name: "overflow")))
        XCTAssertEqual(store.load().count, UserThemeStore.maxUserThemes)
    }

    func testUpdate_replacesById() {
        let store = UserThemeStore(containerURL: tempDir, onMutated: {})
        var theme = makeUserTheme(name: "Before")
        store.add(theme)

        theme.name = "After"
        store.update(theme)

        XCTAssertEqual(store.load().map(\.name), ["After"])
    }

    func testDelete_removesById() {
        let store = UserThemeStore(containerURL: tempDir, onMutated: {})
        let keep = makeUserTheme(name: "Keep")
        let drop = makeUserTheme(name: "Drop")
        store.add(keep)
        store.add(drop)

        store.delete(id: drop.id)

        XCTAssertEqual(store.load().map(\.name), ["Keep"])
    }

    func testLoad_corruptFile_returnsEmpty() {
        let fileURL = tempDir.appendingPathComponent("user_themes.json")
        try? Data("not json".utf8).write(to: fileURL)
        let store = UserThemeStore(containerURL: tempDir, onMutated: {})
        XCTAssertEqual(store.load(), [])
    }

    func testNilContainer_degradesToEmpty() {
        let store = UserThemeStore(containerURL: nil, onMutated: {})
        XCTAssertEqual(store.load(), [])
        // add cannot persist without a container → returns false, load stays empty.
        XCTAssertFalse(store.add(makeUserTheme()))
        XCTAssertEqual(store.load(), [])
    }

    // trace: a theme with custom sizes survives a write/read round-trip intact.
    func testAddAndLoad_fullAppearance_roundTrips() {
        let store = UserThemeStore(containerURL: tempDir, onMutated: {})
        var appearance = ThemeAppearance.default
        appearance.colors.backgroundColor = CodableColor(.green)
        appearance.keyHeightScale = 1.1
        appearance.keyFontSizeScale = 0.9
        appearance.candidateTextSizeScale = 1.05
        appearance.keyCornerRadius = 12
        appearance.keyBorderWidth = 1.5
        appearance.keyShadowIntensity = 0.4
        let theme = UserTheme(
            id: UUID(),
            name: "Full",
            appearance: appearance,
            createdAt: Date(timeIntervalSince1970: 0),
            updatedAt: Date(timeIntervalSince1970: 0),
        )

        XCTAssertTrue(store.add(theme))

        XCTAssertEqual(store.load(), [theme])
    }

    // trace: a theme file written before the size fields existed (only colors +
    // keyShadowIntensity inside `appearance`) decodes with the new fields filled
    // from ThemeAppearance.default — no theme is lost when the schema grows.
    // Also: a legacy theme carrying the now-removed "fontType" key still decodes
    // (Codable ignores unknown keys) — locks the font-walk-back backward-compat.
    func testThemeAppearance_decodesPartialJSON_fillsMissingWithDefaults() throws {
        let json = Data(#"{ "colors": {}, "keyShadowIntensity": 0.25, "fontType": "iansui" }"#.utf8)
        let appearance = try JSONDecoder().decode(ThemeAppearance.self, from: json)

        XCTAssertEqual(appearance.keyShadowIntensity, 0.25)
        XCTAssertEqual(appearance.colors, .default)
        XCTAssertEqual(appearance.keyHeightScale, ThemeAppearance.default.keyHeightScale)
        XCTAssertEqual(appearance.keyFontSizeScale, ThemeAppearance.default.keyFontSizeScale)
        XCTAssertEqual(appearance.candidateTextSizeScale, ThemeAppearance.default.candidateTextSizeScale)
        XCTAssertEqual(appearance.keyCornerRadius, ThemeAppearance.default.keyCornerRadius)
        XCTAssertEqual(appearance.keyBorderWidth, ThemeAppearance.default.keyBorderWidth)
    }
}
