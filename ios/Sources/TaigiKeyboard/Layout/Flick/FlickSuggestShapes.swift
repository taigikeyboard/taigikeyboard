import SwiftUI

// MARK: - 五邊形 Shape 定義（參考 azooKey）

/// 向上指向的五邊形（用於上方提示氣泡）
struct RoundedPentagonTop: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let points = [
            CGPoint(x: rect.width / 2, y: rect.height),      // 底部中央（尖端）
            CGPoint(x: rect.width, y: 2 * rect.height / 3),  // 右下
            CGPoint(x: rect.width, y: 0),                     // 右上
            CGPoint(x: 0, y: 0),                              // 左上
            CGPoint(x: 0, y: 2 * rect.height / 3),            // 左下
        ]
        path.addRoundedPentagon(using: points)
        return path
    }
}

/// 向下指向的五邊形（用於下方提示氣泡）
struct RoundedPentagonBottom: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let points = [
            CGPoint(x: rect.width / 2, y: 0),         // 頂部中央（尖端）
            CGPoint(x: 0, y: rect.height / 3),        // 左上
            CGPoint(x: 0, y: rect.height),            // 左下
            CGPoint(x: rect.width, y: rect.height),   // 右下
            CGPoint(x: rect.width, y: rect.height / 3), // 右上
        ]
        path.addRoundedPentagon(using: points)
        return path
    }
}

/// 向左指向的五邊形（用於左方提示氣泡）
struct RoundedPentagonLeft: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let points = [
            CGPoint(x: 0, y: 0),                        // 左上
            CGPoint(x: rect.width * 2 / 3, y: 0),       // 右上
            CGPoint(x: rect.width, y: rect.height / 2), // 右中央（尖端）
            CGPoint(x: rect.width * 2 / 3, y: rect.height), // 右下
            CGPoint(x: 0, y: rect.height),              // 左下
        ]
        path.addRoundedPentagon(using: points)
        return path
    }
}

/// 向右指向的五邊形（用於右方提示氣泡）
struct RoundedPentagonRight: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let points = [
            CGPoint(x: rect.width / 3, y: 0),         // 左上
            CGPoint(x: 0, y: rect.height / 2),        // 左中央（尖端）
            CGPoint(x: rect.width / 3, y: rect.height), // 左下
            CGPoint(x: rect.width, y: rect.height),    // 右下
            CGPoint(x: rect.width, y: 0),              // 右上
        ]
        path.addRoundedPentagon(using: points)
        return path
    }
}

// MARK: - Path 擴展：圓角五邊形繪製

extension Path {
    /// 繪製圓角五邊形
    /// - Parameters:
    ///   - points: 五個頂點座標
    ///   - cornerRadius: 圓角半徑（預設 5）
    mutating func addRoundedPentagon(using points: [CGPoint], cornerRadius: CGFloat = 5) {
        guard points.count == 5 else { return }

        for i in 0..<5 {
            let currentPoint = points[i]
            let nextPoint = points[(i + 1) % 5]
            let prevPoint = i == 0 ? points.last! : points[i - 1]

            // 計算前後方向的單位向量
            let directionFromPrev = CGVector(
                dx: currentPoint.x - prevPoint.x,
                dy: currentPoint.y - prevPoint.y
            ).normalized
            let directionToNext = CGVector(
                dx: nextPoint.x - currentPoint.x,
                dy: nextPoint.y - currentPoint.y
            ).normalized

            // 根據圓角半徑計算偏移量
            let offsetFromCurrent1 = CGVector(
                dx: directionFromPrev.dx * cornerRadius,
                dy: directionFromPrev.dy * cornerRadius
            )
            let offsetFromCurrent2 = CGVector(
                dx: directionToNext.dx * cornerRadius,
                dy: directionToNext.dy * cornerRadius
            )

            // 繪製直線段和圓角
            if i == 0 {
                move(to: CGPoint(
                    x: currentPoint.x - offsetFromCurrent1.dx,
                    y: currentPoint.y - offsetFromCurrent1.dy
                ))
            } else {
                addLine(to: CGPoint(
                    x: currentPoint.x - offsetFromCurrent1.dx,
                    y: currentPoint.y - offsetFromCurrent1.dy
                ))
            }

            // 使用二次貝茲曲線繪製圓角
            addQuadCurve(
                to: CGPoint(
                    x: currentPoint.x + offsetFromCurrent2.dx,
                    y: currentPoint.y + offsetFromCurrent2.dy
                ),
                control: currentPoint
            )
        }

        closeSubpath()
    }
}

// MARK: - CGVector 擴展

extension CGVector {
    /// 向量長度
    var length: CGFloat {
        sqrt(dx * dx + dy * dy)
    }

    /// 單位化向量
    var normalized: CGVector {
        let len = length
        guard len > 0 else { return .zero }
        return CGVector(dx: dx / len, dy: dy / len)
    }
}

// MARK: - Shape 擴展：同時繪製填充和邊框

extension Shape {
    /// 同時繪製填充和邊框（參考 azooKey strokeAndFill）
    /// - Parameters:
    ///   - fillContent: 填充顏色
    ///   - strokeContent: 邊框顏色
    ///   - lineWidth: 邊框寬度
    func strokeAndFill<F: ShapeStyle, S: ShapeStyle>(
        fillContent: F,
        strokeContent: S,
        lineWidth: CGFloat
    ) -> some View {
        self.fill(fillContent)
            .overlay(
                self.stroke(strokeContent, lineWidth: lineWidth)
            )
    }
}

// MARK: - Preview

#if DEBUG
struct FlickSuggestShapes_Previews: PreviewProvider {
    static var previews: some View {
        VStack(spacing: 20) {
            Text("五邊形 Shape 預覽").font(.headline)

            HStack(spacing: 20) {
                VStack {
                    RoundedPentagonTop()
                        .strokeAndFill(
                            fillContent: Color.white,
                            strokeContent: Color.gray,
                            lineWidth: 1
                        )
                        .frame(width: 60, height: 80)
                        .shadow(color: .gray, radius: 3, y: 2)
                    Text("Top")
                }

                VStack {
                    RoundedPentagonBottom()
                        .strokeAndFill(
                            fillContent: Color.white,
                            strokeContent: Color.gray,
                            lineWidth: 1
                        )
                        .frame(width: 60, height: 80)
                        .shadow(color: .gray, radius: 3, y: 2)
                    Text("Bottom")
                }
            }

            HStack(spacing: 20) {
                VStack {
                    RoundedPentagonLeft()
                        .strokeAndFill(
                            fillContent: Color.white,
                            strokeContent: Color.gray,
                            lineWidth: 1
                        )
                        .frame(width: 80, height: 60)
                        .shadow(color: .gray, radius: 3, y: 2)
                    Text("Left")
                }

                VStack {
                    RoundedPentagonRight()
                        .strokeAndFill(
                            fillContent: Color.white,
                            strokeContent: Color.gray,
                            lineWidth: 1
                        )
                        .frame(width: 80, height: 60)
                        .shadow(color: .gray, radius: 3, y: 2)
                    Text("Right")
                }
            }
        }
        .padding()
        .background(Color(.systemGray5))
        .previewLayout(.sizeThatFits)
    }
}
#endif
