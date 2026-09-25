import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// The POJ marker options the engine reads when it renders text: the two
/// double-tap folds that compose `o͘` / `ⁿ` (`oo` / `nn`) and ⁿ becomes ᴺ in capitals, the
/// case rule of the nasal marker it composed (`SIÂᴺ` after a capital, or
/// always `ⁿ`; `behavioral-invariants.md` §53).
///
/// Carried as an explicit value so `ComposingState` / `ToneConverter` stay
/// Foundation-pure. The wrapper reads the booleans from
/// `EngineSettingsProvider.current` at call time (live read — see
/// `EngineSettingsProvider`), then passes them through.
public struct PojMarkerOptions: Equatable {
    public let isDoubleTapOOEnabled: Bool
    public let isDoubleTapNNEnabled: Bool
    public let isNasalMarkerUppercaseEnabled: Bool

    public init(isDoubleTapOOEnabled: Bool, isDoubleTapNNEnabled: Bool, isNasalMarkerUppercaseEnabled: Bool) {
        self.isDoubleTapOOEnabled = isDoubleTapOOEnabled
        self.isDoubleTapNNEnabled = isDoubleTapNNEnabled
        self.isNasalMarkerUppercaseEnabled = isNasalMarkerUppercaseEnabled
    }
}
