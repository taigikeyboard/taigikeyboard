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
    /// 點擊佈局選擇按鈕的回調
    let onLayoutTap: () -> Void
    /// 點擊 Emoji 按鈕的回調
    let onEmojiTap: () -> Void
    /// 當前輸入模式
    let currentInputMode: InputMode
    /// 切換輸入模式的回調
    let onInputModeChange: (InputMode) -> Void
    /// 英文模式的 KeyboardKit 預設候選詞視圖（可選）
    let englishAutocompleteView: AnyView?
    /// Whether the engine is currently composing (used to auto-collapse toolbar)
    let isComposing: Bool
    /// 展開狀態（從環境物件取得）
    @EnvironmentObject private var expandState: CandidateExpandState
    /// 工具快捷鍵（輸入模式切換）是否展開
    @State private var isToolShortcutsExpanded = false

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
    /// Toggle 按鈕永遠顯示在左側，切換候選詞與工具快捷鍵（參考 KeyboardKit ToggleToolbar 模式）
    private var candidateBarView: some View {
        HStack(spacing: 0) {
            // Toggle button always visible on the left
            toolShortcutsToggleButton

            // Tool shortcuts: slide up from bottom
            if isToolShortcutsExpanded {
                HStack(spacing: 0) {
                    inputModeSwitcher
                        .offset(y: 7)
                    Spacer()
                    globeButton
                    emojiButton
                    layoutButton
                    settingsButton
                }
                .transition(.move(edge: .bottom))
            }

            // Candidate content: slide down from top
            if !isToolShortcutsExpanded {
                HStack(spacing: 0) {
                    if suggestions.isEmpty {
                        Spacer()
                    } else if currentInputMode == .english, let englishView = englishAutocompleteView {
                        // English mode: KeyboardKit default autocomplete
                        englishView
                            .frame(maxHeight: .infinity)
                    } else {
                        // Taigi mode: scrolling candidate list
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
                                            isSelected: selectedCandidateIndex == index,
                                            onTap: onSuggestionTap,
                                        )
                                        .id("candidate_\(index)")
                                    }
                                }
                                .padding(.horizontal, CandidateViewModels.Spacing.small)
                                .onChange(of: selectedCandidateIndex) { oldIndex, newIndex in
                                    if newIndex >= 0 {
                                        #if DEBUG
                                        logger.debug("[SCROLL] scrollTo candidate index: \(newIndex)")
                                        #endif
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

                        // Expand/collapse chevron for candidate grid
                        candidateBarSeparator

                        Spacer()
                            .frame(width: 0)

                        Button(action: {
                            expandState.toggle()
                        }) {
                            Image(systemName: expandState.isExpanded ? "chevron.up" : "chevron.down")
                                .font(KeyboardModels.Fonts.globalFont(size: 18))
                                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                                .scaleEffect(1.2)
                                .frame(width: 42, height: CandidateViewModels.UI.height)
                                .contentShape(Rectangle())
                                .offset(y: 7)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel(expandState.isExpanded ? "收合候選詞" : "展開候選詞")
                        .accessibilityHint("點擊以\(expandState.isExpanded ? "收合" : "展開")更多候選詞選項")
                    }
                }
                .transition(.move(edge: .top))
            }
        }
        .frame(height: style.height)
        .background(resolvedBackgroundColor)
        .clipped()
        .offset(y: topOffset)
        .onChange(of: currentInputMode) { _, _ in
            // Auto-collapse shortcuts when input mode changes
            withAnimation(.easeInOut(duration: 0.2)) {
                isToolShortcutsExpanded = false
            }
        }
        .onChange(of: isComposing) { _, newValue in
            // Auto-collapse toolbar when user starts typing
            if newValue && isToolShortcutsExpanded {
                withAnimation(.easeInOut(duration: 0.2)) {
                    isToolShortcutsExpanded = false
                }
            }
        }
    }

    // MARK: - 工具快捷鍵展開/收合

    /// "+" 工具快捷鍵展開/收合按鈕（參考 KeyboardKit ToggleToolbar 模式）
    /// Rotates 45° to form "×" when expanded.
    private var toolShortcutsToggleButton: some View {
        Button(action: {
            withAnimation(.easeInOut(duration: 0.2)) {
                isToolShortcutsExpanded.toggle()
            }
        }) {
            Image(systemName: "plus")
                .font(KeyboardModels.Fonts.globalFont(size: 16))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .rotationEffect(.degrees(isToolShortcutsExpanded ? 45 : 0))
                .animation(.easeInOut(duration: 0.2), value: isToolShortcutsExpanded)
                .frame(width: 36, height: CandidateViewModels.UI.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(isToolShortcutsExpanded ? "收合工具列" : "展開工具列")
    }

    /// Globe button for switching keyboards (tap: next keyboard, long-press: keyboard picker)
    private var globeButton: some View {
        Keyboard.NextKeyboardButton {
            Image(systemName: "globe")
                .font(KeyboardModels.Fonts.globalFont(size: 18))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .scaleEffect(1.2)
                .frame(width: 42, height: CandidateViewModels.UI.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .accessibilityLabel("切換鍵盤")
        .accessibilityHint("點擊切換下一個鍵盤，長按選取鍵盤")
    }

    /// Emoji keyboard button
    private var emojiButton: some View {
        Button(action: {
            onEmojiTap()
        }) {
            Image(systemName: "face.smiling")
                .font(KeyboardModels.Fonts.globalFont(size: 18))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .scaleEffect(1.2)
                .frame(width: 42, height: CandidateViewModels.UI.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .buttonStyle(ToolShortcutButtonStyle())
        .accessibilityLabel("Emoji")
        .accessibilityHint("點擊以開啟 Emoji 鍵盤")
    }

    /// Layout selection button
    private var layoutButton: some View {
        Button(action: {
            onLayoutTap()
        }) {
            Image(systemName: "photo")
                .font(KeyboardModels.Fonts.globalFont(size: 18))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .scaleEffect(1.2)
                .frame(width: 42, height: CandidateViewModels.UI.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .buttonStyle(ToolShortcutButtonStyle())
        .accessibilityLabel("佈局選擇")
        .accessibilityHint("點擊以開啟佈局選擇面板")
    }

    /// Settings gear button
    private var settingsButton: some View {
        Button(action: {
            onSettingsTap()
        }) {
            Image(systemName: "gearshape")
                .font(KeyboardModels.Fonts.globalFont(size: 18))
                .foregroundColor(CandidateViewModels.Colors.primaryTextColor)
                .scaleEffect(1.2)
                .frame(width: 42, height: CandidateViewModels.UI.height)
                .contentShape(Rectangle())
                .offset(y: 7)
        }
        .buttonStyle(ToolShortcutButtonStyle())
        .accessibilityLabel("設定")
        .accessibilityHint("點擊以開啟鍵盤設定")
    }

    /// Vertical separator line between candidate bar sections
    private var candidateBarSeparator: some View {
        Rectangle()
            .fill(CandidateViewModels.Colors.separatorColor)
            .frame(width: 1.0, height: 32)
            .offset(y: 7)
    }

    // MARK: - 輸入模式切換

    /// 輸入模式切換按鈕組
    private var inputModeSwitcher: some View {
        HStack(spacing: 4) {
            inputModeButton(mode: .poj, label: "POJ")
            inputModeButton(mode: .tl, label: "TL")
            inputModeButton(mode: .english, label: "EN")
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
                .font(KeyboardModels.Fonts.globalFont(size: 16))
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
            (suggestion.text, UserFrequencyService.frequency(for: suggestion.text))
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
                VStack(alignment: .center, spacing: 0) {
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

    /// Tool shortcut button press style (scale + opacity feedback)
    struct ToolShortcutButtonStyle: SwiftUI.ButtonStyle {
        func makeBody(configuration: Configuration) -> some View {
            configuration.label
                .opacity(configuration.isPressed ? 0.5 : 1.0)
                .scaleEffect(configuration.isPressed ? 0.85 : 1.0)
                .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
        }
    }
}
