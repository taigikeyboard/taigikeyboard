import Combine
import Foundation

/// A minimal write-only view of the composing-context state that the
/// keyboard extension needs updated when composing starts/stops. The
/// KeyboardKit `KeyboardContext` conforms to this (see
/// `KeyboardContext+Composing`), so ComposingManager stays
/// Foundation-only.
protocol ComposingContextSink: AnyObject {
    var isComposingText: Bool { get set }
}

/// iOS platform wrapper around the pure `ComposingState` engine.
///
/// Responsibilities kept in this file (non-candidate, iOS-specific):
/// - `ObservableObject` + `@Published` fan-out for SwiftUI,
/// - `ComposingDelegate` / `ComposingContextSink` wiring (UIKit side effects),
/// - reading `EngineSettingsProvider.current` per intent and threading
///   `mode` + `toneToggles` into `ComposingState.apply(...)`.
///
/// The engine boundary lives in `ComposingState.swift` /
/// `ComposingTransition.swift` — this wrapper is intentionally
/// mechanical (see `composing-state-boundary.md` §2.4).
public class ComposingManager: ObservableObject, ComposingStateProvider {
    // MARK: - Engine State

    private var state = ComposingState()

    // MARK: - Published Mirror

    @Published public private(set) var isComposing: Bool = false
    @Published public private(set) var composingText: String = ""
    @Published public private(set) var rawInput: String = ""

    /// SwiftUI mirror of the engine-owned `ComposingState.selectedCandidateIndex`.
    /// All writes flow through either `dispatch(_:)` (buffer intents) or
    /// `setSelectedCandidateIndex(_:)` (UI-driven selection) so the wrapper
    /// never desyncs from the pure state.
    @Published public private(set) var selectedCandidateIndex: Int = -1

    // MARK: - Collaborators

    private weak var contextSink: ComposingContextSink?
    weak var delegate: (any ComposingDelegate)?

    private let settingsProvider: EngineSettingsProvider

    // MARK: - Init

    init(settingsProvider: EngineSettingsProvider = SharedSettings.shared) {
        self.settingsProvider = settingsProvider
    }

    func setContextSink(_ sink: ComposingContextSink) {
        contextSink = sink
    }

    // MARK: - Composing Operations

    public func startComposing(with text: String) {
        dispatch(.start(text))
    }

    public func appendCharacter(_ char: String) {
        dispatch(.append(char))
    }

    /// Retroactively replace the last raw-input character (used by TPS auto-correct).
    /// Intentionally does NOT reset `selectedCandidateIndex` — unlike `appendCharacter`
    /// / `startComposing`, replacement is a correction and preserves candidate selection.
    public func replaceLastCharacter(with replacement: String) {
        dispatch(.replaceLast(replacement))
    }

    public func appendHyphen() {
        dispatch(.appendHyphen)
    }

    public func deleteBackward() {
        dispatch(.deleteBackward)
    }

    /// Commit the derived `composingText` (tone-marked form) to the backing text.
    public func commitComposition() {
        dispatch(.commitDerived)
    }

    /// Commit the literal raw keystrokes (no tone conversion / segmentation).
    /// Used when Enter is pressed at candidate index 0, so English words or
    /// partially-typed romanization pass through unchanged.
    public func commitRawInput() {
        dispatch(.commitRaw)
    }

    /// Commit the given candidate text and leave composing state.
    ///
    /// Takes a raw `String` rather than `Autocomplete.Suggestion` so this
    /// file stays engine-pure. The KK adapter side passes `suggestion.text`.
    public func selectSuggestion(text: String) {
        dispatch(.selectSuggestion(text))
    }

    /// Commit the currently-selected candidate, given only the visible
    /// candidate text strings. KK-side callers pass
    /// `suggestions.map(\.text)` at the boundary.
    public func confirmSelectedCandidate(availableTexts: [String]) -> Bool {
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableTexts.count
        else { return false }

        selectSuggestion(text: availableTexts[selectedCandidateIndex])
        return true
    }

    /// Clear all state (e.g. keyboard teardown).
    public func reset() {
        dispatch(.reset)
    }

    /// Update the candidate-bar selection (tap or keyboard arrow).
    /// Routes through the engine so the pure state stays authoritative.
    public func setSelectedCandidateIndex(_ index: Int) {
        state.setSelectedCandidateIndex(index)
        if selectedCandidateIndex != index { selectedCandidateIndex = index }
    }

    // MARK: - Transition Application (three-phase, see boundary doc §2.4)

    private func dispatch(_ intent: ComposingState.Intent) {
        let settings = settingsProvider.current
        let transition = state.apply(
            intent,
            mode: settings.inputMode,
            toneToggles: settings.toneToggles,
        )

        // Phase 1 — mutate published mirror (guarded-inequality writes keep
        // idle→idle silent and avoid redundant SwiftUI invalidation).
        let engineIsComposing = state.isComposing
        let engineRaw = state.rawInput
        if isComposing != engineIsComposing { isComposing = engineIsComposing }
        if rawInput != engineRaw { rawInput = engineRaw }
        if !transition.effects.isEmpty, composingText != transition.derivedDisplay {
            composingText = transition.derivedDisplay
        }
        if selectedCandidateIndex != transition.newSelectedIndex {
            selectedCandidateIndex = transition.newSelectedIndex
        }

        // Phase 2 — execute platform effects in the order the engine emitted.
        for effect in transition.effects {
            delegate?.execute(effect)
        }

        // Phase 3 — notify composing-context sink once state is settled.
        contextSink?.isComposingText = engineIsComposing
    }
}
