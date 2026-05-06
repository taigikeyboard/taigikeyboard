import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Supplies the engine with an `EngineSettings` that reflects the current
/// state of the underlying store.
///
/// `current` is expected to return a *live-reading* implementation — each
/// call chain like `provider.current.inputMode` reads the most recent
/// `UserDefaults` value. This preserves the keyboard extension's
/// live-update behavior: the user changes a setting in the host app,
/// `UserDefaults.didChangeNotification` fires, and the next engine query
/// sees the new value without the controller being reconstructed.
///
/// Do NOT return a one-shot snapshot from `current`. If a caller needs
/// consistency across multiple reads, it should capture a local copy.
protocol EngineSettingsProvider: AnyObject {
    var current: EngineSettings { get }
}
