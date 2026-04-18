import KeyboardKit
import SwiftUI

extension View {
    /// Mount all four keyboard-extension overlays (expanded candidates + layout /
    /// symbol / settings panels) on top of the host view.
    ///
    /// The parent view retains ownership of `CandidateExpandState` and
    /// `OverlayPanelState`, plus any `onChange` handlers that bridge to state
    /// the overlays themselves don't need to see (e.g. `composingManager`).
    func withKeyboardOverlays(
        panels: Binding<OverlayPanelState>,
        expandState: CandidateExpandState,
        suggestions: [Autocomplete.Suggestion],
        selectedCandidateIndex: Int,
        onSuggestionTap: @escaping (Autocomplete.Suggestion) -> Void,
        isTranslateSwapped: Bool,
        onTranslateToggle: @escaping () -> Void,
        candidateStyle: CandidateView.Style,
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
                    .offset(y: CandidateViewModels.UI.height)
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
                    .offset(y: CandidateViewModels.UI.height)
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
                    )
                    .offset(y: CandidateViewModels.UI.height)
                }
            },
            alignment: .topLeading,
        )
    }
}
