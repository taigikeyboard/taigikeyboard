// 中文: 單行 CSV 解析與跳脫(RFC 4180 雙引號規則)— 用於頻次 / NextWord 關聯資料的匯出匯入。
// 中文: parseLine + escape 為對等對應,確保 round-trip 不失真。

package com.siansiansu.taigikeyboard.ime.dictionary

// Single-line CSV parsing and escaping for frequency/association data export/import
object DictionaryCsvCodec {
    // Handles RFC 4180 "" escaping so round-trip with escape() is lossless
    fun parseLine(line: String): List<String> {
        val fields = mutableListOf<String>()
        val current = StringBuilder()
        var inQuotes = false
        var i = 0
        while (i < line.length) {
            val char = line[i]
            when {
                char == '"' && inQuotes && i + 1 < line.length && line[i + 1] == '"' -> {
                    current.append('"')
                    i++
                }

                char == '"' -> {
                    inQuotes = !inQuotes
                }

                char == ',' && !inQuotes -> {
                    fields.add(current.toString())
                    current.clear()
                }

                else -> {
                    current.append(char)
                }
            }
            i++
        }
        fields.add(current.toString())
        return fields
    }

    fun escape(field: String): String =
        if (field.contains(",") || field.contains("\"") || field.contains("\n")) {
            "\"${field.replace("\"", "\"\"")}\""
        } else {
            field
        }
}
