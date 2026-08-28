// Where this input method keeps what it learns from the user.

import Foundation

/// Resolves the per-user directory the learning databases live in.
///
/// `~/Library/Application Support/<bundle id>/`. Not `Caches`: what is stored
/// there is the user's own typing history, and a purge would silently reset
/// their candidate ranking.
///
/// NOT excluded from Time Machine. iOS excludes the same three databases from
/// its backups (`docs/architecture/behavioral-invariants.md` §29), but that
/// decision was about user data leaving the device through iCloud; Time Machine
/// is a local backup the user set up themselves, and learned rankings are
/// exactly the kind of hard-to-recreate data a restore should bring back.
/// NAMED CROSS-PLATFORM DIVERGENCE (`cross-platform-alignment.md` §3,
/// intentional).
enum UserDataDirectory {
    enum Failure: Error, CustomStringConvertible {
        case noBundleIdentifier

        var description: String {
            switch self {
            case .noBundleIdentifier: "the running bundle has no identifier"
            }
        }
    }

    /// The shipped location, created if absent.
    static func standard() throws -> URL {
        try container(in: .applicationSupportDirectory)
    }

    /// This bundle's own directory inside one of the user's library folders,
    /// created if absent.
    ///
    /// Keyed on the running bundle's own identifier rather than on a constant,
    /// so the directory follows `Info.plist` — the single source of bundle
    /// identity everything else in this package reads too. The search path is a
    /// parameter because the same per-bundle shape is wanted in more than one
    /// of them: learning data in Application Support, a staged update installer
    /// in Caches (`UpdatePackageDownload`).
    static func container(in searchPath: FileManager.SearchPathDirectory) throws -> URL {
        guard let bundleIdentifier = Bundle.main.bundleIdentifier else {
            throw Failure.noBundleIdentifier
        }
        let library = try FileManager.default.url(
            for: searchPath,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true,
        )
        return try created(library.appendingPathComponent(bundleIdentifier))
    }

    /// Creates `directory` if it does not exist and returns it. Split out so a
    /// test can hand the stores a temporary directory without going near the
    /// user's real one.
    static func created(_ directory: URL) throws -> URL {
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true,
        )
        return directory
    }
}
