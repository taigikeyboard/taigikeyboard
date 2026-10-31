package com.siansiansu.taigikeyboard.ime.theme

import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

/** The theme-photo file names the saved themes still reference — what a sweep keeps. */
fun List<UserTheme>.referencedPhotoFiles(): Set<String> =
    mapNotNullTo(mutableSetOf()) {
        it.appearance.colors.background
            ?.asImage
            ?.file
    }

/**
 * A user-created, named, persisted keyboard theme: id + name + the full
 * [ThemeAppearance] bundle + epoch-millis timestamps. Persisted as a JSON list
 * in DataStore (`PrefHelper.userThemes`). Mirrors iOS UserTheme (Android stores
 * the id as a UUID string and timestamps as epoch millis instead of Swift
 * `UUID` / `Date`).
 */
data class UserTheme(
    val id: String,
    val name: String,
    val appearance: ThemeAppearance,
    val createdAt: Long,
    val updatedAt: Long,
    // The built-in theme this one was customized from (its reset target), or null for a theme
    // started blank. Absent in themes saved before the field existed -> null.
    val baseThemeId: String? = null,
) {
    fun toJson(): JSONObject =
        JSONObject().apply {
            put("id", id)
            put("name", name)
            put("appearance", appearance.toJson())
            put("createdAt", createdAt)
            put("updatedAt", updatedAt)
            baseThemeId?.let { put("baseThemeId", it) }
        }

    /**
     * This theme under a new id and [name], stamped [now] — what the Duplicate card action saves.
     * A photo background shares the original's file: the photo sweep keeps every file a saved theme
     * still references, so deleting either theme leaves the other's photo in place. Mirrors iOS
     * `UserTheme.duplicated(named:now:)`.
     */
    fun duplicated(
        name: String,
        now: Long = System.currentTimeMillis(),
    ): UserTheme = copy(id = UUID.randomUUID().toString(), name = name, createdAt = now, updatedAt = now)

    companion object {
        /**
         * Decodes one object; null when the id is missing (a malformed entry). Every null
         * color role is filled from [UserThemeSeed] here — the single JSON -> model
         * boundary — so a theme saved before the seed existed is scheme-invariant on every
         * read path (store, `PrefHelper.loadUserThemes`, `observeUserThemes`) with no
         * migration write. (iOS seeds in `UserThemeStore.load`, its single read path.)
         */
        fun fromJson(obj: JSONObject): UserTheme? {
            val id = obj.optString("id").takeIf { it.isNotBlank() } ?: return null
            val appearance =
                obj.optJSONObject("appearance")?.let { ThemeAppearance.fromJson(it) } ?: ThemeAppearance.DEFAULT
            return UserTheme(
                id = id,
                name = obj.optString("name"),
                appearance = appearance.copy(colors = appearance.colors.seededForUserTheme()),
                createdAt = obj.optLong("createdAt"),
                updatedAt = obj.optLong("updatedAt"),
                // isNull first: optString turns a JSON null into the string "null".
                baseThemeId = if (obj.isNull("baseThemeId")) null else obj.optString("baseThemeId").takeIf { it.isNotBlank() },
            )
        }

        /** Decodes a JSON array string into a theme list; empty/corrupt -> []. */
        fun decodeList(json: String): List<UserTheme> {
            if (json.isBlank() || json == "[]") return emptyList()
            return try {
                val array = JSONArray(json)
                (0 until array.length()).mapNotNull { index ->
                    array.optJSONObject(index)?.let { fromJson(it) }
                }
            } catch (e: Exception) {
                emptyList()
            }
        }

        /** Encodes a theme list to a JSON array string. */
        fun encodeList(themes: List<UserTheme>): String {
            val array = JSONArray()
            themes.forEach { array.put(it.toJson()) }
            return array.toString()
        }
    }
}
