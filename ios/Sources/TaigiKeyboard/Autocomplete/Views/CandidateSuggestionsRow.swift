// 中文: 候選詞橫向滾動列 — 台語模式下顯示 LazyHStack + 展開 chevron;英打模式下交給 KeyboardKit。

import KeyboardKit
import SwiftUI

/// 候選詞滾動列
///
/// 顯示候選詞橫向滾動列表、右側擴充候選詞 chevron。
/// 空建議時顯示 spacer；英打模式時改顯示 KeyboardKit 預設候選詞視圖。
struct CandidateSuggestionsRow: View {
    let suggestions: [AutocompleteSuggestion]
    let selectedCandidateIndex: Int
    let onSuggestionTap: (AutocompleteSuggestion) -> Void
    let isTranslateSwapped: Bool
    let candidateDisplayMode: CandidateDisplayMode
    let isTPSLayout: Bool
    let orMapsToER: Bool
    let currentInputMode: InputMode
    let englishAutocompleteView: AnyView?

    @EnvironmentObject private var expandState: CandidateExpandState
    @Environment(\.candidateTheme) private var theme
    // Resolves the expand-chevron a11y label under the display-language picker (live-switch on read).
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        HStack(spacing: 0) {
            if suggestions.isEmpty {
                Spacer()
            } else if currentInputMode == .english, let englishView = englishAutocompleteView {
                englishView
                    .frame(maxHeight: .infinity)
                    .autocompleteToolbarStyle(englishCandidateToolbarStyle)
            } else {
                taigiCandidateList

                separator

                Spacer()
                    .frame(width: 0)

                expandChevronButton
            }
        }
    }

    /// English (En-mode) suggestions render through KeyboardKit's own autocomplete
    /// toolbar, which paints item text with a system-adaptive color that ignores the
    /// active app theme — so on a themed gradient the words night-flip (white on a light
    /// gradient in system dark mode, dark on the 暗眠山貓 dark gradient in light mode).
    /// Push the theme's candidate text role into the toolbar item style so En-mode words
    /// match the Taigi candidates. The toolbar re-applies its `style.item` to every item,
    /// so the toolbar-level style (NOT `.autocompleteToolbarItemStyle`, which the toolbar
    /// overrides) is the lever that wins. `candidateTextColor` nil (預設 adaptive) keeps
    /// `theme.primaryTextColor` == `Color(.label)`, preserving the prior adaptive behavior.
    /// Mirrors Android's role-first `EnglishCandidateCell` (#425).
    private var englishCandidateToolbarStyle: AutocompleteToolbarStyle {
        var style = AutocompleteToolbarStyle.standard
        style.item.titleColor = theme.primaryTextColor
        style.item.subtitleColor = theme.secondaryTextColor
        return style
    }

    /// 台語模式下的候選詞橫向滾動列表
    private var taigiCandidateList: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            ScrollViewReader { proxy in
                LazyHStack(spacing: CandidateViewModels.UI.buttonSpacing) {
                    let displaySuggestions = Array(
                        suggestions.prefix(CandidateViewModels.UI.maxDisplayCount),
                    )
                    ForEach(
                        Array(displaySuggestions.enumerated()),
                        id: \.offset,
                    ) { index, suggestion in
                        CandidateButtonView(
                            suggestion: suggestion,
                            isTranslateSwapped: isTranslateSwapped,
                            candidateDisplayMode: candidateDisplayMode,
                            isTPSLayout: isTPSLayout,
                            orMapsToER: orMapsToER,
                            isSelected: selectedCandidateIndex == index,
                            isFirstCandidate: index == 0,
                            onTap: onSuggestionTap,
                        )
                        .id("candidate_\(index)")
                    }
                }
                .padding(.horizontal, CandidateViewModels.Spacing.small)
                .onChange(of: selectedCandidateIndex) { _, newIndex in
                    if newIndex >= 0 {
                        let animationDuration = if #available(iOS 16.0, *) { 0.25 } else { 0.15 }
                        withAnimation(.easeInOut(duration: animationDuration)) {
                            proxy.scrollTo("candidate_\(newIndex)", anchor: .center)
                        }
                    }
                }
            }
        }
        .scrollDisabled(false)
        .frame(maxHeight: .infinity)
    }

    /// 候選詞列與 chevron 之間的垂直分隔線
    private var separator: some View {
        Rectangle()
            .fill(CandidateViewModels.Colors.separatorColor)
            .frame(width: 1.0, height: 32)
            .offset(y: 7)
    }

    /// 展開/收合擴充候選詞視圖的 chevron 按鈕
    private var expandChevronButton: some View {
        Button(action: { expandState.toggle() }) {
            Image(latinSystemName: expandState.isExpanded ? "chevron.up" : "chevron.down")
                .font(KeyboardFonts.globalFont(size: 18))
                .foregroundColor(theme.primaryTextColor)
                .scaleEffect(1.2)
                .frame(width: 42, height: theme.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(lang.string(.keyboardExpandCandidates))
    }
}
