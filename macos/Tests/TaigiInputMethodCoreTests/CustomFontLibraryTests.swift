// The typefaces a user adds: what the library takes in, what it refuses, and what it leaves on disk.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The import gate. Every case here is about a file the library must NOT take —
/// the happy path needs a typeface no other face on the running Mac carries the
/// name of, which a test process cannot arrange (`TestFixtures` registers the
/// bundled roster process-wide, and the system carries the rest). What a
/// successful import looks like is dogfood item S30.
///
/// What the refusals have in common is the property worth pinning: a rejected
/// import leaves NOTHING behind — no copy, no registration, no picker row.
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
            [CustomFont(fileName: "mine.ttf", postScriptName: "Iansui-Regular", displayName: "Iansui Regular")],
        )
        XCTAssertEqual(library.font(fileName: "mine.ttf")?.postScriptName, "Iansui-Regular")
        XCTAssertNil(
            library.activatedFont(fileName: "mine.ttf"),
            "a file in the directory is not a typeface this process activated",
        )
    }

    // MARK: - Stored names

    /// The stored name is built here, never taken from the picked file: it
    /// becomes a path, and the name it came from is the user's to choose.
    func testSanitized_reducesANameToWhatMayBeAPathComponent() {
        XCTAssertEqual(CustomFontLibrary.sanitized("My Font"), "my-font")
        XCTAssertEqual(CustomFontLibrary.sanitized("../../etc/passwd"), "etc-passwd")
        XCTAssertEqual(CustomFontLibrary.sanitized("源樣明體"), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized(""), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized("..."), "typeface")
        XCTAssertEqual(CustomFontLibrary.sanitized(String(repeating: "a", count: 200)).count, 64)
    }

    // MARK: -

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
