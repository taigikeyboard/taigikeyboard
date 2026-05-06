// 中文: 候選詞列的容器 view。負責 toolbar 展開狀態與輸入模式 / 組字切換時的自動收合。

import KeyboardKit
import SwiftUI

/// 候選詞列容器視圖
///
/// Stateful orchestrator：擁有 `isToolShortcutsExpanded` 狀態、處理輸入模式 /
/// 組字切換時的自動收合；實際按鈕列與候選詞滾動由 `ToolShortcutsToolbar` 與
/// `CandidateSuggestionsRow` 負責。
struct CandidateView: View {
    /// 候選詞建議列表
    let suggestions: [Autocomplete.Suggestion]
    /// 當前選中的候選詞索引
    let selectedCandidateIndex: Int
    /// 點擊候選詞時的回調
    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    /// 是否交換漢字與羅馬字顯示位置
    let isTranslateSwapped: Bool
    /// 點擊設定按鈕的回調
    let onSettingsTap: () -> Void
    /// 點擊佈局選擇按鈕的回調
    let onLayoutTap: () -> Void
    /// 點擊符號面板按鈕的回調
    let onSymbolTap: () -> Void
    /// 點擊收合鍵盤按鈕的回調
    let onDismissKeyboard: () -> Void
    /// 當前輸入模式
    let currentInputMode: InputMode
    /// 切換輸入模式的回調
    let onInputModeChange: (InputMode) -> Void
    /// 英文模式的 KeyboardKit 預設候選詞視圖（可選）
    let englishAutocompleteView: AnyView?
    /// Whether the engine is currently composing (used to auto-collapse toolbar)
    let isComposing: Bool
    /// 是否為 TPS 佈局模式（影響候選詞顯示與 commit 邏輯）
    let isTPSLayout: Bool
    /// TPS 模式下 `or` 是否映射為 ㄜ
    let orMapsToER: Bool

    /// 工具快捷鍵（輸入模式切換）是否展開
    @State private var isToolShortcutsExpanded = false

    /// 候選詞視圖樣式
    @Environment(\.candidateViewStyle) private var style
    /// 系統顏色模式（淺色/深色）
    @Environment(\.colorScheme) private var colorScheme

    /// iOS 版本兼容的候選詞列上邊距
    /// iOS 26+ 使用較大負偏移，舊版本使用較小負偏移以避免顯示問題
    private var topOffset: CGFloat {
        if #available(iOS 26.0, *) {
            -6
        } else {
            -2
        }
    }

    var body: some View {
        HStack(spacing: 0) {
            ToolShortcutsToolbar(
                isExpanded: $isToolShortcutsExpanded,
                currentInputMode: currentInputMode,
                onInputModeChange: onInputModeChange,
                onSymbolTap: onSymbolTap,
                onLayoutTap: onLayoutTap,
                onDismissKeyboard: onDismissKeyboard,
                onSettingsTap: onSettingsTap,
            )

            if !isToolShortcutsExpanded {
                CandidateSuggestionsRow(
                    suggestions: suggestions,
                    selectedCandidateIndex: selectedCandidateIndex,
                    onSuggestionTap: onSuggestionTap,
                    isTranslateSwapped: isTranslateSwapped,
                    isTPSLayout: isTPSLayout,
                    orMapsToER: orMapsToER,
                    currentInputMode: currentInputMode,
                    englishAutocompleteView: englishAutocompleteView,
                )
                .transition(.move(edge: .top))
            }
        }
        .frame(height: style.height)
        .background(style.resolvedBarBackground(for: colorScheme))
        .clipped()
        .offset(y: topOffset)
        .onChange(of: currentInputMode) { _, _ in
            autoCollapseIfNeeded()
        }
        .onChange(of: isComposing) { _, newValue in
            if newValue {
                autoCollapseIfNeeded()
            }
        }
    }

    /// 使用者在設定中啟用自動收合時，關閉工具快捷鍵列。
    private func autoCollapseIfNeeded() {
        guard SharedSettings.shared.isToolbarAutoCollapse, isToolShortcutsExpanded else { return }
        withAnimation(.easeInOut(duration: 0.2)) {
            isToolShortcutsExpanded = false
        }
    }
}
