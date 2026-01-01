import KeyboardKit
import SwiftUI

struct TaigiKeyboardView: View {
    let services: Keyboard.Services
    let layout: KeyboardLayout
    let emojiKeyboardView: () -> AnyView
    let calloutStyle: Callouts.CalloutStyle

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    @ObservedObject var composingManager: ComposingManager

    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let onTranslateToggle: () -> Void

    @StateObject private var expandState = CandidateExpandState()
    @State private var currentInputMode: InputMode = SharedSettings.shared.inputMode

    var body: some View {
        // 根據 keyboardCase 轉換候選詞大小寫
        let suggestions = SuggestionCaseTransformer.transform(
            autocompleteContext.suggestions,
            composingText: composingManager.composingText,
            keyboardCase: keyboardContext.keyboardCase,
            inputMode: SharedSettings.shared.inputMode
        )
        let frequentWords = CandidateView.getSharedFrequentWords(in: suggestions)
        let isTranslateSwapped = keyboardContext.isTranslateSwapped

        // 響應式獲取選中狀態
        let selectedCandidateIndex = composingManager.selectedCandidateIndex

        // 根據 KeyboardContext 動態選擇候選詞樣式
        let candidateStyle = CandidateView.Style.adaptive(for: keyboardContext)

        // KeyboardKit 10: 使用 layout: 和 services: 參數
        KeyboardView(
            layout: layout,
            services: services,
            buttonContent: { params in
                // 使用自訂的按鈕內容，傳入標準視圖作為後備
                TaigiButtonContent(
                    action: params.item.action,
                    keyboardContext: keyboardContext,
                    standardContent: params.view
                )
            },
            buttonView: { $0.view },
            collapsedView: { $0.view },
            emojiKeyboard: { _ in
                // KeyboardKit 10: ISEmojiView 需要明確設置高度
                emojiKeyboardView()
                    .frame(height: layout.totalHeight)
            },
            toolbar: { params in
                // 統一使用 CandidateView，英文模式傳入 KeyboardKit 預設視圖
                CandidateView(
                    suggestions: suggestions,
                    frequentWords: frequentWords,
                    selectedCandidateIndex: selectedCandidateIndex,
                    onSuggestionTap: onSuggestionTap,
                    isTranslateSwapped: isTranslateSwapped,
                    onTranslateToggle: onTranslateToggle,
                    onSettingsTap: { [unowned services] in
                        services.actionHandler.handle(.settings)
                    },
                    currentInputMode: currentInputMode,
                    onInputModeChange: { newMode in
                        currentInputMode = newMode
                        SharedSettings.shared.inputMode = newMode
                    },
                    englishAutocompleteView: currentInputMode == .english ? AnyView(params.view) : nil
                )
                .environmentObject(expandState)
                .candidateViewStyle(candidateStyle)
            },
        )
        .keyboardButtonStyle { params in
            // 套用自訂字型
            var style = params.standardStyle()
            let fontProvider = ButtonFontProvider(keyboardContext: params.context)
            style.keyboardFont = fontProvider.buttonKeyboardFont(for: params.action)
            return style
        }
        .keyboardCalloutActions(Callouts.taigiToneActions)
        .keyboardCalloutStyle(calloutStyle)
        .overlay(
            ExpandedCandidateOverlay(
                suggestions: suggestions,
                frequentWords: frequentWords,
                selectedCandidateIndex: selectedCandidateIndex,
                onSuggestionTap: onSuggestionTap,
                isTranslateSwapped: isTranslateSwapped,
                onTranslateToggle: onTranslateToggle,
                onCollapse: {
                    expandState.collapse()
                },
                isExpanded: expandState.isExpanded,
            )
            .candidateViewStyle(candidateStyle)
            .offset(y: 2), // 稍微下移展開候選詞網格位置
            alignment: .topLeading,
        )
        .background(
            // iOS 26 Liquid Glass：使用極低透明度保持觸控功能，同時讓系統 Liquid Glass 透出
            keyboardContext.isLiquidGlassEnabled ? Color.white.opacity(0.001) : Color.keyboardBackground
        )
    }
}
