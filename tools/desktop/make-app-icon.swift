// Generates the desktop app icon — macos/App/AppIcon.icns and
// windows/resources/TaigiKeyboard.ico — from one 台 outline, so the two
// desktop platforms cannot drift apart. Run it by hand after changing
// anything below:
//
//     swift tools/desktop/make-app-icon.swift
//     swift tools/desktop/make-app-icon.swift --check   (verify, write nothing)
//
// Requires macOS: it rasterises through CoreText and packs the .icns with
// `iconutil`. That is fine because BOTH outputs are committed artefacts —
// the Windows release build consumes the .ico and never regenerates it. Do
// NOT wire this into bundle-app.sh, release-app.sh, a Cargo build script or
// a Makefile release prerequisite: generating during a build would let the
// build machine's font version and rasteriser decide what ships.
//
// iOS and Android keep the "Tâi" wordmark and are not touched here. This
// script reads nothing under ios/ or android/ and writes only the two paths
// named above.
//
// The mark is the one the Mac menu bar already wears (scripts/
// make-menubar-icon.swift), on an opaque tile rather than as a template: the
// menu bar's negative-alpha trick has no equivalent on Windows, which never
// recolours a tray icon, so a knocked-out glyph is invisible on one theme or
// the other. An opaque tile carries its own contrast onto any background.

import AppKit
import CoreText
import CryptoKit
import ImageIO

// MARK: - The design

let glyph = "台"
let fontPostScriptName = "PingFangTC-Semibold"
/// The outline this icon was approved with. `NSFont(name:)` substitutes rather
/// than fails for some names, and Apple can change a glyph under a stable
/// PostScript name in an OS update — either would silently reshape a committed
/// artefact. Regenerating then fails loudly instead of producing a diff nobody
/// can read. Update this ONLY together with a reviewed icon change.
let approvedOutlineFingerprint = "4a24567095da319efa0f0284986163581c459b300822c1b7fadc9efbb5c76f81"

/// Sampled from the icon this replaces, so the new mark stays in the family:
/// a pure white tile with Apple's near-black ink.
let tileColor = (red: 1.0, green: 1.0, blue: 1.0)
let inkColor = (red: 0x1D / 255.0, green: 0x1D / 255.0, blue: 0x1F / 255.0)
/// Both from `make-menubar-icon.swift`, so the desktop mark and the menu-bar
/// mark are the same shape at different jobs.
let glyphInsetRatio: CGFloat = 0.13
let cornerRadiusRatio: CGFloat = 3.0 / 16.0

/// `iconutil`'s members. The name carries the point size and the scale; the
/// pixel size is their product, and every one is rasterised from the outline
/// rather than downsampled from a larger page.
let iconsetMembers: [(name: String, pixels: Int)] = [
    ("icon_16x16", 16), ("icon_16x16@2x", 32),
    ("icon_32x32", 32), ("icon_32x32@2x", 64),
    ("icon_128x128", 128), ("icon_128x128@2x", 256),
    ("icon_256x256", 256), ("icon_256x256@2x", 512),
    ("icon_512x512", 512), ("icon_512x512@2x", 1024),
]

/// Windows asks for an exact match before it scales: 16/20/24/32/40/48/64 for
/// tray, title bar and menus, 24 upward for the taskbar across DPI settings,
/// and 256 as the largest an .ico can carry.
let windowsSizes = [16, 20, 24, 32, 40, 48, 64, 96, 256]

// MARK: - Paths

let repositoryRoot = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()   // tools/desktop/
    .deletingLastPathComponent()   // tools/
    .deletingLastPathComponent()   // repository root
let icnsURL = repositoryRoot.appendingPathComponent("macos/App/AppIcon.icns")
let icoURL = repositoryRoot.appendingPathComponent("windows/resources/TaigiKeyboard.ico")
let icoPacker = repositoryRoot.appendingPathComponent("tools/windows/make-ico.py")

let isCheckOnly = CommandLine.arguments.contains("--check")

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("error: \(message)\n".utf8))
    exit(1)
}

// MARK: - The glyph

/// The glyph as an outline, so it centres on its own ink rather than on a text
/// line box — a CJK glyph's line box carries ascender and descender slack that
/// would push it visibly off centre at 16 px.
func glyphOutline() -> CGPath {
    guard let font = NSFont(name: fontPostScriptName, size: 100) else {
        fail("font '\(fontPostScriptName)' is not installed")
    }
    guard font.fontName == fontPostScriptName else {
        fail("font '\(fontPostScriptName)' resolved to '\(font.fontName)'")
    }
    var characters = Array(glyph.utf16)
    var glyphID = CGGlyph()
    let ctFont = font as CTFont
    guard characters.count == 1,
          CTFontGetGlyphsForCharacters(ctFont, &characters, &glyphID, 1),
          let path = CTFontCreatePathForGlyph(ctFont, glyphID, nil)
    else {
        fail("'\(glyph)' has no outline in \(fontPostScriptName)")
    }
    return path
}

/// A stable serialisation of the outline's own segments — not of the file it
/// came from, which carries every other glyph and a version number.
func fingerprint(of path: CGPath) -> String {
    var text = ""
    path.applyWithBlock { element in
        let e = element.pointee
        let points = UnsafeBufferPointer(start: e.points, count: {
            switch e.type {
            case .moveToPoint, .addLineToPoint: return 1
            case .addQuadCurveToPoint: return 2
            case .addCurveToPoint: return 3
            case .closeSubpath: return 0
            @unknown default: return 0
            }
        }())
        text += "\(e.type.rawValue)"
        for point in points {
            // Six decimals: far finer than any pixel grid here, coarse enough
            // that a last-bit difference in the same outline cannot trip it.
            text += String(format: " %.6f %.6f", point.x, point.y)
        }
        text += "\n"
    }
    let digest = SHA256.hash(data: Data(text.utf8))
    return digest.map { String(format: "%02x", $0) }.joined()
}

// MARK: - Rasterising

/// Drawn in an explicit sRGB context rather than a device-dependent one, so
/// the two constants above mean the same colour on every Mac — a device space
/// leaves the numbers open to the host's colour environment, which is a
/// different and worse problem than the byte-level variance this script
/// already declines to promise.
func render(pixels: Int, outline: CGPath) -> Data {
    guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else {
        fail("sRGB is unavailable on this system")
    }
    guard let context = CGContext(
        data: nil, width: pixels, height: pixels,
        bitsPerComponent: 8, bytesPerRow: 0, space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        fail("could not allocate a \(pixels)×\(pixels) bitmap")
    }
    let side = CGFloat(pixels)

    let tile = CGPath(
        roundedRect: CGRect(x: 0, y: 0, width: side, height: side),
        cornerWidth: side * cornerRadiusRatio,
        cornerHeight: side * cornerRadiusRatio,
        transform: nil
    )
    context.setFillColor(red: tileColor.red, green: tileColor.green, blue: tileColor.blue, alpha: 1)
    context.addPath(tile)
    context.fillPath()

    // Scale the outline into the inset box, then centre it on its own bounds.
    let bounds = outline.boundingBox
    let target = side - side * glyphInsetRatio * 2
    let scale = min(target / bounds.width, target / bounds.height)
    var scaling = CGAffineTransform(scaleX: scale, y: scale)
    guard let scaled = outline.copy(using: &scaling) else { fail("could not scale the outline") }
    let scaledBounds = scaled.boundingBox
    var centring = CGAffineTransform(
        translationX: (side - scaledBounds.width) / 2 - scaledBounds.minX,
        y: (side - scaledBounds.height) / 2 - scaledBounds.minY
    )
    guard let placed = scaled.copy(using: &centring) else { fail("could not centre the outline") }
    context.setFillColor(red: inkColor.red, green: inkColor.green, blue: inkColor.blue, alpha: 1)
    context.addPath(placed)
    context.fillPath()

    guard let image = context.makeImage() else {
        fail("could not read back the \(pixels)×\(pixels) page")
    }
    let png = NSMutableData()
    guard let destination = CGImageDestinationCreateWithData(png, "public.png" as CFString, 1, nil)
    else {
        fail("could not open a PNG encoder")
    }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else {
        fail("could not encode the \(pixels)×\(pixels) page")
    }
    return png as Data
}

// MARK: - Running a tool

@discardableResult
func run(_ launchPath: String, _ arguments: [String]) -> Int32 {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: launchPath)
    process.arguments = arguments
    do { try process.run() } catch { fail("could not run \(launchPath): \(error)") }
    process.waitUntilExit()
    return process.terminationStatus
}

// MARK: - Main

let outline = glyphOutline()
let outlineFingerprint = fingerprint(of: outline)
if approvedOutlineFingerprint == "PLACEHOLDER" {
    FileHandle.standardError.write(Data("""
        note: no approved outline fingerprint recorded yet. This run's outline is
              \(outlineFingerprint)
              Put it in `approvedOutlineFingerprint` once the icon is reviewed.

        """.utf8))
} else if outlineFingerprint != approvedOutlineFingerprint {
    fail("""
        '\(glyph)' in \(fontPostScriptName) no longer matches the approved outline.
          approved: \(approvedOutlineFingerprint)
          this Mac: \(outlineFingerprint)
        The font changed under a stable name. Review the rendered icon before
        updating `approvedOutlineFingerprint`.
        """)
}

let staging = URL(fileURLWithPath: NSTemporaryDirectory())
    .appendingPathComponent("taigi-app-icon-\(ProcessInfo.processInfo.processIdentifier)")
let iconset = staging.appendingPathComponent("AppIcon.iconset")
try? FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
defer { try? FileManager.default.removeItem(at: staging) }

for member in iconsetMembers {
    let png = render(pixels: member.pixels, outline: outline)
    let url = iconset.appendingPathComponent("\(member.name).png")
    do { try png.write(to: url) } catch { fail("could not write \(url.path): \(error)") }
}

var windowsPages: [String] = []
for size in windowsSizes {
    let png = render(pixels: size, outline: outline)
    let url = staging.appendingPathComponent("win-\(size).png")
    do { try png.write(to: url) } catch { fail("could not write \(url.path): \(error)") }
    windowsPages.append(url.path)
}

// Everything is rendered and packed into staging first, so a font, rasteriser,
// `iconutil` or packer failure leaves BOTH committed artefacts untouched. The
// two replacements below are then sequential, not transactional: a failure
// between them (a full disk, a read-only checkout) can leave the .icns updated
// and the .ico not. `git status` shows that immediately and a re-run fixes it,
// which is proportionate for a generator run by hand.
let stagedIcns = staging.appendingPathComponent("AppIcon.icns")
let stagedIco = staging.appendingPathComponent("TaigiKeyboard.ico")
guard run("/usr/bin/iconutil", ["--convert", "icns", iconset.path, "--output", stagedIcns.path]) == 0 else {
    fail("iconutil could not pack the iconset")
}
guard run("/usr/bin/env", ["python3", icoPacker.path, stagedIco.path] + windowsPages) == 0 else {
    fail("make-ico.py could not pack the Windows icon")
}

if isCheckOnly {
    let sameIcns = (try? Data(contentsOf: stagedIcns)) == (try? Data(contentsOf: icnsURL))
    let sameIco = (try? Data(contentsOf: stagedIco)) == (try? Data(contentsOf: icoURL))
    print("AppIcon.icns        \(sameIcns ? "up to date" : "STALE")")
    print("TaigiKeyboard.ico   \(sameIco ? "up to date" : "STALE")")
    exit(sameIcns && sameIco ? 0 : 1)
}

for (staged, committed) in [(stagedIcns, icnsURL), (stagedIco, icoURL)] {
    do {
        _ = try FileManager.default.replaceItemAt(committed, withItemAt: staged)
    } catch {
        fail("could not replace \(committed.path): \(error)")
    }
}
print("wrote \(icnsURL.path)")
print("wrote \(icoURL.path)")
