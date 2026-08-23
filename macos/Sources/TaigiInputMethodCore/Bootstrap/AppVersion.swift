// The one place this process reads its own marketing version.

import Foundation

/// The version this bundle shipped as (`CFBundleShortVersionString`).
///
/// Read once and shared, so the settings pane, the update comparison, and the
/// alert that quotes "you have X" cannot each read the bundle their own way
/// and disagree. Empty when the key is missing — a build error, and every
/// reader treats it as "unknown" rather than inventing a number.
enum AppVersion {
    static let installed =
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? ""
}
