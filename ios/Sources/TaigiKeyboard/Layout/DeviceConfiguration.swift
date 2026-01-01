import KeyboardKit
import LocalAuthentication
import UIKit

/// 螢幕尺寸分級
/// 參考 KeyboardKit v9 的裝置分級設計
enum ScreenSizeClass {
    case phoneCompact      // iPhone SE, iPhone mini (寬度 < 375)
    case phoneRegular      // iPhone 標準 (375 ≤ 寬度 < 414)
    case phoneLarge        // iPhone Plus/Max (寬度 ≥ 414)
    case pad               // iPad

    static var current: ScreenSizeClass {
        let width = UIScreen.main.bounds.width
        let isIPad = UIDevice.current.userInterfaceIdiom == .pad

        if isIPad { return .pad }

        switch width {
        case ..<375: return .phoneCompact
        case 375..<414: return .phoneRegular
        default: return .phoneLarge
        }
    }
}

/// 裝置偵測與佈局參數配置
struct DeviceConfiguration {
    let isIPad: Bool
    let isSmallIPhone: Bool

    init(context: KeyboardContext) {
        self.isIPad = UIDevice.current.userInterfaceIdiom == .pad
        self.isSmallIPhone = Self.detectSmallIPhone(context: context)
    }

    /// iPhone SE 偵測（支援 Touch ID 的 iPhone）
    /// 判斷條件：非 iPad、使用 Touch ID
    /// iOS 17+ 中只有 iPhone SE 支援 Touch ID
    private static func detectSmallIPhone(context: KeyboardContext) -> Bool {
        guard context.deviceType == .phone else { return false }

        let laContext = LAContext()
        var error: NSError?

        // 必須先呼叫 canEvaluatePolicy 才能取得 biometryType
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)

        // 只有 iPhone SE 使用 Touch ID
        return laContext.biometryType == .touchID
    }
}
