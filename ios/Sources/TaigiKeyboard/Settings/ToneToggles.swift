import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// POJ preprocessing toggles the composing engine needs when deriving display text.
///
/// Carried as an explicit value so `ComposingState` / `ToneConverter` stay
/// Foundation-pure. The wrapper reads the booleans from
/// `EngineSettingsProvider.current` at call time (live read — see
/// `EngineSettingsProvider`), then passes them through.
public struct ToneToggles: Equatable {
    public let isDoubleTapOOEnabled: Bool
    public let isDoubleTapNNEnabled: Bool

    public init(isDoubleTapOOEnabled: Bool, isDoubleTapNNEnabled: Bool) {
        self.isDoubleTapOOEnabled = isDoubleTapOOEnabled
        self.isDoubleTapNNEnabled = isDoubleTapNNEnabled
    }
}
