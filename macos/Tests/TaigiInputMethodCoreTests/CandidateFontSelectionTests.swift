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
            size: .standard, fontSelection: .custom(customFont("one.ttf")),
        )
        let second = CandidateMetrics(
            size: .standard, fontSelection: .custom(customFont("two.ttf")),
        )

        XCTAssertNotEqual(first, second)
        XCTAssertNotEqual(first, CandidateMetrics(size: .standard))
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
            size: .standard, fontSelection: .builtIn(.iansui),
        )
        XCTAssertEqual(bundled.itemHeight, bundled.candidateFontSize + bundled.verticalPadding)

        // The custom font here does not resolve, so it measures in the system
        // font — whose line box at 20pt is taller than 20pt. What the case
        // pins is that the custom branch MEASURES rather than assuming.
        let custom = CandidateMetrics(
            size: .standard, fontSelection: .custom(customFont("one.ttf")),
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

    /// A family's weights are its upright, normal-width faces, lightest first,
    /// one per style — each one a face of that family, at that style.
    func testAFamilysWeights_areUprightNormalWidthFacesLightestFirst() throws {
        let (family, faces) = try TestFixtures.anyFamilyWithWeights()

        XCTAssertEqual(faces.map(\.weight), faces.map(\.weight).sorted())
        XCTAssertEqual(Set(faces.map(\.style)).count, faces.count)
        for face in faces {
            let font = try XCTUnwrap(
                CandidateFontSelection.installed(family: family, face: face.style).faceQuery
                    .flatMap { RegisteredFace.font($0, ofSize: 20) },
            )
            let traits = NSFontManager.shared.traits(of: font)
            XCTAssertEqual(font.familyName, family)
            XCTAssertFalse(traits.contains(.italicFontMask), "\(face.style) is not upright")
            XCTAssertFalse(traits.contains(.condensedFontMask), "\(face.style) is condensed")
        }
    }

    /// A picked weight draws in that face; a style the family does not have is
    /// refused rather than substituted with another of its faces.
    func testAPickedWeight_drawsInThatFace_andAnUnknownStyleIsRefused() throws {
        let (family, faces) = try TestFixtures.anyFamilyWithWeights()
        let lightest = try XCTUnwrap(faces.first)
        let heaviest = try XCTUnwrap(faces.last)

        let light = CandidateFontSelection.installed(family: family, face: lightest.style).font(ofSize: 20)
        let heavy = CandidateFontSelection.installed(family: family, face: heaviest.style).font(ofSize: 20)

        XCTAssertNotEqual(light.fontName, heavy.fontName)
        XCTAssertFalse(RegisteredFace.isRegistered(.face(family: family, style: "No Such Style 4f9a")))
    }

    /// Two weights of one family are two metrics values, so the panel cache
    /// rebuilds rather than serving cells set in the other weight.
    func testMetrics_differBetweenTwoWeightsOfOneFamily_soThePanelsRebuild() throws {
        let (family, faces) = try TestFixtures.anyFamilyWithWeights()

        let first = CandidateMetrics(size: .standard, fontSelection: .installed(family: family, face: faces[0].style))
        let second = CandidateMetrics(size: .standard, fontSelection: .installed(family: family, face: faces[1].style))

        XCTAssertNotEqual(first, second)
    }

    /// With no weight picked the family draws its `.family` match, and that
    /// is the style the pane shows selected.
    func testTheDefaultStyle_isTheFaceAFamilyDrawsWithNoWeightPicked() throws {
        let (family, _) = try TestFixtures.anyFamilyWithWeights()

        let drawn = CandidateFontSelection.installed(family: family).font(ofSize: 20)
        let defaultStyle = try XCTUnwrap(RegisteredFace.defaultStyle(ofFamily: family))
        let named = try XCTUnwrap(RegisteredFace.font(.face(family: family, style: defaultStyle), ofSize: 20))

        XCTAssertEqual(drawn.fontName, named.fontName)
    }

    /// The inline row measures an installed face's line box like a custom one:
    /// neither carries the bundled roster's fits-the-row guarantee.
    func testInlineHeight_isMeasuredForAnInstalledFace() throws {
        let family = try TestFixtures.anyInstalledFamily()
        let metrics = CandidateMetrics(
            size: .standard, fontSelection: .installed(family: family),
        )

        XCTAssertTrue(metrics.fontSelection.requiresLineBoxMeasurement)
        XCTAssertGreaterThanOrEqual(metrics.itemHeight, metrics.candidateFontSize + metrics.verticalPadding)
    }
}
