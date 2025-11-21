import KeyboardKit
import SwiftUI

struct TaigiKeyboardView: View {
    let state: Keyboard.State
    let services: Keyboard.Services
    let emojiKeyboardView: () -> AnyView
    let calloutStyle: Callouts.CalloutStyle

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    @ObservedObject var composingManager: ComposingManager

    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let onTranslateToggle: () -> Void
    let onCollapse: () -> Void

    @StateObject private var expandState = CandidateExpandState()

    var body: some View {
        let suggestions = autocompleteContext.suggestions
        let frequentWords = CandidateView.getSharedFrequentWords(in: suggestions)
        let isTranslateSwapped = keyboardContext.isTranslateSwapped

        // 響應式獲取選中狀態
        let selectedCandidateIndex = composingManager.selectedCandidateIndex

        // 根據 KeyboardContext 動態選擇候選詞樣式
        let candidateStyle = CandidateView.Style.adaptive(for: keyboardContext)

        #if DEBUG
        // Debug log: 視圖更新時的選中狀態
        let _ = print("[UI] TaigiKeyboardView body 執行，selectedCandidateIndex: \(selectedCandidateIndex)")
        #endif

        KeyboardView(
            state: state,
            services: services,
            renderBackground: false,
            buttonContent: { $0.view },
            buttonView: { $0.view },
            collapsedView: { $0.view },
            emojiKeyboard: { _ in
                emojiKeyboardView()
            },
            toolbar: { _ in
                CandidateView(
                    suggestions: suggestions,
                    frequentWords: frequentWords,
                    selectedCandidateIndex: selectedCandidateIndex,
                    onSuggestionTap: onSuggestionTap,
                    isTranslateSwapped: isTranslateSwapped,
                    onTranslateToggle: onTranslateToggle,
                    onSettingsTap: { [unowned services] in
                        services.actionHandler.handle(.settings)
                    }
                )
                .environmentObject(expandState)
                .candidateViewStyle(candidateStyle)
            },
        )
        .keyboardCalloutActions(CustomCalloutActions.directBuilder)
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
