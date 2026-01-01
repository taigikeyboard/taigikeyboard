import Foundation
import KeyboardKit
import OSLog
import SwiftUI

/// 組字管理器
///
/// 管理台語輸入的組字狀態，維護兩個狀態：
/// - `rawInput`: 原始輸入（保留數字聲調，用於 Trie 搜尋）
/// - `composingText`: 顯示文字（聲調已轉換，用於 UI 顯示和輸出）
public class ComposingManager: ObservableObject {

    // MARK: - 屬性

    private let logger = Logger(subsystem: "com.siansiansu.taigikeyboard", category: "ComposingManager")

    private enum ComposingState {
        case idle
        case composing(raw: String, display: String)
    }

    private var state: ComposingState = .idle {
        didSet { syncStateToProperties() }
    }

    @Published public private(set) var isComposing: Bool = false
    @Published public private(set) var composingText: String = ""
    @Published public private(set) var rawInput: String = ""
    @Published public var suggestions: [Autocomplete.Suggestion] = []
    @Published public var selectedCandidateIndex: Int = 0

    private var composingStartLength: Int = 0
    private weak var keyboardContext: KeyboardContext?
    private weak var keyboardViewController: KeyboardViewController?

    private var inputMode: InputMode {
        SharedSettings.shared.inputMode
    }

    // MARK: - 初始化

    public init() {}

    public func setKeyboardContext(_ context: KeyboardContext) {
        keyboardContext = context
    }

    func setKeyboardViewController(_ controller: KeyboardViewController?) {
        keyboardViewController = controller
    }

    // MARK: - 組字操作

    public func startComposing(with text: String) {
        composingStartLength = text.count
        selectedCandidateIndex = 0
        updateComposingState(.composing(raw: text, display: text))
    }

    public func appendCharacter(_ char: String) {
        guard isComposing else {
            startComposing(with: char)
            return
        }

        selectedCandidateIndex = 0
        let newRawInput = rawInput + char
        var finalDisplayText = composingText + char

        // 字符組合轉換（oo → o͘, nn → ⁿ）
        if let transformedText = checkCharacterCombination(currentText: finalDisplayText, input: char) {
            finalDisplayText = transformedText
        }

        // 聲調轉換（1-9）
        if let number = Int(char), (1...9).contains(number) {
            if let toneConvertedText = applyToneConversion(currentText: finalDisplayText, toneNumber: number) {
                finalDisplayText = toneConvertedText
            }
        }

        logger.debug("[COMPOSING] char='\(char, privacy: .public)' rawInput='\(newRawInput, privacy: .public)' display='\(finalDisplayText, privacy: .public)'")
        updateComposingState(.composing(raw: newRawInput, display: finalDisplayText))
    }

    public func appendHyphen() {
        appendCharacter("-")
    }

    public func deleteBackward() {
        guard isComposing, !composingText.isEmpty else { return }

        // 先嘗試聲調還原
        if let restoredText = attemptToneRestoration() {
            let newRawInput = String(rawInput.dropLast())
            updateComposingState(.composing(raw: newRawInput, display: restoredText))
            return
        }

        // 一般字符刪除（ⁿ 對應 rawInput 的 nn）
        let lastChar = composingText.last
        let rawDeleteCount = (lastChar == "ⁿ") ? 2 : 1
        let newDisplayText = String(composingText.dropLast())
        let newRawInput = String(rawInput.dropLast(rawDeleteCount))

        if newDisplayText.isEmpty {
            updateComposingState(.idle)
            selectedCandidateIndex = -1
            suggestions = []
            keyboardViewController?.deleteBackwardManually()
        } else {
            updateComposingState(.composing(raw: newRawInput, display: newDisplayText))
        }
    }

    public func commitComposition() {
        guard isComposing, !composingText.isEmpty else { return }

        let textToInsert = composingText
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []
        keyboardViewController?.textDocumentProxy.insertText(textToInsert)
        keyboardViewController?.state.autocompleteContext.reset()
    }

    public func selectSuggestion(_ suggestion: Autocomplete.Suggestion) {
        guard isComposing else { return }

        if let proxy = keyboardViewController?.textDocumentProxy {
            proxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
            proxy.unmarkText()
            proxy.insertText(suggestion.text)
        }

        state = .idle
        syncStateToProperties()
        selectedCandidateIndex = -1
        suggestions = []
        keyboardViewController?.resetAutocomplete()
        keyboardViewController?.state.autocompleteContext.reset()
    }

    /// 移動到下一個候選詞（循環：0 → 1 → ... → N → 0）
    public func moveToNextCandidate(availableSuggestions: [Autocomplete.Suggestion]) -> Bool {
        guard isComposing, !availableSuggestions.isEmpty else { return false }

        if selectedCandidateIndex >= availableSuggestions.count - 1 {
            selectedCandidateIndex = 0
        } else {
            selectedCandidateIndex += 1
        }
        return true
    }

    public func confirmSelectedCandidate(availableSuggestions: [Autocomplete.Suggestion]) -> Bool {
        guard isComposing,
              selectedCandidateIndex >= 0,
              selectedCandidateIndex < availableSuggestions.count else { return false }

        selectSuggestion(availableSuggestions[selectedCandidateIndex])
        return true
    }

    // MARK: - 字符轉換

    /// 檢查字符組合轉換（oo → o͘, nn → ⁿ）
    /// - Parameter currentText: 當前組字文字
    /// - Parameter input: 新輸入的字符
    /// - Returns: 轉換後的新文字，如果無轉換則回傳 nil
    private func checkCharacterCombination(currentText: String, input: String) -> String? {
        let settings = SharedSettings.shared

        // POJ 模式：檢查 oo → o͘ 轉換
        if inputMode == .poj {
            if input.lowercased() == "o", currentText.count >= 2 {
                let previousChar = String(currentText.dropLast())
                if previousChar.lowercased().hasSuffix("o"), settings.enableDoubleTapOO {
                    // 保持原始大小寫
                    let wasUppercase = currentText.dropLast().last?.isUppercase == true
                    let replacement = wasUppercase ? "O͘" : "o͘"

                    // 返回轉換後的新文字
                    let newText = String(currentText.dropLast(2)) + replacement
                    return newText
                }
            }
        }

        // 只在 POJ 模式支援：檢查 nn → ⁿ 轉換（鼻化音）
        // 台羅（TL）模式保持 nn 不變
        if inputMode == .poj, input.lowercased() == "n", settings.enableDoubleTapNN, currentText.count >= 3 {
            let lastTwoChars = String(currentText.suffix(3).dropLast()) // 排除剛加入的 n
            if lastTwoChars.count >= 2 {
                let secondLastChar = lastTwoChars.last!
                let thirdLastChar = lastTwoChars.dropLast().last!

                // 檢查是否為「元音 + n」的模式
                if String(secondLastChar).lowercased() == "n" {
                    let vowels = "aeiouAEIOU"
                    if vowels.contains(thirdLastChar) {
                        // 返回轉換後的新文字
                        let vowelWithNasal = String(thirdLastChar) + "ⁿ"
                        let newText = String(currentText.dropLast(3)) + vowelWithNasal
                        return newText
                    }
                }
            }
        }

        return nil
    }

    /// 應用聲調轉換
    /// - Parameter currentText: 當前組字文字
    /// - Parameter toneNumber: 聲調數字
    /// - Returns: 轉換後的文字，如果無轉換則回傳 nil
    private func applyToneConversion(currentText: String, toneNumber: Int) -> String? {
        // 安全檢查：只處理有效聲調數字（1-9）
        // 聲調 1 和 4 會由 ToneConverter 移除數字但不加調號
        guard (1 ... 9).contains(toneNumber) else {
            return nil
        }

        // 確保組字文字不為空
        guard !currentText.isEmpty else {
            return nil
        }

        // 使用 ToneConverter 轉換
        let converted = ToneConverter.convertToToneMarks(currentText, mode: inputMode)

        // 如果轉換成功（結果不同）
        if converted != currentText {
            return converted
        }

        return nil
    }

    /// 清除所有狀態（用於鍵盤重置）
    public func reset() {
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []
    }

    /// 同步狀態到屬性
    private func syncStateToProperties() {
        switch state {
        case .idle:
            isComposing = false
            composingText = ""
            rawInput = ""
            composingStartLength = 0

        case .composing(let raw, let display):
            isComposing = true
            rawInput = raw
            composingText = display
            keyboardViewController?.setMarkedText(display)
        }

        keyboardContext?.isComposingText = isComposing
    }

    /// 統一的狀態更新方法
    private func updateComposingState(_ newState: ComposingState) {
        state = newState

        switch newState {
        case .idle:
            keyboardViewController?.clearMarkedText()
            keyboardViewController?.resetAutocomplete()
        case .composing:
            keyboardViewController?.performAutocomplete()
        }
    }

    /// 嘗試聲調還原
    /// - Returns: 還原後的文字，如果無法還原則回傳 nil
    private func attemptToneRestoration() -> String? {
        guard !composingText.isEmpty else { return nil }
        return ToneConverter.restoreTone(composingText, mode: inputMode)
    }
}
