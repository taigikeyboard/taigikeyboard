import KeyboardKit
import SwiftUI

/// 台語 Flick 鍵盤主視圖
///
/// 整合 Flick 按鍵網格與候選詞工具列的完整鍵盤視圖
struct TaigiFlickKeyboardView: View {
    let services: Keyboard.Services

    @ObservedObject var autocompleteContext: AutocompleteContext
    @ObservedObject var keyboardContext: KeyboardContext
    @ObservedObject var composingManager: ComposingManager

    let onSuggestionTap: (Autocomplete.Suggestion) -> Void
    let onTranslateToggle: () -> Void

    /// 文字輸入回調
    let onTextInput: (String) -> Void
    /// 刪除回調
    let onDelete: () -> Void
    /// 空白回調
    let onSpace: () -> Void
    /// Enter 回調
    let onReturn: () -> Void
    /// 切換輸入法回調
    let onGlobe: () -> Void
    /// 切換到 QWERTY 回調
    let onSwitchToQwerty: () -> Void

    @StateObject private var expandState = CandidateExpandState()
    @State private var currentInputMode: InputMode = SharedSettings.shared.inputMode

    /// Flick 內部 Tab 狀態
    @State private var currentFlickTab: FlickTab = .taigi
    /// ABC 鍵盤大小寫狀態
    @State private var isShiftActive: Bool = false

    var body: some View {
        let suggestions = SuggestionCaseTransformer.transform(
            autocompleteContext.suggestions,
            composingText: composingManager.composingText,
            keyboardCase: keyboardContext.keyboardCase,
            inputMode: SharedSettings.shared.inputMode
        )
        let frequentWords = CandidateView.getSharedFrequentWords(in: suggestions)
        let isTranslateSwapped = keyboardContext.isTranslateSwapped
        let selectedCandidateIndex = composingManager.selectedCandidateIndex
        let candidateStyle = CandidateView.Style.adaptive(for: keyboardContext)

        VStack(spacing: 0) {
            // 候選詞工具列
            CandidateView(
                suggestions: suggestions,
                frequentWords: frequentWords,
                selectedCandidateIndex: selectedCandidateIndex,
                onSuggestionTap: onSuggestionTap,
                isTranslateSwapped: isTranslateSwapped,
                onTranslateToggle: onTranslateToggle,
                onSettingsTap: { [unowned services] in
                    services.actionHandler.handle(.settings)
                },
                currentInputMode: currentInputMode,
                onInputModeChange: { newMode in
                    currentInputMode = newMode
                    SharedSettings.shared.inputMode = newMode
                },
                englishAutocompleteView: nil
            )
            .environmentObject(expandState)
            .candidateViewStyle(candidateStyle)

            // Flick 鍵盤網格
            flickKeyboardContent
        }
        .overlay(
            ExpandedCandidateOverlay(
                suggestions: suggestions,
                frequentWords: frequentWords,
                selectedCandidateIndex: selectedCandidateIndex,
                onSuggestionTap: onSuggestionTap,
                isTranslateSwapped: isTranslateSwapped,
                onTranslateToggle: onTranslateToggle,
                onCollapse: {
                    expandState.collapse()
                },
                isExpanded: expandState.isExpanded
            )
            .candidateViewStyle(candidateStyle)
            .offset(y: 2),
            alignment: .topLeading
        )
        .background(
            keyboardContext.isLiquidGlassEnabled
                ? Color.white.opacity(0.001)
                : FlickColors.keyboardBackground
        )
    }

    // MARK: - Flick 鍵盤內容

    @ViewBuilder
    private var flickKeyboardContent: some View {
        let layout = selectFlickLayout()

        FlickKeyboardGrid(
            layout: layout,
            keyboardContext: keyboardContext,
            onTextInput: onTextInput,
            onSystemAction: handleSystemAction
        )
    }

    /// 根據當前 Tab 和輸入模式選擇 Flick 佈局
    private func selectFlickLayout() -> FlickLayout {
        switch currentFlickTab {
        case .taigi:
            // 台語主鍵盤：根據 POJ/TL 模式選擇
            switch SharedSettings.shared.inputMode {
            case .poj:
                return FlickTaigiLayout.taigiTonePOJ
            default:
                return FlickTaigiLayout.taigiTone
            }
        case .number:
            return FlickTaigiLayout.flickNumber
        case .abc:
            // 根據 shift 狀態選擇大小寫佈局
            return isShiftActive
                ? FlickTaigiLayout.flickAbcUpper
                : FlickTaigiLayout.flickAbc
        }
    }

    /// 處理系統按鍵動作
    private func handleSystemAction(_ key: FlickSystemKey) {
        switch key {
        case .delete:
            onDelete()
        case .space:
            onSpace()
        case .enter:
            onReturn()
        case .globe:
            onGlobe()
        case .switchToQwerty:
            onSwitchToQwerty()
        case .translate:
            onTranslateToggle()

        // Flick 內部 Tab 切換
        case .flickTabTaigi:
            currentFlickTab = .taigi
        case .flickTabNumber:
            currentFlickTab = .number
        case .flickTabAbc:
            currentFlickTab = .abc
        case .flickTabShift:
            isShiftActive.toggle()
        }
    }
}

// MARK: - Flick 鍵盤網格（參考 azooKey UnifiedKeysView：使用 ZStack + position）

private struct FlickKeyboardGrid: View {
    let layout: FlickLayout
    let keyboardContext: KeyboardContext

    let onTextInput: (String) -> Void
    let onSystemAction: (FlickSystemKey) -> Void

    private var isPad: Bool {
        keyboardContext.deviceType == .pad
    }

    var body: some View {
        GeometryReader { geometry in
            let screenWidth = geometry.size.width
            let vSpacing = FlickDesign.verticalSpacing(screenWidth: screenWidth)
            let keySize = calculateKeySize(screenWidth: screenWidth)
            let hSpacing = FlickDesign.horizontalSpacing(
                screenWidth: screenWidth,
                columnCount: CGFloat(layout.columnCount),
                keyWidth: keySize.width
            )
            let keysWidth = keysWidth(keySize: keySize, hSpacing: hSpacing)
            let keysHeight = keysHeight(keySize: keySize, vSpacing: vSpacing)

            // 使用 ZStack + position 精確定位（參考 azooKey）
            ZStack {
                ForEach(allKeyPositions(), id: \.self) { position in
                    if let keyDef = findKeyDef(at: position) {
                        let info = keyData(
                            position: position,
                            keySize: keySize,
                            hSpacing: hSpacing,
                            vSpacing: vSpacing
                        )

                        FlickKeyView(
                            keyDef: keyDef,
                            size: info.size,
                            onInput: onTextInput,
                            onSystemAction: onSystemAction
                        )
                        .frame(width: info.contentSize.width, height: info.contentSize.height)
                        .contentShape(Rectangle())
                        .position(x: info.position.x, y: info.position.y)
                    }
                }
            }
            .frame(width: keysWidth, height: keysHeight)
            .frame(maxWidth: .infinity) // 置中
        }
        .frame(height: keyboardHeight)
    }

    /// 計算按鍵的位置和尺寸（參考 azooKey keyData）
    private func keyData(
        position: FlickKeyPosition,
        keySize: CGSize,
        hSpacing: CGFloat,
        vSpacing: CGFloat
    ) -> (position: CGPoint, size: CGSize, contentSize: CGSize) {
        // 計算實際按鍵尺寸（考慮跨行/跨列）
        let width = keySize.width * CGFloat(position.width) + hSpacing * CGFloat(position.width - 1)
        let height = keySize.height * CGFloat(position.height) + vSpacing * CGFloat(position.height - 1)

        // 計算中心點位置
        let dx = width * 0.5 + keySize.width * CGFloat(position.column) + hSpacing * CGFloat(position.column)
        let dy = height * 0.5 + keySize.height * CGFloat(position.row) + vSpacing * CGFloat(position.row)

        // 內容尺寸（包含間距，用於點擊區域）
        let contentWidth = width + hSpacing
        let contentHeight = height + vSpacing

        return (CGPoint(x: dx, y: dy), CGSize(width: width, height: height), CGSize(width: contentWidth, height: contentHeight))
    }

    /// 取得所有按鍵位置
    private func allKeyPositions() -> [FlickKeyPosition] {
        layout.keys.keys.sorted { ($0.row, $0.column) < ($1.row, $1.column) }
    }

    /// 鍵盤總寬度
    private func keysWidth(keySize: CGSize, hSpacing: CGFloat) -> CGFloat {
        keySize.width * CGFloat(layout.columnCount) + hSpacing * CGFloat(layout.columnCount - 1)
    }

    /// 鍵盤總高度
    private func keysHeight(keySize: CGSize, vSpacing: CGFloat) -> CGFloat {
        keySize.height * CGFloat(layout.rowCount) + vSpacing * CGFloat(layout.rowCount - 1)
    }

    /// 鍵盤高度（動態計算）
    private var keyboardHeight: CGFloat {
        let screenWidth = UIScreen.main.bounds.width
        return FlickDesign.keysAreaHeight(screenWidth: screenWidth, isPad: isPad)
    }

    /// 計算單一按鍵大小（參考 azooKey keyViewSize）
    private func calculateKeySize(screenWidth: CGFloat) -> CGSize {
        FlickDesign.keySize(
            screenWidth: screenWidth,
            isPad: isPad,
            rowCount: layout.rowCount,
            columnCount: layout.columnCount
        )
    }

    /// 查找指定位置的按鍵定義
    private func findKeyDef(at position: FlickKeyPosition) -> FlickKeyDef? {
        for (pos, keyDef) in layout.keys {
            if pos.row == position.row && pos.column == position.column {
                return keyDef
            }
        }
        return nil
    }
}

// MARK: - FlickKeyPosition Comparable

extension FlickKeyPosition: Comparable {
    static func < (lhs: FlickKeyPosition, rhs: FlickKeyPosition) -> Bool {
        if lhs.row != rhs.row {
            return lhs.row < rhs.row
        }
        return lhs.column < rhs.column
    }
}
