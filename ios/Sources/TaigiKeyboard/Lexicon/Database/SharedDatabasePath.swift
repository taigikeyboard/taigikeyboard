import Foundation

/// Resolves on-disk paths for SQLite databases stored in the App Group container.
///
/// All user-data SQLite repositories (custom dictionary, user frequency,
/// next-word association) share the same container so the keyboard extension
/// can read what the host app writes. Centralized here so the directory-create
/// + path-append boilerplate is in one place.
enum SharedDatabasePath {
    /// Returns the absolute path to `<appGroupContainer>/<filename>`,
    /// creating the container directory if needed.
    static func resolve(filename: String) throws -> String {
        guard let containerURL = SharedSettings.sharedContainerURL else {
            throw LexiconError.databaseNotFound
        }
        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true,
        )
        return containerURL.appendingPathComponent(filename).path
    }
}
