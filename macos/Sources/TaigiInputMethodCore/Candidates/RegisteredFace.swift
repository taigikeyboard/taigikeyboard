// Resolving a PostScript name or a family name to the face Core Text has registered right now.

import AppKit
import CoreText

/// A PostScript name, or a family name, resolved through Core Text's
/// registration list rather than through `NSFont(name:)`.
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
    /// What a face is asked for by. A PostScript name is one face; a family
    /// name is answered with its upright regular member, or whichever member
    /// it has when there is no such one.
    enum Query: Hashable {
        case postScript(String)
        case family(String)
    }

    /// Whether a face carrying `postScriptName` is registered — in this
    /// process, or on the system.
    static func isRegistered(named postScriptName: String) -> Bool {
        isRegistered(.postScript(postScriptName))
    }

    /// Whether a registered face answers `query`.
    static func isRegistered(_ query: Query) -> Bool {
        descriptor(for: query) != nil
    }

    /// The file the face registered under `postScriptName` was read from, or
    /// nil because no registered face carries that name.
    static func fileURL(named postScriptName: String) -> URL? {
        guard let matched = descriptor(for: .postScript(postScriptName)) else { return nil }
        return CTFontDescriptorCopyAttribute(matched, kCTFontURLAttribute) as? URL
    }

    /// The face registered under `postScriptName` at `size`, or nil because no
    /// registered face carries that name.
    static func font(named postScriptName: String, ofSize size: CGFloat) -> NSFont? {
        font(.postScript(postScriptName), ofSize: size)
    }

    /// The face answering `query` at `size`, or nil because none does.
    ///
    /// Every candidate label and every width measurement asks this, up to a few
    /// hundred times per keystroke, for an answer that changes only when
    /// something is registered or unregistered — so a resolved face is kept
    /// until `forgetResolvedFaces()` says the list moved.
    static func font(_ query: Query, ofSize size: CGFloat) -> NSFont? {
        let key = FontKey(query: query, size: size)
        if let cached = lock.withLock({ resolvedFonts[key] }) {
            return cached
        }
        guard let matched = descriptor(for: query),
              let font = NSFont(descriptor: matched as NSFontDescriptor, size: size)
        else { return nil }
        lock.withLock { resolvedFonts[key] = font }
        return font
    }

    /// Every family registered right now, sorted as Finder sorts, minus the
    /// families in `excluding` and the OS's hidden UI faces (`.`-prefixed).
    ///
    /// "Registered right now" is wider than "the OS has installed": the
    /// bundled roster and an activated custom font are registered by THIS
    /// process and appear here too, which is what `excluding` is for — those
    /// have rows of their own, and a second row for the same file would select
    /// nothing that survives a restart. Core Text keeps no cheap answer to
    /// "what did this process register" on macOS (matching on
    /// `kCTFontRegistrationScopeAttribute` answers nothing; filtering every
    /// available descriptor costs ~57 ms), so the caller names them
    /// (`FontManagementPage.reload`). Measured on this Mac: 261 families in
    /// 13–33 ms, sort included.
    static func installedFamilies(excluding: Set<String>) -> [String] {
        guard let names = CTFontManagerCopyAvailableFontFamilyNames() as? [String] else { return [] }
        return names
            .filter { !$0.hasPrefix(".") && !excluding.contains($0) }
            .sorted { $0.localizedStandardCompare($1) == .orderedAscending }
    }

    /// Drops what was resolved, because the registration list has moved.
    ///
    /// Called by `CustomFontLibrary.withdraw`, and by `FontRegistryObserver`
    /// for every change Core Text reports — a typeface installed or removed in
    /// Font Book included. Nothing is kept for a query that resolved to
    /// nothing, so a registration that ADDS a face never has a stale miss to
    /// clear.
    static func forgetResolvedFaces() {
        lock.withLock {
            resolvedDescriptors.removeAll()
            resolvedFonts.removeAll()
        }
    }

    /// The registered descriptor answering `query`, memoized on a hit.
    ///
    /// Built from a query made fresh on each miss — a descriptor kept from an
    /// earlier match would be the very cache this type exists to step around,
    /// which is why the memo is cleared whenever the list moves rather than
    /// consulted past it.
    private static func descriptor(for query: Query) -> CTFontDescriptor? {
        if let cached = lock.withLock({ resolvedDescriptors[query] }) {
            return cached
        }
        let matched: CTFontDescriptor? = switch query {
        case let .postScript(name):
            matchedDescriptor(attributes: [kCTFontNameAttribute: name], verifying: kCTFontNameAttribute, equals: name)
        case let .family(family):
            // Only the family is mandatory; the traits steer the match toward
            // the face a picker row means when it names a family.
            matchedDescriptor(
                attributes: [
                    kCTFontFamilyNameAttribute: family,
                    kCTFontTraitsAttribute: [
                        kCTFontWeightTrait: 0.0,
                        kCTFontSlantTrait: 0.0,
                        kCTFontSymbolicTrait: 0,
                    ] as [CFString: Any],
                ],
                verifying: kCTFontFamilyNameAttribute,
                equals: family,
            )
        }
        if let matched {
            lock.withLock { resolvedDescriptors[query] = matched }
        }
        return matched
    }

    /// One descriptor match with `key` mandatory, refused unless the match
    /// carries `expected` under `key`.
    ///
    /// Checked rather than trusted: matching SUBSTITUTES when nothing carries
    /// the value, the same way `NSFont(name:)` does, and a substituted face is
    /// not the one that was asked for — a family name asked for as a
    /// PostScript name matches `Menlo-Regular` for `Menlo`, and is refused.
    private static func matchedDescriptor(
        attributes: [CFString: Any], verifying key: CFString, equals expected: String,
    ) -> CTFontDescriptor? {
        let query = CTFontDescriptorCreateWithAttributes(attributes as CFDictionary)
        guard let matched = CTFontDescriptorCreateMatchingFontDescriptor(query, Set([key]) as CFSet),
              CTFontDescriptorCopyAttribute(matched, key) as? String == expected
        else { return nil }
        return matched
    }

    private struct FontKey: Hashable {
        let query: Query
        let size: CGFloat
    }

    /// Locked rather than actor-isolated because `CandidateMetrics` is a
    /// `Sendable` value whose font properties are read wherever a cell is
    /// measured.
    private nonisolated(unsafe) static var resolvedDescriptors: [Query: CTFontDescriptor] = [:]
    private nonisolated(unsafe) static var resolvedFonts: [FontKey: NSFont] = [:]
    private static let lock = NSLock()
}
