// 設定頁的「回報問題」用診斷資訊蒐集器。純本機讀取,不主動上傳,
// 由使用者在 UI 上自行決定是否複製或分享。

import Foundation
import UIKit

// MARK: - DiagnosticInfo

/// Snapshot of local device and app state for user-initiated bug reports.
/// No data is transmitted — the user controls when and how to share.
// 裝置與 App 版本快照,給使用者在回報 bug 時手動複製貼上。
struct DiagnosticInfo {
    let appVersion: String
    let buildNumber: String
    let osVersion: String
    let deviceModel: String

    /// Formats the info as plain text suitable for copy/share in a bug report.
    // 把欄位格式化為純文字,供使用者複製到 bug report。
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
// 診斷資訊蒐集器(static-only namespace enum),按需讀取版本與裝置資訊。
enum DiagnosticService {
    // 蒐集 App 版本、build 號、iOS 版本、機型 identifier 並組成 DiagnosticInfo。
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
    // 透過 uname(2) 取得硬體機型 identifier(如 "iPhone14,5"),不會回傳行銷名稱。
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
