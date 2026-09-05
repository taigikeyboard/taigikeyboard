package com.siansiansu.taigikeyboard.ime.dictionary

// Single-line CSV parsing and escaping for frequency/association data export/import
object DictionaryCsvCodec {
    // Handles RFC 4180 "" escaping so round-trip with escape() is lossless.
    // CROSS-PLATFORM INVARIANT — mirrors ios/.../App/Tabs/Dictionary/Utilities/CSVDocument.swift parseLine.
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

    // Frequency CSV: word,tl,count carrying the (漢字, 羅馬字) pair (Core Principle #7).
    // CROSS-PLATFORM INVARIANT — mirrors ios/.../App/Tabs/Dictionary/Utilities/CSVParsers.swift
    // (encode/decodeFrequencyCSV). Drift causes silent divergence; both serialize byte-for-byte.
    // Pins INVARIANT_USER_FREQ_PAIR_KEY (docs/architecture/behavioral-invariants.md §28).
    fun encodeFrequencyCSV(rows: List<Triple<String, String, Int>>): String =
        buildString {
            for ((word, tl, count) in rows) {
                append("${escape(word)},${escape(tl)},$count\n")
            }
        }

    // 3 columns → (word, tl, count); legacy 2 columns → (word, "", count);
    // any other column count is skipped (exact discriminator, not >= 2, so a
    // malformed row is never silently eaten as legacy).
    fun decodeFrequencyCSV(csv: String): List<Triple<String, String, Int>> {
        val entries = mutableListOf<Triple<String, String, Int>>()
        for (line in csv.split("\n")) {
            val trimmed = line.trim()
            if (trimmed.isEmpty()) continue
            val columns = parseLine(trimmed).map { it.trim() }
            val word: String
            val tl: String
            val countField: String
            when (columns.size) {
                3 -> {
                    word = columns[0]
                    tl = columns[1]
                    countField = columns[2]
                }
                2 -> {
                    word = columns[0]
                    tl = ""
                    countField = columns[1]
                }
                else -> continue
            }
            val count = countField.toIntOrNull() ?: continue
            if (word.isEmpty() || count <= 0) continue
            entries.add(Triple(word, tl, count))
        }
        return entries
    }
}
