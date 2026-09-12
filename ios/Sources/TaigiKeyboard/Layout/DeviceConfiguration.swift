import LocalAuthentication
import UIKit

/// Screen size classification based on device screen width
enum ScreenSizeClass {
    case phoneCompact // iPhone SE, iPhone mini (width < 375)
    case phoneRegular // Standard iPhone (375 ≤ width < 414)
    case phoneLarge // iPhone Plus/Max (width ≥ 414)
    case pad // iPad

    static var current: ScreenSizeClass {
        let width = UIScreen.main.bounds.width
        let isIPad = UIDevice.current.userInterfaceIdiom == .pad

        if isIPad {
            return .pad
        }

        switch width {
        case ..<375: return .phoneCompact
        case 375 ..< 414: return .phoneRegular
        default: return .phoneLarge
        }
    }
}

/// Device detection for layout parameter configuration
struct DeviceConfiguration {
    let isIPad: Bool
    let isSmallIPhone: Bool

    init() {
        isIPad = UIDevice.current.userInterfaceIdiom == .pad
        isSmallIPhone = Self.isTouchIDDevice
    }

    /// Cached: biometry type is hardware-fixed, never changes at runtime.
    /// On iOS 17+, only iPhone SE supports Touch ID.
    static let isTouchIDDevice: Bool = {
        guard UIDevice.current.userInterfaceIdiom == .phone else { return false }
        let laContext = LAContext()
        var error: NSError?
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
        return laContext.biometryType == .touchID
    }()
}
