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
/// exactly as it came: it is the name `RegisteredFace` is asked for, and a
/// lowercased or otherwise tidied copy would resolve to nothing.
struct CustomFont: Hashable, Sendable {
    let fileName: String
    let postScriptName: String

    /// What the picker lists this typeface as: the stored file's name without
    /// its extension.
    ///
    /// The file's name rather than the name the font file declares (USER
    /// 2026-09-10). A row is something the user has to recognise as the thing
    /// they added, and what they added was a file they chose and named; a font
    /// can declare a name that means nothing to them, and the two desktop
    /// platforms read a different name out of the same file anyway — Core Text
    /// gives "Iansui Regular" where DirectWrite gives "Iansui".
    ///
    /// Derived rather than stored, so there is no second copy of a name to
    /// drift from the file it belongs to. Untrusted text still: shown, never
    /// logged (`CustomFontLibrary.sanitized`).
    var displayName: String {
        (fileName as NSString).deletingPathExtension
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
        case unreadable(String)
        case notCopied(String)
        case noFace
        case nameAlreadyResolves(String)
        case registrationFailed(String)
        case didNotResolve(String)
        case noAvailableName

        var description: String {
            switch self {
            case let .unsupportedFileType(fileExtension):
                "\(fileExtension.isEmpty ? "a file with no extension" : ".\(fileExtension)") is not a font file"
            case .notARegularFile:
                "not a regular file"
            case let .tooLarge(bytes):
                "the file is \(bytes / (1024 * 1024)) MB, over the \(CustomFontLibrary.maximumFileSize / (1024 * 1024)) MB limit"
            case let .unreadable(reason):
                "could not read the file: \(reason)"
            case let .notCopied(reason):
                "could not copy the file into the library: \(reason)"
            case .noFace:
                "the file carries no typeface this Mac can read"
            case let .nameAlreadyResolves(postScriptName):
                "another typeface is already called \(postScriptName)"
            case let .registrationFailed(reason):
                "the typeface did not activate: \(reason)"
            case let .didNotResolve(postScriptName):
                "the typeface activated but \(postScriptName) still draws in another face"
            case .noAvailableName:
                "every name this typeface could be stored under is taken"
            }
        }
    }

    /// Why a font could not leave the library.
    enum RemovalFailure: Error, CustomStringConvertible {
        case stillInUse(String)
        case invalidFileName
        case notRemoved(String)

        var description: String {
            switch self {
            case let .stillInUse(reason): "the typeface is still in use: \(reason)"
            case .invalidFileName: "invalid typeface file name"
            case let .notRemoved(reason): "could not remove the typeface file: \(reason)"
            }
        }
    }

    /// What a font file may be named (`taigi-desktop-storage::font_library`).
    nonisolated static var allowedFileExtensions: Set<String> {
        FontLibraryStorage.allowedExtensions
    }

    /// The most one file may weigh (`taigi-desktop-storage::font_library`).
    nonisolated static var maximumFileSize: Int {
        FontLibraryStorage.maxFileSize
    }

    /// How many name attempts one stem tries before giving up (`taigi-desktop-storage::font_library`).
    nonisolated static var maximumNameAttempts: Int {
        FontLibraryStorage.maxNameAttempts
    }

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
    /// What this process registered with Core Text and has not withdrawn again,
    /// by stored file name — so a second registration is not asked of Core
    /// Text, which reports one as an error, and so a removal knows what it has
    /// to withdraw.
    private var registrations: [String: Registration] = [:]

    /// A live registration, and whether the face it activated was then
    /// confirmed to draw out of this library's own file.
    ///
    /// The two facts are one value because they are one lifecycle, and separate
    /// fields because they are not the same permission: a face that resolved to
    /// somebody else's typeface is still this process's to unregister, and must
    /// still never reach the candidate window.
    private struct Registration {
        let font: CustomFont
        /// Every family the file carries — all of a `.ttc`'s, since registering
        /// the URL registered every face in it. Read when the file was, so the
        /// Manage Fonts pane's exclusion costs no parse (`registeredFamilies`).
        let families: Set<String>
        var isDrawable: Bool
    }

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
        let target: URL
        if let directoryOverride {
            target = directoryOverride
        } else {
            guard let folderName = FontLibraryStorage.fontsFolderName else {
                throw FontLibraryStorage.BridgeUnavailable.missingFolderName
            }
            target = try UserDataDirectory.standard().appendingPathComponent(folderName)
        }
        let resolved = try UserDataDirectory.created(target)
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
        guard let url = try? storedURL(for: fileName) else { return nil }
        return makeFont(at: url)
    }

    /// The typeface `fileName` is drawing in right now, or nil because this
    /// process never got it drawing or its file has since gone.
    ///
    /// The RENDERING path's question, and cheap on purpose: a dictionary hit
    /// and one `stat`. Activation itself is a lifecycle event — `activate` at
    /// launch and when the picker's selection changes — so the candidate window
    /// neither parses a font file nor talks to Core Text on the way to a
    /// keystroke's candidates. The `stat` stays: a file deleted from under us
    /// keeps drawing out of a live registration, and a window set in a typeface
    /// the user threw away is a lie.
    func activatedFont(fileName: String) -> CustomFont? {
        guard let registration = registrations[fileName], registration.isDrawable,
              let url = try? storedURL(for: fileName),
              fileManager.fileExists(atPath: url.path)
        else { return nil }
        return registration.font
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
        guard let url = try? storedURL(for: fileName) else { return nil }
        // Registered once already: Core Text reports a second registration of
        // the same file as an error, and nothing about the answer has changed
        // since the first one — including a "no", when the face turned out to
        // draw as somebody else's.
        if let registration = registrations[fileName] {
            return registration.isDrawable ? activatedFont(fileName: fileName) : nil
        }
        guard let faces = readFaces(at: url), let first = faces.first else { return nil }
        if let reason = register(url) {
            logger.error("[FONT] custom typeface did not activate: \(reason)")
            return nil
        }
        let font = CustomFont(fileName: fileName, postScriptName: first.postScriptName)
        // Ownership is recorded on the registration, not on the verdict below:
        // a registration this process made is this process's to withdraw even
        // when the withdrawal fails, and `remove` is what retries it.
        registrations[fileName] = Registration(font: font, families: Set(faces.map(\.familyName)), isDrawable: false)
        // Registered is not drawn: a name a system face also carries resolves
        // to that face instead, and a row drawing in someone else's typeface is
        // worse than one that fell back.
        guard draws(font.postScriptName, from: url) else {
            if let reason = withdraw(url) {
                logger.error("[FONT] rejected typeface stayed registered: \(reason)")
            }
            logger.error("[FONT] custom typeface resolves to another face")
            return nil
        }
        registrations[fileName]?.isDrawable = true
        return font
    }

    /// Copies `url`'s typeface into the library via `taigi-desktop-storage`,
    /// activates it, and answers what it became.
    ///
    /// Validation runs against the COPY, not the file the user picked: the copy
    /// is what will be registered and drawn, and the original may change or go
    /// away. Anything that fails after the copy takes the copy back out again,
    /// so a refused import leaves no file behind.
    @discardableResult
    func addFont(from url: URL) throws -> CustomFont {
        let dir = try directory()
        let fileName = try FontLibraryStorage.copyIn(source: url, into: dir)
        guard let destination = try storedURL(for: fileName) else {
            try? FontLibraryStorage.removeStored(fileName: fileName, in: dir)
            throw ImportFailure.notCopied("invalid stored file name")
        }

        do {
            let font = try take(in: destination)
            cachedFonts = nil
            return font
        } catch {
            discard(destination, in: dir)
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
    /// (`AppearanceSettingsView.remove`).
    func remove(_ font: CustomFont) throws {
        let dir = try directory()
        guard let url = try storedURL(for: font.fileName) else {
            throw RemovalFailure.invalidFileName
        }
        if let reason = withdraw(url) {
            throw RemovalFailure.stillInUse(reason)
        }
        try FontLibraryStorage.removeStored(fileName: font.fileName, in: dir)
        cachedFonts = nil
    }

    /// Every family the files this process registered carry. What the
    /// Manage Fonts pane leaves out of the OS-installed list: these are rows of the
    /// library already, and a second row naming the same file as an installed
    /// family would select something no restart re-activates
    /// (`RegisteredFace.installedFamilies`). By name, so a family the OS also
    /// has goes missing from that list while a custom file carries a face of
    /// it — accepted: the custom row draws that family already.
    func registeredFamilies() -> Set<String> {
        registrations.values.reduce(into: []) { $0.formUnion($1.families) }
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
        guard let faces = readFaces(at: url), let first = faces.first else {
            throw ImportFailure.noFace
        }
        // Registering the URL registers every face in it, so a collection's
        // other faces are checked too: a name that already resolves would make
        // the picker's new row draw in whichever face won.
        for face in faces where RegisteredFace.isRegistered(named: face.postScriptName) {
            throw ImportFailure.nameAlreadyResolves(face.postScriptName)
        }
        if let reason = register(url) {
            throw ImportFailure.registrationFailed(reason)
        }
        let font = CustomFont(fileName: url.lastPathComponent, postScriptName: first.postScriptName)
        // Before the verdict below, so a `didNotResolve` throw reaches
        // `discard` with the registration owned rather than orphaned.
        registrations[font.fileName] = Registration(
            font: font, families: Set(faces.map(\.familyName)), isDrawable: false,
        )
        // Resolving is not enough: the name has to resolve to THIS file. A name
        // another face already carries would otherwise pass the check while the
        // picker's new row drew in that other face.
        guard draws(first.postScriptName, from: url) else {
            throw ImportFailure.didNotResolve(first.postScriptName)
        }
        registrations[font.fileName]?.isDrawable = true
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
    private func discard(_ url: URL, in dir: URL) {
        if let reason = withdraw(url) {
            logger.error("[FONT] rolled-back import stayed registered: \(reason)")
            return
        }
        do {
            try FontLibraryStorage.removeStored(fileName: url.lastPathComponent, in: dir)
        } catch {
            logger.error("[FONT] rolled-back import left its copy behind: \(error)")
        }
    }

    /// Resolves `fileName` inside `Fonts/`, returning `nil` when `fileName`
    /// could escape the library directory.
    private func storedURL(for fileName: String) throws -> URL? {
        guard let safeName = FontLibraryStorage.sanitizedComponent(fileName) else { return nil }
        let dir = try directory()
        return dir.appendingPathComponent(safeName, isDirectory: false)
    }

    /// `name` reduced to what one path component may hold, delegated to
    /// `taigi-desktop-storage::sanitized_stem`.
    nonisolated static func sanitized(_ name: String) -> String {
        FontLibraryStorage.sanitizedStem(name)
    }

    // MARK: - Core Text

    /// The two names a face in a file declares.
    private struct FaceNames {
        let postScriptName: String
        let familyName: String
    }

    /// The faces `url`'s file declares, without registering it. Nil when the
    /// file is not a font this Mac can read.
    private func readFaces(at url: URL) -> [FaceNames]? {
        guard let descriptors = CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor] else {
            return nil
        }
        return descriptors.compactMap { descriptor in
            guard let postScriptName = CTFontDescriptorCopyAttribute(descriptor, kCTFontNameAttribute) as? String,
                  let familyName = CTFontDescriptorCopyAttribute(descriptor, kCTFontFamilyNameAttribute) as? String
            else { return nil }
            return FaceNames(postScriptName: postScriptName, familyName: familyName)
        }
    }

    /// The font `url`'s first face declares, or nil because the file is not one
    /// this Mac can read. A `.ttc` answers with its first face, which is the one
    /// the picker offers.
    private func makeFont(at url: URL) -> CustomFont? {
        guard let first = readFaces(at: url)?.first else { return nil }
        return CustomFont(fileName: url.lastPathComponent, postScriptName: first.postScriptName)
    }

    /// Gives up this process's registration of `url`, answering nil when there
    /// is none left to give up or why Core Text refused.
    ///
    /// A refusal KEEPS the ownership: Core Text refuses while something still
    /// holds the font, and forgetting the registration then would leave one
    /// nothing can reach. The caller decides what a refusal means — `remove`
    /// tells the user, an import's rollback logs it.
    private func withdraw(_ url: URL) -> String? {
        let fileName = url.lastPathComponent
        guard registrations[fileName] != nil else { return nil }
        if let reason = unregister(url) {
            return reason
        }
        registrations[fileName] = nil
        RegisteredFace.forgetResolvedFaces()
        return nil
    }

    /// Registers `url` for this process, answering nil on success or why not.
    ///
    /// Process scope: the settings window and the candidate window are one
    /// process, and a typeface this app took in is not one the user's other
    /// apps asked for.
    private func register(_ url: URL) -> String? {
        registrationChange(of: url, registering: true)
    }

    /// Unregisters `url`, answering nil on success or why not — `inUse` being
    /// the case a caller has to act on.
    private func unregister(_ url: URL) -> String? {
        registrationChange(of: url, registering: false)
    }

    private func registrationChange(of url: URL, registering: Bool) -> String? {
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

    /// Whether `postScriptName` draws out of `url` — the file the library
    /// stores — rather than out of some other face carrying the same name.
    private func draws(_ postScriptName: String, from url: URL) -> Bool {
        guard let source = RegisteredFace.fileURL(named: postScriptName) else { return false }
        return source.standardizedFileURL == url.standardizedFileURL
    }

    /// The directory read straight through via `taigi-desktop-storage`, skipping
    /// what does not parse.
    private func scan() -> [CustomFont] {
        guard let directory = try? directory() else { return [] }
        return FontLibraryStorage.listStored(in: directory)
            .compactMap { makeFont(at: directory.appendingPathComponent($0, isDirectory: false)) }
            // Keep UI collation in Swift: `taigi-desktop-storage::stored_file_names`
            // returns deterministic codepoint order, whereas macOS sorts picker rows
            // by localized Finder-style `displayName` with `fileName` as the
            // tie-break (`mine.ttf` and `mine.otf` list under one title).
            .sorted {
                switch $0.displayName.localizedStandardCompare($1.displayName) {
                case .orderedAscending: true
                case .orderedDescending: false
                case .orderedSame: $0.fileName < $1.fileName
                }
            }
    }
}
