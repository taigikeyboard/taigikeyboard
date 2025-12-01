import KeyboardKit
import OSLog
import SwiftUI

/// 候選詞列視圖
/// 顯示自動完成建議的水平滾動列表
struct CandidateView: View {
    /// 候選詞建議列表
    let suggestions: [Autocomplete.Suggestion]
    /// 常用詞集合（用於優先顯示）
    let frequentWords: Set<String>
    /// 當前選中的候選詞索引
    let selectedCandidateIndex: Int
    /// 點擊候選詞時的回調
    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    /// 是否交換漢字與羅馬字顯示位置
    let isTranslateSwapped: Bool
    /// 切換翻譯模式的回調
    let onTranslateToggle: () -> Void
    /// 點擊設定按鈕的回調
    let onSettingsTap: () -> Void
    /// 展開狀態（從環境物件取得）
    @EnvironmentObject private var expandState: CandidateExpandState

    // MARK: - 樣式環境變數
    /// 候選詞視圖樣式
    @Environment(\.candidateViewStyle) private var style
    /// 系統顏色模式（淺色/深色）
    @Environment(\.colorScheme) private var colorScheme

    #if DEBUG
        private let logger = Logger(
            subsystem: LexiconConstants.Logging.subsystem,
            category: "CandidateView",
        )
    #endif

    /// 是否顯示展開按鈕
    private var shouldShowExpandButton: Bool {
        !suggestions.isEmpty
    }

    /// iOS 版本兼容的候選詞列上邊距
    /// iOS 26+ 使用較大負偏移，舊版本使用較小負偏移以避免顯示問題
    private var topOffset: CGFloat {
        if #available(iOS 26.0, *) {
            return -6  // iOS 26+ 保持現有設定
        } else {
            return -2  // iOS 26 以下增加 padding（減少負偏移）
        }
    }

    var body: some View {
        candidateBarView
    }

    /// 候選詞列主視圖
    /// 包含水平滾動的候選詞列表和展開/收合按鈕
    private var candidateBarView: some View {
        HStack(spacing: 0) {
            if suggestions.isEmpty {
                // 候選詞為空時顯示齒輪按鈕
                Button(action: {
                    onSettingsTap()
                }) {
                    Image(systemName: "gearshape.fill")
                        .font(KeyboardModels.Fonts.globalFont(size: 18))
                        .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                        .scaleEffect(1.2)
                        .frame(width: 42, height: CandidateViewModels.UI.height)
                        .contentShape(Rectangle())
                        .offset(y: 7)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("設定")
                .accessibilityHint("點擊以開啟鍵盤設定")

                Spacer()
            } else {
                ScrollView(.horizontal, showsIndicators: false) {
                    ScrollViewReader { proxy in
                        LazyHStack(spacing: CandidateViewModels.UI.buttonSpacing) {
                            // 限制最大顯示數量以優化效能
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
                                    isSelected: selectedCandidateIndex == index,
                                    onTap: onSuggestionTap,
                                )
                                .id("candidate_\(index)")  // 為每個候選詞設定 ID，用於自動滾動
                            }
                        }
                        .padding(.horizontal, CandidateViewModels.Spacing.small)
                        .onChange(of: selectedCandidateIndex) { oldIndex, newIndex in
                            // 當選中索引改變時，自動滾動到對應的候選詞
                            #if DEBUG
                            print("[SCROLL] selectedCandidateIndex 變更為: \(newIndex)")
                            #endif
                            if newIndex >= 0 {
                                // iOS 版本兼容性：舊版本使用較短動畫時間
                                let animationDuration = if #available(iOS 16.0, *) { 0.25 } else { 0.15 }
                                withAnimation(.easeInOut(duration: animationDuration)) {
                                    proxy.scrollTo("candidate_\(newIndex)", anchor: .center)
                                }
                                #if DEBUG
                                print("[SCROLL] 滾動到候選詞索引: \(newIndex)")
                                #endif
                            }
                        }
                    }
                }
                .scrollDisabled(false)
                .frame(maxHeight: .infinity)
            }

            if shouldShowExpandButton {
                Rectangle()
                    .fill(CandidateViewModels.Colors.separatorColor)
                    .frame(width: 1.0, height: 32) // 固定高度，上下留空間
                    .offset(y: 7) // 垂直分隔線往下移動 3pt

                Spacer()
                    .frame(width: 0) // 進一步減少分隔線和按鈕之間的間距，讓按鈕更往左移

                Button(action: {
                    expandState.toggle()
                }) {
                    Image(systemName: expandState.isExpanded ? "chevron.up" : "chevron.down")
                        .font(KeyboardModels.Fonts.globalFont(size: 18)) // 使用全域字型
                        .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                        .scaleEffect(1.2) // 增加縮放從 1.1 到 1.2
                        .frame(width: 42, height: CandidateViewModels.UI.height) // 增加寬度從 36 到 42
                        .contentShape(Rectangle())
                        .offset(y: 7) // 展開收合按鈕往下移動 3pt
                }
                .buttonStyle(.plain)
                .accessibilityLabel(expandState.isExpanded ? "收合候選詞" : "展開候選詞")
                .accessibilityHint("點擊以\(expandState.isExpanded ? "收合" : "展開")更多候選詞選項")
            }
        }
        .frame(height: style.height)
        .background(resolvedBackgroundColor)
        .offset(y: topOffset) // iOS 版本兼容的上邊距設定
    }

    // MARK: - 樣式解析器

    /// 解析實際的背景色
    /// 支援 iOS 26 的 Liquid Glass 透明效果
    private var resolvedBackgroundColor: Color {
        // 檢查是否為 Liquid Glass 模式
        let isLiquidGlassEnabled = (style.itemStyle.cornerRadius == 9 && style.backgroundColor == nil)

        if isLiquidGlassEnabled {
            // iOS 26 Liquid Glass：使用極低透明度保持觸控功能，同時讓系統 Liquid Glass 透出
            return Color.white.opacity(0.001)
        } else {
            // 非 Liquid Glass 模式使用原有邏輯
            return style.backgroundColor ?? Color.keyboardBackground(for: colorScheme)
        }
    }
}

extension CandidateView {

    /// 計算候選詞中的常用詞集合
    /// 根據使用頻率動態計算閾值，選出最常用的詞彙
    /// - Parameter suggestions: 候選詞列表
    /// - Returns: 常用詞的集合
    static func getSharedFrequentWords(in suggestions: [Autocomplete.Suggestion]) -> Set<String> {
        guard !suggestions.isEmpty else { return Set<String>() }

        let frequencies = suggestions.map { suggestion in
            (suggestion.text, UserFrequencyService.getFrequency(for: suggestion.text))
        }

        let candidatesWithMinFreq = frequencies.filter { $0.1 >= 2 }
        guard !candidatesWithMinFreq.isEmpty else { return Set<String>() }

        let allFreqs = frequencies.map(\.1)
        let maxFreq = allFreqs.max() ?? 0
        let avgFreq = allFreqs.reduce(0, +) / max(1, allFreqs.count)

        let threshold = max(2, min(avgFreq, maxFreq / 3))

        let frequentCandidates = candidatesWithMinFreq.filter { $0.1 >= threshold }

        let maxFrequentCount = max(1, min(3, suggestions.count / 2))
        let sortedFrequent = frequentCandidates.sorted { $0.1 > $1.1 }

        return Set(sortedFrequent.prefix(maxFrequentCount).map(\.0))
    }

    /// 單個候選詞按鈕視圖
    private struct CandidateButtonView: View {
        /// 候選詞資料
        let suggestion: Autocomplete.Suggestion
        /// 是否交換漢字與羅馬字顯示
        let isTranslateSwapped: Bool
        /// 是否為當前選中項
        let isSelected: Bool
        /// 點擊回調
        let onTap: (Autocomplete.Suggestion) -> Void

        @State private var isPressed: Bool = false
        @Environment(\.colorScheme) private var colorScheme
        @Environment(\.candidateViewStyle) private var style

        private var backgroundColor: Color {
            return style.itemStyle.resolvedBackgroundColor(
                for: colorScheme,
                isSelected: isSelected,
                isPressed: isPressed,
                isLiquidGlassEnabled: isLiquidGlassEnabled
            )
        }

        private var isLiquidGlassEnabled: Bool {
            // 從 TaigiKeyboardView 的樣式推斷 Liquid Glass 狀態
            return style.itemStyle.cornerRadius == 9
        }

        private var cornerRadius: CGFloat {
            // 暫時使用固定值，之後在 TaigiKeyboardView 中根據 KeyboardContext 設定
            return style.itemStyle.cornerRadius ?? 8
        }

        /// 計算要顯示的主要文字
        /// 根據交換設定決定顯示內容
        /// showHanjiMode 固定為 true
        private var displayTitle: String {
            if isTranslateSwapped, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
                return subtitle
            } else {
                return suggestion.text
            }
        }

        /// 計算要顯示的副標題文字
        /// showHanjiMode 固定為 true，永遠顯示副標題
        private var displaySubtitle: String? {
            if isTranslateSwapped {
                return suggestion.text
            } else {
                return suggestion.subtitle
            }
        }

        var body: some View {
            Button(action: {
                let suggestionToHandle: Autocomplete.Suggestion
                // showHanjiMode 固定為 true
                if isTranslateSwapped, let subtitle = suggestion.subtitle, !subtitle.isEmpty {
                    let originalTextLength = suggestion.text.count
                    let newTextLength = subtitle.count
                    let additionalDeleteCount = max(0, originalTextLength - newTextLength)

                    suggestionToHandle = Autocomplete.Suggestion(
                        text: subtitle,
                        title: subtitle,
                        subtitle: suggestion.text,
                        additionalDeleteCount: additionalDeleteCount,
                        additionalInfo: suggestion.additionalInfo
                    )
                } else {
                    suggestionToHandle = suggestion
                }
                onTap(suggestionToHandle)
            }) {
                HStack(alignment: .bottom, spacing: CandidateViewModels.Spacing.small) {
                    Text(displayTitle)
                        .font(KeyboardModels.Fonts.globalFont(
                            size: CandidateViewModels.UI.primaryFontSize
                        ))
                        .fontWeight(.regular)
                        .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                        .lineLimit(1)

                    if let subtitle = displaySubtitle, !subtitle.isEmpty, subtitle != displayTitle {
                        Text(subtitle)
                            .font(KeyboardModels.Fonts.globalFont(
                                size: CandidateViewModels.UI.secondaryFontSize
                            ))
                            .foregroundColor(CandidateViewModels.Colors.secondaryTextColor)
                            .lineLimit(1)
                    }
                }
                .padding(.horizontal, style.itemStyle.horizontalPadding)
                .padding(.vertical, style.itemStyle.verticalPadding)
            }
            .background(
                RoundedRectangle(cornerRadius: cornerRadius)
                    .fill(backgroundColor)
                    .padding(.horizontal, -2) // 減少水平擴展，縮小點擊區域
                    .padding(.vertical, -4)   // 減少垂直擴展，縮小點擊區域
            )
            .offset(y: 5) // 讓整個候選詞項目背景往下移動
            .scaleEffect(isPressed ? 0.95 : 1.0)
            .buttonStyle(PlainButtonStyle())
            .onLongPressGesture(minimumDuration: 0, maximumDistance: .infinity, pressing: { pressing in
                withAnimation(.easeInOut(duration: 0.1)) {
                    isPressed = pressing
                }
            }, perform: {})
        }
    }
}
