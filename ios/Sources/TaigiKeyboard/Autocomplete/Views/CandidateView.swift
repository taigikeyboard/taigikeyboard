import KeyboardKit
import OSLog
import SwiftUI

/// 候選詞列視圖
///
/// 顯示自動完成建議的水平滾動列表。
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
    /// 當前輸入模式
    let currentInputMode: InputMode
    /// 切換輸入模式的回調
    let onInputModeChange: (InputMode) -> Void
    /// 英文模式的 KeyboardKit 預設候選詞視圖（可選）
    let englishAutocompleteView: AnyView?
    /// 展開狀態（從環境物件取得）
    @EnvironmentObject private var expandState: CandidateExpandState

    // MARK: - 環境變數
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

    /// 是否顯示展開按鈕（英文模式不顯示）
    private var shouldShowExpandButton: Bool {
        !suggestions.isEmpty && currentInputMode != .english
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
                // 候選詞為空時顯示齒輪按鈕和輸入模式切換按鈕
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

                // 輸入模式切換按鈕
                inputModeSwitcher
                    .offset(y: 7)

                Spacer()
            } else {
                // 有候選詞時的顯示區域
                if currentInputMode == .english, let englishView = englishAutocompleteView {
                    // 英文模式：使用 KeyboardKit 預設候選詞視圖
                    englishView
                        .frame(maxHeight: .infinity)
                } else {
                    // 台語模式：使用自定義候選詞列表
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
                                logger.debug("[SCROLL] selectedCandidateIndex 變更為: \(newIndex)")
                                #endif
                                if newIndex >= 0 {
                                    // iOS 版本兼容性：舊版本使用較短動畫時間
                                    let animationDuration = if #available(iOS 16.0, *) { 0.25 } else { 0.15 }
                                    withAnimation(.easeInOut(duration: animationDuration)) {
                                        proxy.scrollTo("candidate_\(newIndex)", anchor: .center)
                                    }
                                    #if DEBUG
                                    logger.debug("[SCROLL] 滾動到候選詞索引: \(newIndex)")
                                    #endif
                                }
                            }
                        }
                    }
                    .scrollDisabled(false)
                    .frame(maxHeight: .infinity)
                }
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

    // MARK: - 輸入模式切換

    /// 輸入模式切換按鈕組
    private var inputModeSwitcher: some View {
        HStack(spacing: 4) {
            inputModeButton(mode: .poj, label: "POJ")
            inputModeButton(mode: .tl, label: "TL")
            inputModeButton(mode: .english, label: "En")
        }
        .padding(.horizontal, 8)
    }

    /// 單個輸入模式按鈕
    private func inputModeButton(mode: InputMode, label: String) -> some View {
        let isSelected = currentInputMode == mode
        return Button(action: {
            onInputModeChange(mode)
        }) {
            Text(label)
                .font(KeyboardModels.Fonts.globalFont(size: 14))
                .fontWeight(isSelected ? .semibold : .regular)
                .foregroundColor(isSelected ? .white : CandidateViewModels.Colors.primaryTextColor)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isSelected ? Color.accentColor : Color.clear)
                )
                .animation(nil, value: isSelected)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(label) 輸入模式")
        .accessibilityHint(isSelected ? "目前選擇" : "點擊切換至 \(label) 模式")
    }

    // MARK: - 樣式

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
        let suggestion: Autocomplete.Suggestion
        let isTranslateSwapped: Bool
        let isSelected: Bool
        let onTap: (Autocomplete.Suggestion) -> Void

        @State private var isPressed: Bool = false
        @Environment(\.colorScheme) private var colorScheme
        @Environment(\.candidateViewStyle) private var style

        private var displayTitle: String {
            CandidateCellHelper.displayTitle(for: suggestion, isTranslateSwapped: isTranslateSwapped)
        }

        private var displaySubtitle: String? {
            CandidateCellHelper.displaySubtitle(for: suggestion, isTranslateSwapped: isTranslateSwapped)
        }

        private var backgroundColor: Color {
            style.itemStyle.resolvedBackgroundColor(
                for: colorScheme,
                isSelected: isSelected,
                isPressed: isPressed,
                isLiquidGlassEnabled: CandidateCellHelper.isLiquidGlassEnabled(cornerRadius: style.itemStyle.cornerRadius)
            )
        }

        private var cornerRadius: CGFloat {
            style.itemStyle.cornerRadius ?? 8
        }

        var body: some View {
            Button(action: {
                onTap(CandidateCellHelper.suggestionToHandle(for: suggestion, isTranslateSwapped: isTranslateSwapped))
            }) {
                HStack(alignment: .bottom, spacing: CandidateViewModels.Spacing.small) {
                    Text(displayTitle)
                        .font(KeyboardModels.Fonts.globalFont(
                            size: CandidateCellHelper.titleFontSize(isTranslateSwapped: isTranslateSwapped)
                        ))
                        .fontWeight(.regular)
                        .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                        .lineLimit(1)

                    if let subtitle = displaySubtitle, !subtitle.isEmpty, subtitle != displayTitle {
                        Text(subtitle)
                            .font(KeyboardModels.Fonts.globalFont(
                                size: CandidateCellHelper.subtitleFontSize(isTranslateSwapped: isTranslateSwapped)
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
