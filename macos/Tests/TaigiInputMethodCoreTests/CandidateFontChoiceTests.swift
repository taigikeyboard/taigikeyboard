// The candidate window's typeface roster: what it names, and that it resolves.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The font picker's roster. Three things have to agree for an option to draw
/// in the face it names — the file `bundle-app.sh` copies, the PostScript name
/// `CandidateFontChoice` asks AppKit for, and the case the picker writes — and
/// nothing but these tests notices when they drift: a name that does not
/// resolve falls back to the system font, which is a picker whose options all
/// look the same rather than a crash.
@MainActor
final class CandidateFontChoiceTests: XCTestCase {
    /// The shipped app activates these through Info.plist's
    /// `ATSApplicationFontsPath`, which does nothing for a test process outside
    /// any bundle — so every case here would otherwise assert against the
    /// system-font fallback and pass vacuously.
    override func setUp() {
        super.setUp()
        XCTAssertEqual(
            TestFixtures.unregisterableFontFiles, [],
            "the bundled typefaces must activate for this suite to mean anything",
        )
    }

    /// The typeface a fresh Mac renders in (USER 2026-08-23). iOS starts on
    /// Open Huninn instead — the key spelling is shared, the default is not.
    func testSystemFont_isTheMacOSDefault() {
        XCTAssertEqual(SettingsStore.Keys.fontType.defaultValue, .system)
        XCTAssertNil(CandidateFontChoice.system.postScriptName)
    }

    /// The raw values are iOS's `FontType`'s verbatim, which is what lets one
    /// choice have one name across the platforms
    /// (`ios/Sources/TaigiKeyboard/Settings/SettingsModels.swift`).
    func testRawValues_areTheIOSFontTypeSpellings() {
        XCTAssertEqual(
            CandidateFontChoice.allCases.map(\.rawValue),
            ["system", "openHuninn", "iansui", "genYoMin", "genYoGothic"],
        )
    }

    /// The drift this suite exists for: a case whose font file stopped shipping,
    /// or whose PostScript name is not the one inside the file, silently falls
    /// back to the system font.
    func testEveryPostScriptName_resolvesToItsOwnFace() {
        for choice in CandidateFontChoice.allCases {
            let font = choice.font(ofSize: 20)
            guard let postScriptName = choice.postScriptName else {
                XCTAssertEqual(font, .systemFont(ofSize: 20))
                continue
            }
            XCTAssertEqual(
                font.fontName, postScriptName,
                "\(choice) fell back to the system font instead of resolving its own face",
            )
        }
    }

    /// A face that never activated must not leave a cell blank — the case a
    /// bundle assembled without `Contents/Resources/Fonts` produces. Asked
    /// through the by-name seam, because no case can name an unresolvable face
    /// while the fixtures have the real ones registered.
    func testAnUnresolvableFace_fallsBackToTheSystemFont() {
        XCTAssertNil(NSFont(name: "NoSuchFace-Regular", size: 20), "the premise: nothing carries this name")

        XCTAssertEqual(
            CandidateFontChoice.font(named: "NoSuchFace-Regular", ofSize: 20),
            .systemFont(ofSize: 20),
        )
        XCTAssertEqual(CandidateFontChoice.font(named: nil, ofSize: 20), .systemFont(ofSize: 20))
    }

    /// `NSFont(name:)` substitutes rather than fails for some names — a FAMILY
    /// name resolves to whichever face of it AppKit picks — so a name that
    /// comes back as a different face is the system font's job too: a
    /// substituted typeface is one the user did not pick.
    func testASubstitutedFace_fallsBackToTheSystemFont() {
        // A family name, not a PostScript name: macOS resolves it to
        // `Menlo-Regular`, which is the substitution this guards against.
        let familyName = "Menlo"
        XCTAssertNotEqual(
            NSFont(name: familyName, size: 20)?.fontName, familyName,
            "the premise: this name resolves, to a DIFFERENT PostScript name",
        )

        XCTAssertEqual(CandidateFontChoice.font(named: familyName, ofSize: 20), .systemFont(ofSize: 20))
    }

    // MARK: - What the choice reaches

    /// The measurement and the labels go through one resolution path, so a
    /// width can never be taken in a font the text is not drawn in.
    func testCellLabels_areSetInTheChosenFace() {
        for choice in CandidateFontChoice.allCases {
            let metrics = CandidateMetrics(textSize: .medium, windowSize: .medium, fontChoice: choice)
            let view = CandidateItemView(style: .sequoia, metrics: metrics)
            view.configure(CandidateCellContent(text: "候選", annotation: "hāu-suán"))

            // Three: the digit hint, then the two scripts. The digit is set in
            // the SYSTEM font whatever the user chose — it names a key rather
            // than belonging to the Taigi text.
            let labels = view.subviews.compactMap { $0 as? NSTextField }
            XCTAssertEqual(labels.count, 3)
            XCTAssertEqual(
                labels.map(\.font?.fontName),
                [
                    metrics.indexFont.fontName,
                    metrics.candidateFont.fontName,
                    metrics.annotationFont.fontName,
                ],
            )
            XCTAssertEqual(
                labels.map(\.font?.pointSize),
                [metrics.indexFontSize, metrics.candidateFontSize, metrics.annotationFontSize],
            )
        }
    }

    /// A cell's constraints bake the metrics in, so a font change has to reach
    /// the panel cache's equality check or the window would keep drawing in the
    /// old face until something else rebuilt it (`CandidatePanel.panel(for:)`).
    func testMetrics_differByFontChoice_soThePanelsRebuild() {
        let system = CandidateMetrics(textSize: .medium, windowSize: .medium, fontChoice: .system)
        for choice in CandidateFontChoice.allCases where choice != .system {
            XCTAssertNotEqual(
                system, CandidateMetrics(textSize: .medium, windowSize: .medium, fontChoice: choice),
            )
        }
    }

    func testArranged_keepsTheFontChoice() {
        let stacked = CandidateMetrics(
            textSize: .large, windowSize: .small, fontChoice: .genYoMin,
        ).arranged(.stacked)

        XCTAssertEqual(stacked.fontChoice, .genYoMin)
        XCTAssertEqual(stacked.candidateFont.fontName, CandidateFontChoice.genYoMin.postScriptName)
    }

    /// The floor is memoized across metrics values, so every face has to get
    /// the floor measured in ITS font. The four bundled faces all render 「永」
    /// at one em, so a cache keyed on the size alone would pass this today —
    /// what it pins is that the floor is the chosen face's own measurement,
    /// which is the property a fifth face would break first.
    func testPrimaryColumnFloor_isTheChosenFacesOwnMeasurement() {
        for choice in CandidateFontChoice.allCases {
            let metrics = CandidateMetrics(textSize: .medium, windowSize: .medium, fontChoice: choice)
            XCTAssertEqual(
                metrics.primaryColumnFloor,
                max(metrics.candidateFontSize, metrics.measurePrimaryWidth("永")),
                "\(choice): the cached floor is another typeface's",
            )
        }
    }
}
