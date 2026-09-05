import Foundation
import UIKit

// MARK: - DiagnosticInfo

/// Snapshot of local device and app state for user-initiated bug reports.
/// No data is transmitted — the user controls when and how to share.
struct DiagnosticInfo {
    let appVersion: String
    let buildNumber: String
    let osVersion: String
    let deviceModel: String

    /// Formats the info as plain text suitable for copy/share in a bug report.
    func formatted() -> String {
        """
        App: v\(appVersion) (\(buildNumber))
        OS: iOS \(osVersion)
        Device: \(deviceModel)
        """
    }
}

// MARK: - DiagnosticService

/// Gathers local diagnostic information on demand.
/// All methods are static — no instance state needed.
enum DiagnosticService {
    @MainActor
    static func gather() -> DiagnosticInfo {
        let appVersion = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "unknown"
        let buildNumber = Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "unknown"
        let osVersion = UIDevice.current.systemVersion
        let deviceModel = Self.deviceModelIdentifier()

        return DiagnosticInfo(
            appVersion: appVersion,
            buildNumber: buildNumber,
            osVersion: osVersion,
            deviceModel: deviceModel,
        )
    }

    // MARK: - Private helpers

    /// Returns the hardware model identifier (e.g. "iPhone14,5") via utsname.
    private static func deviceModelIdentifier() -> String {
        var info = utsname()
        uname(&info)
        return withUnsafeBytes(of: &info.machine) { bytes in
            bytes
                .prefix(while: { $0 != 0 })
                .map { Character(UnicodeScalar($0)) }
                .reduce("") { $0 + String($1) }
        }
    }
}
