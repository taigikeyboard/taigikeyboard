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
            colorScheme: .light,
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
            colorScheme: .light,
            legacyColorSettings: legacy,
            userThemes: [],
        )
        XCTAssertEqual(resolved.colors, legacy)
        XCTAssertEqual(resolved.keyShadowIntensity, 0)
    }

    // trace: themeId matches a user theme → that theme's colors + shadow (colorScheme ignored for user themes)
    func testResolved_knownUserTheme_returnsThemeColorsAndShadow() {
        let id = UUID()
        let themeColors = customized()
        let theme = makeUserTheme(id: id, colors: themeColors, shadow: 0.4)
        let resolved = ThemeResolver.resolved(
            themeId: id.uuidString,
            colorScheme: .dark,
            legacyColorSettings: .default,
            userThemes: [theme],
        )
        XCTAssertEqual(resolved, ResolvedKeyboardTheme(colors: themeColors, keyShadowIntensity: 0.4))
    }

    // trace: a genuinely unknown id (not "default", not a user UUID, not a built-in) → fallback to legacy buffer
    func testResolved_unknownId_fallsBackToLegacyBuffer() {
        let legacy = customized()
        let resolved = ThemeResolver.resolved(
            themeId: "no_such_theme",
            colorScheme: .light,
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
            colorScheme: .light,
            legacyColorSettings: .default,
            userThemes: [other],
        )
        XCTAssertEqual(resolved, ResolvedKeyboardTheme(colors: .default, keyShadowIntensity: 0))
    }

    // MARK: - Built-in themes (v3.6.2 PR-2b)

    // trace: built-in id "catppuccin" + .light → BuiltInThemes.theme(id:)!.colors(for: .light); shadow 0
    func testResolved_builtIn_lightPicksLightVariant() {
        let expected = BuiltInThemes.theme(id: "catppuccin")!
        let resolved = ThemeResolver.resolved(
            themeId: "catppuccin",
            colorScheme: .light,
            legacyColorSettings: customized(),
            userThemes: [],
        )
        XCTAssertEqual(resolved.colors, expected.colors(for: .light))
        XCTAssertEqual(resolved.keyShadowIntensity, 0)
    }

    // trace: same built-in id + .dark → the dark variant; light != dark proves colorScheme drives the pick
    func testResolved_builtIn_darkPicksDarkVariant() {
        let expected = BuiltInThemes.theme(id: "catppuccin")!
        let resolved = ThemeResolver.resolved(
            themeId: "catppuccin",
            colorScheme: .dark,
            legacyColorSettings: .default,
            userThemes: [],
        )
        XCTAssertEqual(resolved.colors, expected.colors(for: .dark))
        XCTAssertNotEqual(expected.colors(for: .light), expected.colors(for: .dark))
    }

    // trace: a user theme whose UUID happens to equal a built-in id is impossible (built-in ids are non-UUID),
    // but a built-in id must NOT be shadowed by the userThemes list → built-in branch wins over fallback
    func testResolved_builtIn_resolvesEvenWithUnrelatedUserThemes() {
        let other = makeUserTheme(id: UUID(), colors: customized())
        let resolved = ThemeResolver.resolved(
            themeId: "nord",
            colorScheme: .dark,
            legacyColorSettings: .default,
            userThemes: [other],
        )
        XCTAssertEqual(resolved.colors, BuiltInThemes.theme(id: "nord")!.colors(for: .dark))
    }
}
