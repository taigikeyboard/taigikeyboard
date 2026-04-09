import Foundation
import KeyboardKit
import SwiftUI

/// Delegate protocol for text input and autocomplete operations.
///
/// Decouples ComposingManager from KeyboardViewController,
/// allowing Input/ to be independent of _Keyboard/.
protocol ComposingDelegate: AnyObject {
    func insertText(_ text: String)
    func deleteBackward()
    func setMarkedText(_ text: String)
    func clearMarkedText()
    func resetAutocomplete()
    func performAutocomplete()
    func resetAutocompleteContext()
}

/// Composing manager
///
/// Manages Taigi input composing state with rawInput as single source of truth.
/// - `rawInput`: Original keystrokes (e.g. "gua2") — used for Trie search
/// - `composingText`: Derived display text (e.g. "guá") — computed via ToneConverter on every state change
public class ComposingManager: ObservableObject, ComposingStateProvider {
    // MARK: - Properties

    private let logger = DebugLogger(category: "ComposingManager")

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
    @Published public var suggestions: [Autocomplete.Suggestion] = []
    @Published public var selectedCandidateIndex: Int = 0

    private weak var keyboardContext: KeyboardContext?
    weak var delegate: (any ComposingDelegate)?

    private var inputMode: InputMode {
        SharedSettings.shared.inputMode
    }

    // MARK: - 初始化

    public init() {}

    public func setKeyboardContext(_ context: KeyboardContext) {
        keyboardContext = context
    }

    // MARK: - 組字操作

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
        let newRawInput = rawInput + char
        updateComposingState(.composing(raw: newRawInput))
    }

    public func replaceLastCharacter(with replacement: String) {
        guard isComposing, !rawInput.isEmpty else { return }
        let newRawInput = String(rawInput.dropLast()) + replacement
        updateComposingState(.composing(raw: newRawInput))
    }

    public func appendHyphen() {
        appendCharacter("-")
    }

    public func deleteBackward() {
        guard isComposing, !rawInput.isEmpty else { return }

        let newRawInput = String(rawInput.dropLast())
        if newRawInput.isEmpty {
            updateComposingState(.idle)
            selectedCandidateIndex = -1
            suggestions = []
            delegate?.deleteBackward()
        } else {
            updateComposingState(.composing(raw: newRawInput))
        }
    }

    public func commitComposition() {
        guard isComposing, !composingText.isEmpty else { return }

        let textToInsert = composingText
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []
        delegate?.insertText(textToInsert)
        delegate?.resetAutocompleteContext()
    }

    /// Commit raw input text (literal keystrokes) without tone conversion or segmentation.
    /// Used when Enter is pressed to output the exact text the user typed (e.g., English words).
    public func commitRawInput() {
        guard isComposing, !rawInput.isEmpty else { return }

        let textToInsert = rawInput
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []
        delegate?.insertText(textToInsert)
        delegate?.resetAutocompleteContext()
    }

    public func selectSuggestion(_ suggestion: Autocomplete.Suggestion) {
        guard isComposing else { return }

        delegate?.clearMarkedText()
        delegate?.insertText(suggestion.text)

        state = .idle
        syncStateToProperties()
        selectedCandidateIndex = -1
        suggestions = []
        delegate?.resetAutocomplete()
        delegate?.resetAutocompleteContext()
    }

    public func confirmSelectedCandidate(availableSuggestions: [Autocomplete.Suggestion]) -> Bool {
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableSuggestions.count else { return false }

        selectSuggestion(availableSuggestions[selectedCandidateIndex])
        return true
    }

    // MARK: - Display Derivation

    /// Derive display text from raw input
    ///
    /// Converts raw input to tone-marked form via ToneConverter.
    /// ToneConverter already handles hyphen-separated syllables internally
    /// (splits by "-", converts each syllable, rejoins with "-").
    private func deriveDisplay(from raw: String) -> String {
        guard !raw.isEmpty else { return "" }

        // TPS symbols are already display-ready — no tone conversion needed
        if TPSConverter.containsTPS(raw) { return raw }

        return ToneConverter.convertToToneMarks(raw, mode: inputMode)
    }

    /// 清除所有狀態（用於鍵盤重置）
    public func reset() {
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []
    }

    /// Sync state enum to published properties
    private func syncStateToProperties() {
        switch state {
        case .idle:
            isComposing = false
            composingText = ""
            rawInput = ""

        case let .composing(raw):
            isComposing = true
            rawInput = raw
            composingText = deriveDisplay(from: raw)
            delegate?.setMarkedText(composingText)
        }

        keyboardContext?.isComposingText = isComposing
    }

    /// 統一的狀態更新方法
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
