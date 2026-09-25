// Per-process decoded-photo cache for ThemeBackgroundSurface, keyed by the theme image file name.

import SwiftUI
import UIKit

/// Which decode of a theme photo a render site needs: the stored 1280 px photo for the
/// keyboard and the editor preview, or a small downsample for cards and row thumbnails.
enum ThemeImageVariant: Hashable {
    case full
    case thumbnail

    /// Longest edge of a thumbnail decode — covers a theme card at 3×.
    static let thumbnailLongEdge: CGFloat = 480
}

/// Decodes theme photos from the App Group `ThemeImageStore` directory OFF the main actor
/// and keeps them decoded (`preparingForDisplay`, so a redraw never re-decodes the JPEG).
/// Render sites read `cached(_:_:)` (lookup only, no I/O) and fall back to `load(_:_:)`,
/// which decodes in a detached task; concurrent loads of one photo share a single decode.
/// Full photos and thumbnails are cached separately: the full cache holds at most two
/// (the keyboard's + the editor's) so the keyboard extension, under its 64 MB cap, drops
/// a previous theme's photo once another is loaded. A replaced photo gets a new file name,
/// so a stale entry is never served.
final class ThemeImageCache {
    static let shared = ThemeImageCache(store: ThemeImageStore(containerURL: SharedSettings.sharedContainerURL))
    private static let fullCountLimit = 2
    private static let fullCostLimitBytes = 14 * 1024 * 1024
    private static let thumbnailCostLimitBytes = 8 * 1024 * 1024

    private struct Key: Hashable {
        let file: String
        let variant: ThemeImageVariant
    }

    /// The one store both the host app (writer) and this cache (reader) resolve photos through.
    let store: ThemeImageStore
    private let fullCache = NSCache<NSString, UIImage>()
    private let thumbnailCache = NSCache<NSString, UIImage>()
    @MainActor private var inFlight: [Key: Task<UIImage?, Never>] = [:]

    init(store: ThemeImageStore) {
        self.store = store
        fullCache.countLimit = Self.fullCountLimit
        fullCache.totalCostLimit = Self.fullCostLimitBytes
        thumbnailCache.totalCostLimit = Self.thumbnailCostLimitBytes
    }

    /// The already-decoded photo, or nil. Never touches the disk.
    func cached(_ file: String, _ variant: ThemeImageVariant) -> UIImage? {
        cache(for: variant).object(forKey: file as NSString)
    }

    /// The decoded photo, decoding it off the main actor on a miss; nil when the file is
    /// missing / undecodable.
    @MainActor
    func load(_ file: String, _ variant: ThemeImageVariant) async -> UIImage? {
        if let cached = cached(file, variant) {
            return cached
        }
        let key = Key(file: file, variant: variant)
        if let pending = inFlight[key] {
            return await pending.value
        }
        guard let url = store.url(for: file) else { return nil }
        let task = Task.detached(priority: .userInitiated) { Self.decode(url, variant) }
        inFlight[key] = task
        let image = await task.value
        inFlight[key] = nil
        if let image {
            let cost = image.cgImage.map { $0.bytesPerRow * $0.height } ?? 0
            cache(for: variant).setObject(image, forKey: file as NSString, cost: cost)
        }
        return image
    }

    /// Drops every decoded photo (after a sweep removed files, so a deleted theme's photo
    /// does not linger in the host process).
    func removeAll() {
        fullCache.removeAllObjects()
        thumbnailCache.removeAllObjects()
    }

    private func cache(for variant: ThemeImageVariant) -> NSCache<NSString, UIImage> {
        switch variant {
        case .full: fullCache
        case .thumbnail: thumbnailCache
        }
    }

    private static func decode(_ url: URL, _ variant: ThemeImageVariant) -> UIImage? {
        switch variant {
        case .full:
            UIImage(contentsOfFile: url.path)?.preparingForDisplay()
        case .thumbnail:
            (try? Data(contentsOf: url)).flatMap { ThemeImageStore.downsampled($0, maxLongEdge: ThemeImageVariant.thumbnailLongEdge) }
        }
    }
}

/// Shows `content` with the decoded theme photo, or `placeholder` until it is decoded (or
/// when the file is missing). A cache hit renders the photo on the first frame; a miss
/// loads it off the main actor and swaps it in.
struct ThemePhotoImage<Content: View, Placeholder: View>: View {
    let file: String
    let variant: ThemeImageVariant
    @ViewBuilder let content: (UIImage) -> Content
    @ViewBuilder let placeholder: () -> Placeholder

    private struct Loaded {
        let file: String
        let variant: ThemeImageVariant
        let image: UIImage
    }

    @State private var loaded: Loaded?

    var body: some View {
        Group {
            if let image = ThemeImageCache.shared.cached(file, variant) ?? loadedImage {
                content(image)
            } else {
                placeholder()
            }
        }
        .task(id: "\(file)|\(variant)") {
            guard ThemeImageCache.shared.cached(file, variant) == nil,
                  let image = await ThemeImageCache.shared.load(file, variant) else { return }
            loaded = Loaded(file: file, variant: variant, image: image)
        }
    }

    /// The loaded image only while it still belongs to the current file / variant.
    private var loadedImage: UIImage? {
        guard let loaded, loaded.file == file, loaded.variant == variant else { return nil }
        return loaded.image
    }
}
