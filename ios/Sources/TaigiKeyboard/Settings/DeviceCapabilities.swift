// 裝置能力偵測,提供 Settings 預設值的依據。
// 目前只用來決定地球鍵 (globe key) 是否預設顯示:iPad 與 Touch ID iPhone 預設開,Face ID iPhone 預設關。

import LocalAuthentication
import UIKit

/// Device-level capability probes used to pick default settings values.
///
/// Currently drives the globe-key default: on iPad (stacked modal shows no
/// native switcher) and iPhone SE with Touch ID (no system gesture to switch
/// keyboards) the globe key should be visible by default; on modern iPhones
/// with Face ID the system shortcut already exists, so the default is off.
// 裝置能力偵測 enum;static-only,沒有狀態。
enum DeviceCapabilities {
    /// Whether the current device should show the globe key by default.
    /// `true` for iPad and Touch-ID iPhones, `false` otherwise.
    // 地球鍵預設值。iPad / Touch ID iPhone 回傳 true,其它回傳 false。
    static var prefersGlobeKeyByDefault: Bool {
        if UIDevice.current.userInterfaceIdiom == .pad { return true }
        guard UIDevice.current.userInterfaceIdiom == .phone else { return false }

        let laContext = LAContext()
        var error: NSError?
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
        return laContext.biometryType == .touchID
    }
}
