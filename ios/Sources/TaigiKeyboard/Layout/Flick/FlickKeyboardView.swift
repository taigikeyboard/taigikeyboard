import KeyboardKit
import OSLog
import SwiftUI

#if DEBUG
private let previewLogger = Logger(
    subsystem: "com.siansiansu.taigikeyboard",
    category: "FlickPreview"
)
#endif

/// Flick 鍵盤視圖
///
/// 完整的 Flick 鍵盤，包含：
/// - 候選詞工具列
/// - Flick 按鍵網格
/// - 系統功能整合
struct FlickKeyboardView: View {
    let layout: FlickLayout
    let keyboardContext: KeyboardContext

    /// 文字輸入回調
    let onTextInput: (String) -> Void

    /// 系統動作回調
    let onSystemAction: (FlickSystemKey) -> Void

    /// 候選詞視圖
    let toolbarView: AnyView?

    private var isPad: Bool {
        keyboardContext.deviceType == .pad
    }

    var body: some View {
        VStack(spacing: 0) {
            // 候選詞工具列
            if let toolbar = toolbarView {
                toolbar
            }

            // Flick 按鍵網格
            keyboardGrid
        }
        .background(FlickColors.keyboardBackground)
    }

    // MARK: - 按鍵網格（參考 azooKey 動態設計）

    @ViewBuilder
    private var keyboardGrid: some View {
        GeometryReader { geometry in
            let screenWidth = geometry.size.width
            let vSpacing = FlickDesign.verticalSpacing(screenWidth: screenWidth)
            let keySize = calculateKeySize(screenWidth: screenWidth)
            let hSpacing = FlickDesign.horizontalSpacing(
                screenWidth: screenWidth,
                columnCount: CGFloat(layout.columnCount),
                keyWidth: keySize.width
            )

            VStack(spacing: vSpacing) {
                ForEach(0..<layout.rowCount, id: \.self) { row in
                    HStack(spacing: hSpacing) {
                        ForEach(0..<layout.columnCount, id: \.self) { column in
                            keyViewAt(
                                row: row,
                                column: column,
                                keySize: keySize,
                                hSpacing: hSpacing,
                                vSpacing: vSpacing
                            )
                        }
                    }
                }
            }
        }
        .frame(height: keyboardHeight)
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

    /// 取得指定位置的按鍵視圖
    @ViewBuilder
    private func keyViewAt(
        row: Int,
        column: Int,
        keySize: CGSize,
        hSpacing: CGFloat,
        vSpacing: CGFloat
    ) -> some View {
        let position = FlickKeyPosition(row: row, column: column)

        if let keyDef = layout.keys[position] {
            let actualSize = adjustedKeySize(
                for: position,
                baseSize: keySize,
                hSpacing: hSpacing,
                vSpacing: vSpacing
            )

            FlickKeyView(
                keyDef: keyDef,
                size: actualSize,
                onInput: onTextInput,
                onSystemAction: onSystemAction
            )
        } else if !isOccupiedBySpanningKey(row: row, column: column) {
            Color.clear
                .frame(width: keySize.width, height: keySize.height)
        }
    }

    /// 調整跨行/跨列按鍵的大小
    private func adjustedKeySize(
        for position: FlickKeyPosition,
        baseSize: CGSize,
        hSpacing: CGFloat,
        vSpacing: CGFloat
    ) -> CGSize {
        for (pos, _) in layout.keys {
            if pos.row == position.row && pos.column == position.column {
                let width = baseSize.width * CGFloat(pos.width) + hSpacing * CGFloat(pos.width - 1)
                let height = baseSize.height * CGFloat(pos.height) + vSpacing * CGFloat(pos.height - 1)
                return CGSize(width: width, height: height)
            }
        }
        return baseSize
    }

    /// 檢查該位置是否被其他按鍵的跨行/跨列佔用
    private func isOccupiedBySpanningKey(row: Int, column: Int) -> Bool {
        for (pos, _) in layout.keys {
            if pos.width > 1 || pos.height > 1 {
                let rowRange = pos.row..<(pos.row + pos.height)
                let colRange = pos.column..<(pos.column + pos.width)

                if rowRange.contains(row) && colRange.contains(column) {
                    if row != pos.row || column != pos.column {
                        return true
                    }
                }
            }
        }
        return false
    }
}

// MARK: - Flick 鍵盤容器

/// Flick 鍵盤容器（整合 KeyboardKit 的 Toolbar）
///
/// 注意：此容器已被 TaigiFlickKeyboardView 取代，保留供舊程式碼相容
struct FlickKeyboardContainer: View {
    @ObservedObject var keyboardContext: KeyboardContext
    @ObservedObject var autocompleteContext: AutocompleteContext

    let onTextInput: (String) -> Void
    let onDelete: () -> Void
    let onReturn: () -> Void
    let onSpace: () -> Void
    let onGlobe: () -> Void
    let onSwitchToQwerty: () -> Void

    /// Flick 內部 Tab 狀態
    @State private var currentFlickTab: FlickTab = .taigi
    @State private var isShiftActive: Bool = false

    var body: some View {
        let layout = selectLayout()

        FlickKeyboardView(
            layout: layout,
            keyboardContext: keyboardContext,
            onTextInput: onTextInput,
            onSystemAction: handleSystemAction,
            toolbarView: nil
        )
    }

    /// 根據當前 Tab 和輸入模式選擇佈局
    private func selectLayout() -> FlickLayout {
        switch currentFlickTab {
        case .taigi:
            let inputMode = SharedSettings.shared.inputMode
            switch inputMode {
            case .poj:
                return FlickTaigiLayout.taigiTonePOJ
            default:
                return FlickTaigiLayout.taigiTone
            }
        case .number:
            return FlickTaigiLayout.flickNumber
        case .abc:
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
            break
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

// MARK: - Preview

#if DEBUG
struct FlickKeyboardView_Previews: PreviewProvider {
    static var previews: some View {
        FlickKeyboardView(
            layout: FlickTaigiLayout.taigiTone,
            keyboardContext: .preview,
            onTextInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
            onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") },
            toolbarView: AnyView(
                Text("候選詞區域")
                    .frame(height: 44)
                    .frame(maxWidth: .infinity)
                    .background(Color(.systemGray5))
            )
        )
        .previewLayout(.sizeThatFits)
    }
}
#endif
