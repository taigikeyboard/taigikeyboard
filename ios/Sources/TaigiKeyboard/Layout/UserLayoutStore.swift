import Foundation

/// Persists custom layouts as a JSON file in the App Group container, the way `UserThemeStore`
/// keeps user themes: excluded from OS backup (`behavioral-invariants.md` §29), capped, and
/// bumping a cross-process revision (`onMutated`) after every successful write.
final class UserLayoutStore {
    static let maxLayouts = 5
    static let fileName = "user_layouts.json"

    private let fileURL: URL?
    private let onMutated: () -> Void

    /// - Parameters:
    ///   - containerURL: App Group container (nil → the store is empty and every write fails).
    ///   - onMutated: bumps the cross-process layout revision.
    init(containerURL: URL?, onMutated: @escaping () -> Void) {
        fileURL = containerURL?.appendingPathComponent(Self.fileName)
        self.onMutated = onMutated
    }

    /// Saved layouts; `[]` when absent or unreadable.
    func load() -> [CustomLayout] {
        read() ?? []
    }

    /// Replaces the layout with the same id, or — below the cap — adds it right after the layout
    /// with id `anchor` (appended when `anchor` is nil or absent). Returns whether it was written;
    /// a file that cannot be read is never overwritten.
    func save(_ layout: CustomLayout, after anchor: UUID? = nil) -> Bool {
        guard var layouts = read() else { return false }
        if let index = layouts.firstIndex(where: { $0.id == layout.id }) {
            layouts[index] = layout
        } else {
            guard layouts.count < Self.maxLayouts else { return false }
            let index = layouts.firstIndex { $0.id == anchor }.map { $0 + 1 } ?? layouts.endIndex
            layouts.insert(layout, at: index)
        }
        return write(layouts)
    }

    /// Removes the layout with `id`. Returns whether the file now lacks it.
    func delete(id: UUID) -> Bool {
        guard let layouts = read() else { return false }
        let remaining = layouts.filter { $0.id != id }
        return remaining.count == layouts.count || write(remaining)
    }

    // MARK: - Private

    /// One array element decoded on its own, so a malformed record drops alone.
    private struct Record: Decodable {
        let layout: CustomLayout?

        init(from decoder: Decoder) throws {
            layout = try? CustomLayout(from: decoder)
        }
    }

    /// The valid saved layouts (`CustomLayout.isValid`); `[]` when there is no file, nil when the
    /// file exists but is not a JSON array.
    private func read() -> [CustomLayout]? {
        guard let fileURL else { return nil }
        guard FileManager.default.fileExists(atPath: fileURL.path) else { return [] }
        guard let data = try? Data(contentsOf: fileURL),
              let records = try? JSONDecoder().decode([Record].self, from: data)
        else { return nil }
        return records.compactMap(\.layout).filter(\.isValid)
    }

    private func write(_ layouts: [CustomLayout]) -> Bool {
        guard let fileURL, let data = try? JSONEncoder().encode(layouts) else { return false }
        do {
            try data.write(to: fileURL, options: .atomic)
            fileURL.excludeFromBackup()
            onMutated()
            return true
        } catch {
            return false
        }
    }
}
