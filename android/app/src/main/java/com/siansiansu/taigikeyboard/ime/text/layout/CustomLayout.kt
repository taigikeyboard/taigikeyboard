// A user-made romanization layout: a built-in key table with its keys rearranged, and the JSON list
// it is stored in (PrefHelper, DataStore). The record matches iOS `CustomLayout` field for field;
// the tables it is checked against live in CustomLayoutTables. Mirrors iOS Layout/CustomLayout.swift.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import org.json.JSONArray
import org.json.JSONObject

/**
 * The script of a key table: Lohankha and the MOE layouts have different TL and POJ tables. The
 * stored value is its input mode. Mirrors iOS `LayoutScript`.
 */
enum class LayoutScript(
    val inputMode: String,
) {
    TL("tl"),
    POJ("poj"),
    ;

    companion object {
        fun fromStorage(value: String): LayoutScript? = entries.firstOrNull { it.inputMode == value }
    }
}

/** A function key as stored in a record (iOS `CustomLayout.FunctionKey` raw values), and the key code it stands for. */
enum class FunctionKey(
    val storageValue: String,
    val code: Int,
) {
    SHIFT("shift", KeyCode.SHIFT),
    BACKSPACE("backspace", KeyCode.DELETE),
    NUMERIC("numeric", KeyCode.VIEW_SYMBOLS),
    EMOJI("emoji", KeyCode.SWITCH_TO_MEDIA_CONTEXT),
    SPACE("space", KeyCode.SPACE),
    TRANSLATE("translate", KeyCode.TRANSLATE),
    RETURN("return", KeyCode.ENTER),
    ;

    companion object {
        fun ofCode(code: Int): FunctionKey? = entries.firstOrNull { it.code == code }

        fun fromStorage(value: String): FunctionKey? = entries.firstOrNull { it.storageValue == value }
    }
}

/**
 * A custom layout over the [base] × [script] table, below its digit row:
 * - [keys] fills the character keys in reading order; null marks a removed key.
 * - [functionKeys] fills the function keys in reading order; each stays in its own row.
 * - [hasEmojiKey] false hides the emoji key, the one function key that can be removed.
 * The globe key and the email / URL key are not part of it: the keyboard places them.
 */
data class CustomLayout(
    val id: String,
    val name: String,
    val base: String,
    val script: LayoutScript,
    val keys: List<String?>,
    val functionKeys: List<FunctionKey>,
    val hasEmojiKey: Boolean,
) {
    fun toJson(): JSONObject =
        JSONObject().apply {
            put("id", id)
            put("name", name)
            put("base", base)
            put("script", script.inputMode)
            put("keys", JSONArray().apply { keys.forEach { put(it ?: JSONObject.NULL) } })
            put("functionKeys", JSONArray().apply { functionKeys.forEach { put(it.storageValue) } })
            put("hasEmojiKey", hasEmojiKey)
        }

    companion object {
        /** The most custom layouts a user can keep, as for user themes. */
        const val MAX_CUSTOM_LAYOUTS = 5

        /**
         * Decodes the stored list, dropping each record that fails to decode or that [tables] cannot
         * show ([CustomLayoutTables.isValid]); the others stay. A record saved before function keys
         * could move takes the base's order; one saved before the emoji key could go keeps it.
         */
        fun decodeList(
            json: String,
            tables: CustomLayoutTables,
        ): List<CustomLayout> {
            val array = runCatching { JSONArray(json) }.getOrNull() ?: return emptyList()
            return (0 until array.length()).mapNotNull { index ->
                runCatching { fromJson(array.getJSONObject(index), tables) }.getOrNull()?.takeIf(tables::isValid)
            }
        }

        fun encodeList(layouts: List<CustomLayout>): String = JSONArray().apply { layouts.forEach { put(it.toJson()) } }.toString()

        /**
         * The selected layout in [layouts] while its base is the stored layout: TPS entered through
         * the input mode suspends it until leaving TPS restores the base.
         */
        fun active(
            layouts: List<CustomLayout>,
            selectedId: String,
            layoutType: String,
        ): CustomLayout? = layouts.firstOrNull { it.id == selectedId && it.base == layoutType }

        /** [layouts] with [layout] in place of its saved version, or added; null when adding passes the cap. */
        fun upserted(
            layouts: List<CustomLayout>,
            layout: CustomLayout,
        ): List<CustomLayout>? {
            val index = layouts.indexOfFirst { it.id == layout.id }
            return when {
                index >= 0 -> layouts.toMutableList().also { it[index] = layout }
                layouts.size >= MAX_CUSTOM_LAYOUTS -> null
                else -> layouts + layout
            }
        }

        private fun fromJson(
            json: JSONObject,
            tables: CustomLayoutTables,
        ): CustomLayout? {
            val base = json.getString("base")
            val script = LayoutScript.fromStorage(json.getString("script")) ?: return null
            val keys = json.getJSONArray("keys")
            val functionKeys = json.optJSONArray("functionKeys")
            return CustomLayout(
                id = json.getString("id"),
                name = json.getString("name"),
                base = base,
                script = script,
                keys = (0 until keys.length()).map { if (keys.isNull(it)) null else keys.get(it) as String },
                functionKeys =
                    if (functionKeys == null) {
                        tables.baseTable(base, script)?.functionKeys ?: return null
                    } else {
                        (0 until functionKeys.length()).map { FunctionKey.fromStorage(functionKeys.getString(it)) ?: return null }
                    },
                hasEmojiKey = json.optBoolean("hasEmojiKey", true),
            )
        }
    }
}
