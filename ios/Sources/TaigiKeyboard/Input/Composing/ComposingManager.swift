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

/// Manages Taigi input composing state with `rawInput` as the single source of truth.
///
/// - `rawInput`: Original keystrokes (e.g. `"gua2"`) — used for Trie search.
/// - `composingText`: Derived display text (e.g. `"guá"`) — re-computed via `ToneConverter`
///   on every state change.
///
/// Text side-effects (insertText / deleteBackward / markedText) are sent through
/// `ComposingDelegate`, keeping this file independent of `_Keyboard/`.
public class ComposingManager: ObservableObject, ComposingStateProvider {
    // MARK: - State

    private enum ComposingState {
        case idle
        case composing(raw: String)
    }

    private var state: ComposingState = .idle {
        didSet { syncStateToProperties() }
    }

    @Published public private(set) var isComposing: Bool = false
    @Published public private(set) var composingText: String = ""
    @Published public private(set) var rawInput: String = ""
    @Published public var selectedCandidateIndex: Int = 0

    // MARK: - Collaborators

    private weak var contextSink: ComposingContextSink?
    weak var delegate: (any ComposingDelegate)?

    private let settingsProvider: EngineSettingsProvider

    private var inputMode: InputMode {
        settingsProvider.current.inputMode
    }

    // MARK: - Init

    init(settingsProvider: EngineSettingsProvider = SharedSettings.shared) {
        self.settingsProvider = settingsProvider
    }

    func setContextSink(_ sink: ComposingContextSink) {
        contextSink = sink
    }

    // MARK: - Composing Operations

    public func startComposing(with text: String) {
        selectedCandidateIndex = 0
        updateComposingState(.composing(raw: text))
    }

    public func appendCharacter(_ char: String) {
        guard isComposing else {
            startComposing(with: char)
            return
        }
        selectedCandidateIndex = 0
        updateComposingState(.composing(raw: rawInput + char))
    }

    /// Retroactively replace the last raw-input character (used by TPS auto-correct).
    /// Intentionally does NOT reset `selectedCandidateIndex` — unlike `appendCharacter`
    /// / `startComposing`, replacement is a correction and preserves candidate selection.
    public func replaceLastCharacter(with replacement: String) {
        guard isComposing, !rawInput.isEmpty else { return }
        let newRaw = String(rawInput.dropLast()) + replacement
        updateComposingState(.composing(raw: newRaw))
    }

    public func appendHyphen() {
        appendCharacter("-")
    }

    public func deleteBackward() {
        guard isComposing, !rawInput.isEmpty else { return }

        let newRaw = String(rawInput.dropLast())
        if newRaw.isEmpty {
            // Exit composing and delete one char from the backing text.
            // Order matters: idle transition (clearMarkedText + resetAutocomplete)
            // must run BEFORE delegate?.deleteBackward() so markedText is cleared
            // before the backing text mutates.
            updateComposingState(.idle)
            clearSelectionAndSuggestions()
            delegate?.deleteBackward()
        } else {
            updateComposingState(.composing(raw: newRaw))
        }
    }

    /// Commit the derived `composingText` (tone-marked form) to the backing text.
    public func commitComposition() {
        guard isComposing, !composingText.isEmpty else { return }
        commit(text: composingText)
    }

    /// Commit the literal raw keystrokes (no tone conversion / segmentation).
    /// Used when Enter is pressed at candidate index 0, so English words or
    /// partially-typed romanization pass through unchanged.
    public func commitRawInput() {
        guard isComposing, !rawInput.isEmpty else { return }
        commit(text: rawInput)
    }

    /// Commit the given candidate text and leave composing state.
    ///
    /// Takes a raw `String` rather than `Autocomplete.Suggestion` so this
    /// file stays engine-pure. The KK adapter side passes `suggestion.text`.
    public func selectSuggestion(text: String) {
        guard isComposing else { return }

        // Ordering contract with the text document proxy:
        //   clearMarkedText → insertText → state=.idle/sync → resetAutocomplete.
        // The direct `state = .idle` + manual `syncStateToProperties()` is
        // intentional — routing through `updateComposingState(.idle)` would
        // re-trigger `clearMarkedText` after `insertText`, which duplicates work
        // and resets autocomplete in the wrong order.
        delegate?.clearMarkedText()
        delegate?.insertText(text)

        state = .idle
        syncStateToProperties()
        clearSelectionAndSuggestions()
        delegate?.resetAutocomplete()
        delegate?.resetAutocompleteContext()
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
        updateComposingState(.idle)
        clearSelectionAndSuggestions()
    }

    // MARK: - Private Helpers

    /// Shared commit flow for `commitComposition` and `commitRawInput`.
    /// The `text` argument must be captured before calling — `updateComposingState(.idle)`
    /// clears `composingText` and `rawInput` via `syncStateToProperties()`.
    private func commit(text: String) {
        updateComposingState(.idle)
        clearSelectionAndSuggestions()
        delegate?.insertText(text)
        delegate?.resetAutocompleteContext()
    }

    private func clearSelectionAndSuggestions() {
        selectedCandidateIndex = -1
    }

    /// Derive display text from raw input.
    /// TPS symbols are already display-ready; POJ/TL go through `ToneConverter`,
    /// which handles hyphen-separated syllables internally.
    private func deriveDisplay(from raw: String) -> String {
        guard !raw.isEmpty else { return "" }
        if TPSTables.containsTPS(raw) { return raw }
        return ToneConverter.convertToToneMarks(raw, mode: inputMode)
    }

    /// Sync the state enum to published properties and notify the delegate of
    /// marked-text changes. Guards no-op writes so `@Published` doesn't fan out
    /// redundant `objectWillChange` events on idle→idle transitions.
    private func syncStateToProperties() {
        switch state {
        case .idle:
            if isComposing { isComposing = false }
            if !composingText.isEmpty { composingText = "" }
            if !rawInput.isEmpty { rawInput = "" }

        case let .composing(raw):
            if !isComposing { isComposing = true }
            if rawInput != raw { rawInput = raw }
            let display = deriveDisplay(from: raw)
            if composingText != display { composingText = display }
            delegate?.setMarkedText(display)
        }

        contextSink?.isComposingText = isComposing
    }

    /// Unified state transition. The enum didSet triggers `syncStateToProperties()`,
    /// this method then fires delegate hooks appropriate for the new state.
    private func updateComposingState(_ newState: ComposingState) {
        state = newState
        switch newState {
        case .idle:
            delegate?.clearMarkedText()
            delegate?.resetAutocomplete()
        case .composing:
            delegate?.performAutocomplete()
        }
    }
}
