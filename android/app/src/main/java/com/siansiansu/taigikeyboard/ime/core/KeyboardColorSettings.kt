package com.siansiansu.taigikeyboard.ime.core

import android.graphics.Color
import org.json.JSONObject

/**
 * Custom keyboard color settings.
 *
 * Each color field is nullable -- null means "use default theme color".
 * Stored as JSON in DataStore via colorSettings preference.
 *
 * Matches iOS KeyboardColorSettings structure.
 */
data class KeyboardColorSettings(
    val backgroundColor: Int? = null,
    val keyTextColor: Int? = null,
    val normalKeyFillColor: Int? = null,
    val specialKeyFillColor: Int? = null,
    val candidateTextColor: Int? = null,
    val candidateBackgroundColor: Int? = null,
) {
    fun toJson(): String {
        val json = JSONObject()
        backgroundColor?.let { json.put("backgroundColor", it) }
        keyTextColor?.let { json.put("keyTextColor", it) }
        normalKeyFillColor?.let { json.put("normalKeyFillColor", it) }
        specialKeyFillColor?.let { json.put("specialKeyFillColor", it) }
        candidateTextColor?.let { json.put("candidateTextColor", it) }
        candidateBackgroundColor?.let { json.put("candidateBackgroundColor", it) }
        return json.toString()
    }

    companion object {
        fun fromJson(json: String): KeyboardColorSettings {
            if (json.isBlank() || json == "{}") return KeyboardColorSettings()
            return try {
                val obj = JSONObject(json)
                KeyboardColorSettings(
                    backgroundColor = obj.optIntOrNull("backgroundColor"),
                    keyTextColor = obj.optIntOrNull("keyTextColor"),
                    normalKeyFillColor = obj.optIntOrNull("normalKeyFillColor"),
                    specialKeyFillColor = obj.optIntOrNull("specialKeyFillColor"),
                    candidateTextColor = obj.optIntOrNull("candidateTextColor"),
                    candidateBackgroundColor = obj.optIntOrNull("candidateBackgroundColor"),
                )
            } catch (e: Exception) {
                KeyboardColorSettings()
            }
        }

        private fun JSONObject.optIntOrNull(key: String): Int? {
            return if (has(key)) getInt(key) else null
        }
    }
}
