import Foundation

/// Persists user-created themes as a JSON file in the App Group container.
///
/// Design constraints (per `docs/ui/theme-presets-brainstorm.md`):
/// - **Excluded from OS backup** (F-Exclude): the file is marked
///   `isExcludedFromBackup`, uniform with the user-data DBs. A `UserDefaults`
///   plist cannot be selectively excluded, so themes live in a standalone file.
/// - **Cross-process refresh**: every mutation bumps a `UserDefaults` revision
///   counter (`onMutated`) so the keyboard extension re-resolves on its next
///   render even when `selectedThemeId` is unchanged (active-theme edit).
/// - **Cap**: at most ``maxUserThemes`` themes (USER 2026-06-07).
///
/// Reads go straight to disk (`load()`) — callers are infrequent (resolver on
/// init + `didChangeNotification`), so no in-memory cache is needed and the
/// host-app writer / extension reader stay consistent without invalidation.
final class UserThemeStore {
    /// Maximum number of user themes (USER 2026-06-07). At cap, `add` no-ops.
    static let maxUserThemes = 5

    private let fileURL: URL?
    private let onMutated: () -> Void

    /// - Parameters:
    ///   - containerURL: App Group container (nil if provisioning failed →
    ///     store degrades to empty, never crashes).
    ///   - onMutated: bumps the cross-process theme-revision counter.
    init(containerURL: URL?, onMutated: @escaping () -> Void) {
        fileURL = containerURL?.appendingPathComponent("user_themes.json")
        self.onMutated = onMutated
    }

    /// Loads all persisted themes; returns `[]` when absent or corrupt.
    func load() -> [UserTheme] {
        guard let fileURL, let data = try? Data(contentsOf: fileURL) else { return [] }
        return (try? JSONDecoder().decode([UserTheme].self, from: data)) ?? []
    }

    /// Appends a theme and persists. Returns `false` when already at
    /// ``maxUserThemes`` OR when the write fails (nil container / encode /
    /// I/O error) — so callers never report a save that did not happen.
    @discardableResult
    func add(_ theme: UserTheme) -> Bool {
        var themes = load()
        guard themes.count < Self.maxUserThemes else { return false }
        themes.append(theme)
        return save(themes)
    }

    /// Replaces the theme with the same `id` (no-op if absent).
    func update(_ theme: UserTheme) {
        var themes = load()
        guard let index = themes.firstIndex(where: { $0.id == theme.id }) else { return }
        themes[index] = theme
        save(themes)
    }

    /// Removes the theme with the given `id` (no-op if absent).
    func delete(id: UUID) {
        var themes = load()
        let filtered = themes.filter { $0.id != id }
        guard filtered.count != themes.count else { return }
        themes = filtered
        save(themes)
    }

    // MARK: - Private

    /// Persists the list. Returns `false` when there is no container, encoding
    /// fails, or the write throws — leaving any prior file intact.
    @discardableResult
    private func save(_ themes: [UserTheme]) -> Bool {
        guard let fileURL else { return false }
        guard let data = try? JSONEncoder().encode(themes) else { return false }
        do {
            try data.write(to: fileURL, options: .atomic)
            excludeFromBackup(fileURL)
            onMutated()
            return true
        } catch {
            return false
        }
    }

    /// Marks the file excluded from OS backup (best-effort; mirrors
    /// `SQLiteConnectionManager`'s user-data exclusion).
    private func excludeFromBackup(_ url: URL) {
        var url = url
        var resourceValues = URLResourceValues()
        resourceValues.isExcludedFromBackup = true
        try? url.setResourceValues(resourceValues)
    }
}
