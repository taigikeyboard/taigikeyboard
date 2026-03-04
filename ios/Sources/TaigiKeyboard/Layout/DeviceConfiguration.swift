import KeyboardKit
import LocalAuthentication
import UIKit

/// Screen size classification
/// Based on KeyboardKit v9 device classification design
enum ScreenSizeClass {
    case phoneCompact      // iPhone SE, iPhone mini (width < 375)
    case phoneRegular      // Standard iPhone (375 ≤ width < 414)
    case phoneLarge        // iPhone Plus/Max (width ≥ 414)
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

/// Device detection and layout parameter configuration
struct DeviceConfiguration {
    let isIPad: Bool
    let isSmallIPhone: Bool

    init(context: KeyboardContext) {
        self.isIPad = UIDevice.current.userInterfaceIdiom == .pad
        self.isSmallIPhone = Self.detectSmallIPhone(context: context)
    }

    /// Detect iPhone SE (iPhones with Touch ID)
    /// Criteria: Not iPad, uses Touch ID
    /// On iOS 17+, only iPhone SE supports Touch ID
    private static func detectSmallIPhone(context: KeyboardContext) -> Bool {
        // Use UIKit native API for device type (KeyboardKit 10 no longer provides deviceType)
        guard UIDevice.current.userInterfaceIdiom == .phone else { return false }

        let laContext = LAContext()
        var error: NSError?

        // Must call canEvaluatePolicy first to get biometryType
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)

        // Only iPhone SE uses Touch ID
        return laContext.biometryType == .touchID
    }
}
