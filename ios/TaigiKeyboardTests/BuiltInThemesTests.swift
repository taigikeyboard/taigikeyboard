@testable import TaigiKeyboard
import SwiftUI
import XCTest

/// Tests for the built-in theme table (v3.6.2 PR-2b).
///
/// The id invariants are load-bearing: `SharedSettings.resolvedAppearance(for:)`
/// uses `UUID(uuidString:) != nil` to decide whether to read the user-theme
/// file, and `ThemeId.default` is the legacy-buffer sentinel. A built-in id
/// that is a UUID string or equals "default" would be mis-routed.
final class BuiltInThemesTests: XCTestCase {
    // trace: every shipped built-in must have at least one variant, or colors(for:) degrades to .default
    func testAll_eachThemeHasAtLeastOneVariant() {
        for theme in BuiltInThemes.all {
            XCTAssertTrue(
                theme.light != nil || theme.dark != nil,
                "Built-in '\(theme.id)' has neither light nor dark variant",
            )
        }
    }

    // trace: theme(id:) lookup is by id equality → ids must be unique
    func testAll_idsAreUnique() {
        let ids = BuiltInThemes.all.map(\.id)
        XCTAssertEqual(ids.count, Set(ids).count, "Built-in ids must be unique: \(ids)")
    }

    // trace: id == "default" would collide with the legacy-buffer sentinel
    func testAll_idsAreNotDefaultSentinel() {
        for theme in BuiltInThemes.all {
            XCTAssertNotEqual(theme.id, ThemeId.default, "Built-in id must not be the 'default' sentinel")
        }
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
        XCTAssertEqual(BuiltInThemes.theme(id: "catppuccin")?.id, "catppuccin")
        XCTAssertNil(BuiltInThemes.theme(id: "no_such_theme"))
        XCTAssertNil(BuiltInThemes.theme(id: ThemeId.default))
    }

    // trace: a dark-only built-in (light == nil) → .light request falls back to the dark variant.
    // Uses a synthetic theme: every shipped built-in currently has both variants, so Nord no longer
    // exercises this branch (it has an authored light). Test the ladder directly via colors(for:).
    func testColorsForScheme_darkOnlyFallsBackToDark() {
        let darkColors = BuiltInThemes.theme(id: "nord")!.colors(for: .dark)
        let darkOnly = BuiltInTheme(id: "test_dark_only", displayName: "Dark Only", light: nil, dark: darkColors)
        XCTAssertEqual(darkOnly.colors(for: .light), darkColors, "light request must fall back to dark when no light")
        XCTAssertEqual(darkOnly.colors(for: .dark), darkColors)
    }

    // trace: a light-only built-in (dark == nil) → .dark request falls back to the light variant
    func testColorsForScheme_lightOnlyFallsBackToLight() {
        let lightColors = BuiltInThemes.theme(id: "catppuccin")!.colors(for: .light)
        let lightOnly = BuiltInTheme(id: "test_light_only", displayName: "Light Only", light: lightColors, dark: nil)
        XCTAssertEqual(lightOnly.colors(for: .dark), lightColors, "dark request must fall back to light when no dark")
        XCTAssertEqual(lightOnly.colors(for: .light), lightColors)
    }

    // trace: colors(for:) returns the requested variant when both exist
    func testColorsForScheme_picksRequestedVariant() {
        let catppuccin = BuiltInThemes.theme(id: "catppuccin")!
        XCTAssertEqual(catppuccin.colors(for: .light), catppuccin.light)
        XCTAssertEqual(catppuccin.colors(for: .dark), catppuccin.dark)
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
}
