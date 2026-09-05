// 把鍵盤擴充的四個 overlay(展開候選列 + 版面 / 符號 / 設定面板)貼到主 View 上的 SwiftUI helper。

import KeyboardKit
import SwiftUI

extension View {
    /// Mount all four keyboard-extension overlays (expanded candidates + layout /
    /// symbol / settings panels) on top of the host view.
    ///
    /// The parent view retains ownership of `CandidateExpandState` and
    /// `OverlayPanelState`, plus any `onChange` handlers that bridge to state
    /// the overlays themselves don't need to see (e.g. `composingManager`).
    // 主 View 呼叫此 modifier 一次掛上四個 overlay。狀態仍由父 View 持有。
    func withKeyboardOverlays(
        panels: Binding<OverlayPanelState>,
        expandState: CandidateExpandState,
        suggestions: [AutocompleteSuggestion],
        selectedCandidateIndex: Int,
        onSuggestionTap: @escaping (AutocompleteSuggestion) -> Void,
        isTranslateSwapped: Bool,
        candidateDisplayMode: CandidateDisplayMode,
        onTranslateToggle: @escaping () -> Void,
        onCandidateDisplayModeChange: @escaping (CandidateDisplayMode) -> Void,
        candidateStyle: CandidateView.Style,
        candidateTheme: CandidateTheme,
        isTPSLayout: Bool,
        orMapsToER: Bool,
        onSymbolInsert: @escaping (String) -> Void,
        onOpenSettingsApp: @escaping () -> Void,
    ) -> some View {
        overlay(
            Group {
                if expandState.isExpanded {
                    ExpandedCandidateOverlay(
                        suggestions: suggestions,
                        selectedCandidateIndex: selectedCandidateIndex,
                        onSuggestionTap: onSuggestionTap,
                        isTranslateSwapped: isTranslateSwapped,
                        candidateDisplayMode: candidateDisplayMode,
                        onTranslateToggle: onTranslateToggle,
                        onCollapse: { expandState.collapse() },
                        isTPSLayout: isTPSLayout,
                        orMapsToER: orMapsToER,
                    )
                    .candidateViewStyle(candidateStyle)
                    .offset(y: 2)
                }
            },
            alignment: .topLeading,
        )
        .overlay(
            Group {
                if panels.wrappedValue.isLayoutExpanded {
                    LayoutSelectionOverlay(
                        isExpanded: true,
                        onDismiss: { panels.wrappedValue.isLayoutExpanded = false },
                    )
                    .offset(y: candidateTheme.height)
                }
            },
            alignment: .topLeading,
        )
        .overlay(
            Group {
                if panels.wrappedValue.isSymbolExpanded {
                    SymbolSelectionOverlay(
                        isExpanded: true,
                        onSymbolInsert: onSymbolInsert,
                        onDismiss: {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                panels.wrappedValue.isSymbolExpanded = false
                            }
                        },
                    )
                    .offset(y: candidateTheme.height)
                }
            },
            alignment: .topLeading,
        )
        .overlay(
            Group {
                if panels.wrappedValue.isSettingsExpanded {
                    SettingsSelectionOverlay(
                        isExpanded: true,
                        onDismiss: { panels.wrappedValue.isSettingsExpanded = false },
                        onOpenApp: onOpenSettingsApp,
                        onCandidateDisplayModeChange: onCandidateDisplayModeChange,
                    )
                    .offset(y: candidateTheme.height)
                }
            },
            alignment: .topLeading,
        )
    }
}
