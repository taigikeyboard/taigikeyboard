// Generates App/MenuBarIcon.tiff, the image Info.plist names in
// `tsInputMethodIconFileKey`. Run it by hand after changing anything below:
//
//     swift scripts/make-menubar-icon.swift
//
// Deliberately NOT part of `bundle-app.sh`: the output is a committed artefact,
// so generating it during a bundle would let the build machine's font version
// and rasteriser decide what ships.
//
// The shape follows Apple's own CJK input methods, measured from
// `/System/Library/Input Methods/TCIM.app/Contents/PlugIns/TCIM_Extension.appex`:
// a two-page TIFF (16×16 at 72dpi, 32×32 at 144dpi) whose alpha is a NEGATIVE —
// an opaque rounded square with the glyph knocked out, so that the template
// rendering Info.plist asks for shows the menu bar through the glyph.

import AppKit
import CoreText

// The glyph outline comes from a font, but the font is a build-time input, not a
// runtime one: nothing on a user's machine reads it. Pinned to an exact
// PostScript name so the same source cannot render differently on two machines.
let glyph = "台"
let fontPostScriptName = "PingFangTC-Semibold"
// Fraction of the side left clear on each edge of the glyph, and the corner
// radius as a fraction of the side (3px on Apple's 16px page).
let glyphInsetRatio: CGFloat = 0.13
let cornerRadiusRatio: CGFloat = 3.0 / 16.0
let sidePoints = 16
let scales = [1, 2]

let outputURL = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()   // scripts/
    .deletingLastPathComponent()   // macos/
    .appendingPathComponent("App/MenuBarIcon.tiff")

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("error: \(message)\n".utf8))
    exit(1)
}

/// The glyph as an outline, so it centres on its own ink rather than on a text
/// line box — a CJK glyph's line box carries ascender and descender slack that
/// would push it visibly off centre at 16px.
func glyphOutline() -> CGPath {
    guard let font = NSFont(name: fontPostScriptName, size: 100) else {
        fail("font '\(fontPostScriptName)' is not installed")
    }
    // NSFont(name:) substitutes rather than fails for some names, which would
    // silently change the committed artefact.
    guard font.fontName == fontPostScriptName else {
        fail("font '\(fontPostScriptName)' resolved to '\(font.fontName)'")
    }
    let ctFont = font as CTFont
    var glyphID = CGGlyph()
    var characters = Array(glyph.utf16)
    guard characters.count == 1,
          CTFontGetGlyphsForCharacters(ctFont, &characters, &glyphID, 1),
          let path = CTFontCreatePathForGlyph(ctFont, glyphID, nil)
    else {
        fail("'\(glyph)' has no outline in \(fontPostScriptName)")
    }
    return path
}

func makePage(scale: Int, outline: CGPath) -> NSBitmapImageRep {
    let pixels = sidePoints * scale
    guard let page = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .calibratedRGB, bytesPerRow: 0, bitsPerPixel: 0
    ) else {
        fail("could not allocate a \(pixels)×\(pixels) bitmap")
    }
    // Point size below pixel size is what makes this the @2x page: the TIFF
    // records the ratio as its resolution, and AppKit reads it back as a scale.
    page.size = NSSize(width: sidePoints, height: sidePoints)
    guard let context = NSGraphicsContext(bitmapImageRep: page)?.cgContext else {
        fail("could not draw into the \(pixels)×\(pixels) bitmap")
    }

    // Drawing is in points; the context already carries the scale.
    let side = CGFloat(sidePoints)
    context.setFillColor(NSColor.black.cgColor)
    context.addPath(CGPath(
        roundedRect: CGRect(x: 0, y: 0, width: side, height: side),
        cornerWidth: side * cornerRadiusRatio, cornerHeight: side * cornerRadiusRatio,
        transform: nil
    ))
    context.fillPath()

    // Move the ink box's origin to zero, scale it to the inset square, then
    // centre it — as a CTM change, so the outline itself is scale-independent.
    let ink = outline.boundingBoxOfPath
    let inner = side * (1 - 2 * glyphInsetRatio)
    let glyphScale = min(inner / ink.width, inner / ink.height)
    context.concatenate(CGAffineTransform(
        translationX: (side - ink.width * glyphScale) / 2,
        y: (side - ink.height * glyphScale) / 2
    ).scaledBy(x: glyphScale, y: glyphScale)
        .translatedBy(x: -ink.minX, y: -ink.minY))
    context.setBlendMode(.destinationOut)
    context.addPath(outline)
    context.fillPath()
    return page
}

let outline = glyphOutline()
let pages = scales.map { makePage(scale: $0, outline: outline) }
guard let tiff = NSBitmapImageRep.representationOfImageReps(in: pages, using: .tiff, properties: [:]) else {
    fail("could not encode the pages as a TIFF")
}
do {
    try tiff.write(to: outputURL)
} catch {
    fail("could not write \(outputURL.path): \(error.localizedDescription)")
}

// Read back rather than trust the encoder: a TIFF that lost its resolution tags
// still loads, and only shows up as a menu-bar icon drawn at the wrong size.
guard let written = NSImage(contentsOf: outputURL) else {
    fail("\(outputURL.path) is not readable as an image")
}
guard written.representations.count == scales.count else {
    fail("expected \(scales.count) pages, got \(written.representations.count)")
}
for (scale, page) in zip(scales, written.representations) {
    let pixels = sidePoints * scale
    guard page.pixelsWide == pixels, page.pixelsHigh == pixels,
          page.size == NSSize(width: sidePoints, height: sidePoints)
    else {
        fail("page \(scale)× is \(page.pixelsWide)×\(page.pixelsHigh) at \(page.size), expected \(pixels)×\(pixels) at \(sidePoints)pt")
    }
}
print("✓ \(outputURL.path) — \(scales.map { "\(sidePoints * $0)px" }.joined(separator: " + ")), '\(glyph)' in \(fontPostScriptName)")
