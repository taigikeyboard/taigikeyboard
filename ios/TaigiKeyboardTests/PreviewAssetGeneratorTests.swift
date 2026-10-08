import KeyboardKit
import SwiftUI
@testable import TaigiKeyboard
import UIKit
import XCTest

/// Regenerates the Theme / Layout card screenshots in `LayoutPreviewAssets.xcassets` by
/// rendering the real keyboard (`KeyboardPreviewPanel`) — run through the `previews` skill.
///
/// Skipped unless `TEST_RUNNER_PREVIEW_ASSETS_DIR` (the absolute `.xcassets` path) is set on
/// the `xcodebuild` command line, so ordinary test runs never touch the assets. Every PNG an
/// imageset's `Contents.json` names is overwritten in place; `Contents.json` is left as is.
@MainActor
final class PreviewAssetGeneratorTests: XCTestCase {
    private static let assetsDirectoryEnvironmentKey = "PREVIEW_ASSETS_DIR"

    /// The width at which the card aspect holds the key area with even top / bottom margins
    /// (the keyboard's own bottom inset) and no candidate bar.
    private static let renderWidth: CGFloat = 372
    private static let renderScale: CGFloat = 3
    /// Taller than any keyboard, so the first layout pass is never height-bound.
    private static let initialWindowHeight: CGFloat = 600
    /// Room under the bottom row, matching the inset above the top row (KeyboardKit's vertical
    /// key inset); the fitting height ends flush with the bottom row.
    private static let bottomInset: CGFloat = 4.5
    /// The `ThemeCardMetrics` card at @3x (720×454).
    private static let outputSize = CGSize(
        width: ThemeCardMetrics.width * renderScale,
        height: (ThemeCardMetrics.width * renderScale / ThemeCardMetrics.previewAspectRatio).rounded(),
    )

    /// The system keyboard backdrop behind Liquid Glass, sampled from iOS 27 device
    /// screenshots. The OS draws it, so an offscreen render has none of its own.
    private static let lightBackdrop = UIColor(red: 0xCA / 255, green: 0xD2 / 255, blue: 0xDF / 255, alpha: 1)
    private static let darkBackdrop = UIColor(red: 0x32 / 255, green: 0x32 / 255, blue: 0x32 / 255, alpha: 1)

    /// Layout cards show the default (adaptive) theme; theme cards show the default layout.
    private static let themeCardLayout = KeyboardLayoutType.phahTaigi

    private struct GeneratorError: Error, CustomStringConvertible {
        let description: String
    }

    private struct RenderTarget {
        let themeId: String
        let layout: KeyboardLayoutType
        /// A dark-only palette (Catppuccin) renders its single universal image dark.
        let isDarkOnly: Bool
    }

    private struct RenderJob {
        let outputURL: URL
        let target: RenderTarget
        let colorScheme: ColorScheme
    }

    func testRegeneratePreviewAssets() throws {
        guard let assetsPath = ProcessInfo.processInfo.environment[Self.assetsDirectoryEnvironmentKey] else {
            throw XCTSkip("set TEST_RUNNER_\(Self.assetsDirectoryEnvironmentKey) to regenerate previews")
        }
        // iPad shows the phone-shaped previews too, so they must come from an iPhone render.
        guard UIDevice.current.userInterfaceIdiom == .phone else {
            return XCTFail("run the preview generator on an iPhone simulator")
        }
        // Map every image before writing any, so an unmapped imageset leaves the catalog untouched.
        let jobs = try Self.renderJobs(in: URL(fileURLWithPath: assetsPath))

        // Render paths read `SharedSettings.shared` directly (layout, input mode, font,
        // punctuation width), so the previews come from the factory settings: drop the
        // App Group domain, render, then put the simulator's own settings back.
        let defaults = SharedSettings.sharedUserDefaults
        let savedDomain = defaults.persistentDomain(forName: SharedSettings.appGroupId)
        defer {
            if let savedDomain {
                defaults.setPersistentDomain(savedDomain, forName: SharedSettings.appGroupId)
            } else {
                defaults.removePersistentDomain(forName: SharedSettings.appGroupId)
            }
        }
        defaults.removePersistentDomain(forName: SharedSettings.appGroupId)

        for job in jobs {
            try autoreleasepool {
                SharedSettings.shared.keyboardLayoutType = job.target.layout
                let image = try Self.render(job)
                try XCTUnwrap(image.pngData()).write(to: job.outputURL)
            }
        }
        print("preview.generate.complete count=\(jobs.count)")
    }

    // MARK: - Asset mapping

    private struct ImagesetContents: Decodable {
        struct Image: Decodable {
            struct Appearance: Decodable {
                let appearance: String
                let value: String
            }

            let filename: String?
            let appearances: [Appearance]?
        }

        let images: [Image]
    }

    private static func renderJobs(in assetsURL: URL) throws -> [RenderJob] {
        let imagesetURLs = try FileManager.default
            .contentsOfDirectory(at: assetsURL, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "imageset" }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
        XCTAssertFalse(imagesetURLs.isEmpty, "no imagesets in \(assetsURL.path)")

        return try imagesetURLs.flatMap { imagesetURL -> [RenderJob] in
            let target = try target(forImageset: imagesetURL.deletingPathExtension().lastPathComponent)
            let contents = try JSONDecoder().decode(
                ImagesetContents.self,
                from: Data(contentsOf: imagesetURL.appendingPathComponent("Contents.json")),
            )
            return contents.images.compactMap { image in
                guard let filename = image.filename else { return nil }
                return RenderJob(
                    outputURL: imagesetURL.appendingPathComponent(filename),
                    target: target,
                    colorScheme: colorScheme(for: image, isDarkOnly: target.isDarkOnly),
                )
            }
        }
    }

    private static func target(forImageset name: String) throws -> RenderTarget {
        if let layout = KeyboardLayoutType.allCases.first(where: { $0.previewImageName == name }) {
            return RenderTarget(themeId: ThemeId.default, layout: layout, isDarkOnly: false)
        }
        guard let theme = BuiltInThemes.all.first(where: { $0.previewImageName == name }) else {
            throw GeneratorError(description: "imageset \(name) is no layout's or built-in theme's previewImageName")
        }
        return RenderTarget(themeId: theme.id, layout: themeCardLayout, isDarkOnly: theme.light == nil)
    }

    /// An explicit luminosity wins; a single universal image follows the theme's palette.
    private static func colorScheme(for image: ImagesetContents.Image, isDarkOnly: Bool) -> ColorScheme {
        if let luminosity = image.appearances?.first(where: { $0.appearance == "luminosity" }) {
            return luminosity.value == "dark" ? .dark : .light
        }
        return isDarkOnly ? .dark : .light
    }

    // MARK: - Rendering

    private static func render(_ job: RenderJob) throws -> UIImage {
        let colorScheme = job.colorScheme
        let appearance = ThemeResolver.resolved(themeId: job.target.themeId, colorScheme: colorScheme, userThemes: [])
        let panel = KeyboardPreviewPanel(
            appearance: appearance,
            appliesThemeShadow: false,
            colorScheme: colorScheme,
            isLiquidGlassEnabled: true,
        )
        .environment(\.colorScheme, colorScheme)

        let isDark = colorScheme == .dark
        let host = UIHostingController(rootView: panel)
        host.view.backgroundColor = isDark ? darkBackdrop : lightBackdrop
        // Deliberately the scene-less (iOS 26-deprecated) init: a `UIWindow(windowScene:)` takes
        // the device safe area, which shifts the keyboard down and clips the bottom row
        // (`safeAreaRegions = []` on the host does not undo it).
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: renderWidth, height: initialWindowHeight))
        window.overrideUserInterfaceStyle = isDark ? .dark : .light
        window.rootViewController = host
        window.isHidden = false
        defer { window.isHidden = true }
        // Measure only after `onAppear` has configured the preview context and SwiftUI settled:
        // measured earlier, the height comes out short and drops the inset below the bottom row.
        RunLoop.main.run(until: Date().addingTimeInterval(1.0))
        let fittingSize = host.sizeThatFits(in: CGSize(width: renderWidth, height: .greatestFiniteMagnitude))
        window.frame = CGRect(x: 0, y: 0, width: renderWidth, height: fittingSize.height + bottomInset)
        host.view.frame = window.bounds
        RunLoop.main.run(until: Date().addingTimeInterval(0.2))
        host.view.layoutIfNeeded()

        let bounds = host.view.bounds
        let format = UIGraphicsImageRendererFormat()
        format.scale = renderScale
        format.preferredRange = .standard
        var isCaptured = false
        let fullImage = UIGraphicsImageRenderer(bounds: bounds, format: format).image { _ in
            isCaptured = host.view.drawHierarchy(in: bounds, afterScreenUpdates: true)
        }
        guard isCaptured else {
            throw GeneratorError(description: "drawHierarchy captured no image for \(job.outputURL.lastPathComponent)")
        }
        return cropKeyArea(of: fullImage)
    }

    /// Bottom-anchored crop to the card aspect (drops the candidate bar), scaled to `outputSize`.
    private static func cropKeyArea(of image: UIImage) -> UIImage {
        let pixelsPerPoint = outputSize.width / image.size.width
        let cropHeight = outputSize.height / pixelsPerPoint
        let format = UIGraphicsImageRendererFormat()
        format.scale = 1
        // 8-bit opaque RGB: the default extended range writes 16-bit RGBA, ~4x the bytes.
        format.preferredRange = .standard
        format.opaque = true
        return UIGraphicsImageRenderer(size: outputSize, format: format).image { _ in
            image.draw(in: CGRect(
                x: 0,
                y: -(image.size.height - cropHeight) * pixelsPerPoint,
                width: outputSize.width,
                height: image.size.height * pixelsPerPoint,
            ))
        }
    }
}
