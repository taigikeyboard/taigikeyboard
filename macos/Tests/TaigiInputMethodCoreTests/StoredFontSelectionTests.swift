// The three stored keys, decoded and encoded as one selection.

@testable import TaigiInputMethodCore
import XCTest

/// `StoredFontSelection` is the one codec for `fontType` + `customFontFile` +
/// `installedFontFamily`; the pane writes through it and the store reads
/// through it, so what it says about half-written pairs is what both do.
final class StoredFontSelectionTests: XCTestCase {
    func testABundledRawValue_decodesToThatFace_ignoringStaleCompanions() {
        let stored = StoredFontSelection(
            fontType: "genYoMin", customFontFile: "left-over.ttf", installedFontFamily: "Helvetica",
        )

        XCTAssertEqual(stored, .builtIn(.genYoMin))
    }

    func testAnUnknownRawValue_decodesToTheSystemFace() {
        XCTAssertEqual(
            StoredFontSelection(fontType: "comicSans", customFontFile: "", installedFontFamily: ""),
            .builtIn(.system),
        )
    }

    func testCustomWithAFile_decodesToThatFile() {
        XCTAssertEqual(
            StoredFontSelection(fontType: "custom", customFontFile: "mine.ttf", installedFontFamily: ""),
            .customFile("mine.ttf"),
        )
    }

    func testInstalledWithAFamily_decodesToThatFamily() {
        XCTAssertEqual(
            StoredFontSelection(fontType: "installed", customFontFile: "", installedFontFamily: "Helvetica"),
            .installedFamily("Helvetica"),
        )
    }

    /// Three keys are three writes; a reader landing between them sees the
    /// default face, not a half-written selection.
    func testCustomOrInstalled_withNothingBesideIt_decodesToTheSystemFace() {
        XCTAssertEqual(
            StoredFontSelection(fontType: "custom", customFontFile: "", installedFontFamily: ""),
            .builtIn(.system),
        )
        XCTAssertEqual(
            StoredFontSelection(fontType: "installed", customFontFile: "", installedFontFamily: ""),
            .builtIn(.system),
        )
    }

    /// Encoding writes every key, so a bundled or installed pick clears a
    /// stale file name and a custom pick clears a stale family — and every
    /// value round-trips through its own three keys.
    func testEncoding_roundTrips_andClearsTheOtherKinds() {
        for selection in [
            StoredFontSelection.builtIn(.iansui), .customFile("mine.ttf"), .installedFamily("Helvetica"),
        ] {
            let decoded = StoredFontSelection(
                fontType: selection.fontType,
                customFontFile: selection.customFontFile,
                installedFontFamily: selection.installedFontFamily,
            )
            XCTAssertEqual(decoded, selection)
        }
        XCTAssertEqual(StoredFontSelection.installedFamily("Helvetica").customFontFile, "")
        XCTAssertEqual(StoredFontSelection.customFile("mine.ttf").installedFontFamily, "")
        XCTAssertEqual(StoredFontSelection.builtIn(.iansui).customFontFile, "")
    }
}
