// The typefaces the user added themselves: where they are kept, how one is taken in, and what activates it.

import AppKit
import CoreText

/// One typeface the user added, as the library holds it.
///
/// `fileName` is the identity. An import never overwrites — a name already
/// taken gets a numeric suffix — so one file name is one set of bytes for as
/// long as the file exists, which is what lets a selection, a metrics value and
/// a cached panel all agree on WHICH custom font is meant
/// (`CandidateFontSelection`).
///
/// `postScriptName` is what Core Text read out of the file's first face, kept
/// exactly as it came: it is the name `NSFont(name:)` is asked for, and a
/// lowercased or otherwise tidied copy would resolve to nothing.
///
/// `displayName` is the font's own name, shown in the picker. Both names come
/// out of a file the user chose — untrusted text. They are displayed, never
/// logged and never used to build a path.
struct CustomFont: Hashable, Identifiable, Sendable {
    let fileName: String
    let postScriptName: String
    let displayName: String

    /// The stored file name IS the identity — imports never overwrite — so a
    /// table's selection cannot survive into a different typeface.
    var id: String {
        fileName
    }
}

/// The user's own typefaces: the directory they are copied into, the import
/// that validates one, and the Core Text registration that makes it drawable.
///
/// Kept as files in a directory with no index beside them. The directory IS the
/// list, and Core Text reads both names back out of each file, so there is one
/// authority rather than a catalogue to reconcile with the files it describes.
///
/// `@MainActor` because registration and `NSFont` resolution are AppKit-side,
/// and because the settings window and the candidate window — the only two
/// callers — are both on the main actor of this one process.
@MainActor
final class CustomFontLibrary {
    /// The process-wide library. The candidate window resolves the selected
    /// font through it on every show, so its cache is worth sharing.
    static let shared = CustomFontLibrary()

    /// Where the fonts are kept when it is not the user's own container — what
    /// a test drives, so no case writes into the typefaces of whoever is
    /// running it.
    private let directoryOverride: URL?

    /// A library kept in `directory`, or in the user's own container when that
    /// is nil. The parameter is for tests; the app has `shared`.
    init(directory: URL? = nil) {
        directoryOverride = directory
    }

    /// Why a file could not join the library. Every case leaves the library
    /// exactly as it was — a rejected import copies nothing and registers
    /// nothing.
    enum ImportFailure: Error, CustomStringConvertible {
        case unsupportedFileType(String)
        case notARegularFile
        case tooLarge(bytes: Int)
        case noFace
        case nameAlreadyResolves(String)
        case registrationFailed(String)
        case didNotResolve(String)

        var description: String {
            switch self {
            case let .unsupportedFileType(fileExtension):
                "\(fileExtension.isEmpty ? "a file with no extension" : ".\(fileExtension)") is not a font file"
            case .notARegularFile:
                "not a regular file"
            case let .tooLarge(bytes):
                "the file is \(bytes / (1024 * 1024)) MB, over the \(CustomFontLibrary.maximumFileSize / (1024 * 1024)) MB limit"
            case .noFace:
                "the file carries no typeface this Mac can read"
            case let .nameAlreadyResolves(postScriptName):
                "another typeface is already called \(postScriptName)"
            case let .registrationFailed(reason):
                "the typeface did not activate: \(reason)"
            case let .didNotResolve(postScriptName):
                "the typeface activated but \(postScriptName) still draws in another face"
            }
        }
    }

    /// Why a font could not leave the library.
    enum RemovalFailure: Error, CustomStringConvertible {
        case stillInUse(String)

        var description: String {
            switch self {
            case let .stillInUse(reason): "the typeface is still in use: \(reason)"
            }
        }
    }

    /// What a font file may be named. Collections are taken in as well as
    /// single faces; a `.ttc` contributes its first face to the picker, and its
    /// others are checked for name collisions because registering the URL
    /// registers the whole collection.
    nonisolated static let allowedFileExtensions: Set<String> = ["ttf", "otf", "ttc"]

    /// The most one file may weigh. The largest bundled face is already over
    /// 20 MB, so this is generous rather than tight — a screening limit against
    /// a mistaken pick, not a security boundary.
    nonisolated static let maximumFileSize = 64 * 1024 * 1024

    /// The directory name inside the per-user container.
    private nonisolated static let directoryName = "Fonts"

    private let fileManager = FileManager.default
    /// The directory, resolved once. Resolving it creates it, and the container
    /// cannot move under a running process — so asking again per candidate
    /// window would be three filesystem calls a keystroke for a constant.
    private var cachedDirectory: URL?
    /// The scan's answer, kept until an import or a removal moves it. The
    /// PICKER's roster: reading every file's descriptors is a parse per file,
    /// and the rendering path never needs the other files
    /// (`activate(fileName:)` resolves the one it is given).
    private var cachedFonts: [CustomFont]?
    /// What this process has registered, by stored file name — so a second
    /// activation is not asked of Core Text, which reports one as an error, and
    /// so the per-show path can skip the name check it already made.
    private var activatedFonts: [String: CustomFont] = [:]

    private let logger = DebugLogger(category: "CustomFont")

    /// `~/Library/Application Support/<bundle id>/Fonts/`, created if absent.
    ///
    /// Under the same per-bundle container as the learning databases
    /// (`UserDataDirectory`) rather than a path of its own: one directory is
    /// what a user backs up, and what an uninstall takes away.
    func directory() throws -> URL {
        if let cachedDirectory {
            return cachedDirectory
        }
        let resolved = try UserDataDirectory.created(
            directoryOverride
                ?? UserDataDirectory.standard().appendingPathComponent(Self.directoryName),
        )
        cachedDirectory = resolved
        return resolved
    }

    /// Every typeface the library holds, by display name.
    ///
    /// A file that no longer parses is skipped rather than failing the scan:
    /// one unreadable file must not empty the picker of the others.
    func installedFonts() -> [CustomFont] {
        if let cachedFonts {
            return cachedFonts
        }
        let fonts = scan()
        cachedFonts = fonts
        return fonts
    }

    /// The font stored as `fileName`, or nil because it is gone or unreadable.
    ///
    /// Read from that one file rather than out of the scan: this is the
    /// rendering path's question, and parsing the whole library to answer it
    /// would put every unselected typeface through Core Text at launch.
    func font(fileName: String) -> CustomFont? {
        guard let directory = try? directory() else { return nil }
        return makeFont(at: directory.appendingPathComponent(fileName))
    }

    /// The typeface `fileName` is drawing in right now, or nil because this
    /// process never activated it or its file has since gone.
    ///
    /// The RENDERING path's question, and cheap on purpose: a dictionary hit
    /// and one `stat`. Activation itself is a lifecycle event — `activate` at
    /// launch and when the picker's selection changes — so the candidate window
    /// neither parses a font file nor talks to Core Text on the way to a
    /// keystroke's candidates. The `stat` stays: a file deleted from under us
    /// keeps drawing out of a live registration, and a window set in a typeface
    /// the user threw away is a lie.
    func activatedFont(fileName: String) -> CustomFont? {
        guard let font = activatedFonts[fileName],
              let url = try? directory().appendingPathComponent(fileName),
              fileManager.fileExists(atPath: url.path)
        else { return nil }
        return font
    }

    /// Makes `fileName`'s typeface drawable in this process and answers what it
    /// is, or nil because the file is missing, unreadable, or would not
    /// activate.
    ///
    /// Called for the SELECTED font — at launch, and again when the user picks
    /// one — rather than for the whole library: activating fonts the user did
    /// not choose spends launch time parsing them and puts their bytes through
    /// the text stack for nothing.
    @discardableResult
    func activate(fileName: String) -> CustomFont? {
        guard !fileName.isEmpty,
              let url = try? directory().appendingPathComponent(fileName)
        else { return nil }
        if activatedFonts[fileName] != nil {
            return activatedFont(fileName: fileName)
        }
        guard let font = makeFont(at: url) else { return nil }
        if let reason = register(url) {
            logger.error("[FONT] custom typeface did not activate: \(reason)")
            return nil
        }
        // Registered is not drawn: a name a system face also carries resolves
        // to that face instead, and a row drawing in someone else's typeface is
        // worse than one that fell back.
        guard draws(font.postScriptName, from: url) else {
            _ = unregister(url)
            logger.error("[FONT] custom typeface resolves to another face")
            return nil
        }
        activatedFonts[fileName] = font
        return font
    }

    /// Copies `url`'s typeface into the library, activates it, and answers what
    /// it became.
    ///
    /// Validation runs against the COPY, not the file the user picked: the copy
    /// is what will be registered and drawn, and the original may change or go
    /// away. Anything that fails after the copy takes the copy back out again,
    /// so a refused import leaves no file behind.
    @discardableResult
    func addFont(from url: URL) throws -> CustomFont {
        let fileExtension = url.pathExtension.lowercased()
        guard Self.allowedFileExtensions.contains(fileExtension) else {
            throw ImportFailure.unsupportedFileType(fileExtension)
        }
        let values = try url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey])
        guard values.isRegularFile == true, values.isSymbolicLink != true else {
            throw ImportFailure.notARegularFile
        }
        let size = values.fileSize ?? 0
        guard size <= Self.maximumFileSize else { throw ImportFailure.tooLarge(bytes: size) }

        let destination = try unusedURL(for: url)
        try fileManager.copyItem(at: url, to: destination)
        do {
            let font = try take(in: destination)
            cachedFonts = nil
            return font
        } catch {
            discard(destination)
            throw error
        }
    }

    /// Unregisters `font` and deletes its file.
    ///
    /// Unregistration first, and its failure is the caller's to hear: Core Text
    /// refuses while something still holds the font, and deleting the file
    /// under a live registration would leave this process drawing from a file
    /// that no longer exists. The candidate window's panels retain the fonts
    /// they were built with, so a caller releases those before calling this
    /// (`CustomFontsPage.remove`).
    func remove(_ font: CustomFont) throws {
        let url = try directory().appendingPathComponent(font.fileName)
        if activatedFonts[font.fileName] != nil {
            if let reason = unregister(url) {
                throw RemovalFailure.stillInUse(reason)
            }
            activatedFonts[font.fileName] = nil
        }
        try fileManager.removeItem(at: url)
        cachedFonts = nil
    }

    /// Forgets the scanned roster, so the next `installedFonts()` re-reads the
    /// directory. For a caller that knows the files changed without going
    /// through this library — a test, or a pane re-read after the user may have
    /// been in Finder.
    func invalidateCache() {
        cachedFonts = nil
    }

    // MARK: - The import, step by step

    /// Registers the copy at `url`, checks that the face it names is the one
    /// that draws, and answers the font that joined the library.
    private func take(in url: URL) throws -> CustomFont {
        guard let descriptors = readFaces(at: url), let first = descriptors.first else {
            throw ImportFailure.noFace
        }
        // Registering the URL registers every face in it, so a collection's
        // other faces are checked too: a name that already resolves would make
        // the picker's new row draw in whichever face won.
        for face in descriptors where resolves(face.postScriptName) {
            throw ImportFailure.nameAlreadyResolves(face.postScriptName)
        }
        if let reason = register(url) {
            throw ImportFailure.registrationFailed(reason)
        }
        let font = CustomFont(
            fileName: url.lastPathComponent,
            postScriptName: first.postScriptName,
            displayName: first.displayName,
        )
        activatedFonts[font.fileName] = font
        // Resolving is not enough: the name has to resolve to THIS file. A name
        // another face already carries would otherwise pass the check while the
        // picker's new row drew in that other face.
        guard draws(first.postScriptName, from: url) else {
            throw ImportFailure.didNotResolve(first.postScriptName)
        }
        return font
    }

    /// Takes a half-finished import back out: the registration if this process
    /// made one, then the file.
    ///
    /// The file is deleted only once the registration is gone. Core Text
    /// refuses to unregister a font that is in use, and deleting under a live
    /// registration would leave this process holding a face backed by nothing —
    /// so a failed unregistration KEEPS both the file and the ownership of it,
    /// and the import's own error is what the user is told. The leftover is a
    /// picker row, which `remove` can retry; the alternative is a dangling
    /// registration nothing can reach.
    private func discard(_ url: URL) {
        let fileName = url.lastPathComponent
        if activatedFonts[fileName] != nil {
            if let reason = unregister(url) {
                logger.error("[FONT] rolled-back import stayed registered: \(reason)")
                return
            }
            activatedFonts[fileName] = nil
        }
        do {
            try fileManager.removeItem(at: url)
        } catch {
            logger.error("[FONT] rolled-back import left its copy behind: \(error)")
        }
    }

    /// Where `url`'s bytes will be copied: a sanitized name inside the library,
    /// suffixed until it is one no file there has.
    ///
    /// The stored name is built from characters this method chose, never from
    /// the picked file's own name verbatim — a name is untrusted text, and this
    /// one becomes a path.
    private func unusedURL(for url: URL) throws -> URL {
        let directory = try directory()
        let fileExtension = url.pathExtension.lowercased()
        let base = Self.sanitized(url.deletingPathExtension().lastPathComponent)
        var candidate = "\(base).\(fileExtension)"
        var suffix = 2
        while fileManager.fileExists(atPath: directory.appendingPathComponent(candidate).path) {
            candidate = "\(base)-\(suffix).\(fileExtension)"
            suffix += 1
        }
        return directory.appendingPathComponent(candidate)
    }

    /// `name` reduced to the characters a file name may hold here: ASCII
    /// letters, digits, `-` and `_`. Everything else — separators, dots, the
    /// name's own script — becomes `-`, and a name left with nothing is
    /// replaced outright, so the result can neither escape the directory nor
    /// hide an extension.
    nonisolated static func sanitized(_ name: String) -> String {
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyz0123456789-_")
        let reduced = String(name.lowercased().unicodeScalars.map { allowed.contains($0) ? Character($0) : "-" })
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
        return reduced.isEmpty ? "typeface" : String(reduced.prefix(64))
    }

    // MARK: - Core Text

    /// The faces `url`'s file declares, without registering it. Nil when the
    /// file is not a font this Mac can read.
    private func readFaces(at url: URL) -> [(postScriptName: String, displayName: String)]? {
        guard let descriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] else {
            return nil
        }
        return descriptors.compactMap { descriptor in
            guard let postScriptName = CTFontDescriptorCopyAttribute(descriptor, kCTFontNameAttribute) as? String
            else { return nil }
            let displayName = CTFontDescriptorCopyAttribute(descriptor, kCTFontDisplayNameAttribute) as? String
            return (postScriptName, displayName ?? postScriptName)
        }
    }

    /// The font `url`'s first face declares, or nil because the file is not one
    /// this Mac can read. A `.ttc` answers with its first face, which is the one
    /// the picker offers.
    private func makeFont(at url: URL) -> CustomFont? {
        guard let first = readFaces(at: url)?.first else { return nil }
        return CustomFont(
            fileName: url.lastPathComponent,
            postScriptName: first.postScriptName,
            displayName: first.displayName,
        )
    }

    /// Registers `url` for this process, answering nil on success or why not.
    ///
    /// Process scope: the settings window and the candidate window are one
    /// process, and a typeface this app took in is not one the user's other
    /// apps asked for.
    private func register(_ url: URL) -> String? {
        registration(of: url, registering: true)
    }

    /// Unregisters `url`, answering nil on success or why not — `inUse` being
    /// the case a caller has to act on.
    private func unregister(_ url: URL) -> String? {
        registration(of: url, registering: false)
    }

    private func registration(of url: URL, registering: Bool) -> String? {
        var error: Unmanaged<CFError>?
        let changed = registering
            ? CTFontManagerRegisterFontsForURL(url as CFURL, .process, &error)
            : CTFontManagerUnregisterFontsForURL(url as CFURL, .process, &error)
        return changed ? nil : Self.reason(from: error)
    }

    private nonisolated static func reason(from error: Unmanaged<CFError>?) -> String {
        guard let error = error?.takeRetainedValue() else { return "unknown error" }
        return CFErrorCopyDescription(error) as String? ?? "error \(CFErrorGetCode(error))"
    }

    /// Whether `postScriptName` names a face that draws — and that face itself,
    /// not a substitution. The check `CandidateFontChoice` runs for the bundled
    /// roster, asked here of a name the library is about to publish.
    private func resolves(_ postScriptName: String) -> Bool {
        CandidateFontChoice.font(named: postScriptName, ofSize: NSFont.systemFontSize)
            .fontName == postScriptName
    }

    /// Whether `postScriptName` draws out of `url` — the file the library
    /// stores — rather than out of some other face carrying the same name.
    private func draws(_ postScriptName: String, from url: URL) -> Bool {
        let font = CandidateFontChoice.font(named: postScriptName, ofSize: NSFont.systemFontSize)
        guard font.fontName == postScriptName,
              let source = CTFontCopyAttribute(font as CTFont, kCTFontURLAttribute) as? URL
        else { return false }
        return source.standardizedFileURL == url.standardizedFileURL
    }

    /// The directory read straight through, skipping what does not parse.
    private func scan() -> [CustomFont] {
        guard let directory = try? directory(),
              let files = try? fileManager.contentsOfDirectory(
                  at: directory,
                  includingPropertiesForKeys: [.isRegularFileKey],
              )
        else { return [] }
        return files
            .filter { Self.allowedFileExtensions.contains($0.pathExtension.lowercased()) }
            .filter { (try? $0.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile) == true }
            .compactMap(makeFont(at:))
            .sorted { $0.displayName.localizedStandardCompare($1.displayName) == .orderedAscending }
    }
}
