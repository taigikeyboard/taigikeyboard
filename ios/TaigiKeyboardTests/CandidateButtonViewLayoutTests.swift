// Pins that the first-candidate highlight stays inside the candidate bar with a gap at
// the top and bottom: it used to extend 4 pt past the text and shift 5 pt down, so the
// bar's clip cut its bottom edge flat.

import KeyboardKit
import SwiftUI
@testable import TaigiKeyboard
import XCTest

@MainActor
final class CandidateButtonViewLayoutTests: XCTestCase {
    /// Rows (top → bottom) of the rendered bar that contain the pure-red highlight.
    private func highlightRows(textSizeScale: CGFloat) throws -> (rows: [Int], height: Int, theme: CandidateTheme) {
        let standard = CandidateTheme.resolved(candidateTextSizeScale: textSizeScale, colorSettings: .default)
        let theme = CandidateTheme(
            height: standard.height,
            primaryFontSize: standard.primaryFontSize,
            secondaryFontSize: standard.secondaryFontSize,
            primaryTextColor: .black,
            secondaryTextColor: .gray,
            surface: nil,
            firstCandidateHighlightColor: Color(red: 1, green: 0, blue: 0),
            pressedCandidateColor: nil,
            gridLineColor: nil,
            emojiChrome: nil,
        )
        let bar = CandidateButtonView(
            suggestion: AutocompleteSuggestion(text: "tâi-gí", title: "tâi-gí", subtitle: "台語"),
            isHanjiFirst: true,
            candidateDisplayMode: .sideBySide,
            contentHasSubtitles: true,
            isTPSLayout: false,
            isSelected: true,
            isFirstCandidate: true,
            onTap: { _ in },
        )
        .frame(width: 160, height: theme.height)
        .clipped()
        .background(Color.white)
        .candidateTheme(theme)

        let renderer = ImageRenderer(content: bar)
        renderer.scale = 1
        let image = try XCTUnwrap(renderer.cgImage)
        let width = image.width, height = image.height
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        let context = try XCTUnwrap(CGContext(
            data: &pixels, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue,
        ))
        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        let rows = (0 ..< height).filter { y in
            (0 ..< width).contains { x in
                let i = (y * width + x) * 4
                return pixels[i] > 200 && pixels[i + 1] < 60 && pixels[i + 2] < 60
            }
        }
        return (rows, height, theme)
    }

    func testHighlight_staysInsideBarWithGap() throws {
        for scale in [CGFloat(0.8), 1, 1.3] {
            let (rows, height, theme) = try highlightRows(textSizeScale: scale)
            let first = try XCTUnwrap(rows.first, "no highlight rendered at scale \(scale)")
            let last = try XCTUnwrap(rows.last)
            let gap = Int(CandidateButtonView.highlightInset(primaryFontSize: theme.primaryFontSize).rounded(.down))
            XCTAssertGreaterThanOrEqual(first, gap, "highlight touches the bar top at scale \(scale)")
            XCTAssertLessThanOrEqual(last, height - 1 - gap, "highlight clipped at the bar bottom at scale \(scale)")
        }
    }
}
