import Foundation
import KeyboardKit
import SwiftUI

/// 組字管理器 - 管理台語輸入的組字狀態
/// 符合主流輸入法的組字模式，直接在輸入框顯示組字文字
/// 組字中的文字可以修改，確認後不可修改
public class ComposingManager: ObservableObject {

    // MARK: - Composing State

    /// 組字狀態 - 單一真相來源
    private enum ComposingState {
        case idle                           // 閒置（非組字模式）
        case composing(text: String)        // 組字中（含組字文字）
    }

    /// 當前組字狀態
    private var state: ComposingState = .idle {
        didSet {
            // 自動同步所有相關狀態
            syncStateToProperties()
        }
    }

    // MARK: - Published Properties

    /// 是否正在組字中（從 state 衍生的 computed property）
    @Published public private(set) var isComposing: Bool = false

    /// 組字的文字內容（從 state 衍生）
    @Published public private(set) var composingText: String = ""

    /// 組字開始時的文字長度（用於計算要刪除的字數）
    private var composingStartLength: Int = 0

    /// 候選詞列表（簡化：只用於更新狀態通知）
    @Published public var suggestions: [Autocomplete.Suggestion] = []

    /// 當前選中的候選詞索引
    /// 第 0 個候選詞永遠是當前組字文字，第 1 個位置開始是建議候選詞
    /// 預設為 0，表示選中當前組字文字
    @Published public var selectedCandidateIndex: Int = 0

    // MARK: - Private Properties

    /// 鍵盤上下文（用於觸發 UI 更新）
    private weak var keyboardContext: KeyboardContext?

    /// 鍵盤控制器（用於自動完成觸發）
    private weak var keyboardViewController: KeyboardViewController?

    /// 輸入模式（POJ/TL）
    private var inputMode: InputMode {
        SharedSettings.shared.inputMode
    }

    // MARK: - Initialization

    public init() {}

    /// 設定鍵盤上下文（用於觸發 UI 更新）
    public func setKeyboardContext(_ context: KeyboardContext) {
        keyboardContext = context
    }

    /// 設定鍵盤控制器（用於自動完成觸發）
    func setKeyboardViewController(_ controller: KeyboardViewController?) {
        keyboardViewController = controller
    }

    // MARK: - Public Methods

    /// 開始組字（使用 KeyboardKit 架構）
    public func startComposing(with text: String) {

        composingStartLength = text.count
        selectedCandidateIndex = 0  // 預設選中第一個候選詞

        // 使用統一的狀態更新方法
        updateComposingState(.composing(text: text))
    }

    /// 追加字元到組字（使用 KeyboardKit 架構）
    public func appendCharacter(_ char: String) {
        guard isComposing else {
            startComposing(with: char)
            return
        }

        selectedCandidateIndex = 0  // 重置為第一個候選詞

        // 計算基本的新文字
        var finalText = composingText + char

        // 檢查字符組合轉換（如 oo → o͘, nn → ⁿ）
        if let transformedText = checkCharacterCombination(currentText: finalText, input: char) {
            finalText = transformedText
        }

        // 如果是數字，嘗試聲調轉換
        // 排除聲調 1 和 4（兩者都不做聲調標記，保留數字顯示）
        if let number = Int(char), (2 ... 9).contains(number), number != 4 {
            if let toneConvertedText = applyToneConversion(currentText: finalText, toneNumber: number) {
                finalText = toneConvertedText
            }
        }

        // 使用統一的狀態更新方法（只調用一次）
        updateComposingState(.composing(text: finalText))
    }

    /// 處理連字號輸入
    public func appendHyphen() {
        appendCharacter("-")
    }

    /// 刪除組字的字元（Backspace）- 使用 KeyboardKit 架構
    public func deleteBackward() {
        guard isComposing, !composingText.isEmpty else { return }

        // 先嘗試聲調還原
        if let restoredText = attemptToneRestoration() {
            // 更新為還原後的文字
            updateComposingState(.composing(text: restoredText))
            return
        }

        // 一般字符刪除
        let newText = String(composingText.dropLast())

        if newText.isEmpty {
            // 清空組字狀態，退出組字模式
            updateComposingState(.idle)
            selectedCandidateIndex = -1
            suggestions = []

            // 手動刪除剩餘字符（修復單字母需要按兩次的問題）
            keyboardViewController?.deleteBackwardManually()
        } else {
            // 更新為剩餘內容
            updateComposingState(.composing(text: newText))
        }
    }

    /// 確認組字（Enter 鍵）- 確保 markedText 完全清除
    public func commitComposition() {
        guard isComposing, !composingText.isEmpty else { return }


        // 儲存組字文字（因為狀態變更後會清空）
        let textToInsert = composingText

        // 更新狀態為 idle（會自動清除 markedText）
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []

        // 插入組字文字到文檔
        keyboardViewController?.textDocumentProxy.insertText(textToInsert)

        // 明確清空 AutocompleteContext 的候選詞（使用 reset 方法）
        keyboardViewController?.state.autocompleteContext.reset()
    }

    /// 選擇候選詞（替換 markedText 並確認提交）
    public func selectSuggestion(_ suggestion: Autocomplete.Suggestion) {
        guard isComposing else { return }


        // 直接清除 markedText（不提交）並插入候選詞
        if let proxy = keyboardViewController?.textDocumentProxy {
            // 清除 markedText 狀態但不提交內容
            proxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
            proxy.unmarkText()

            // 檢測並保留連字符前綴
            let hyphenPrefix = extractHyphenPrefix(from: composingText)
            let suggestionWithoutHyphens = suggestion.text.trimmingCharacters(in: CharacterSet(charactersIn: "-"))

            let textToInsert: String
            if suggestionWithoutHyphens.isEmpty {
                // 候選詞本身是純連字符,直接使用候選詞
                textToInsert = suggestion.text
            } else {
                // 候選詞包含實際內容,保留前綴
                textToInsert = hyphenPrefix + suggestion.text
            }

            // 插入候選詞文字（包含前綴）
            proxy.insertText(textToInsert)

        }

        // 更新狀態為 idle
        state = .idle
        syncStateToProperties()
        selectedCandidateIndex = -1
        suggestions = []

        // 觸發重置自動完成並明確清空候選詞
        keyboardViewController?.resetAutocomplete()
        // 明確清空 AutocompleteContext 的候選詞（使用 reset 方法）
        keyboardViewController?.state.autocompleteContext.reset()

    }

    /// 提取連字符前綴
    private func extractHyphenPrefix(from text: String) -> String {
        var prefix = ""
        for char in text {
            if char == "-" {
                prefix.append(char)
            } else {
                break
            }
        }
        return prefix
    }

    /// 移動到下一個候選詞（空白鍵功能）
    /// 使用外部傳入的候選詞列表，避免狀態同步問題
    /// 循環邏輯：
    /// - 第 0 個：當前組字文字
    /// - 第 1-N 個：建議候選詞
    /// - 循環：0 → 1 → ... → N → 0
    /// - Parameter availableSuggestions: 當前可用的候選詞列表
    /// - Returns: 是否成功移動選中狀態
    public func moveToNextCandidate(availableSuggestions: [Autocomplete.Suggestion]) -> Bool {

        guard self.isComposing, !availableSuggestions.isEmpty else {
            return false
        }

        // 循環邏輯：0(組字文字) → 1(建議0) → ... → N(建議N-1) → 0(回到組字文字)
        let nextIndex: Int
        if self.selectedCandidateIndex >= availableSuggestions.count - 1 {
            // 從最後一個候選詞回到第一個候選詞（組字文字）
            nextIndex = 0
        } else {
            // 移動到下一個候選詞
            nextIndex = self.selectedCandidateIndex + 1
        }

        self.selectedCandidateIndex = nextIndex

        return true
    }

    /// 確認當前選中的候選詞
    /// - Parameter availableSuggestions: 當前可用的候選詞列表
    /// - Returns: 是否成功確認選擇
    public func confirmSelectedCandidate(availableSuggestions: [Autocomplete.Suggestion]) -> Bool {
        guard isComposing else { return false }

        // 確認當前選中的候選詞
        if selectedCandidateIndex >= 0 && selectedCandidateIndex < availableSuggestions.count {
            let selectedSuggestion = availableSuggestions[selectedCandidateIndex]
            selectSuggestion(selectedSuggestion)
            return true
        }

        return false
    }


    // MARK: - Private Methods

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
        // 安全檢查：只處理有效聲調數字
        guard (2 ... 9).contains(toneNumber) else {
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
        // 使用統一的狀態更新方法
        updateComposingState(.idle)
        selectedCandidateIndex = -1
        suggestions = []
    }


    /// 同步狀態到屬性（從 state 衍生所有屬性值）
    private func syncStateToProperties() {
        switch state {
        case .idle:
            // 非組字模式：清理所有狀態
            isComposing = false
            composingText = ""
            composingStartLength = 0

            // 注意：不在這裡呼叫 clearMarkedText()
            // 避免與 replaceMarkedText 產生衝突

        case .composing(let text):
            // 組字模式：設置組字狀態
            isComposing = true
            composingText = text

            // 同步 markedText - 必須顯示
            keyboardViewController?.setMarkedText(text)
        }

        // 更新 KeyboardContext
        keyboardContext?.isComposingText = isComposing
    }

    /// 統一的狀態更新方法 - 所有狀態變更都必須通過這裡
    private func updateComposingState(_ newState: ComposingState) {

        // 更新狀態（會自動觸發 syncStateToProperties）
        state = newState

        // 根據新狀態觸發相應的後續動作
        switch newState {
        case .idle:
            // 組字結束，清除 markedText 並觸發重置自動完成
            keyboardViewController?.clearMarkedText()
            keyboardViewController?.resetAutocomplete()

        case .composing:
            // 組字中，觸發自動完成
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
