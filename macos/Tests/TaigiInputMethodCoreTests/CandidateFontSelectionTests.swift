// The resolved typeface: two custom fonts are two values, and an installed family resolves by name.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// `CandidateFontSelection` is what the candidate window is actually drawn
/// from, and the reason it exists rather than the shared roster's enum: two
/// typefaces the user added have to be two DIFFERENT values, or the second
/// would inherit the first's panels and the first's measured widths.
///
/// What the two STORED keys resolve to is `SettingsStoreTests`, beside the rest
/// of that store's key handling.
@MainActor
final class CandidateFontSelectionTests: XCTestCase {
    private func customFont(_ fileName: String) -> CustomFont {
        CustomFont(fileName: fileName, postScriptName: "Whatever-Regular")
    }

    // MARK: - Two custom fonts are two typefaces

    /// The panel cache compares metrics (`CandidatePanel.panel(for:)`). Two
    /// custom selections that compared equal would leave the window drawing the
    /// first font's cells after the user picked the second.
    func testMetrics_differBetweenTwoCustomFonts_soThePanelsRebuild() {
        let first = CandidateMetrics(
            textSize: .medium, windowSize: .medium, fontSelection: .custom(customFont("one.ttf")),
        )
        let second = CandidateMetrics(
            textSize: .medium, windowSize: .medium, fontSelection: .custom(customFont("two.ttf")),
        )

        XCTAssertNotEqual(first, second)
        XCTAssertNotEqual(first, CandidateMetrics(textSize: .medium, windowSize: .medium))
    }

    /// An unresolvable face falls back to the system font, like the bundled
    /// roster's own missing-file case — a custom row must never draw nothing.
    func testAnUnavailableCustomFace_fallsBackToTheSystemFont() {
        let selection = CandidateFontSelection.custom(customFont("one.ttf"))

        XCTAssertNil(NSFont(name: "Whatever-Regular", size: 20), "the premise: nothing carries this name")
        XCTAssertEqual(selection.font(ofSize: 20), .systemFont(ofSize: 20))
    }

    /// The inline row is a point size plus padding for the bundled four, whose
    /// line boxes are known to fit it. A face the user brought gets its own
    /// measured line box instead, so a tall one is not clipped — and the
    /// bundled arithmetic is left exactly as it shipped.
    func testInlineHeight_isMeasuredForACustomFace_andUnchangedForABundledOne() {
        let bundled = CandidateMetrics(
            textSize: .medium, windowSize: .medium, fontSelection: .builtIn(.iansui),
        )
        XCTAssertEqual(bundled.itemHeight, bundled.candidateFontSize + bundled.verticalPadding)

        // The custom font here does not resolve, so it measures in the system
        // font — whose line box at 20pt is taller than 20pt. What the case
        // pins is that the custom branch MEASURES rather than assuming.
        let custom = CandidateMetrics(
            textSize: .medium, windowSize: .medium, fontSelection: .custom(customFont("one.ttf")),
        )
        XCTAssertGreaterThan(custom.itemHeight, custom.candidateFontSize + custom.verticalPadding)
    }

    // MARK: - Installed families

    /// A family the OS has resolves to a member of THAT family — not to a
    /// substitute, which is what descriptor matching hands back for a name
    /// nothing carries. The family comes from the live list: none is
    /// guaranteed on every Mac.
    func testAnInstalledFamily_resolvesToAFaceOfThatFamily() throws {
        let family = try TestFixtures.anyInstalledFamily()

        let font = CandidateFontSelection.installed(family: family).font(ofSize: 20)

        XCTAssertEqual(font.familyName, family)
        XCTAssertEqual(font.pointSize, 20)
    }

    /// A family the OS does not have falls back to the system font, like a
    /// custom face that did not activate.
    func testAnUnknownFamily_fallsBackToTheSystemFont() {
        let selection = CandidateFontSelection.installed(family: "No Such Family 4f9a")

        XCTAssertFalse(RegisteredFace.isRegistered(.family("No Such Family 4f9a")))
        XCTAssertEqual(selection.font(ofSize: 20), .systemFont(ofSize: 20))
    }

    /// The installed list is the registration list minus what the caller
    /// names: hidden UI faces never, and the families this process registered
    /// itself — the bundled roster (`TestFixtures`) would otherwise show up
    /// twice, once as its own row and once as a family whose selection no
    /// restart re-activates.
    func testInstalledFamilies_leaveOutTheFamiliesTheyAreAskedTo() throws {
        XCTAssertEqual(TestFixtures.unregisterableFontFiles, [])
        let bundled = try XCTUnwrap(RegisteredFace.font(.postScript("Iansui-Regular"), ofSize: 12)?.familyName)
        XCTAssertTrue(RegisteredFace.installedFamilies(excluding: []).contains(bundled))

        let filtered = RegisteredFace.installedFamilies(excluding: [bundled])

        XCTAssertFalse(filtered.contains(bundled))
        XCTAssertFalse(filtered.contains { $0.hasPrefix(".") }, "the OS's hidden UI faces are not offered")
        XCTAssertEqual(filtered, filtered.sorted { $0.localizedStandardCompare($1) == .orderedAscending })
    }

    /// The inline row measures an installed face's line box like a custom one:
    /// neither carries the bundled roster's fits-the-row guarantee.
    func testInlineHeight_isMeasuredForAnInstalledFace() throws {
        let family = try TestFixtures.anyInstalledFamily()
        let metrics = CandidateMetrics(
            textSize: .medium, windowSize: .medium, fontSelection: .installed(family: family),
        )

        XCTAssertTrue(metrics.fontSelection.requiresLineBoxMeasurement)
        XCTAssertGreaterThanOrEqual(metrics.itemHeight, metrics.candidateFontSize + metrics.verticalPadding)
    }
}
