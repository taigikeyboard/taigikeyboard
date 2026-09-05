// 鍵盤佈局用的裝置/螢幕分類工具。
// ScreenSizeClass 依螢幕寬度判別 phoneCompact / phoneRegular / phoneLarge / pad,
// DeviceConfiguration 額外快取 Touch ID 偵測(iOS 17+ 上 = iPhone SE)。

import LocalAuthentication
import UIKit

/// Screen size classification based on device screen width
// 依螢幕寬度做的尺寸分類 — 影響 layout 選擇與 callout 行為。
enum ScreenSizeClass {
    // iPhone SE / iPhone mini(寬 < 375)。
    case phoneCompact // iPhone SE, iPhone mini (width < 375)
    // 標準 iPhone(375 ≤ 寬 < 414)。
    case phoneRegular // Standard iPhone (375 ≤ width < 414)
    // iPhone Plus / Max(寬 ≥ 414)。
    case phoneLarge // iPhone Plus/Max (width ≥ 414)
    // iPad。
    case pad // iPad

    // 由 UIScreen 量測當前裝置取得的尺寸分類。
    static var current: ScreenSizeClass {
        let width = UIScreen.main.bounds.width
        let isIPad = UIDevice.current.userInterfaceIdiom == .pad

        if isIPad { return .pad }

        switch width {
        case ..<375: return .phoneCompact
        case 375 ..< 414: return .phoneRegular
        default: return .phoneLarge
        }
    }
}

/// Device detection for layout parameter configuration
// 給 layout 取得「是否 iPad / 是否小尺寸 iPhone」的工具。
struct DeviceConfiguration {
    let isIPad: Bool
    let isSmallIPhone: Bool

    init() {
        isIPad = UIDevice.current.userInterfaceIdiom == .pad
        isSmallIPhone = Self.isTouchIDDevice
    }

    /// Cached: biometry type is hardware-fixed, never changes at runtime.
    /// On iOS 17+, only iPhone SE supports Touch ID.
    // 快取 — 生物辨識型別屬硬體層級,執行期不變。iOS 17+ 上只剩 iPhone SE 用 Touch ID。
    static let isTouchIDDevice: Bool = {
        guard UIDevice.current.userInterfaceIdiom == .phone else { return false }
        let laContext = LAContext()
        var error: NSError?
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
        return laContext.biometryType == .touchID
    }()
}
