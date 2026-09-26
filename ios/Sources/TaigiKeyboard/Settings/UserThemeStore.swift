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
    static let fileName = "user_themes.json"

    private let fileURL: URL?
    private let onMutated: () -> Void

    /// - Parameters:
    ///   - containerURL: App Group container (nil if provisioning failed →
    ///     store degrades to empty, never crashes).
    ///   - onMutated: bumps the cross-process theme-revision counter.
    init(containerURL: URL?, onMutated: @escaping () -> Void) {
        fileURL = containerURL?.appendingPathComponent(Self.fileName)
        self.onMutated = onMutated
    }

    /// Loads all persisted themes; returns `[]` when absent or corrupt. Every theme's
    /// `nil` color roles are filled from `UserThemeSeed` so a theme saved before the
    /// seed existed is scheme-invariant too (no migration write).
    func load() -> [UserTheme] {
        guard let fileURL, let data = try? Data(contentsOf: fileURL) else { return [] }
        let themes = (try? JSONDecoder().decode([UserTheme].self, from: data)) ?? []
        return themes.map { theme in
            var seeded = theme
            seeded.appearance.colors = theme.appearance.colors.seededForUserTheme()
            return seeded
        }
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
            fileURL.excludeFromBackup()
            onMutated()
            return true
        } catch {
            return false
        }
    }
}

extension URL {
    /// Marks the file or directory excluded from OS backup (best-effort). User data — theme
    /// JSON, theme photos — is excluded like the user-data databases
    /// (`UserDataOpening`), see `behavioral-invariants.md` §29.
    func excludeFromBackup() {
        var url = self
        var resourceValues = URLResourceValues()
        resourceValues.isExcludedFromBackup = true
        try? url.setResourceValues(resourceValues)
    }
}
