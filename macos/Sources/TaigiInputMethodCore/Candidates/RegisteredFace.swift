// Resolving a PostScript name to the face Core Text has registered right now.

import AppKit
import CoreText

/// A PostScript name resolved through Core Text's registration list rather than
/// through `NSFont(name:)`.
///
/// `NSFont(name:)` looks for an existing font object before it builds one, and
/// `CTFontManagerUnregisterFontsForURL` does not clear what that lookup cached.
/// Measured on macOS 26.5: register a file, unregister it, delete it, then
/// register a second file carrying the same PostScript name, and
/// `NSFont(name:)` answers with the FIRST file's URL at every step, while
/// `CTFontManagerCopyAvailableFontFamilyNames` correctly drops and re-adds the
/// family. That is what refused every re-import of a typeface the user had
/// removed (`CustomFontLibrary`), and what would have left the candidate window
/// drawing out of the file they threw away.
///
/// Descriptor matching reflects the registration list, so it stops matching the
/// moment a font is unregistered. It costs about seven times a cached
/// `NSFont(name:)` lookup, which is why the answers are memoized below.
enum RegisteredFace {
    /// Whether a face carrying `postScriptName` is registered — in this
    /// process, or on the system.
    static func isRegistered(named postScriptName: String) -> Bool {
        descriptor(named: postScriptName) != nil
    }

    /// The file the face registered under `postScriptName` was read from, or
    /// nil because no registered face carries that name.
    static func fileURL(named postScriptName: String) -> URL? {
        guard let matched = descriptor(named: postScriptName) else { return nil }
        return CTFontDescriptorCopyAttribute(matched, kCTFontURLAttribute) as? URL
    }

    /// The face registered under `postScriptName` at `size`, or nil because no
    /// registered face carries that name.
    ///
    /// Every candidate label and every width measurement asks this, up to a few
    /// hundred times per keystroke, for an answer that changes only when
    /// something is registered or unregistered — so a resolved face is kept
    /// until `forgetResolvedFaces()` says the list moved.
    static func font(named postScriptName: String, ofSize size: CGFloat) -> NSFont? {
        let key = "\(postScriptName)@\(size)"
        if let cached = lock.withLock({ resolvedFaces[key] }) {
            return cached
        }
        guard let matched = descriptor(named: postScriptName) else { return nil }
        let font = NSFont(descriptor: matched as NSFontDescriptor, size: size)
        // Only a hit is kept. A name that resolves to nothing is the case a
        // later registration is expected to change, and it is off the drawing
        // path anyway — a custom selection reaches the window only once the
        // library has it activated (`SettingsStore.candidateFontSelection`).
        if let font {
            lock.withLock { resolvedFaces[key] = font }
        }
        return font
    }

    /// Drops what `font(named:ofSize:)` resolved, because the registration list
    /// has moved.
    ///
    /// Only a withdrawal has to say so (`CustomFontLibrary.withdraw`). Nothing
    /// is kept for a name that resolved to nothing, and a name that DID resolve
    /// cannot be registered a second time — the import that would carries the
    /// refusal `nameAlreadyResolves` — so registering never invalidates
    /// anything. The bundled roster is activated once from the bundle's
    /// Info.plist before any candidate is drawn and never withdrawn; a typeface
    /// installed or removed in Font Book while this process runs is not noticed
    /// until it next restarts.
    static func forgetResolvedFaces() {
        lock.withLock { resolvedFaces.removeAll() }
    }

    /// The registered descriptor carrying `postScriptName`, matched from a
    /// query built fresh each time — a descriptor kept from an earlier match
    /// would be the very cache this type exists to step around.
    ///
    /// The matched descriptor's name is checked rather than trusted: matching
    /// SUBSTITUTES when nothing carries the name, the same way `NSFont(name:)`
    /// does, and a substituted face is not the one that was asked for. A family
    /// name resolves this way too — `Menlo` matches `Menlo-Regular`, and is
    /// refused, since the caller asked for a PostScript name.
    private static func descriptor(named postScriptName: String) -> CTFontDescriptor? {
        let query = CTFontDescriptorCreateWithAttributes(
            [kCTFontNameAttribute: postScriptName] as CFDictionary,
        )
        guard let matched = CTFontDescriptorCreateMatchingFontDescriptor(
            query, Set([kCTFontNameAttribute]) as CFSet,
        ),
            CTFontDescriptorCopyAttribute(matched, kCTFontNameAttribute) as? String == postScriptName
        else { return nil }
        return matched
    }

    /// Resolved faces by name and size. Locked rather than actor-isolated
    /// because `CandidateMetrics` is a `Sendable` value whose font properties
    /// are read wherever a cell is measured.
    private nonisolated(unsafe) static var resolvedFaces: [String: NSFont] = [:]
    private static let lock = NSLock()
}
