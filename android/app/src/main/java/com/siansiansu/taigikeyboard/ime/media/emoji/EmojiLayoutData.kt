
package com.siansiansu.taigikeyboard.ime.media.emoji

import android.content.Context
import android.graphics.Paint
import android.graphics.Typeface
import androidx.core.graphics.PaintCompat
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import java.io.BufferedReader
import java.io.IOException
import java.io.InputStreamReader
import java.lang.Exception
import java.util.*

private const val TAG = "EmojiLayoutData"

// CLDR 格式的分類標記
private const val CATEGORY_START = "["
private const val CATEGORY_END = "]"

typealias EmojiLayoutDataMap = EnumMap<EmojiCategory, MutableList<EmojiSet>>

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

fun parseRawEmojiSpecsFile(
    context: Context,
    path: String,
): EmojiLayoutDataMap {
    val logger = CompositionRoot.shared(context).logger
    val layouts = EmojiLayoutDataMap(EmojiCategory::class.java)
    for (category in EmojiCategory.entries) {
        layouts[category] = mutableListOf()
    }
    var reader: BufferedReader? = null
    try {
        reader = BufferedReader(
            InputStreamReader(context.assets.open(path)),
        )
        val paint = Paint().apply {
            typeface = Typeface.DEFAULT
        }
        var ec: EmojiCategory? = null
        var emojiEditorList: MutableList<EmojiKeyData>? = null

        fun commitEmojiEditorList() {
            emojiEditorList?.let {
                if (it.isNotEmpty()) {
                    layouts[ec]?.add(EmojiSet(it.toList()))
                }
            }
            emojiEditorList = null
        }

        for (line in reader.readLines()) {
            // 處理註解行
            if (line.startsWith("#")) {
                continue
            }

            // 處理分類標記 [category]
            if (line.startsWith(CATEGORY_START) && line.endsWith(CATEGORY_END)) {
                commitEmojiEditorList()
                val categoryId = line.substring(1, line.length - 1)
                ec = try {
                    EmojiCategory.valueOf(categoryId.uppercase(Locale.ENGLISH))
                } catch (e: Exception) {
                    logger.w(TAG, "[PARSE] Unknown category: $categoryId")
                    null
                }
                continue
            }

            // 處理空行
            if (line.trim().isEmpty() || ec == null) {
                continue
            }

            // 處理資料行：emoji;name;keywords 格式
            val isVariation = line.startsWith("\t")
            val data = line.trim().split(";")

            if (data.isNotEmpty()) {
                val emojiStr = data[0].trim()
                if (emojiStr.isEmpty()) continue

                // 轉換 emoji 字串為 code points
                val codePoints = emojiStringToCodePoints(emojiStr)

                // 解析名稱欄位（CLDR 格式中可能為空）
                val base = emojiEditorList?.firstOrNull()
                val name = if (data.size > 1 && data[1].isNotBlank()) {
                    data[1].trim()
                } else {
                    base?.name ?: ""
                }

                // 解析關鍵字欄位（使用 | 分隔）
                val keywords = if (data.size > 2 && data[2].isNotBlank()) {
                    data[2].trim().split("|").map { it.trim() }
                } else {
                    emptyList()
                }

                val key = EmojiKeyData(codePoints, name, name, keywords)

                // 檢查系統字型是否能渲染此 emoji
                if (PaintCompat.hasGlyph(paint, key.getCodePointsAsString())) {
                    if (isVariation) {
                        // 這是一個變體（例如 skin tone），加到編輯列表中
                        emojiEditorList?.add(key)
                    } else {
                        // 這是一個獨立的 emoji，先提交之前的列表
                        commitEmojiEditorList()
                        // 建立新的編輯列表
                        emojiEditorList = mutableListOf(key)
                    }
                } else {
                    // 系統無法渲染此 emoji，如果不是變體則提交並清空列表
                    if (!isVariation) {
                        commitEmojiEditorList()
                    }
                }
            }
        }
        commitEmojiEditorList()
    } catch (e: IOException) {
        logger.e(TAG, "[PARSE] parseRawEmojiSpecsFile(): $e")
    } finally {
        if (reader != null) {
            try {
                reader.close()
            } catch (e: IOException) {
                logger.e(TAG, "[PARSE] parseRawEmojiSpecsFile(): $e")
            }
        }
    }
    return layouts
}
