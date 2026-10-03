// The four stored keys, decoded and encoded as one selection.

@testable import TaigiInputMethodCore
import XCTest

/// `StoredFontSelection` is the one codec for `fontType` + `customFontFile` +
/// `installedFontFamily` + `installedFontFace`; the pane writes through it and
/// the store reads through it, so what it says about half-written keys is what
/// both do.
final class StoredFontSelectionTests: XCTestCase {
    private func decode(
        _ fontType: String, file: String = "", family: String = "", face: String = "",
    ) -> StoredFontSelection {
        StoredFontSelection(
            fontType: fontType, customFontFile: file, installedFontFamily: family, installedFontFace: face,
        )
    }

    func testABundledRawValue_decodesToThatFace_ignoringStaleCompanions() {
        XCTAssertEqual(
            decode("genYoMin", file: "left-over.ttf", family: "Helvetica", face: "Bold"),
            .builtIn(.genYoMin),
        )
    }

    func testAnUnknownRawValue_decodesToTheSystemFace() {
        XCTAssertEqual(decode("comicSans"), .builtIn(.system))
    }

    func testCustomWithAFile_decodesToThatFile() {
        XCTAssertEqual(decode("custom", file: "mine.ttf"), .customFile("mine.ttf"))
    }

    func testInstalledWithAFamily_decodesToThatFamily() {
        XCTAssertEqual(decode("installed", family: "Helvetica"), .installedFamily("Helvetica"))
        XCTAssertEqual(
            decode("installed", family: "Helvetica Neue", face: "Bold"),
            .installedFamily("Helvetica Neue", face: "Bold"),
        )
    }

    /// The keys are separate writes; a reader landing between them sees the
    /// default face, not a half-written selection.
    func testCustomOrInstalled_withNothingBesideIt_decodesToTheSystemFace() {
        XCTAssertEqual(decode("custom"), .builtIn(.system))
        XCTAssertEqual(decode("installed", face: "Bold"), .builtIn(.system))
    }

    /// Encoding writes every key, so a bundled or installed pick clears a
    /// stale file name, a custom pick clears a stale family and face, and
    /// another family's pick clears a face — and every value round-trips
    /// through its own four keys.
    func testEncoding_roundTrips_andClearsTheOtherKinds() {
        for selection in [
            StoredFontSelection.builtIn(.iansui), .customFile("mine.ttf"), .installedFamily("Helvetica"),
            .installedFamily("Helvetica Neue", face: "Light"),
        ] {
            let decoded = decode(
                selection.fontType,
                file: selection.customFontFile,
                family: selection.installedFontFamily,
                face: selection.installedFontFace,
            )
            XCTAssertEqual(decoded, selection)
        }
        XCTAssertEqual(StoredFontSelection.installedFamily("Helvetica").customFontFile, "")
        XCTAssertEqual(StoredFontSelection.installedFamily("Helvetica").installedFontFace, "")
        XCTAssertEqual(StoredFontSelection.customFile("mine.ttf").installedFontFamily, "")
        XCTAssertEqual(StoredFontSelection.customFile("mine.ttf").installedFontFace, "")
        XCTAssertEqual(StoredFontSelection.builtIn(.iansui).customFontFile, "")
    }

    /// One table row per family: its row is the stored selection's at any
    /// weight, and only for installed families.
    func testAFamily_isTheSameTypefaceAtAnyWeight() {
        let boldHelvetica = StoredFontSelection.installedFamily("Helvetica Neue", face: "Bold")

        XCTAssertTrue(StoredFontSelection.installedFamily("Helvetica Neue").isSameTypeface(as: boldHelvetica))
        XCTAssertFalse(StoredFontSelection.installedFamily("Helvetica").isSameTypeface(as: boldHelvetica))
        XCTAssertFalse(StoredFontSelection.customFile("Helvetica Neue").isSameTypeface(as: boldHelvetica))
        XCTAssertTrue(StoredFontSelection.customFile("a.ttf").isSameTypeface(as: .customFile("a.ttf")))
    }
}
