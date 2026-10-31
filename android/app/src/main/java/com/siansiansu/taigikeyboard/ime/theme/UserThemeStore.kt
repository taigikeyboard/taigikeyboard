package com.siansiansu.taigikeyboard.ime.theme

/**
 * Persists user-created themes as a JSON list. I/O is injected via [read] /
 * [write] lambdas (DataStore-backed in production, in-memory in tests) so the
 * store stays free of platform persistence APIs and is directly unit-testable.
 *
 * Cap: at most [MAX_USER_THEMES] themes (USER 2026-06-07). At cap, [add] no-ops
 * and returns false. Mirrors iOS UserThemeStore — Android persists via DataStore
 * JSON instead of an App Group file and bumps no revision counter (idle refresh
 * is handled by an appearance-republish Flow; see docs/ui/android-theme-port.md).
 */
class UserThemeStore(
    private val read: () -> String,
    private val write: (String) -> Unit,
    /** Runs after every persisted mutation with the new theme list (photo sweep; mirrors iOS `onMutated`). */
    private val onMutated: (List<UserTheme>) -> Unit = {},
) {
    /** Loads all persisted themes (seeded at decode, see [UserTheme.fromJson]); [] when absent or corrupt. */
    fun load(): List<UserTheme> = UserTheme.decodeList(read())

    /**
     * Adds a theme right after the theme with id [anchorId] (appended when null or absent) and
     * persists. Returns false when already at [MAX_USER_THEMES]; true means accepted into the
     * write path (the DataStore-backed [write] updates the in-memory cache synchronously,
     * persists asynchronously — unlike iOS where false also signals a failed durable write).
     */
    fun add(
        theme: UserTheme,
        anchorId: String? = null,
    ): Boolean {
        val themes = load()
        if (themes.size >= MAX_USER_THEMES) return false
        val anchor = themes.indexOfFirst { it.id == anchorId }
        persist(themes.toMutableList().also { it.add(if (anchor >= 0) anchor + 1 else it.size, theme) })
        return true
    }

    /** Replaces the theme with the same id (no-op if absent). */
    fun update(theme: UserTheme) {
        val themes = load()
        val index = themes.indexOfFirst { it.id == theme.id }
        if (index < 0) return
        persist(themes.toMutableList().also { it[index] = theme })
    }

    /** Removes the theme with [id] (no-op if absent). */
    fun delete(id: String) {
        val themes = load()
        val filtered = themes.filterNot { it.id == id }
        if (filtered.size == themes.size) return
        persist(filtered)
    }

    private fun persist(themes: List<UserTheme>) {
        write(UserTheme.encodeList(themes))
        onMutated(themes)
    }

    companion object {
        /** Maximum number of user themes (USER 2026-06-07). At cap, [add] no-ops. */
        const val MAX_USER_THEMES = 5
    }
}
