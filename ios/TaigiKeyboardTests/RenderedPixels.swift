// Test support: render a SwiftUI view and read its RGBA pixels (row 0 = top).

import SwiftUI
import XCTest

struct RenderedPixels {
    let width: Int
    let height: Int
    private let rgba: [UInt8]

    @MainActor
    init(_ view: some View, scale: CGFloat) throws {
        let renderer = ImageRenderer(content: view)
        renderer.scale = scale
        let image = try XCTUnwrap(renderer.cgImage)
        width = image.width
        height = image.height
        var rgba = [UInt8](repeating: 0, count: width * height * 4)
        let context = try XCTUnwrap(CGContext(
            data: &rgba, width: width, height: height, bitsPerComponent: 8, bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue,
        ))
        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        self.rgba = rgba
    }

    /// (red, green, blue) at column `x`, row `y`.
    func rgb(x: Int, y: Int) -> (UInt8, UInt8, UInt8) {
        let i = (y * width + x) * 4
        return (rgba[i], rgba[i + 1], rgba[i + 2])
    }
}
