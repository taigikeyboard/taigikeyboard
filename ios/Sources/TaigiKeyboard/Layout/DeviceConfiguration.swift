import KeyboardKit
import LocalAuthentication
import UIKit

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
