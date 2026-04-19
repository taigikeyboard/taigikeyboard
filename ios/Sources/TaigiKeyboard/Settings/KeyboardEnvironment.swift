import Foundation

/// Keyboard-extension UI-facing settings contract.
///
/// Non-engine counterpart to `EngineSettingsProvider`: covers appearance,
/// input-mode writes, full-access writes, and exposes the App Group
/// `UserDefaults` the controller binds to `UserDefaults.didChangeNotification`
/// for cross-process settings sync.
///
/// ### Live-read semantics
/// Each property access reads the backing store fresh, so a setting change
/// in the host app becomes visible on the next read inside the keyboard
/// extension without the controller being reconstructed. This mirrors the
/// contract `EngineSettingsProvider` documents for engine-layer reads.
///
/// ### Consistency boundary
/// `snapshot()` is the per-render-cycle consistency anchor. Hot render paths
/// should call it once and read the returned struct, rather than referencing
/// the live properties multiple times in the same body evaluation.
///
/// ### Scope
/// This is keyboard-extension / UI-facing only. Engine-layer code must not
/// depend on this protocol — it remains on `EngineSettings` /
/// `EngineSettingsProvider` (Foundation-only, read-only).
protocol KeyboardEnvironment: AnyObject {
    var inputMode: InputMode { get set }
    var isFullAccessEnabled: Bool { get set }
    var colorSettings: KeyboardColorSettings { get }
    var keyFontSizeScale: CGFloat { get }
    var keyBorderWidth: CGFloat { get }
    var candidateTextSizeScale: CGFloat { get }
    var fontType: FontType { get }
    var keyboardLayoutType: KeyboardLayoutType { get }

    /// App Group `UserDefaults` used as the notification filter for
    /// `UserDefaults.didChangeNotification`. The controller observes changes
    /// on this object to pick up writes made by the host app.
    var settingsUserDefaults: UserDefaults { get }

    func snapshot() -> SettingsSnapshot
}

extension SharedSettings: KeyboardEnvironment {
    var settingsUserDefaults: UserDefaults {
        Self.sharedUserDefaults
    }
}
