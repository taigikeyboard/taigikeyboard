import Combine
import Foundation

/// Minimal write-only view of the composing-context state that the keyboard
/// extension needs updated when composing starts/stops. KeyboardKit's
/// `KeyboardContext` conforms via `KeyboardContext+Composing` so this
/// wrapper stays Foundation-only.
protocol ComposingContextSink: AnyObject {
    var isComposingText: Bool { get set }
}

/// iOS platform wrapper around the Rust shared-core composing engine
/// (`engine/composing` crate, accessed through
/// `RustEngineBridge.composing*` methods).
///
/// Engine state (phase + raw input + selectedCandidateIndex) lives inside
/// the Rust singleton `EngineHandle`; this wrapper:
/// - mirrors the latest response into `@Published` properties for SwiftUI,
/// - dispatches the bridge-emitted `Effect[]` through `ComposingDelegate`
///   in proto-list order,
/// - notifies the `ComposingContextSink` once state is settled.
///
/// Lifecycle: `currentGeneration` ticks once per real input-context
/// change (per plan §4.2 + Codex P1.4). The bridge passes it on every call;
/// the engine compares against last-seen and silently drops state on
/// mismatch.
public class ComposingManager: ObservableObject, ComposingStateProvider {
    // MARK: - Published Mirror

    @Published public private(set) var isComposing: Bool = false
    @Published public private(set) var composingText: String = ""
    @Published public private(set) var rawInput: String = ""
    @Published public private(set) var selectedCandidateIndex: Int = -1

    // MARK: - Lifecycle Generation

    /// Bumped by `KeyboardViewController` lifecycle hooks (commit 11) when
    /// a real input-context change is detected. Engine-side generation
    /// mismatch then drops state silently before applying the next request.
    private var currentGeneration: UInt64 = 1

    /// `true` while the platform is dispatching effects from a self-driven
    /// commit. Suppresses redundant generation bumps from `textWillChange`
    /// firing on candidate taps / self-commits.
    public internal(set) var selfCommitInProgress: Bool = false

    // MARK: - Collaborators

    private weak var contextSink: ComposingContextSink?
    weak var delegate: (any ComposingDelegate)?

    private let settingsProvider: EngineSettingsProvider
    private let logger = DebugLogger(category: "ComposingManager")

    // MARK: - Init

    init(settingsProvider: EngineSettingsProvider = SharedSettings.shared) {
        self.settingsProvider = settingsProvider
    }

    func setContextSink(_ sink: ComposingContextSink) {
        contextSink = sink
    }

    /// Called by `KeyboardViewController` lifecycle hooks (per plan §4.2)
    /// when a NEW input context is detected. Subsequent bridge calls carry
    /// the bumped generation; engine drops stale state silently.
    public func bumpGeneration() {
        currentGeneration &+= 1
    }

    // MARK: - Composing Operations

    public func startComposing(with text: String) {
        logger.debug("[COMPOSE] fn=startComposing text='\(text)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingStart(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    public func appendCharacter(_ char: String) {
        logger.debug("[COMPOSE] fn=appendCharacter char='\(char)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppend(
            char,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    public func appendHyphen() {
        logger.debug("[COMPOSE] fn=appendHyphen")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingAppendHyphen(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    /// TPS auto-correct — preserves `selectedCandidateIndex`.
    public func replaceLastCharacter(with replacement: String) {
        logger.debug("[COMPOSE] fn=replaceLastCharacter replacement='\(replacement)'")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingReplaceLast(
            replacement,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    public func deleteBackward() {
        logger.debug("[COMPOSE] fn=deleteBackward")
        let settings = settingsProvider.current
        apply(RustEngineBridge.composingDeleteBackward(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    public func commitComposition() {
        logger.debug("[COMPOSE] fn=commitComposition")
        let settings = settingsProvider.current
        applyAsSelfCommit(RustEngineBridge.composingCommitDerived(
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    public func commitRawInput() {
        logger.debug("[COMPOSE] fn=commitRawInput")
        applyAsSelfCommit(RustEngineBridge.composingCommitRaw(generation: currentGeneration))
    }

    public func selectSuggestion(text: String) {
        logger.debug("[COMPOSE] fn=selectSuggestion len=\(text.count)")
        applyAsSelfCommit(RustEngineBridge.composingSelectSuggestion(text, generation: currentGeneration))
    }

    public func commitPreeditThenInsertExternal(_ text: String) {
        logger.debug("[COMPOSE] fn=commitPreeditThenInsertExternal len=\(text.count)")
        let settings = settingsProvider.current
        applyAsSelfCommit(RustEngineBridge.composingCommitPreeditThenInsertExternal(
            text,
            mode: settings.inputMode,
            toggles: settings.toneToggles,
            generation: currentGeneration
        ))
    }

    /// Commit the currently-selected candidate, given the visible candidate
    /// strings. KK-side callers pass `suggestions.map(\.text)`.
    public func confirmSelectedCandidate(availableTexts: [String]) -> Bool {
        logger.debug("[COMPOSE] fn=confirmSelectedCandidate index=\(selectedCandidateIndex) count=\(availableTexts.count)")
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableTexts.count
        else { return false }
        selectSuggestion(text: availableTexts[selectedCandidateIndex])
        return true
    }

    public func reset() {
        logger.debug("[COMPOSE] fn=reset")
        applyAsSelfCommit(RustEngineBridge.composingReset(generation: currentGeneration))
    }

    public func setSelectedCandidateIndex(_ index: Int) {
        logger.debug("[COMPOSE] fn=setSelectedCandidateIndex index=\(index)")
        apply(RustEngineBridge.composingSetSelectedCandidateIndex(index, generation: currentGeneration))
    }

    // MARK: - Apply Transition (three-phase, see boundary doc §2.4)

    private func applyAsSelfCommit(_ transition: RustEngineBridge.ComposingTransition) {
        selfCommitInProgress = true
        defer { selfCommitInProgress = false }
        apply(transition)
    }

    private func apply(_ transition: RustEngineBridge.ComposingTransition) {
        // Phase 1 — mutate published mirror (guarded-inequality writes keep
        // idle→idle silent and avoid redundant SwiftUI invalidation).
        if isComposing != transition.isComposing { isComposing = transition.isComposing }
        if rawInput != transition.rawInput { rawInput = transition.rawInput }
        if !transition.effects.isEmpty, composingText != transition.displayText {
            composingText = transition.displayText
        }
        if selectedCandidateIndex != transition.selectedCandidateIndex {
            selectedCandidateIndex = transition.selectedCandidateIndex
        }

        // Phase 2 — execute platform effects in proto-list order.
        for effect in transition.effects {
            delegate?.execute(effect)
        }

        // Phase 3 — notify composing-context sink once state is settled.
        contextSink?.isComposingText = transition.isComposing
    }
}
