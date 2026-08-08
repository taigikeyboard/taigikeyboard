
package com.siansiansu.taigikeyboard.ime.media.emoji

// Loads the shared emoji set (taigi-emojis dist/emoji.json) into the IME's emoji-palette model
import android.content.Context
import android.graphics.Paint
import android.graphics.Typeface
import androidx.core.graphics.PaintCompat
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import java.util.EnumMap
import java.util.Locale

private const val TAG = "EmojiLayoutData"

// Bundled by Gradle: android/app sourceSets mounts the in-repo taigi-emojis/dist dir as an
// assets source, so dist/emoji.json lands at the assets root. Single source of truth.
private const val EMOJI_JSON_ASSET = "emoji.json"

typealias EmojiLayoutDataMap = EnumMap<EmojiCategory, MutableList<EmojiSet>>

// Moshi DTOs mirroring taigi-emojis dist/emoji.json (schema frozen by that repo's output-contract +
// drift test). Only the fields this IME consumes are declared; Moshi ignores the rest (cp, subgroup,
// version, keywordsByLocale).
private data class EmojiJsonRoot(
    val categories: List<EmojiJsonCategory> = emptyList(),
)

private data class EmojiJsonCategory(
    val id: String = "",
    val emoji: List<EmojiJsonEmoji> = emptyList(),
)

private data class EmojiJsonEmoji(
    val base: String = "",
    val name: String = "",
    val variations: List<String> = emptyList(),
    val keywords: List<String> = emptyList(),
)

private val emojiJsonAdapter by lazy {
    Moshi
        .Builder()
        .add(KotlinJsonAdapterFactory())
        .build()
        .adapter(EmojiJsonRoot::class.java)
}

// 將 emoji 字串轉換為 code points 列表
private fun emojiStringToCodePoints(emoji: String): List<Int> {
    val codePoints = mutableListOf<Int>()
    var i = 0
    while (i < emoji.length) {
        val codePoint = emoji.codePointAt(i)
        codePoints.add(codePoint)
        i += Character.charCount(codePoint)
    }
    return codePoints
}

private fun EmojiJsonEmoji.toKeyData(glyph: String): EmojiKeyData =
    EmojiKeyData(emojiStringToCodePoints(glyph), name, name, keywords)

/**
 * Loads the bundled `emoji.json` (taigi-emojis) into an [EmojiLayoutDataMap].
 *
 * Category ids in the JSON (`smileys_emotion`, `people_body`, …) map 1:1 to [EmojiCategory]
 * once upper-cased. Each emoji becomes an [EmojiSet] of `[base] + variations`; entries whose
 * glyph the system font cannot render are dropped (a non-renderable base drops the whole set,
 * matching the previous root.txt behavior).
 */
fun loadEmojiLayoutData(context: Context): EmojiLayoutDataMap {
    val logger = CompositionRoot.shared(context).logger
    val layouts = EmojiLayoutDataMap(EmojiCategory::class.java)
    for (category in EmojiCategory.entries) {
        layouts[category] = mutableListOf()
    }

    val root =
        try {
            context.assets
                .open(EMOJI_JSON_ASSET)
                .bufferedReader()
                .use { emojiJsonAdapter.fromJson(it.readText()) }
        } catch (e: Exception) {
            logger.e(TAG, "[PARSE] loadEmojiLayoutData(): $e")
            null
        } ?: return layouts

    val paint = Paint().apply { typeface = Typeface.DEFAULT }

    for (category in root.categories) {
        val ec =
            try {
                EmojiCategory.valueOf(category.id.uppercase(Locale.ENGLISH))
            } catch (e: IllegalArgumentException) {
                logger.w(TAG, "[PARSE] Unknown category: ${category.id}")
                continue
            }

        for (emoji in category.emoji) {
            val baseKey = emoji.toKeyData(emoji.base)
            // Glyph gate: skip the whole set if the device font can't render the base emoji.
            if (!PaintCompat.hasGlyph(paint, baseKey.getCodePointsAsString())) continue

            val keys = mutableListOf(baseKey)
            for (variation in emoji.variations) {
                val variationKey = emoji.toKeyData(variation)
                if (PaintCompat.hasGlyph(paint, variationKey.getCodePointsAsString())) {
                    keys.add(variationKey)
                }
            }
            layouts[ec]?.add(EmojiSet(keys))
        }
    }
    return layouts
}
