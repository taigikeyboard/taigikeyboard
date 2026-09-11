// The typefaces a user adds: what the library takes in, what it refuses, and what it leaves on disk.

import AppKit
import CoreText
@testable import TaigiInputMethodCore
import XCTest

/// The import gate: what the library takes in, what it refuses, and what a
/// refusal leaves behind — nothing, which is the property the refusal cases
/// have in common (no copy, no registration, no picker row).
///
/// A successful import needs a typeface no face on the running Mac carries the
/// name of, and the repository ships none the fixtures do not already register.
/// `withBundledFaceWithdrawn` makes one by borrowing a bundled face for the
/// length of a case. What an import looks like on a real Mac, with a file the
/// user chose, is dogfood item S30.
@MainActor
final class CustomFontLibraryTests: XCTestCase {
    private var directory: URL!
    private var library: CustomFontLibrary!

    override func setUpWithError() throws {
        try super.setUpWithError()
        let scratch = try TestFixtures.scratchDirectory()
        addTeardownBlock { try? FileManager.default.removeItem(at: scratch) }
        directory = scratch
        library = CustomFontLibrary(directory: scratch)
    }

    // MARK: - What the library refuses

    func testAddFont_withAFileThatIsNotAFont_isRefusedAndCopiesNothing() throws {
        let notAFont = try write("notes.txt", bytes: Data("hello".utf8))

        XCTAssertThrowsError(try library.addFont(from: notAFont)) { error in
            guard case CustomFontLibrary.ImportFailure.unsupportedFileType("txt") = error else {
                return XCTFail("unexpected refusal: \(error)")
            }
        }
        XCTAssertEqual(try storedFileNames(), [])
    }

    /// The extension says font, the bytes do not. Refused after the copy, which
    /// is what makes the leftover check below the point of the case.
    func testAddFont_withFontBytesThatDoNotParse_isRefusedAndLeavesNoCopy() throws {
        let garbage = try write("broken.ttf", bytes: Data(repeating: 0, count: 4096))

        XCTAssertThrowsError(try library.addFont(from: garbage)) { error in
            guard case CustomFontLibrary.ImportFailure.noFace = error else {
                return XCTFail("unexpected refusal: \(error)")
            }
        }
        XCTAssertEqual(try storedFileNames(), [], "a refused import left its copy behind")
        XCTAssertEqual(library.installedFonts(), [])
    }

    func testAddFont_withADirectoryNamedLikeAFont_isRefused() throws {
        let directoryURL = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("pretend-\(UUID().uuidString).otf")
        try FileManager.default.createDirectory(at: directoryURL, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directoryURL) }

        XCTAssertThrowsError(try library.addFont(from: directoryURL)) { error in
            guard case CustomFontLibrary.ImportFailure.notARegularFile = error else {
                return XCTFail("unexpected refusal: \(error)")
            }
        }
        XCTAssertEqual(try storedFileNames(), [])
    }

    /// A typeface whose name is already drawing — here one of the bundled four,
    /// which `TestFixtures` has registered — cannot be told apart from the face
    /// already carrying that name, so it is refused rather than added as a row
    /// that would draw in something else.
    func testAddFont_withAFaceWhoseNameAlreadyDraws_isRefusedAndLeavesNoCopy() throws {
        XCTAssertEqual(
            TestFixtures.unregisterableFontFiles, [],
            "the premise: the bundled faces are registered in this process",
        )
        let bundled = TestFixtures.fontDirectory.appendingPathComponent("iansui_regular.ttf")

        XCTAssertThrowsError(try library.addFont(from: bundled)) { error in
            guard case CustomFontLibrary.ImportFailure.nameAlreadyResolves = error else {
                return XCTFail("unexpected refusal: \(error)")
            }
        }
        XCTAssertEqual(try storedFileNames(), [])
    }

    // MARK: - What the library holds

    /// One unreadable file must not take the others out of the picker.
    func testInstalledFonts_skipsWhatWillNotParse() throws {
        try Data(repeating: 7, count: 512).write(to: directory.appendingPathComponent("broken.ttf"))
        try Data("csv,row".utf8).write(to: directory.appendingPathComponent("notes.csv"))

        XCTAssertEqual(library.installedFonts(), [])
    }

    /// The roster's names come out of the files themselves — no index beside
    /// them to drift, and no registration needed to read one.
    func testInstalledFonts_readsTheNamesOutOfTheFile() throws {
        let bundled = TestFixtures.fontDirectory.appendingPathComponent("iansui_regular.ttf")
        try FileManager.default.copyItem(at: bundled, to: directory.appendingPathComponent("mine.ttf"))

        XCTAssertEqual(
            library.installedFonts(),
            [CustomFont(fileName: "mine.ttf", postScriptName: "Iansui-Regular")],
        )
        XCTAssertEqual(library.font(fileName: "mine.ttf")?.postScriptName, "Iansui-Regular")
        XCTAssertEqual(
            library.installedFonts().first?.displayName, "mine",
            "the picker lists the stored file's name, not the name the font declares",
        )
        XCTAssertNil(
            library.activatedFont(fileName: "mine.ttf"),
            "a file in the directory is not a typeface this process activated",
        )
    }

    // MARK: - The round trip

    /// Add, remove, add the same typeface again — the sequence a user runs when
    /// they change their mind, and the one that was refused with "another
    /// typeface is already called …" until the library stopped asking
    /// `NSFont(name:)` whether a name was taken (`RegisteredFace`).
    ///
    /// The second import comes from a differently NAMED source, so it is stored
    /// beside a different file name than the first: the stale AppKit lookup
    /// answers with the file it cached, so a case where both imports land on
    /// one path could pass on a coincidence. What is asserted is therefore the
    /// resolved face's own URL, not merely its PostScript name.
    ///
    /// The premise the refusal cases cannot have: a PostScript name that
    /// nothing on the running Mac carries. Built by borrowing one of the
    /// bundled faces — withdrawn from this process for the length of the case,
    /// and put back after — since the repository ships no font the fixtures do
    /// not register.
    func testAddFont_afterTheSameTypefaceWasRemoved_isTakenInAgainAndDrawsOutOfTheNewFile() throws {
        let bundled = TestFixtures.fontDirectory.appendingPathComponent("genyogothic2tw_r.otf")
        try withBundledFaceWithdrawn(bundled, named: "GenYoGothic2TW-R") {
            let first = try library.addFont(from: bundled)
            XCTAssertEqual(first.postScriptName, "GenYoGothic2TW-R")
            XCTAssertEqual(try storedFileNames(), [first.fileName])
            XCTAssertEqual(
                CandidateFontSelection.custom(first).font(ofSize: 13).fontName, "GenYoGothic2TW-R",
            )
            // The family the registered file carries is what the installed
            // list is told to leave out.
            XCTAssertEqual(library.registeredFamilies(), ["GenYoGothic2 TW"])

            try library.remove(first)
            XCTAssertEqual(library.registeredFamilies(), [], "a removed file still reports its family")
            XCTAssertEqual(try storedFileNames(), [], "removal left the file behind")
            XCTAssertEqual(library.installedFonts(), [])
            XCTAssertFalse(
                RegisteredFace.isRegistered(named: "GenYoGothic2TW-R"),
                "the name is still claimed after the typeface was removed",
            )
            XCTAssertEqual(
                CandidateFontSelection.custom(first).font(ofSize: 13).fontName,
                NSFont.systemFont(ofSize: 13).fontName,
                "a removed typeface still draws",
            )

            let renamedSource = try write("borrowed-face.otf", bytes: Data(contentsOf: bundled))
            let second = try library.addFont(from: renamedSource)
            XCTAssertTrue(
                second.fileName.hasSuffix("borrowed-face.otf"), "unexpected stored name \(second.fileName)",
            )
            XCTAssertEqual(library.installedFonts(), [second])
            XCTAssertEqual(
                library.activatedFont(fileName: second.fileName)?.postScriptName,
                "GenYoGothic2TW-R",
                "the re-imported typeface did not activate",
            )
            XCTAssertEqual(
                try fileURL(drawnBy: CandidateFontSelection.custom(second).font(ofSize: 13)),
                try library.directory().appendingPathComponent(second.fileName).standardizedFileURL,
                "the candidate window draws out of the file the user removed",
            )
        }
    }

    // MARK: - Stored names

    /// The stored name is ALSO the name the picker shows, so the user's own
    /// spelling survives — their capitals, their spaces, their script.
    func testSanitized_keepsThePickedNameAsTheUserSpelledIt() {
        XCTAssertEqual(CustomFontLibrary.sanitized("My Font"), "My Font")
        XCTAssertEqual(CustomFontLibrary.sanitized("源樣明體"), "源樣明體")
        XCTAssertEqual(CustomFontLibrary.sanitized("SnailFont-Pomacea"), "SnailFont-Pomacea")
        XCTAssertEqual(CustomFontLibrary.sanitized("jf-openhuninn-2.1"), "jf-openhuninn-2.1")
    }

    /// What it does not keep: anything that would make the name more than one
    /// path component, anything Windows refuses, and anything that draws
    /// nothing.
    func testSanitized_reducesANameToWhatMayBeAPathComponent() {
        XCTAssertEqual(CustomFontLibrary.sanitized("../../etc/passwd"), "etc-passwd")
        XCTAssertEqual(CustomFontLibrary.sanitized("a:b|c?d*e\"f<g>h"), "a-b-c-d-e-f-g-h")
        XCTAssertEqual(CustomFontLibrary.sanitized(""), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("..."), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("  . - "), "typeface")
    }

    /// Windows resolves a device name on the part before the first dot,
    /// ignoring case and trailing spaces — so `CON.foo.ttf` is `CON` to it.
    /// One naming rule for both platforms means refusing them here too.
    func testSanitized_refusesAWindowsDeviceName() {
        XCTAssertEqual(CustomFontLibrary.sanitized("CON"), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("nul"), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("CON.foo"), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("Com1"), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("COM¹"), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("CONSOLE"), "CONSOLE", "only the whole name is a device")
    }

    /// A name that draws nothing, or draws the rest of itself somewhere else.
    /// ZWNJ and ZWJ stay: they join letters and emoji, and dropping them
    /// rewrites text the user meant.
    func testSanitized_dropsTheInvisibleCharactersButKeepsTheJoiners() {
        XCTAssertEqual(CustomFontLibrary.sanitized("Fo\u{202E}nt"), "Font")
        XCTAssertEqual(CustomFontLibrary.sanitized("Fo\u{200B}nt"), "Font")
        XCTAssertEqual(CustomFontLibrary.sanitized("Fo\u{FEFF}nt"), "Font")
        XCTAssertEqual(CustomFontLibrary.sanitized("Fo\u{0009}nt"), "Font")
        XCTAssertEqual(CustomFontLibrary.sanitized("\u{200D}"), "typeface", "a joiner alone is still nothing")
        XCTAssertEqual(CustomFontLibrary.sanitized("क\u{200D}ष"), "क\u{200D}ष")
    }

    /// The cap counts Unicode scalars, the unit the Windows port counts, and
    /// the edges are trimmed again afterwards so a cut cannot leave a dot or a
    /// space behind.
    func testSanitized_capsTheLengthInScalarsAndTrimsWhatTheCutLeaves() {
        XCTAssertEqual(CustomFontLibrary.sanitized(String(repeating: "a", count: 200)).unicodeScalars.count, 64)
        XCTAssertEqual(CustomFontLibrary.sanitized(String(repeating: "字", count: 200)).unicodeScalars.count, 64)
        XCTAssertEqual(CustomFontLibrary.sanitized(String(repeating: "a", count: 63) + " tail"), String(repeating: "a", count: 63))
    }

    // MARK: -

    /// Runs `body` with `url`'s bundled face unregistered, so its PostScript
    /// name is one no face on this Mac carries — and registers it again
    /// afterwards, whether or not `body` threw, since the fixtures register the
    /// bundled roster once for the whole process and every other suite draws in
    /// it.
    private func withBundledFaceWithdrawn(
        _ url: URL, named postScriptName: String, _ body: () throws -> Void,
    ) throws {
        XCTAssertEqual(
            TestFixtures.unregisterableFontFiles, [],
            "the premise: the bundled faces are registered in this process",
        )
        guard CTFontManagerUnregisterFontsForURL(url as CFURL, .process, nil) else {
            return XCTFail("the bundled face could not be withdrawn")
        }
        // Registration is process-wide state every other suite draws in, so the
        // restoration runs however the body ends — and it takes the library's
        // own registrations with it first, since a case that failed part way
        // may have left one live.
        defer {
            library.invalidateCache()
            for font in library.installedFonts() {
                XCTAssertNoThrow(try library.remove(font), "a test typeface stayed registered")
            }
            XCTAssertTrue(
                CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil),
                "the bundled face was not put back for the other suites",
            )
        }
        // A Mac with this typeface installed for the user cannot have the
        // premise: the name stays claimed however this process registers it.
        try XCTSkipIf(
            RegisteredFace.isRegistered(named: postScriptName),
            "\(postScriptName) is installed on this Mac",
        )
        try body()
    }

    /// The file `font` is read from — what tells a face resolved out of the
    /// library's current file apart from one AppKit cached under the same name.
    private func fileURL(drawnBy font: NSFont) throws -> URL {
        try XCTUnwrap(CTFontCopyAttribute(font as CTFont, kCTFontURLAttribute) as? URL)
            .standardizedFileURL
    }

    private func write(_ name: String, bytes: Data) throws -> URL {
        let url = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("\(UUID().uuidString)-\(name)")
        try bytes.write(to: url)
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return url
    }

    private func storedFileNames() throws -> [String] {
        let contents = try? FileManager.default.contentsOfDirectory(atPath: library.directory().path)
        return (contents ?? []).sorted()
    }
}
