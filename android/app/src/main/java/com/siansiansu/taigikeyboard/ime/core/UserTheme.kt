// 中文: 使用者自訂主題 — 身分 + 名稱 + ThemeAppearance + 建立/更新時間(epoch ms)。對齊 iOS UserTheme。
// 中文: 以 JSON list 持久化於 DataStore.userThemes。

package com.siansiansu.taigikeyboard.ime.core

import org.json.JSONArray
import org.json.JSONObject

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
) {
    fun toJson(): JSONObject =
        JSONObject().apply {
            put("id", id)
            put("name", name)
            put("appearance", appearance.toJson())
            put("createdAt", createdAt)
            put("updatedAt", updatedAt)
        }

    companion object {
        /** Decodes one object; null when the id is missing (a malformed entry). */
        fun fromJson(obj: JSONObject): UserTheme? {
            val id = obj.optString("id").takeIf { it.isNotBlank() } ?: return null
            return UserTheme(
                id = id,
                name = obj.optString("name"),
                appearance = obj.optJSONObject("appearance")?.let { ThemeAppearance.fromJson(it) }
                    ?: ThemeAppearance.DEFAULT,
                createdAt = obj.optLong("createdAt"),
                updatedAt = obj.optLong("updatedAt"),
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
