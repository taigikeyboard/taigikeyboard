import KeyboardKit
import SwiftUI

/// 候選詞滾動列
///
/// 顯示候選詞橫向滾動列表、右側擴充候選詞 chevron。
/// 空建議時顯示 spacer；英打模式時改顯示 KeyboardKit 預設候選詞視圖。
struct CandidateSuggestionsRow: View {
    let suggestions: [Autocomplete.Suggestion]
    let selectedCandidateIndex: Int
    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let isTranslateSwapped: Bool
    let isTPSLayout: Bool
    let orMapsToER: Bool
    let currentInputMode: InputMode
    let englishAutocompleteView: AnyView?

    @EnvironmentObject private var expandState: CandidateExpandState
    @Environment(\.candidateTheme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            if suggestions.isEmpty {
                Spacer()
            } else if currentInputMode == .english, let englishView = englishAutocompleteView {
                englishView
                    .frame(maxHeight: .infinity)
            } else {
                taigiCandidateList

                separator

                Spacer()
                    .frame(width: 0)

                expandChevronButton
            }
        }
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
                            isTPSLayout: isTPSLayout,
                            orMapsToER: orMapsToER,
                            isSelected: selectedCandidateIndex == index,
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
            Image(systemName: expandState.isExpanded ? "chevron.up" : "chevron.down")
                .font(KeyboardFonts.globalFont(size: 18))
                .foregroundColor(theme.primaryTextColor)
                .scaleEffect(1.2)
                .frame(width: 42, height: theme.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(expandState.isExpanded ? "收合候選詞" : "展開候選詞")
        .accessibilityHint("點擊以\(expandState.isExpanded ? "收合" : "展開")更多候選詞選項")
    }
}
