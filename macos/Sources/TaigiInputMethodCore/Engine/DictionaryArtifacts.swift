// Locates the four read-only data files the Rust lexicon engine loads.

import Foundation

/// Absolute paths to the dictionary data the engine memory-maps at install time.
///
/// Resolution is a value type taking a base directory rather than a lookup
/// against `Bundle.main`, because the tests that prove the engine really loads
/// these files run outside any app bundle and must be able to point at the
/// repository's copies.
struct DictionaryArtifacts: Equatable {
    /// The prefix index driving candidate lookup.
    let triePath: String
    /// The dictionary records themselves.
    let dictionaryBinPath: String
    /// The bundled bigram table behind next-word prediction.
    let associationBinPath: String
    /// The TL syllable inventory that continuous input segments against.
    let syllableInventoryPath: String

    /// Every file is required. The engine tolerates an empty syllable-inventory
    /// path by loading no inventory, but for an assembled `.app` that state is a
    /// corrupt bundle rather than a supported configuration: continuous input
    /// would silently return no candidates at all.
    init(baseURL: URL) throws {
        triePath = try Self.validatedPath(baseURL: baseURL, fileName: "dictionary.fst")
        dictionaryBinPath = try Self.validatedPath(baseURL: baseURL, fileName: "dictionary.bin")
        associationBinPath = try Self.validatedPath(baseURL: baseURL, fileName: "association.bin")
        syllableInventoryPath = try Self.validatedPath(baseURL: baseURL, fileName: "syllables.fst")
    }

    private static func validatedPath(baseURL: URL, fileName: String) throws -> String {
        let url = baseURL.appendingPathComponent(fileName)
        let size = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize
        // Size rather than mere existence: a truncated copy fails inside the
        // engine's mmap with far less to go on than a named file does here.
        guard let size, size > 0 else {
            throw DictionaryArtifactsError.unusableFile(fileName: fileName, directory: baseURL.path)
        }
        return url.path
    }
}

enum DictionaryArtifactsError: Error, Equatable {
    case unusableFile(fileName: String, directory: String)
    /// `Bundle.main.resourceURL` is optional in principle; for a bundle this
    /// process is running from, a `nil` means the bundle is malformed.
    case noBundleResourceDirectory
}
