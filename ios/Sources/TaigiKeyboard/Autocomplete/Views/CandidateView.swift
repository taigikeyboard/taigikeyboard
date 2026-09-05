// Candidate row container — owns toolbar expansion and auto-collapse on mode / composing changes.

import KeyboardKit
import SwiftUI

/// Stateful orchestrator: owns `isToolShortcutsExpanded` and auto-collapses on input-mode or
/// composing changes. `ToolShortcutsToolbar` and `CandidateSuggestionsRow` do the drawing.
struct CandidateView: View {
    let suggestions: [AutocompleteSuggestion]
    let selectedCandidateIndex: Int
    let onSuggestionTap: (AutocompleteSuggestion) -> Void
    /// Swaps the 漢字 / 羅馬字 display positions.
    let isTranslateSwapped: Bool
    let candidateDisplayMode: CandidateDisplayMode
    let onSettingsTap: () -> Void
    let onLayoutTap: () -> Void
    let onSymbolTap: () -> Void
    let onDismissKeyboard: () -> Void
    let currentInputMode: InputMode
    let onInputModeChange: (InputMode) -> Void
    let englishAutocompleteView: AnyView?
    /// Whether the engine is currently composing (used to auto-collapse toolbar)
    let isComposing: Bool
    /// TPS layout — affects candidate rendering and commit logic.
    let isTPSLayout: Bool
    /// Whether `or` maps to ㄜ in TPS mode.
    let orMapsToER: Bool

    @State private var isToolShortcutsExpanded = false

    @Environment(\.candidateViewStyle) private var style
    @Environment(\.colorScheme) private var colorScheme

    /// Larger negative offset on iOS 26+, smaller on older versions to avoid clipping.
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
                    candidateDisplayMode: candidateDisplayMode,
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

    private func autoCollapseIfNeeded() {
        guard SharedSettings.shared.isToolbarAutoCollapse, isToolShortcutsExpanded else { return }
        withAnimation(.easeInOut(duration: 0.2)) {
            isToolShortcutsExpanded = false
        }
    }
}
