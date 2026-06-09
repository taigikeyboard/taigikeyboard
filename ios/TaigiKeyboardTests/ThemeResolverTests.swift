@testable import TaigiKeyboard
import SwiftUI
import XCTest

/// Tests for `ThemeResolver` — the pure mapping from a selected theme id to
/// the `ThemeAppearance` the renderer consumes (v3.6.2; per-theme appearance:
/// colors + shadow + 5 size scalars. Font is global, not part of a theme).
///
/// Behavior under test:
/// - `default` → the legacy free-pick appearance verbatim (so uncustomized
///   users stay all-nil and customized users keep their look with no migration).
/// - a known `UserTheme` id → that theme's full appearance.
/// - a known built-in id → factory sizes + the built-in's colorScheme
///   color variant.
/// - an unknown id (deleted user theme, stale built-in id) → fall back to the
///   full legacy appearance.
final class ThemeResolverTests: XCTestCase {
    private func customized() -> KeyboardColorSettings {
        var cs = KeyboardColorSettings()
        cs.backgroundColor = CodableColor(.red)
        return cs
    }

    private func makeAppearance(
        colors: KeyboardColorSettings,
        shadow: Double = 0,
    ) -> ThemeAppearance {
        var appearance = ThemeAppearance.default
        appearance.colors = colors
        appearance.keyShadowIntensity = shadow
        return appearance
    }

    private func makeUserTheme(
        id: UUID,
        appearance: ThemeAppearance,
    ) -> UserTheme {
        UserTheme(
            id: id,
            name: "T",
            appearance: appearance,
            createdAt: Date(timeIntervalSince1970: 0),
            updatedAt: Date(timeIntervalSince1970: 0),
        )
    }

    // trace: themeId == ThemeId.default → returns legacyAppearance verbatim
    func testResolved_default_returnsLegacyAppearanceAllNil() {
        let resolved = ThemeResolver.resolved(
            themeId: ThemeId.default,
            colorScheme: .light,
            legacyAppearance: .default,
            userThemes: [],
        )
        XCTAssertEqual(resolved, .default)
    }

    // trace: default user who customized colors/sizes stays on "default" → keeps the whole appearance
    func testResolved_default_returnsCustomizedLegacyVerbatim() {
        var legacy = makeAppearance(colors: customized(), shadow: 0.2)
        legacy.keyHeightScale = 1.1
        let resolved = ThemeResolver.resolved(
            themeId: ThemeId.default,
            colorScheme: .light,
            legacyAppearance: legacy,
            userThemes: [],
        )
        XCTAssertEqual(resolved, legacy)
    }

    // trace: themeId matches a user theme → that theme's FULL appearance (colors + sizes + shadow)
    func testResolved_knownUserTheme_carriesFullAppearance() {
        let id = UUID()
        var appearance = makeAppearance(colors: customized(), shadow: 0.3)
        appearance.keyHeightScale = 1.1
        appearance.keyFontSizeScale = 0.9
        let theme = makeUserTheme(id: id, appearance: appearance)
        let resolved = ThemeResolver.resolved(
            themeId: id.uuidString,
            colorScheme: .dark,
            legacyAppearance: .default,
            userThemes: [theme],
        )
        XCTAssertEqual(resolved, appearance)
    }

    // trace: a genuinely unknown id → fall back to the FULL legacy appearance
    func testResolved_unknownId_fallsBackToLegacyAppearance() {
        let legacy = makeAppearance(colors: customized(), shadow: 0.4)
        let resolved = ThemeResolver.resolved(
            themeId: "no_such_theme",
            colorScheme: .light,
            legacyAppearance: legacy,
            userThemes: [],
        )
        XCTAssertEqual(resolved, legacy)
    }

    // trace: a previously-selected user theme was deleted (id no longer in list) → fallback legacy
    func testResolved_deletedUserTheme_fallsBackToLegacyAppearance() {
        let staleId = UUID()
        let other = makeUserTheme(id: UUID(), appearance: makeAppearance(colors: customized()))
        let resolved = ThemeResolver.resolved(
            themeId: staleId.uuidString,
            colorScheme: .light,
            legacyAppearance: .default,
            userThemes: [other],
        )
        XCTAssertEqual(resolved, .default)
    }

    // MARK: - Built-in themes

    // trace: built-in id "standardBlue" + .light → catalog branch returns its
    // gradient palette (≠ .default); scalars stay factory (shadow 0).
    func testResolved_builtIn_lightResolvesThroughCatalog() {
        let expected = BuiltInThemes.theme(id: "standardBlue")!
        let resolved = ThemeResolver.resolved(
            themeId: "standardBlue",
            colorScheme: .light,
            legacyAppearance: makeAppearance(colors: customized()),
            userThemes: [],
        )
        XCTAssertEqual(resolved.colors, expected.colors(for: .light))
        XCTAssertNotEqual(resolved.colors, .default, "standardBlue defines a gradient palette")
        XCTAssertTrue(resolved.colors.hasBackgroundGradient, "standardBlue paints a background gradient")
        XCTAssertEqual(resolved.keyShadowIntensity, 0)
    }

    // trace: same built-in id + .dark → catalog branch wins over the legacy appearance
    func testResolved_builtIn_darkResolvesThroughCatalog() {
        let expected = BuiltInThemes.theme(id: "standardBlue")!
        let resolved = ThemeResolver.resolved(
            themeId: "standardBlue",
            colorScheme: .dark,
            legacyAppearance: .default,
            userThemes: [],
        )
        XCTAssertEqual(resolved.colors, expected.colors(for: .dark))
    }

    // trace: built-in themes define colors only → factory sizes/shadow, regardless of legacy appearance
    func testResolved_builtIn_usesFactorySizes() {
        var legacy = makeAppearance(colors: customized(), shadow: 0.5)
        legacy.keyHeightScale = 1.15
        let resolved = ThemeResolver.resolved(
            themeId: "standardBlue",
            colorScheme: .dark,
            legacyAppearance: legacy,
            userThemes: [],
        )
        XCTAssertEqual(resolved.keyHeightScale, ThemeAppearance.default.keyHeightScale)
        XCTAssertEqual(resolved.keyFontSizeScale, ThemeAppearance.default.keyFontSizeScale)
        XCTAssertEqual(resolved.candidateTextSizeScale, ThemeAppearance.default.candidateTextSizeScale)
        XCTAssertEqual(resolved.keyCornerRadius, ThemeAppearance.default.keyCornerRadius)
        XCTAssertEqual(resolved.keyBorderWidth, ThemeAppearance.default.keyBorderWidth)
        XCTAssertEqual(resolved.keyShadowIntensity, 0)
    }

    // trace: a 框線 family theme (id "framedBlue") carries keyBorderWidth=1.0 ON TOP
    // of factory sizes; other scalars stay factory.
    func testResolved_framedFamily_carriesKeyBorderWidth() {
        let resolved = ThemeResolver.resolved(
            themeId: "framedBlue",
            colorScheme: .light,
            legacyAppearance: .default,
            userThemes: [],
        )
        XCTAssertEqual(resolved.keyBorderWidth, 1.0)
        XCTAssertNotEqual(resolved.keyBorderWidth, ThemeAppearance.default.keyBorderWidth, "框線 overrides the factory border")
        XCTAssertEqual(resolved.keyCornerRadius, ThemeAppearance.default.keyCornerRadius, "other scalars stay factory")
    }

    // trace: 經典 / 簡潔 family themes keep the factory border (0)
    func testResolved_classicAndCleanFamilies_keepFactoryKeyBorderWidth() {
        for id in ["standardBlue", "cleanBlue"] {
            let resolved = ThemeResolver.resolved(
                themeId: id,
                colorScheme: .light,
                legacyAppearance: .default,
                userThemes: [],
            )
            XCTAssertEqual(resolved.keyBorderWidth, ThemeAppearance.default.keyBorderWidth, "\(id) keeps the factory border")
        }
    }

    // trace: a built-in id must NOT be shadowed by the userThemes list → built-in branch wins over fallback
    func testResolved_builtIn_resolvesEvenWithUnrelatedUserThemes() {
        let other = makeUserTheme(id: UUID(), appearance: makeAppearance(colors: customized()))
        let resolved = ThemeResolver.resolved(
            themeId: "standardBlue",
            colorScheme: .dark,
            legacyAppearance: .default,
            userThemes: [other],
        )
        XCTAssertEqual(resolved.colors, BuiltInThemes.theme(id: "standardBlue")!.colors(for: .dark))
    }
}
