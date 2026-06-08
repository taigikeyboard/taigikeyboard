@testable import TaigiKeyboard
import SwiftUI
import XCTest

/// Tests for the built-in theme catalog (v3.6.2 scaffold stage).
///
/// The id invariants are load-bearing: `SharedSettings.resolvedAppearance(for:)`
/// uses `UUID(uuidString:) != nil` to decide whether to read the user-theme
/// file, and `ThemeId.default` is the legacy-buffer sentinel. A built-in id
/// that is a UUID string or equals "default" would be mis-routed.
///
/// Scaffold stage: themes carry a `previewImageName` (card screenshot slot) and
/// no palette yet (`light`/`dark` == nil → `colors(for:)` degrades to `.default`).
final class BuiltInThemesTests: XCTestCase {
    // trace: families flatten into `all`; a non-empty catalog is required for the picker shelves
    func testFamilies_flattenIntoAll() {
        XCTAssertFalse(BuiltInThemes.families.isEmpty, "catalog must have at least one family")
        let flattened = BuiltInThemes.families.flatMap(\.themes)
        XCTAssertEqual(BuiltInThemes.all.map(\.id), flattened.map(\.id), "all must equal families flattened")
    }

    // trace: every catalog theme carries a screenshot slot so the card has a preview asset to load
    func testAll_eachThemeHasPreviewImageName() {
        for theme in BuiltInThemes.all {
            XCTAssertNotNil(theme.previewImageName, "Built-in '\(theme.id)' must carry a screenshot slot name")
        }
    }

    // trace: the Standard family head IS the app default (id == sentinel) → reset shows it selected
    func testStandardHead_isDefaultSentinel() {
        let head = BuiltInThemes.families.first?.themes.first
        XCTAssertEqual(head?.id, ThemeId.default)
        XCTAssertEqual(head?.displayName, "預設")
    }

    // trace: only the Standard head may be the default sentinel — any other default id would mis-route
    func testAll_onlyHeadIsDefaultSentinel() {
        XCTAssertEqual(BuiltInThemes.all.filter { $0.id == ThemeId.default }.count, 1)
    }

    // trace: theme(id:) lookup is by id equality → ids must be unique
    func testAll_idsAreUnique() {
        let ids = BuiltInThemes.all.map(\.id)
        XCTAssertEqual(ids.count, Set(ids).count, "Built-in ids must be unique: \(ids)")
    }

    // trace: a UUID-string id would make SharedSettings read the user-theme file for a built-in
    func testAll_idsAreNotUuidStrings() {
        for theme in BuiltInThemes.all {
            XCTAssertNil(
                UUID(uuidString: theme.id),
                "Built-in id '\(theme.id)' must not parse as a UUID (would mis-route to user-theme file)",
            )
        }
    }

    // trace: theme(id:) returns the matching theme and nil for a non-built-in id
    func testThemeById_returnsMatchOrNil() {
        XCTAssertEqual(BuiltInThemes.theme(id: "standardBlue")?.id, "standardBlue")
        XCTAssertNil(BuiltInThemes.theme(id: "no_such_theme"))
        // The default sentinel resolves to the Standard head (the app default card).
        XCTAssertEqual(BuiltInThemes.theme(id: ThemeId.default)?.displayName, "預設")
    }

    // trace: a still-scaffold theme (no palette) → colors(for:) degrades to .default in both schemes
    func testColorsForScheme_scaffoldThemeDegradesToDefault() {
        let theme = BuiltInThemes.theme(id: "swiftyBlue")!
        XCTAssertEqual(theme.colors(for: .light), .default)
        XCTAssertEqual(theme.colors(for: .dark), .default)
    }

    // trace: the Standard gradient themes each carry a ≥2-stop background gradient in both schemes
    func testStandardGradientThemes_carryGradient() {
        for id in ["standardPink", "standardGold", "standardBlue", "standardGreen", "standardPurple"] {
            let theme = BuiltInThemes.theme(id: id)!
            XCTAssertTrue(theme.colors(for: .light).hasBackgroundGradient, "\(id) light must carry a gradient")
            XCTAssertTrue(theme.colors(for: .dark).hasBackgroundGradient, "\(id) dark must carry a gradient")
        }
    }

    // trace: a dark-only theme (light == nil) → .light request falls back to the dark variant
    func testColorsForScheme_darkOnlyFallsBackToDark() {
        let darkColors = makeColors(hex: 0x112233)
        let darkOnly = BuiltInTheme(id: "test_dark_only", displayName: "Dark Only", light: nil, dark: darkColors)
        XCTAssertEqual(darkOnly.colors(for: .light), darkColors, "light request must fall back to dark when no light")
        XCTAssertEqual(darkOnly.colors(for: .dark), darkColors)
    }

    // trace: a light-only theme (dark == nil) → .dark request falls back to the light variant
    func testColorsForScheme_lightOnlyFallsBackToLight() {
        let lightColors = makeColors(hex: 0xAABBCC)
        let lightOnly = BuiltInTheme(id: "test_light_only", displayName: "Light Only", light: lightColors, dark: nil)
        XCTAssertEqual(lightOnly.colors(for: .dark), lightColors, "dark request must fall back to light when no dark")
        XCTAssertEqual(lightOnly.colors(for: .light), lightColors)
    }

    // trace: colors(for:) returns the requested variant when both exist
    func testColorsForScheme_picksRequestedVariant() {
        let light = makeColors(hex: 0x111111)
        let dark = makeColors(hex: 0x222222)
        let theme = BuiltInTheme(id: "test_both", displayName: "Both", light: light, dark: dark)
        XCTAssertEqual(theme.colors(for: .light), light)
        XCTAssertEqual(theme.colors(for: .dark), dark)
    }

    // trace: CodableColor(hex:) — 0x1E1E2E → (30,30,46)/255, alpha 1; high 8 bits ignored
    func testCodableColorHex_decodesRGBComponents() {
        let color = CodableColor(hex: 0x1E1E2E)
        XCTAssertEqual(color.red, 30.0 / 255.0, accuracy: 1e-9)
        XCTAssertEqual(color.green, 30.0 / 255.0, accuracy: 1e-9)
        XCTAssertEqual(color.blue, 46.0 / 255.0, accuracy: 1e-9)
        XCTAssertEqual(color.alpha, 1.0, accuracy: 1e-9)
    }

    // trace: high 8 bits are masked → 0xFF1E1E2E resolves identically to 0x1E1E2E
    func testCodableColorHex_ignoresHighByte() {
        XCTAssertEqual(CodableColor(hex: 0xFF1E1E2E), CodableColor(hex: 0x1E1E2E))
    }

    // MARK: - Helpers

    private func makeColors(hex: UInt32) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.backgroundColor = CodableColor(hex: hex)
        return colors
    }
}
