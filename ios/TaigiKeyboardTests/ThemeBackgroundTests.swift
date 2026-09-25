import SwiftUI
@testable import TaigiKeyboard
import XCTest

/// Tests for `ThemeBackground` / `ThemeGradient` — the single background field a
/// theme carries (USER 2026-09-19: one surface for keyboard + candidate bar), its
/// legacy-key decoding, the angle → unit-point math shared with the overlay
/// backdrops, and the user-theme seed.
final class ThemeBackgroundTests: XCTestCase {
    private func decode(_ json: String) throws -> KeyboardColorSettings {
        try JSONDecoder().decode(KeyboardColorSettings.self, from: Data(json.utf8))
    }

    /// Two arbitrary stops — `ThemeGradient` requires ≥2; these tests only care about the angle.
    private func gradient(angle: Double) -> ThemeGradient {
        ThemeGradient(stops: [CodableColor(hex: 0x000000), CodableColor(hex: 0xFFFFFF)], angle: angle)
    }

    private func panelPoints(angle: Double, fullKeyboardHeight: CGFloat, topInset: CGFloat) throws -> (start: UnitPoint, end: UnitPoint) {
        gradient(angle: angle).unitPoints(in: KeyboardSurfaceSlice(fullHeight: fullKeyboardHeight, topInset: topInset))
    }

    // MARK: - Legacy decode

    // trace: old JSON `backgroundColor` (no `background`) → .solid(that color)
    func testDecode_legacyBackgroundColor_becomesSolid() throws {
        let colors = try decode(#"{ "backgroundColor": { "red": 1, "green": 0, "blue": 0, "alpha": 1 } }"#)
        XCTAssertEqual(colors.background, .solid(CodableColor(hex: 0xFF0000)))
        XCTAssertEqual(colors.solidBackgroundColor, CodableColor(hex: 0xFF0000))
        XCTAssertNil(colors.backgroundGradient)
    }

    // trace: old JSON `backgroundGradient` (2 stops, no angle) → .gradient at the vertical default angle,
    // and it wins over a legacy `backgroundColor` written beside it (gradient overrode the flat fill before)
    func testDecode_legacyBackgroundGradient_becomesVerticalGradient() throws {
        let colors = try decode(#"""
        { "backgroundColor": { "red": 1, "green": 0, "blue": 0, "alpha": 1 },
          "backgroundGradient": { "stops": [ { "red": 0, "green": 0, "blue": 0, "alpha": 1 },
                                             { "red": 1, "green": 1, "blue": 1, "alpha": 1 } ] } }
        """#)
        let gradient = try XCTUnwrap(colors.backgroundGradient)
        XCTAssertEqual(gradient.angle, ThemeGradient.defaultAngle)
        XCTAssertEqual(gradient.stops, [CodableColor(hex: 0x000000), CodableColor(hex: 0xFFFFFF)])
        XCTAssertNil(colors.solidBackgroundColor)
    }

    // trace: a legacy 1-stop gradient is not renderable → falls through to the legacy solid color
    func testDecode_legacySingleStopGradient_fallsBackToSolid() throws {
        let colors = try decode(#"""
        { "backgroundColor": { "red": 0, "green": 1, "blue": 0, "alpha": 1 },
          "backgroundGradient": { "stops": [ { "red": 0, "green": 0, "blue": 0, "alpha": 1 } ] } }
        """#)
        XCTAssertEqual(colors.background, .solid(CodableColor(hex: 0x00FF00)))
    }

    // trace: `candidateBackgroundColor` is dropped on decode — the candidate bar is the keyboard surface
    func testDecode_legacyCandidateBackground_isIgnored() throws {
        let colors = try decode(#"{ "candidateBackgroundColor": { "red": 1, "green": 0, "blue": 0, "alpha": 1 } }"#)
        XCTAssertEqual(colors, .default)
    }

    // trace: an unknown background `type` (written by a newer build) degrades to adaptive, other roles kept
    func testDecode_unknownBackgroundType_degradesToAdaptive() throws {
        let colors = try decode(#"""
        { "background": { "type": "hologram" },
          "keyTextColor": { "red": 0, "green": 0, "blue": 1, "alpha": 1 } }
        """#)
        XCTAssertNil(colors.background)
        XCTAssertEqual(colors.keyTextColor, CodableColor(hex: 0x0000FF))
    }

    // MARK: - Round trip

    // trace: encode writes only the `background` key (type + fields), never the legacy keys; decode restores it
    func testRoundTrip_gradientWithAngle_andNoLegacyKeys() throws {
        var colors = KeyboardColorSettings()
        colors.background = .gradient(ThemeGradient(stops: [CodableColor(hex: 0x112233), CodableColor(hex: 0x445566)], angle: 45))
        let data = try JSONEncoder().encode(colors)
        let json = try XCTUnwrap(String(data: data, encoding: .utf8))
        XCTAssertFalse(json.contains("backgroundColor"), "legacy key must not be written: \(json)")
        XCTAssertFalse(json.contains("backgroundGradient"), "legacy key must not be written: \(json)")
        XCTAssertTrue(json.contains(#""type":"gradient""#), json)
        XCTAssertEqual(try JSONDecoder().decode(KeyboardColorSettings.self, from: data), colors)
    }

    func testRoundTrip_solid() throws {
        var colors = KeyboardColorSettings()
        colors.background = .solid(CodableColor(hex: 0xABCDEF))
        let data = try JSONEncoder().encode(colors)
        XCTAssertTrue(try XCTUnwrap(String(data: data, encoding: .utf8)).contains(#""type":"solid""#))
        XCTAssertEqual(try JSONDecoder().decode(KeyboardColorSettings.self, from: data), colors)
    }

    // MARK: - Angle → unit points

    // trace: CSS convention — 180 = top→bottom edge-to-edge; 90 = left→right; 0 = bottom→top;
    // 135 = top-left → bottom-right corner-to-corner (Chebyshev-normalised diagonal)
    func testUnitPoints_presets() {
        let cases: [(angle: Double, start: UnitPoint, end: UnitPoint)] = [
            (180, UnitPoint(x: 0.5, y: 0), UnitPoint(x: 0.5, y: 1)),
            (0, UnitPoint(x: 0.5, y: 1), UnitPoint(x: 0.5, y: 0)),
            (90, UnitPoint(x: 0, y: 0.5), UnitPoint(x: 1, y: 0.5)),
            (270, UnitPoint(x: 1, y: 0.5), UnitPoint(x: 0, y: 0.5)),
            (135, UnitPoint(x: 0, y: 0), UnitPoint(x: 1, y: 1)),
            (315, UnitPoint(x: 1, y: 1), UnitPoint(x: 0, y: 0)),
            (45, UnitPoint(x: 0, y: 1), UnitPoint(x: 1, y: 0)),
            (225, UnitPoint(x: 1, y: 0), UnitPoint(x: 0, y: 1)),
        ]
        for c in cases {
            let points = gradient(angle: c.angle).unitPoints
            XCTAssertEqual(points.start.x, c.start.x, accuracy: 1e-9, "angle \(c.angle) start.x")
            XCTAssertEqual(points.start.y, c.start.y, accuracy: 1e-9, "angle \(c.angle) start.y")
            XCTAssertEqual(points.end.x, c.end.x, accuracy: 1e-9, "angle \(c.angle) end.x")
            XCTAssertEqual(points.end.y, c.end.y, accuracy: 1e-9, "angle \(c.angle) end.y")
        }
    }

    // trace: a panel covering [topInset, fullHeight] maps the vertical gradient's start ABOVE itself:
    // y' = (0·H − topInset) / (H − topInset) = −topInset / panelHeight (the #429-safe slice shift), end stays 1
    func testPanelUnitPoints_verticalGradient_shiftsStartAbovePanel() throws {
        let points = try panelPoints(angle: ThemeGradient.defaultAngle, fullKeyboardHeight: 300, topInset: 50)
        XCTAssertEqual(points.start.y, -50.0 / 250.0, accuracy: 1e-9)
        XCTAssertEqual(points.end.y, 1, accuracy: 1e-9)
        XCTAssertEqual(points.start.x, 0.5, accuracy: 1e-9)
    }

    // trace: a horizontal gradient is unaffected by the vertical slice except for its y row
    func testPanelUnitPoints_horizontalGradient_keepsXAndRemapsY() throws {
        let points = try panelPoints(angle: 90, fullKeyboardHeight: 300, topInset: 50)
        XCTAssertEqual(points.start.x, 0, accuracy: 1e-9)
        XCTAssertEqual(points.end.x, 1, accuracy: 1e-9)
        XCTAssertEqual(points.start.y, (0.5 * 300 - 50) / 250, accuracy: 1e-9)
        XCTAssertEqual(points.end.y, points.start.y, accuracy: 1e-9)
    }

    // trace: degenerate geometry (panelHeight ≤ 0) → unshifted points, no division by zero
    func testPanelUnitPoints_degenerateGeometry_fallsBackToUnshifted() throws {
        let points = try panelPoints(angle: 180, fullKeyboardHeight: 50, topInset: 50)
        XCTAssertEqual(points.start.y, 0, accuracy: 1e-9)
        XCTAssertEqual(points.end.y, 1, accuracy: 1e-9)
    }

    // MARK: - Photo background

    // trace: `{"type":"image","file":"a.jpg","dim":0.5}` round-trips; dim clamps into 0…0.8; absent dim = default
    func testImageBackground_roundTripAndDimClamp() throws {
        var colors = KeyboardColorSettings()
        colors.background = .image(ThemeImageBackground(file: "a.jpg", dim: 0.5))
        let data = try JSONEncoder().encode(colors)
        XCTAssertTrue(try XCTUnwrap(String(data: data, encoding: .utf8)).contains(#""type":"image""#))
        XCTAssertEqual(try JSONDecoder().decode(KeyboardColorSettings.self, from: data), colors)
        XCTAssertEqual(ThemeImageBackground(file: "a.jpg", dim: 2).dim, ThemeImageBackground.dimRange.upperBound, "dim clamps high")
        XCTAssertEqual(ThemeImageBackground(file: "a.jpg", dim: -1).dim, 0, "dim clamps low")
        let absentDim = try decode(#"{ "background": { "type": "image", "file": "b.jpg" } }"#)
        XCTAssertEqual(absentDim.background?.image, ThemeImageBackground(file: "b.jpg", dim: ThemeImageBackground.defaultDim))
    }

    // trace: an empty file name is not a photo → decode degrades to adaptive (same as an unknown type)
    func testImageBackground_emptyFile_degradesToAdaptive() throws {
        XCTAssertNil(try decode(#"{ "background": { "type": "image", "file": "" } }"#).background)
    }

    // trace: aspect-fill cover rect — a 2:1 photo over a 1:1 keyboard fills the height and centres horizontally;
    // a 1:2 photo fills the width and centres vertically; a panel slice keeps the whole-keyboard framing
    func testCoverRect_aspectFillCentred() {
        let square = CGRect(x: 0, y: 0, width: 100, height: 100)
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: CGSize(width: 200, height: 100), in: square, focus: ThemeImageBackground.centredFocus), CGRect(x: -50, y: 0, width: 200, height: 100))
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: CGSize(width: 100, height: 200), in: square, focus: ThemeImageBackground.centredFocus), CGRect(x: 0, y: -50, width: 100, height: 200))
        let slicedKeyboard = CGRect(x: 0, y: -50, width: 100, height: 300)
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: CGSize(width: 100, height: 100), in: slicedKeyboard, focus: ThemeImageBackground.centredFocus), CGRect(x: -100, y: -50, width: 300, height: 300))
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: .zero, in: square, focus: ThemeImageBackground.centredFocus), square, "degenerate image size falls back to the bounds")
    }

    // trace: focus aligns the cover rect — a 2:1 photo over a 1:1 keyboard overflows 100 horizontally:
    // focusX 0 → x = 0 (left edge shows), 1 → x = 100 − 200 = −100 (right edge shows); the non-overflowing
    // axis ignores focus (100 − 100 = 0); a sliced keyboard keeps its origin offset
    func testCoverRect_focusAlignsOverflowingAxis() {
        let square = CGRect(x: 0, y: 0, width: 100, height: 100)
        let wide = CGSize(width: 200, height: 100)
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: wide, in: square, focus: CGPoint(x: 0, y: 1)), CGRect(x: 0, y: 0, width: 200, height: 100))
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: wide, in: square, focus: CGPoint(x: 1, y: 0)), CGRect(x: -100, y: 0, width: 200, height: 100))
        let slicedKeyboard = CGRect(x: 0, y: -50, width: 100, height: 300)
        XCTAssertEqual(ThemeImageBackground.coverRect(imageSize: CGSize(width: 100, height: 100), in: slicedKeyboard, focus: CGPoint(x: 1, y: 0)), CGRect(x: -200, y: -50, width: 300, height: 300))
    }

    // trace: `focusX` / `focusY` round-trip; absent → 0.5 (old themes stay centred); out of range clamps into 0…1
    func testImageBackground_focusRoundTripDefaultAndClamp() throws {
        var colors = KeyboardColorSettings()
        colors.background = .image(ThemeImageBackground(file: "a.jpg", focusX: 0.2, focusY: 0.9))
        let data = try JSONEncoder().encode(colors)
        XCTAssertEqual(try JSONDecoder().decode(KeyboardColorSettings.self, from: data), colors)
        let absent = try XCTUnwrap(decode(#"{ "background": { "type": "image", "file": "b.jpg", "dim": 0.5 } }"#).background?.image)
        XCTAssertEqual(absent.focus, CGPoint(x: 0.5, y: 0.5))
        XCTAssertEqual(ThemeImageBackground(file: "a.jpg", focusX: -1, focusY: 3).focus, CGPoint(x: 0, y: 1))
    }

    // trace: FocusedPhotoFill renders where coverRect says — a 100×300 photo (top half red, bottom half blue, desaturated ×0.7)
    // over a 100×100 surface overflows 200 vertically: focusY 0 shows rows 0…100 (red), 1 shows rows 200…300 (blue)
    @MainActor
    func testFocusedPhotoFill_movesPixelsWithFocus() throws {
        let format = UIGraphicsImageRendererFormat.default()
        format.scale = 1
        let photo = UIGraphicsImageRenderer(size: CGSize(width: 100, height: 300), format: format).image { context in
            UIColor.red.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 100, height: 150))
            UIColor.blue.setFill()
            context.fill(CGRect(x: 0, y: 150, width: 100, height: 150))
        }
        func centrePixel(focusY: CGFloat) throws -> (red: UInt8, blue: UInt8) {
            let renderer = ImageRenderer(content: FocusedPhotoFill(image: Image(uiImage: photo), focus: CGPoint(x: 0.5, y: focusY)).frame(width: 100, height: 100))
            renderer.scale = 1
            let cgImage = try XCTUnwrap(renderer.cgImage)
            var pixel = [UInt8](repeating: 0, count: 4)
            let context = try XCTUnwrap(CGContext(
                data: &pixel, width: 1, height: 1, bitsPerComponent: 8, bytesPerRow: 4,
                space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue,
            ))
            context.draw(cgImage, in: CGRect(x: -50, y: -50, width: 100, height: 100))
            return (pixel[0], pixel[2])
        }
        // The fill desaturates (×0.7), so compare which channel dominates rather than pure values.
        let top = try centrePixel(focusY: 0)
        XCTAssertGreaterThan(Int(top.red) - Int(top.blue), 100, "focusY 0 shows the red top half: \(top)")
        let bottom = try centrePixel(focusY: 1)
        XCTAssertGreaterThan(Int(bottom.blue) - Int(bottom.red), 100, "focusY 1 shows the blue bottom half: \(bottom)")
    }

    // trace: a 100×300 photo over a 100×100 preview covers 100×300 → overflow (0, −200), vertical axis;
    // dragging down 50 moves the photo with the finger: 0.5 + 50 / −200 = 0.25 (more of the top shows);
    // x never overflows so a sideways drag keeps 0.5; a long drag up clamps at 1
    func testPhotoPositionDrag_followsFingerOnOverflowingAxis() {
        let tall = CGSize(width: 100, height: 300)
        let preview = CGSize(width: 100, height: 100)
        let centred = ThemeImageBackground(file: "a.jpg")
        XCTAssertEqual(PhotoPositionDrag.axis(imageSize: tall, surface: preview), .vertical)
        XCTAssertEqual(PhotoPositionDrag.axis(imageSize: CGSize(width: 200, height: 100), surface: preview), .horizontal)
        XCTAssertNil(PhotoPositionDrag.axis(imageSize: CGSize(width: 50, height: 50), surface: preview), "exact fit has nothing to move")
        let dragged = PhotoPositionDrag.dragged(centred, by: CGSize(width: 30, height: 50), along: .vertical, imageSize: tall, surface: preview)
        XCTAssertEqual(dragged.focus, CGPoint(x: 0.5, y: 0.25), "only the vertical axis moves")
        XCTAssertEqual(dragged.dim, centred.dim, "Fade kept")
        XCTAssertEqual(PhotoPositionDrag.dragged(centred, by: CGSize(width: 0, height: -1000), along: .vertical, imageSize: tall, surface: preview).focusY, 1)
        XCTAssertEqual(PhotoPositionDrag.stepped(centred.with(focus: CGPoint(x: 0.5, y: 0.95)), along: .vertical, increment: true).focusY, 1)
        XCTAssertEqual(PhotoPositionDrag.stepped(centred, along: .horizontal, increment: false).focusX, 0.4, accuracy: 1e-9)
    }

    // trace: the surface pairs the background with its photo tone — dark key text → white overlay,
    // light key text → black; unset role = seed (black text) → white; adaptive (no background) → nil
    func testSurface_pairsBackgroundWithKeyTextTone() {
        var colors = KeyboardColorSettings()
        XCTAssertNil(colors.surface, "adaptive default has no custom surface")
        colors.background = .image(ThemeImageBackground(file: "a.jpg"))
        XCTAssertEqual(colors.surface?.dimsTowardWhite, true)
        colors.keyTextColor = CodableColor(hex: 0xFFFFFF)
        XCTAssertEqual(colors.surface?.dimsTowardWhite, false)
        colors.keyTextColor = CodableColor(hex: 0x1C1C1E)
        XCTAssertEqual(colors.surface, ThemeSurface(background: colors.background!, dimsTowardWhite: true))
    }

    // trace: a panel slice's keyboard rect is the whole keyboard shifted up by the chrome above the panel
    func testSliceKeyboardRect_shiftsUpByTopInset() {
        let slice = KeyboardSurfaceSlice(fullHeight: 300, topInset: 50)
        XCTAssertEqual(slice.keyboardRect(width: 400), CGRect(x: 0, y: -50, width: 400, height: 300))
    }

    // trace: a new-format gradient with one stop is not renderable → decode degrades to adaptive
    func testDecode_newFormatSingleStopGradient_degradesToAdaptive() throws {
        let colors = try decode(#"{ "background": { "type": "gradient", "stops": [ { "red": 0, "green": 0, "blue": 0, "alpha": 1 } ] } }"#)
        XCTAssertNil(colors.background)
    }

    // MARK: - Seed

    // trace: the seed sets every role (no nil) so a user theme never follows light / dark
    func testUserThemeSeed_hasNoNilRole() {
        let seed = UserThemeSeed.colors
        XCTAssertEqual(seed.background, .solid(CodableColor(hex: 0xD4D5DD)))
        XCTAssertNotNil(seed.keyTextColor)
        XCTAssertNotNil(seed.normalKeyFillColor)
        XCTAssertNotNil(seed.specialKeyFillColor)
        XCTAssertNotNil(seed.candidateTextColor)
    }

    // trace: seeding fills only nil roles; set roles (incl. a gradient background) are kept verbatim
    func testSeededForUserTheme_fillsOnlyNilRoles() {
        var colors = KeyboardColorSettings()
        colors.background = .gradient(ThemeGradient(stops: [CodableColor(hex: 0x000000), CodableColor(hex: 0xFFFFFF)], angle: 90))
        colors.keyTextColor = CodableColor(hex: 0x123456)
        let seeded = colors.seededForUserTheme()
        XCTAssertEqual(seeded.background, colors.background)
        XCTAssertEqual(seeded.keyTextColor, CodableColor(hex: 0x123456))
        XCTAssertEqual(seeded.normalKeyFillColor, UserThemeSeed.colors.normalKeyFillColor)
        XCTAssertEqual(seeded.specialKeyFillColor, UserThemeSeed.colors.specialKeyFillColor)
        XCTAssertEqual(seeded.candidateTextColor, UserThemeSeed.colors.candidateTextColor)
        XCTAssertEqual(UserThemeSeed.colors.seededForUserTheme(), UserThemeSeed.colors, "seeding the seed is a no-op")
    }
}
