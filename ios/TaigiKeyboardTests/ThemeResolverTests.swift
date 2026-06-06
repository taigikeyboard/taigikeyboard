@testable import TaigiKeyboard
import SwiftUI
import XCTest

/// Tests for `ThemeResolver` — the pure mapping from a selected theme id to
/// the `ResolvedKeyboardTheme` the renderer consumes (v3.6.2 PR-2a).
///
/// Behavior under test:
/// - `default` → the legacy free-pick buffer verbatim (so uncustomized users
///   stay all-nil and customized users keep their look with no migration).
/// - a known `UserTheme` id → that theme's colors + shadow intensity.
/// - an unknown id (built-in id before PR-2b, or a deleted user theme) →
///   fallback to the legacy buffer.
final class ThemeResolverTests: XCTestCase {
    private func makeUserTheme(
        id: UUID,
        colors: KeyboardColorSettings,
        shadow: Double = 0,
    ) -> UserTheme {
        UserTheme(
            id: id,
            name: "T",
            colors: colors,
            keyShadowIntensity: shadow,
            createdAt: Date(timeIntervalSince1970: 0),
            updatedAt: Date(timeIntervalSince1970: 0),
        )
    }

    private func customized() -> KeyboardColorSettings {
        var cs = KeyboardColorSettings()
        cs.backgroundColor = CodableColor(.red)
        return cs
    }

    // trace: themeId == ThemeId.default → ResolvedKeyboardTheme(colors: legacy, intensity: 0)
    func testResolved_default_returnsLegacyBufferAllNil() {
        let resolved = ThemeResolver.resolved(
            themeId: ThemeId.default,
            legacyColorSettings: .default,
            userThemes: [],
        )
        XCTAssertEqual(resolved, ResolvedKeyboardTheme(colors: .default, keyShadowIntensity: 0))
    }

    // trace: default user who customized colorSettings stays on "default" → keeps their colors
    func testResolved_default_returnsCustomizedLegacyVerbatim() {
        let legacy = customized()
        let resolved = ThemeResolver.resolved(
            themeId: ThemeId.default,
            legacyColorSettings: legacy,
            userThemes: [],
        )
        XCTAssertEqual(resolved.colors, legacy)
        XCTAssertEqual(resolved.keyShadowIntensity, 0)
    }

    // trace: themeId matches a user theme → that theme's colors + shadow
    func testResolved_knownUserTheme_returnsThemeColorsAndShadow() {
        let id = UUID()
        let themeColors = customized()
        let theme = makeUserTheme(id: id, colors: themeColors, shadow: 0.4)
        let resolved = ThemeResolver.resolved(
            themeId: id.uuidString,
            legacyColorSettings: .default,
            userThemes: [theme],
        )
        XCTAssertEqual(resolved, ResolvedKeyboardTheme(colors: themeColors, keyShadowIntensity: 0.4))
    }

    // trace: unknown id (e.g. a built-in id before PR-2b) → fallback to legacy buffer
    func testResolved_unknownId_fallsBackToLegacyBuffer() {
        let legacy = customized()
        let resolved = ThemeResolver.resolved(
            themeId: "catppuccin",
            legacyColorSettings: legacy,
            userThemes: [],
        )
        XCTAssertEqual(resolved, ResolvedKeyboardTheme(colors: legacy, keyShadowIntensity: 0))
    }

    // trace: a previously-selected user theme was deleted (id no longer in list) → fallback legacy
    func testResolved_deletedUserTheme_fallsBackToLegacyBuffer() {
        let staleId = UUID()
        let other = makeUserTheme(id: UUID(), colors: customized())
        let resolved = ThemeResolver.resolved(
            themeId: staleId.uuidString,
            legacyColorSettings: .default,
            userThemes: [other],
        )
        XCTAssertEqual(resolved, ResolvedKeyboardTheme(colors: .default, keyShadowIntensity: 0))
    }
}
