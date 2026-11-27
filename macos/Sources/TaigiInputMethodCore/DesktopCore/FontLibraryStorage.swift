// Swift-side bridge over `taigi-desktop-storage::font_library` via `DesktopCoreBridge`.

import Foundation
import os

/// Routes `CustomFontLibrary`'s disk operations and naming/size rules to the
/// shared Rust storage crate (`desktop/crates/taigi-desktop-storage/src/font_library.rs`)
/// through `taigi-macos-ffi` (`macos/crates/taigi-macos-ffi/src/font_library.rs`).
enum FontLibraryStorage {
    enum BridgeUnavailable: Error, CustomStringConvertible {
        case missingFolderName

        var description: String {
            "font library bridge unavailable"
        }
    }

    private static let cachedRules = OSAllocatedUnfairLock<Taigi_DesktopShell_FontLibraryRules?>(initialState: nil)

    /// Cached rules read from `taigi-desktop-storage`; never caches `nil` so a
    /// transient failure retries on the next caller.
    static func loadRules() -> Taigi_DesktopShell_FontLibraryRules? {
        if let cached = cachedRules.withLock({ $0 }) {
            return cached
        }
        guard let fetched = fetchRules(transport: DesktopCoreBridge.send) else { return nil }
        return cachedRules.withLock { state in
            if let existing = state {
                return existing
            }
            state = fetched
            return fetched
        }
    }

    static func fetchRules(
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) -> Taigi_DesktopShell_FontLibraryRules? {
        var req = Taigi_DesktopShell_FontLibraryRequest()
        req.rules = .init()
        guard let reply = send(req, op: "fontLibraryRules", transport: transport),
              !reply.hasError,
              case let .rules(rules) = reply.result
        else {
            return nil
        }
        return rules
    }

    static var allowedExtensions: Set<String> {
        Set(loadRules()?.allowedExtensions ?? [])
    }

    static var maxFileSize: Int {
        Int(clamping: loadRules()?.maxFileSize ?? 0)
    }

    static var maxNameAttempts: Int {
        Int(loadRules()?.maxNameAttempts ?? 0)
    }

    static var fontsFolderName: String? {
        guard let name = loadRules()?.fontsFolderName, !name.isEmpty else { return nil }
        return name
    }

    static func copyIn(
        source: URL,
        into directory: URL,
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) throws -> String {
        var req = Taigi_DesktopShell_FontLibraryRequest()
        var copyIn = Taigi_DesktopShell_CopyInFontRequest()
        copyIn.directory = directory.path(percentEncoded: false)
        copyIn.source = source.path(percentEncoded: false)
        req.copyIn = copyIn

        guard let reply = send(req, op: "fontLibraryCopyIn", transport: transport) else {
            throw CustomFontLibrary.ImportFailure.notCopied("font library bridge unavailable")
        }
        if reply.hasError {
            throw importFailure(from: reply.error)
        }
        guard case let .copied(copied) = reply.result else {
            throw CustomFontLibrary.ImportFailure.notCopied("font library bridge unavailable")
        }
        return copied.fileName
    }

    static func removeStored(
        fileName: String,
        in directory: URL,
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) throws {
        var req = Taigi_DesktopShell_FontLibraryRequest()
        var remove = Taigi_DesktopShell_RemoveStoredFontRequest()
        remove.directory = directory.path(percentEncoded: false)
        remove.fileName = fileName
        req.removeStored = remove

        guard let reply = send(req, op: "fontLibraryRemoveStored", transport: transport) else {
            throw CustomFontLibrary.RemovalFailure.notRemoved("font library bridge unavailable")
        }
        if reply.hasError {
            throw removalFailure(from: reply.error)
        }
        guard case .ack = reply.result else {
            throw CustomFontLibrary.RemovalFailure.notRemoved("font library bridge unavailable")
        }
    }

    static func listStored(
        in directory: URL,
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) -> [String] {
        var req = Taigi_DesktopShell_FontLibraryRequest()
        var list = Taigi_DesktopShell_ListStoredFontsRequest()
        list.directory = directory.path(percentEncoded: false)
        req.listStored = list

        guard let reply = send(req, op: "fontLibraryListStored", transport: transport),
              !reply.hasError,
              case let .storedFonts(stored) = reply.result
        else {
            return []
        }
        return stored.fileNames
    }

    static func sanitizedStem(
        _ name: String,
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) -> String {
        var req = Taigi_DesktopShell_FontLibraryRequest()
        var sanitize = Taigi_DesktopShell_SanitizedFontStemRequest()
        sanitize.name = name
        req.sanitizedStem = sanitize

        guard let reply = send(req, op: "fontLibrarySanitizedStem", transport: transport),
              !reply.hasError,
              case let .sanitizedStem(sanitized) = reply.result
        else {
            return "typeface"
        }
        return sanitized.stem
    }

    static func sanitizedComponent(
        _ fileName: String,
        transport: ([UInt8]) -> [UInt8] = DesktopCoreBridge.send,
    ) -> String? {
        var req = Taigi_DesktopShell_FontLibraryRequest()
        var sanitize = Taigi_DesktopShell_SanitizedFontComponentRequest()
        sanitize.fileName = fileName
        req.sanitizedComponent = sanitize

        guard let reply = send(req, op: "fontLibrarySanitizedComponent", transport: transport),
              !reply.hasError,
              case let .sanitizedComponent(sanitized) = reply.result,
              sanitized.hasFileName
        else {
            return nil
        }
        return sanitized.fileName
    }

    private static func send(
        _ request: Taigi_DesktopShell_FontLibraryRequest,
        op: String,
        transport: ([UInt8]) -> [UInt8],
    ) -> Taigi_DesktopShell_FontLibraryReply? {
        DesktopCoreBridge.roundtrip(.fontLibrary(request), op: op, transport: transport) {
            if case let .fontLibrary(reply) = $0 {
                reply
            } else {
                nil
            }
        }
    }

    private static func importFailure(
        from error: Taigi_DesktopShell_FontLibraryError,
    ) -> CustomFontLibrary.ImportFailure {
        switch error.kind {
        case let .unsupportedFileType(fileExtension):
            .unsupportedFileType(fileExtension)
        case .notARegularFile:
            .notARegularFile
        case let .tooLargeBytes(bytes):
            .tooLarge(bytes: Int(clamping: bytes))
        case let .unreadable(reason):
            .unreadable(reason)
        case let .notCopied(reason):
            .notCopied(reason)
        case .noAvailableName:
            .noAvailableName
        case .invalidFileName, .notRemoved, nil:
            .notCopied("unexpected font library error")
        }
    }

    private static func removalFailure(
        from error: Taigi_DesktopShell_FontLibraryError,
    ) -> CustomFontLibrary.RemovalFailure {
        switch error.kind {
        case .invalidFileName:
            .invalidFileName
        case let .notRemoved(reason):
            .notRemoved(reason)
        case .unsupportedFileType, .notARegularFile, .tooLargeBytes, .unreadable, .notCopied, .noAvailableName, nil:
            .notRemoved("unexpected font library error")
        }
    }
}
