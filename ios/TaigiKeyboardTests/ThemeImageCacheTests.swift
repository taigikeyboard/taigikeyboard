@testable import TaigiKeyboard
import UIKit
import XCTest

/// Tests for `ThemeImageCache` — the off-main-actor photo decode behind a photo theme
/// background: lookup never decodes, load caches per variant, a thumbnail is downsampled,
/// and a missing file stays nil. Each test uses a fresh temp directory.
@MainActor
final class ThemeImageCacheTests: XCTestCase {
    private var tempDir: URL!

    override func setUp() {
        super.setUp()
        tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("ThemeImageCacheTests.\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: tempDir)
        tempDir = nil
        super.tearDown()
    }

    /// Saves a solid 1280×640 photo (the stored size of a wide pick) and returns its file name.
    private func savedPhoto(_ store: ThemeImageStore) throws -> String {
        let format = UIGraphicsImageRendererFormat.default()
        format.scale = 1
        let image = UIGraphicsImageRenderer(size: CGSize(width: 1280, height: 640), format: format).image { context in
            UIColor.blue.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 1280, height: 640))
        }
        return try XCTUnwrap(store.save(XCTUnwrap(image.jpegData(compressionQuality: 0.9))))
    }

    // trace: cached() is lookup-only — nil before load, the same instance after load
    func testCached_isNilUntilLoaded() async throws {
        let cache = ThemeImageCache(store: ThemeImageStore(containerURL: tempDir))
        let file = try savedPhoto(cache.store)
        XCTAssertNil(cache.cached(file, .full))
        let loaded = await cache.load(file, .full)
        let full = try XCTUnwrap(loaded)
        XCTAssertIdentical(cache.cached(file, .full), full)
        XCTAssertNil(cache.cached(file, .thumbnail), "variants are cached separately")
    }

    // trace: thumbnail = downsample to thumbnailLongEdge (480) → 480×240; full keeps 1280×640
    func testLoad_thumbnailIsDownsampled() async throws {
        let cache = ThemeImageCache(store: ThemeImageStore(containerURL: tempDir))
        let file = try savedPhoto(cache.store)
        let loadedThumbnail = await cache.load(file, .thumbnail)
        let thumbnail = try XCTUnwrap(loadedThumbnail)
        XCTAssertEqual(thumbnail.size.width * thumbnail.scale, ThemeImageVariant.thumbnailLongEdge)
        XCTAssertEqual(thumbnail.size.height * thumbnail.scale, ThemeImageVariant.thumbnailLongEdge / 2)
        let loadedFull = await cache.load(file, .full)
        let full = try XCTUnwrap(loadedFull)
        XCTAssertEqual(full.size.width * full.scale, 1280)
    }

    // trace: two concurrent loads of one photo share a single decode → the same instance
    func testLoad_concurrentLoadsShareOneDecode() async throws {
        let cache = ThemeImageCache(store: ThemeImageStore(containerURL: tempDir))
        let file = try savedPhoto(cache.store)
        async let first = cache.load(file, .full)
        async let second = cache.load(file, .full)
        let (a, b) = await (first, second)
        XCTAssertIdentical(try XCTUnwrap(a), try XCTUnwrap(b))
    }

    // trace: a missing file (deleted photo) loads as nil and caches nothing
    func testLoad_missingFileIsNil() async {
        let cache = ThemeImageCache(store: ThemeImageStore(containerURL: tempDir))
        let image = await cache.load("missing.jpg", .full)
        XCTAssertNil(image)
        XCTAssertNil(cache.cached("missing.jpg", .full))
    }
}
