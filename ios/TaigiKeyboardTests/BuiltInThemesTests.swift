@testable import TaigiKeyboard
import SwiftUI
import XCTest

/// Tests for the built-in theme catalog.
///
/// The id invariants are load-bearing: `SharedSettings.resolvedAppearance(for:)`
/// uses `UUID(uuidString:) != nil` to decide whether to read the user-theme
/// file, and `ThemeId.default` is the legacy-buffer sentinel. A built-in id
/// that is a UUID string or equals "default" would be mis-routed.
///
/// The catalog is a key-STYLE axis: three families (經典 / 框線 / 簡潔) share the
/// same 6 colors and differ only in key style — 經典 fills keys, 框線 / 簡潔 make
/// keys transparent (background shows through), 框線 adding an outline.
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
        XCTAssertEqual(head?.displayNameKey, .themePaletteDefault)
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
        XCTAssertEqual(BuiltInThemes.theme(id: ThemeId.default)?.displayNameKey, .themePaletteDefault)
    }

    // trace: three key-style families (經典/框線/簡潔), each carrying the same 7 colors
    func testFamilies_threeKeyStyleFamiliesEachWithSevenColors() {
        XCTAssertEqual(BuiltInThemes.families.map(\.titleKey), [.themeFamilyClassic, .themeFamilyFramed, .themeFamilyClean])
        for family in BuiltInThemes.families {
            XCTAssertEqual(family.themes.count, 7, "\(family.titleKey) must carry all 7 shared colors")
            XCTAssertEqual(
                family.themes.map(\.displayNameKey),
                [.themePaletteDefault, .themePalettePink, .themePaletteGold, .themePaletteBlue, .themePaletteGreen, .themePalettePurple, .themePaletteCatppuccin],
            )
        }
        XCTAssertEqual(BuiltInThemes.all.count, 21, "3 families × 7 colors")
    }

    // trace: 經典 keeps filled keys; 框線/簡潔 recolor keys to transparent (key == background)
    func testKeyStyleFamilies_framedAndCleanHaveTransparentKeys() {
        let classic = BuiltInThemes.theme(id: "standardBlue")!.colors(for: .light)
        XCTAssertEqual(classic.normalKeyFillColor?.alpha, 1, "經典 keys are opaque (white)")
        for id in ["framedBlue", "cleanBlue"] {
            let colors = BuiltInThemes.theme(id: id)!.colors(for: .light)
            XCTAssertEqual(colors.normalKeyFillColor?.alpha, 0, "\(id) normal key must be transparent")
            XCTAssertEqual(colors.specialKeyFillColor?.alpha, 0, "\(id) special key must be transparent")
        }
    }

    // trace: only the 框線 family draws the outline; 經典/簡潔 leave keyBorderWidth nil
    func testKeyStyleFamilies_onlyFramedCarriesKeyBorderWidth() {
        XCTAssertEqual(BuiltInThemes.theme(id: "framedBlue")?.keyBorderWidth, 1.0)
        XCTAssertNil(BuiltInThemes.theme(id: "standardBlue")?.keyBorderWidth)
        XCTAssertNil(BuiltInThemes.theme(id: "cleanBlue")?.keyBorderWidth)
    }

    // trace: the three families SHARE colors — only the key fill differs (gradient + text identical)
    func testKeyStyleFamilies_shareBackgroundAndTextAcrossFamilies() {
        let classic = BuiltInThemes.theme(id: "standardBlue")!.colors(for: .light)
        for id in ["framedBlue", "cleanBlue"] {
            let variant = BuiltInThemes.theme(id: id)!.colors(for: .light)
            XCTAssertEqual(variant.backgroundGradient, classic.backgroundGradient, "\(id) keeps the same gradient")
            XCTAssertEqual(variant.keyTextColor, classic.keyTextColor, "\(id) keeps the same text color")
        }
    }

    // trace: framed/clean gradient themes keep the gradient → candidate tints still derive from it
    func testKeyStyleFamilies_framedGradientStillCarriesGradient() {
        for id in ["framedPink", "cleanPink"] {
            XCTAssertTrue(BuiltInThemes.theme(id: id)!.colors(for: .light).hasBackgroundGradient, "\(id) must keep the gradient")
        }
    }

    // trace: framed/clean 預設 stay adaptive (bg/text nil) but carry transparent keys; 經典 預設 fully adaptive
    func testKeyStyleFamilies_defaultVariantsAdaptiveWithTransparentKeys() {
        for id in ["framedDefault", "cleanDefault"] {
            let colors = BuiltInThemes.theme(id: id)!.colors(for: .light)
            XCTAssertNil(colors.backgroundColor, "\(id) keeps adaptive background")
            XCTAssertFalse(colors.hasBackgroundGradient, "\(id) is the adaptive 預設, no gradient")
            XCTAssertNil(colors.keyTextColor, "\(id) keeps adaptive text")
            XCTAssertEqual(colors.normalKeyFillColor?.alpha, 0, "\(id) keys are transparent")
        }
        XCTAssertEqual(BuiltInThemes.theme(id: ThemeId.default)?.colors(for: .light), .default, "經典 預設 stays fully adaptive")
    }

    // trace: the Standard gradient themes each carry a ≥2-stop background gradient in both schemes
    func testStandardGradientThemes_carryGradient() {
        for id in ["standardPink", "standardGold", "standardBlue", "standardGreen", "standardPurple"] {
            let theme = BuiltInThemes.theme(id: id)!
            XCTAssertTrue(theme.colors(for: .light).hasBackgroundGradient, "\(id) light must carry a gradient")
            XCTAssertTrue(theme.colors(for: .dark).hasBackgroundGradient, "\(id) dark must carry a gradient")
        }
    }

    // trace: gradient themes are light-only (dark == nil) → dark scheme reuses the
    // light palette unchanged (USER: these themes keep their light look in dark mode)
    func testStandardGradientThemes_stayLightInDarkMode() {
        for id in ["standardPink", "standardGold", "standardBlue", "standardGreen", "standardPurple"] {
            let theme = BuiltInThemes.theme(id: id)!
            XCTAssertNil(theme.dark, "\(id) must not define a dark variant — it stays light in dark mode")
            XCTAssertEqual(theme.colors(for: .dark), theme.colors(for: .light), "\(id) dark scheme must reuse the light palette")
        }
    }

    // trace: 暗眠山貓 is the dark-only Catppuccin Mocha theme across all 3 families —
    // light == nil so BOTH schemes resolve to the dark variant (always dark, the mirror of
    // the 5 light-only gradients). gradient top #1E1E2E (Base) → #181825 (Mantle);
    // key+candidate text #CDD6F4 (Text); 經典 keys (letter + function) share #313244
    // (Surface0); 框線/簡潔 keep transparent keys.
    func testCatppuccinTheme_isDarkOnlyAcrossFamilies() {
        for id in ["standardCatppuccin", "framedCatppuccin", "cleanCatppuccin"] {
            let theme = BuiltInThemes.theme(id: id)!
            XCTAssertNil(theme.light, "\(id) must be dark-only (light == nil)")
            XCTAssertNotNil(theme.dark, "\(id) must define the dark variant")
            let colors = theme.colors(for: .dark)
            XCTAssertEqual(theme.colors(for: .light), colors, "\(id) light request falls back to the dark variant")
            XCTAssertTrue(colors.hasBackgroundGradient, "\(id) must carry the Catppuccin gradient")
            XCTAssertEqual(colors.backgroundGradient?.stops.first, CodableColor(hex: 0x1E1E2E), "\(id) gradient top = Mocha Base")
            XCTAssertEqual(colors.backgroundGradient?.stops.last, CodableColor(hex: 0x181825), "\(id) gradient bottom = Mocha Mantle")
            XCTAssertEqual(colors.keyTextColor, CodableColor(hex: 0xCDD6F4), "\(id) key text = Mocha Text")
            XCTAssertEqual(colors.candidateTextColor, CodableColor(hex: 0xCDD6F4), "\(id) candidate text = Mocha Text")
        }
        // 經典 暗眠山貓: letter + function keys share the Surface0 neutral fill.
        let classic = BuiltInThemes.theme(id: "standardCatppuccin")!.colors(for: .dark)
        XCTAssertEqual(classic.normalKeyFillColor, CodableColor(hex: 0x313244), "經典 暗眠山貓 letter key = Mocha Surface0")
        XCTAssertEqual(classic.specialKeyFillColor, CodableColor(hex: 0x313244), "經典 暗眠山貓 function key shares the Surface0 fill")
        // 框線/簡潔 keep keys transparent (gradient shows through).
        for id in ["framedCatppuccin", "cleanCatppuccin"] {
            let colors = BuiltInThemes.theme(id: id)!.colors(for: .dark)
            XCTAssertEqual(colors.normalKeyFillColor?.alpha, 0, "\(id) letter keys must be transparent")
            XCTAssertEqual(colors.specialKeyFillColor?.alpha, 0, "\(id) function keys must be transparent")
        }
    }

    // trace: candidate tints derive from the gradient top stop — highlight LIGHTENED
    // toward white ×0.5, pressed DEEPENED toward black ×0.65 (each 0-255 component truncated):
    //   櫻花 top E6C2D0 → highlight F2E0E7, pressed 957E87
    //   海風 top BFD2EA → highlight DFE8F4, pressed 7C8898
    func testCodableColor_derivesCandidateTints() {
        let pinkTop = CodableColor(hex: 0xE6C2D0)
        XCTAssertEqual(pinkTop.lightened(towardWhite: KeyboardColorSettings.candidateHighlightLightenFactor), CodableColor(hex: 0xF2E0E7))
        XCTAssertEqual(pinkTop.deepened(by: KeyboardColorSettings.candidatePressedDeepenFactor), CodableColor(hex: 0x957E87))
        let blueTop = CodableColor(hex: 0xBFD2EA)
        XCTAssertEqual(blueTop.lightened(towardWhite: KeyboardColorSettings.candidateHighlightLightenFactor), CodableColor(hex: 0xDFE8F4))
        XCTAssertEqual(blueTop.deepened(by: KeyboardColorSettings.candidatePressedDeepenFactor), CodableColor(hex: 0x7C8898))
    }

    // trace: state priority is press > firstCandidate > selected. The engine sets
    // selectedCandidateIndex=0 while typing, so the first candidate is BOTH selected and
    // first — it must show the LIGHT highlight, not the dark pressed/selection color.
    func testResolvedBackgroundColor_firstCandidateLightBeatsSelection() {
        let style = CandidateView.ItemStyle.standard
        let highlight = Color.green
        let pressed = Color.red

        // typing: first candidate selected (index 0) but not pressed → LIGHT highlight
        XCTAssertEqual(
            style.resolvedBackgroundColor(for: .light, isSelected: true, isFirstCandidate: true,
                                          firstCandidateThemeColor: highlight, pressedThemeColor: pressed),
            highlight, "selected first candidate must show the light highlight, not the dark pressed color")
        // actual press on the first candidate → dark pressed feedback
        XCTAssertEqual(
            style.resolvedBackgroundColor(for: .light, isPressed: true, isFirstCandidate: true,
                                          firstCandidateThemeColor: highlight, pressedThemeColor: pressed),
            pressed, "an actual press still darkens the first candidate")
        // a selected NON-first candidate (hardware nav) → dark pressed/selection
        XCTAssertEqual(
            style.resolvedBackgroundColor(for: .light, isSelected: true, isFirstCandidate: false,
                                          firstCandidateThemeColor: highlight, pressedThemeColor: pressed),
            pressed, "a selected non-first candidate keeps the dark selection color")
    }

    // trace: a gradient theme's CandidateTheme carries non-nil tints; a flat theme leaves them nil (neutral fallback)
    func testCandidateTheme_gradientThemeDerivesTints_flatThemeNil() {
        let gradientColors = BuiltInThemes.theme(id: "standardPink")!.colors(for: .light)
        let gradientTheme = CandidateTheme.resolved(candidateTextSizeScale: 1, colorSettings: gradientColors, screenSizeClass: .phoneCompact)
        XCTAssertNotNil(gradientTheme.firstCandidateHighlightColor, "gradient theme must derive a first-candidate highlight")
        XCTAssertNotNil(gradientTheme.pressedCandidateColor, "gradient theme must derive a pressed tint")

        let flatTheme = CandidateTheme.resolved(candidateTextSizeScale: 1, colorSettings: .default, screenSizeClass: .phoneCompact)
        XCTAssertNil(flatTheme.firstCandidateHighlightColor, "flat theme must leave first-candidate highlight nil")
        XCTAssertNil(flatTheme.pressedCandidateColor, "flat theme must leave pressed tint nil")
    }

    // trace: a dark-only theme (light == nil) → .light request falls back to the dark variant
    func testColorsForScheme_darkOnlyFallsBackToDark() {
        let darkColors = makeColors(hex: 0x112233)
        let darkOnly = BuiltInTheme(id: "test_dark_only", displayNameKey: .themePaletteDefault, light: nil, dark: darkColors)
        XCTAssertEqual(darkOnly.colors(for: .light), darkColors, "light request must fall back to dark when no light")
        XCTAssertEqual(darkOnly.colors(for: .dark), darkColors)
    }

    // trace: a light-only theme (dark == nil) → .dark request falls back to the light variant
    func testColorsForScheme_lightOnlyFallsBackToLight() {
        let lightColors = makeColors(hex: 0xAABBCC)
        let lightOnly = BuiltInTheme(id: "test_light_only", displayNameKey: .themePaletteDefault, light: lightColors, dark: nil)
        XCTAssertEqual(lightOnly.colors(for: .dark), lightColors, "dark request must fall back to light when no dark")
        XCTAssertEqual(lightOnly.colors(for: .light), lightColors)
    }

    // trace: colors(for:) returns the requested variant when both exist
    func testColorsForScheme_picksRequestedVariant() {
        let light = makeColors(hex: 0x111111)
        let dark = makeColors(hex: 0x222222)
        let theme = BuiltInTheme(id: "test_both", displayNameKey: .themePaletteDefault, light: light, dark: dark)
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
