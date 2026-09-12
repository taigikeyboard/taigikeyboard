import LocalAuthentication
import UIKit

/// Device-level capability probes used to pick default settings values.
///
/// Currently drives the globe-key default: on iPad (stacked modal shows no
/// native switcher) and iPhone SE with Touch ID (no system gesture to switch
/// keyboards) the globe key should be visible by default; on modern iPhones
/// with Face ID the system shortcut already exists, so the default is off.
enum DeviceCapabilities {
    /// Whether the current device should show the globe key by default.
    /// `true` for iPad and Touch-ID iPhones, `false` otherwise.
    static var prefersGlobeKeyByDefault: Bool {
        if UIDevice.current.userInterfaceIdiom == .pad {
            return true
        }
        guard UIDevice.current.userInterfaceIdiom == .phone else { return false }

        let laContext = LAContext()
        var error: NSError?
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
        return laContext.biometryType == .touchID
    }
}
