import SwiftUI

// MARK: - Flick 提示氣泡類型

/// Flick 提示氣泡顯示類型
enum FlickSuggestType: Equatable {
    /// 長按時：顯示全部四個方向
    case all
    /// 滑動時：只高亮特定方向
    case flick(FlickDirection)
}

// MARK: - Flick 提示氣泡視圖（參考 azooKey UnifiedFlickSuggestView）

/// Flick 提示氣泡視圖
///
/// 支援兩種模式：
/// - `.all`：長按時顯示四個方向的圓角矩形
/// - `.flick(direction)`：滑動時顯示五邊形，高亮選中方向
struct FlickSuggestView: View {
    let keyDef: FlickKeyDef
    let suggestType: FlickSuggestType
    let size: CGSize
    let spacing: (horizontal: CGFloat, vertical: CGFloat)

    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        switch suggestType {
        case .all:
            allDirectionsSuggest
        case .flick(let targetDirection):
            flickDirectionSuggest(targetDirection: targetDirection)
        }
    }

    // MARK: - 長按模式：全部四方向（圓角矩形）

    @ViewBuilder
    private var allDirectionsSuggest: some View {
        let cornerRadius: CGFloat = 5
        let border = FlickKeyBorder.adaptive(colorScheme: colorScheme)

        VStack(spacing: 0) {
            // 上方
            allSuggestRectangle(direction: .top, cornerRadius: cornerRadius, border: border)
                .offset(y: cornerRadius)
                .zIndex(1)

            HStack(spacing: 0) {
                // 左方
                allSuggestRectangle(direction: .left, cornerRadius: cornerRadius, border: border)
                    .offset(x: cornerRadius)
                    .zIndex(1)

                // 中央（主按鍵）
                Rectangle()
                    .strokeAndFill(
                        fillContent: Color.blue,
                        strokeContent: Color.gray,
                        lineWidth: 0.5
                    )
                    .frame(
                        width: size.width + spacing.horizontal,
                        height: size.height + spacing.vertical
                    )
                    .zIndex(2)
                    .overlay {
                        centerLabel
                    }

                // 右方
                allSuggestRectangle(direction: .right, cornerRadius: cornerRadius, border: border)
                    .offset(x: -cornerRadius)
                    .zIndex(1)
            }
            .zIndex(2)
            .frame(width: size.width + spacing.horizontal, height: size.height + spacing.vertical)

            // 下方
            allSuggestRectangle(direction: .bottom, cornerRadius: cornerRadius, border: border)
                .offset(y: -cornerRadius)
                .zIndex(1)
        }
        .compositingGroup()
        .shadow(color: allSuggestShadowColor, radius: 3)
        .frame(width: size.width, height: size.height)
        .allowsHitTesting(false)
    }

    @ViewBuilder
    private func allSuggestRectangle(
        direction: FlickDirection,
        cornerRadius: CGFloat,
        border: FlickKeyBorder
    ) -> some View {
        let sizeDiff: CGFloat = 2
        let widthDiff: CGFloat = switch direction {
        case .top, .bottom: spacing.horizontal
        case .left, .right: spacing.horizontal / 2 + cornerRadius + sizeDiff
        }
        let heightDiff: CGFloat = switch direction {
        case .top, .bottom: spacing.vertical / 2 + cornerRadius + sizeDiff
        case .left, .right: spacing.vertical
        }

        let isHidden = keyDef.variation(for: direction) == nil

        RoundedRectangle(cornerRadius: cornerRadius)
            .strokeAndFill(
                fillContent: pointedColor,
                strokeContent: border.color,
                lineWidth: border.width
            )
            .frame(width: size.width + widthDiff, height: size.height + heightDiff)
            .overlay {
                let offsetX: CGFloat = switch direction {
                case .bottom, .top: 0
                case .left: -cornerRadius + sizeDiff / 2
                case .right: cornerRadius - sizeDiff / 2
                }
                let offsetY: CGFloat = switch direction {
                case .left, .right: 0
                case .top: -cornerRadius + sizeDiff / 2
                case .bottom: cornerRadius - sizeDiff / 2
                }
                directionLabel(for: direction)
                    .offset(x: offsetX, y: offsetY)
            }
            .opacity(isHidden ? 0 : 1)
    }

    // MARK: - 滑動模式：五邊形氣泡

    @ViewBuilder
    private func flickDirectionSuggest(targetDirection: FlickDirection) -> some View {
        VStack(spacing: spacing.vertical) {
            // 上方
            pointedSuggestViewIfNecessary(direction: .top, targetDirection: targetDirection)
                .offset(y: size.height / 2)
                .zIndex(1)

            HStack(spacing: spacing.horizontal) {
                // 左方
                pointedSuggestViewIfNecessary(direction: .left, targetDirection: targetDirection)
                    .offset(x: size.width / 2)
                    .zIndex(1)

                // 中央按鍵
                let border = FlickKeyBorder.adaptive(colorScheme: colorScheme)
                RoundedRectangle(cornerRadius: 5.0)
                    .strokeAndFill(
                        fillContent: FlickColors.specialKey,
                        strokeContent: border.color,
                        lineWidth: border.width
                    )
                    .frame(width: size.width, height: size.height)
                    .zIndex(0)

                // 右方
                pointedSuggestViewIfNecessary(direction: .right, targetDirection: targetDirection)
                    .offset(x: -size.width / 2)
                    .zIndex(1)
            }
            .frame(width: size.width, height: size.height)

            // 下方
            pointedSuggestViewIfNecessary(direction: .bottom, targetDirection: targetDirection)
                .offset(y: -size.height / 2)
                .zIndex(1)
        }
        .frame(width: size.width, height: size.height)
        .allowsHitTesting(false)
    }

    @ViewBuilder
    private func pointedSuggestViewIfNecessary(
        direction: FlickDirection,
        targetDirection: FlickDirection
    ) -> some View {
        if targetDirection == direction {
            pentagonSuggestView(direction: direction, isHidden: false, isPointed: true)
        } else {
            pentagonSuggestView(direction: direction, isHidden: true, isPointed: false)
        }
    }

    @ViewBuilder
    private func pentagonSuggestView(
        direction: FlickDirection,
        isHidden: Bool,
        isPointed: Bool
    ) -> some View {
        let color = isPointed ? pointedColor : unpointedColor
        let border = FlickKeyBorder.adaptive(colorScheme: colorScheme)

        switch direction {
        case .top:
            pentagonContent(
                shape: RoundedPentagonTop(),
                color: color,
                border: border,
                sizeTimes: (1.2, 1.5),
                paddings: (0, 0, 0.3, 0),
                direction: direction,
                isHidden: isHidden
            )
        case .left:
            pentagonContent(
                shape: RoundedPentagonLeft(),
                color: color,
                border: border,
                sizeTimes: (1.5, 1.2),
                paddings: (0, 0, 0, 0.3),
                direction: direction,
                isHidden: isHidden
            )
        case .right:
            pentagonContent(
                shape: RoundedPentagonRight(),
                color: color,
                border: border,
                sizeTimes: (1.5, 1.2),
                paddings: (0, 0.3, 0, 0),
                direction: direction,
                isHidden: isHidden
            )
        case .bottom:
            pentagonContent(
                shape: RoundedPentagonBottom(),
                color: color,
                border: border,
                sizeTimes: (1.2, 1.5),
                paddings: (0.3, 0, 0, 0),
                direction: direction,
                isHidden: isHidden
            )
        }
    }

    @ViewBuilder
    private func pentagonContent<S: Shape>(
        shape: S,
        color: Color,
        border: FlickKeyBorder,
        sizeTimes: (width: CGFloat, height: CGFloat),
        paddings: (top: CGFloat, leading: CGFloat, bottom: CGFloat, trailing: CGFloat),
        direction: FlickDirection,
        isHidden: Bool
    ) -> some View {
        shape.strokeAndFill(
            fillContent: color,
            strokeContent: border.color,
            lineWidth: border.width
        )
        .frame(width: size.width * sizeTimes.width, height: size.height * sizeTimes.height)
        .shadow(color: suggestShadowColor, radius: 10, y: 5)
        .overlay {
            directionLabel(for: direction, textSize: .large)
                .padding(EdgeInsets(
                    top: size.height * paddings.top,
                    leading: size.width * paddings.leading,
                    bottom: size.height * paddings.bottom,
                    trailing: size.width * paddings.trailing
                ))
        }
        .allowsHitTesting(false)
        .opacity(isHidden ? 0 : 1)
    }

    // MARK: - 標籤

    @ViewBuilder
    private var centerLabel: some View {
        if let text = keyDef.centerCharacter {
            Text(text)
                .font(FlickDesign.mainLabelFont(text: text, width: size.width))
                .foregroundColor(.white)
        }
    }

    @ViewBuilder
    private func directionLabel(
        for direction: FlickDirection,
        textSize: FlickDesign.FontSizeStrategy = .large
    ) -> some View {
        if let text = keyDef.variation(for: direction) {
            Text(text)
                .font(FlickDesign.keyLabelFont(text: text, width: size.width, strategy: textSize))
                .foregroundColor(FlickColors.normalText)
        }
    }

    // MARK: - 顏色（使用 FlickColors 統一定義）

    /// 選中方向的顏色
    private var pointedColor: Color {
        FlickColors.suggestPointed
    }

    /// 未選中方向的顏色
    private var unpointedColor: Color {
        FlickColors.suggestUnpointed
    }

    /// 五邊形氣泡陰影色
    private var suggestShadowColor: Color {
        FlickColors.suggestShadow
    }

    /// 長按全方向陰影色
    private var allSuggestShadowColor: Color {
        FlickColors.suggestShadow
    }
}

// MARK: - FlickDesign 擴展

extension FlickDesign {
    /// 取得指定策略的字體
    static func keyLabelFont(text: String, width: CGFloat, strategy: FontSizeStrategy) -> Font {
        let size = keyLabelFontSize(text: text, width: width, strategy: strategy)
        return .system(size: size, weight: .regular)
    }
}

// MARK: - Preview

#if DEBUG
struct FlickSuggestView_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 40) {
            Text("長按模式 (.all)").font(.headline)
            FlickSuggestView(
                keyDef: FlickTaigiLayout.vowelA,
                suggestType: .all,
                size: CGSize(width: 55, height: 45),
                spacing: (horizontal: 8, vertical: 8)
            )
            .frame(width: 200, height: 200)

            Text("滑動模式 (.flick(.top))").font(.headline)
            FlickSuggestView(
                keyDef: FlickTaigiLayout.vowelA,
                suggestType: .flick(.top),
                size: CGSize(width: 55, height: 45),
                spacing: (horizontal: 8, vertical: 8)
            )
            .frame(width: 200, height: 200)
        }
        .padding()
        .background(FlickColors.keyboardBackground)
        .previewLayout(.sizeThatFits)
    }
}
#endif
