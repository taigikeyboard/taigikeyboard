import OSLog
import SwiftUI

#if DEBUG
private let previewLogger = Logger(
    subsystem: "com.siansiansu.taigikeyboard",
    category: "FlickPreview"
)
#endif

// MARK: - 四向標籤視圖（參考 azooKey DirectionalKeyLabel）

/// 四向標籤視圖
///
/// 使用 SwiftUI 佈局系統（Spacer + frame），而非絕對定位
/// 參考 azooKey KeyLabel.swift 的 DirectionalKeyLabel
struct DirectionalKeyLabel: View {
    let main: String
    let left: String?
    let top: String?
    let right: String?
    let bottom: String?
    let width: CGFloat
    let mainColor: Color
    let hintColor: Color

    var body: some View {
        ZStack {
            // 左右標籤
            HStack {
                optionalLabel(left, isHint: true)
                Spacer(minLength: 0)
                optionalLabel(right, isHint: true)
            }
            .padding(.horizontal, 4)
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            // 上下標籤
            VStack {
                optionalLabel(top, isHint: true)
                Spacer(minLength: 0)
                optionalLabel(bottom, isHint: true)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            // 中心主標籤
            Text(main)
                .font(FlickDesign.mainLabelFont(text: main, width: width))
                .foregroundColor(mainColor)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @ViewBuilder
    private func optionalLabel(_ text: String?, isHint: Bool) -> some View {
        if let text = text {
            Text(text)
                .font(FlickDesign.hintLabelFont(text: text, width: width))
                .foregroundColor(hintColor)
        }
    }
}

// MARK: - 垂直副標籤視圖（參考 azooKey KeyLabel.symbols）

/// 垂直副標籤視圖（主字 + 副字）
///
/// 用於數字、符號按鍵
struct VerticalSubLabel: View {
    let main: String
    let sub: String
    let width: CGFloat
    let mainColor: Color
    let subColor: Color

    var body: some View {
        VStack(spacing: 2) {
            Text(main)
                .font(FlickDesign.mainLabelFont(text: main, width: width))
                .foregroundColor(mainColor)

            if !sub.isEmpty {
                Text(sub)
                    .font(FlickDesign.subLabelFont(text: sub, width: width))
                    .foregroundColor(subColor)
            }
        }
    }
}

// MARK: - Flick 按鍵視圖

/// Flick 按鍵視圖
///
/// 支援：
/// - 點擊：輸入中心字符
/// - 四向滑動：輸入對應方向的變化字符
/// - 長按：輸入長按字符（母音第8聲）
struct FlickKeyView: View {
    let keyDef: FlickKeyDef
    let size: CGSize
    let onInput: (String) -> Void
    let onSystemAction: (FlickSystemKey) -> Void

    @Environment(\.colorScheme) private var colorScheme

    @State private var flickDirection: FlickDirection?
    @State private var isPressed: Bool = false
    @State private var isLongPressed: Bool = false
    @State private var showAllSuggest: Bool = false

    private let flickThreshold: CGFloat = 20
    private let longPressDuration: Double = 0.4  // 參考 azooKey

    var body: some View {
        GeometryReader { _ in
            ZStack {
                keyBackground
                keyContent
            }
            .frame(width: size.width, height: size.height)
            .contentShape(Rectangle())
            .gesture(flickGesture)
            .simultaneousGesture(longPressGesture)
        }
        .frame(width: size.width, height: size.height)
    }

    // MARK: - 背景（參考 azooKey KeyBackground）

    @ViewBuilder
    private var keyBackground: some View {
        let shadow = FlickKeyShadow.adaptive(colorScheme: colorScheme)
        let border = FlickKeyBorder.adaptive(colorScheme: colorScheme)

        RoundedRectangle(cornerRadius: 6)
            .strokeAndFill(
                fillContent: backgroundColor,
                strokeContent: border.color,
                lineWidth: border.width
            )
            .frame(width: size.width, height: size.height)
            .compositingGroup()
            .shadow(color: shadow.color, radius: shadow.radius, x: shadow.x, y: shadow.y)
    }

    private var backgroundColor: Color {
        if isPressed || flickDirection != nil {
            return FlickColors.highlighted
        }

        switch keyDef {
        case .vowel, .consonant, .symbol:
            return FlickColors.normalKey
        case .system(let key):
            switch key {
            case .enter:
                return FlickColors.enterKey
            case .delete:
                return FlickColors.deleteKey
            case .space:
                return FlickColors.spaceKey
            case .globe, .switchToQwerty, .translate,
                 .flickTabTaigi, .flickTabNumber, .flickTabAbc, .flickTabShift:
                return FlickColors.specialKey
            }
        }
    }

    // MARK: - 按鍵內容

    @ViewBuilder
    private var keyContent: some View {
        switch keyDef {
        case .vowel, .consonant, .symbol:
            characterKeyContent
        case .system(let key):
            systemKeyContent(key)
        }
    }

    @ViewBuilder
    private var characterKeyContent: some View {
        ZStack {
            switch keyDef.labelStyle {
            case .directional:
                DirectionalKeyLabel(
                    main: keyDef.displayLabel,
                    left: keyDef.variation(for: .left),
                    top: keyDef.variation(for: .top),
                    right: keyDef.variation(for: .right),
                    bottom: keyDef.variation(for: .bottom),
                    width: size.width,
                    mainColor: FlickColors.normalText,
                    hintColor: FlickColors.hintText
                )

            case .verticalSub:
                VerticalSubLabel(
                    main: keyDef.centerCharacter ?? keyDef.displayLabel,
                    sub: keyDef.allVariations.joined(),
                    width: size.width,
                    mainColor: FlickColors.normalText,
                    subColor: FlickColors.hintText
                )
            }

            // Flick 提示氣泡
            if let suggestType = currentSuggestType {
                FlickSuggestView(
                    keyDef: keyDef,
                    suggestType: suggestType,
                    size: size,
                    spacing: (horizontal: 8, vertical: 8)
                )
            }
        }
    }

    /// 當前提示類型
    private var currentSuggestType: FlickSuggestType? {
        if showAllSuggest {
            return .all
        } else if let direction = flickDirection {
            return .flick(direction)
        }
        return nil
    }

    @ViewBuilder
    private func systemKeyContent(_ key: FlickSystemKey) -> some View {
        let textColor = key == .enter ? FlickColors.enterText : FlickColors.specialText

        Group {
            if let imageName = key.systemImageName {
                Image(systemName: imageName)
                    .font(.system(size: 20, weight: .medium))
            } else {
                Text(key.displayLabel)
                    .font(FlickDesign.mainLabelFont(text: key.displayLabel, width: size.width, weight: .medium))
            }
        }
        .foregroundColor(textColor)
    }

    // MARK: - 手勢處理

    private var flickGesture: some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                isPressed = true
                let translation = value.translation

                if abs(translation.width) > flickThreshold || abs(translation.height) > flickThreshold {
                    // 開始滑動時，關閉全方向氣泡，顯示單方向
                    showAllSuggest = false
                    flickDirection = detectDirection(translation)
                } else {
                    flickDirection = nil
                }
            }
            .onEnded { value in
                defer {
                    isPressed = false
                    flickDirection = nil
                    isLongPressed = false
                    showAllSuggest = false
                }

                let translation = value.translation

                if abs(translation.width) > flickThreshold || abs(translation.height) > flickThreshold {
                    if let direction = detectDirection(translation),
                       let variation = keyDef.variation(for: direction) {
                        handleInput(variation)
                    }
                } else if !isLongPressed {
                    handleTap()
                }
            }
    }

    private var longPressGesture: some Gesture {
        LongPressGesture(minimumDuration: longPressDuration)
            .onEnded { _ in
                isLongPressed = true
                // 長按時顯示全部四方向氣泡
                showAllSuggest = true

                if let longPress = keyDef.longPressCharacter {
                    handleInput(longPress)
                    let generator = UIImpactFeedbackGenerator(style: .medium)
                    generator.impactOccurred()
                }
            }
    }

    private func detectDirection(_ translation: CGSize) -> FlickDirection? {
        let absWidth = abs(translation.width)
        let absHeight = abs(translation.height)

        if absWidth > absHeight {
            return translation.width < 0 ? .left : .right
        } else {
            return translation.height < 0 ? .top : .bottom
        }
    }

    // MARK: - 輸入處理

    private func handleTap() {
        switch keyDef {
        case .vowel(let base, _, _):
            handleInput(base)
        case .consonant(let center, _, _):
            handleInput(center)
        case .symbol(let center, _, _):
            handleInput(center)
        case .system(let key):
            onSystemAction(key)
        }
    }

    private func handleInput(_ text: String) {
        onInput(text)
        let generator = UIImpactFeedbackGenerator(style: .light)
        generator.impactOccurred()
    }
}

// MARK: - Preview

#if DEBUG
struct FlickKeyView_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 12) {
            Text("四向提示模式").font(.caption).foregroundColor(.secondary)
            HStack(spacing: 8) {
                FlickKeyView(
                    keyDef: FlickTaigiLayout.vowelA,
                    size: CGSize(width: 65, height: 55),
                    onInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
                    onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") }
                )
                FlickKeyView(
                    keyDef: FlickTaigiLayout.consonantK,
                    size: CGSize(width: 65, height: 55),
                    onInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
                    onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") }
                )
            }

            Text("垂直副字模式").font(.caption).foregroundColor(.secondary)
            HStack(spacing: 8) {
                FlickKeyView(
                    keyDef: FlickTaigiLayout.number1,
                    size: CGSize(width: 65, height: 55),
                    onInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
                    onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") }
                )
                FlickKeyView(
                    keyDef: FlickTaigiLayout.brackets,
                    size: CGSize(width: 65, height: 55),
                    onInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
                    onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") }
                )
            }

            Text("系統按鍵").font(.caption).foregroundColor(.secondary)
            HStack(spacing: 8) {
                FlickKeyView(
                    keyDef: .system(.enter),
                    size: CGSize(width: 65, height: 55),
                    onInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
                    onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") }
                )
                FlickKeyView(
                    keyDef: .system(.delete),
                    size: CGSize(width: 65, height: 55),
                    onInput: { previewLogger.debug("[PREVIEW] Input: \($0)") },
                    onSystemAction: { previewLogger.debug("[PREVIEW] System: \($0)") }
                )
            }
        }
        .padding()
        .background(FlickColors.keyboardBackground)
        .previewLayout(.sizeThatFits)
    }
}
#endif
