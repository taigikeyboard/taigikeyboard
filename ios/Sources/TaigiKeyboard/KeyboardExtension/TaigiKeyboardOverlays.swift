import KeyboardKit
import SwiftUI

extension View {
    /// Mount the keyboard-extension overlays (expanded candidates + layout / symbol /
    /// settings panels + one-handed callout) on top of the host view.
    ///
    /// The parent view retains ownership of `CandidateExpandState` and
    /// `OverlayPanelState`, plus any `onChange` handlers that bridge to state
    /// the overlays themselves don't need to see (e.g. `composingManager`).
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
        oneHandedMode: OneHandedMode,
        oneHandedCalloutStyle: KeyboardCalloutStyle,
        onSelectOneHandedMode: @escaping (OneHandedMode) -> Void,
        onSelectDismissKeyboard: @escaping () -> Void,
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
        .overlay(alignment: .topTrailing) {
            if panels.wrappedValue.isOneHandedMenuExpanded {
                // Transparent full-cover backdrop: a tap anywhere outside the callout closes it.
                // The callout drops down over the keys — the extension cannot draw above its top edge.
                ZStack(alignment: .topTrailing) {
                    Color.black.opacity(0.001)
                        .onTapGesture { panels.wrappedValue.isOneHandedMenuExpanded = false }
                    OneHandedModeCallout(
                        currentMode: oneHandedMode,
                        style: oneHandedCalloutStyle,
                        onSelectMode: onSelectOneHandedMode,
                        onSelectDismiss: onSelectDismissKeyboard,
                    )
                    .padding(.top, candidateTheme.height)
                    .padding(.trailing, 8)
                }
            }
        }
    }
}
