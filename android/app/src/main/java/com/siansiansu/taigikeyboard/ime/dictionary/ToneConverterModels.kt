// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * Models and constants for tone conversion between number notation and tone marks
 */
object ToneConverterModels {
    /**
     * Input mode for romanization systems
     */
    enum class InputMode {
        /** Pe̍h-ōe-jī (白話字) */
        POJ,

        /** Tâi-lô (台羅) */
        TL,

        /** English passthrough */
        ENGLISH,
    }

    /**
     * POJ number to tone mapping: converts base character + tone number to tone marked character
     */
    val pojNumberToToneMapping =
        mapOf(
            // a (tones 2-9)
            "a2" to "á",
            "a3" to "à",
            "a5" to "â",
            "a6" to "ǎ",
            "a7" to "ā",
            "a8" to "a̍",
            "a9" to "ă",
            "A2" to "Á",
            "A3" to "À",
            "A5" to "Â",
            "A6" to "Ǎ",
            "A7" to "Ā",
            "A8" to "A̍",
            "A9" to "Ă",
            // e (tones 2-9)
            "e2" to "é",
            "e3" to "è",
            "e5" to "ê",
            "e6" to "ě",
            "e7" to "ē",
            "e8" to "e̍",
            "e9" to "ĕ",
            "E2" to "É",
            "E3" to "È",
            "E5" to "Ê",
            "E6" to "Ě",
            "E7" to "Ē",
            "E8" to "E̍",
            "E9" to "Ĕ",
            // i (tones 2-9)
            "i2" to "í",
            "i3" to "ì",
            "i5" to "î",
            "i6" to "ǐ",
            "i7" to "ī",
            "i8" to "i̍",
            "i9" to "ĭ",
            "I2" to "Í",
            "I3" to "Ì",
            "I5" to "Î",
            "I6" to "Ǐ",
            "I7" to "Ī",
            "I8" to "I̍",
            "I9" to "Ĭ",
            // o (tones 2-9)
            "o2" to "ó",
            "o3" to "ò",
            "o5" to "ô",
            "o6" to "ǒ",
            "o7" to "ō",
            "o8" to "o̍",
            "o9" to "ŏ",
            "O2" to "Ó",
            "O3" to "Ò",
            "O5" to "Ô",
            "O6" to "Ǒ",
            "O7" to "Ō",
            "O8" to "O̍",
            "O9" to "Ŏ",
            // o͘ (tones 2-9)
            "o͘2" to "ó͘",
            "o͘3" to "ò͘",
            "o͘5" to "ô͘",
            "o͘6" to "ǒ͘",
            "o͘7" to "ō͘",
            "o͘8" to "o̍͘",
            "o͘9" to "ŏ͘",
            "O͘2" to "Ó͘",
            "O͘3" to "Ò͘",
            "O͘5" to "Ô͘",
            "O͘6" to "Ǒ͘",
            "O͘7" to "Ō͘",
            "O͘8" to "O̍͘",
            "O͘9" to "Ŏ͘",
            // u (tones 2-9)
            "u2" to "ú",
            "u3" to "ù",
            "u5" to "û",
            "u6" to "ǔ",
            "u7" to "ū",
            "u8" to "u̍",
            "u9" to "ŭ",
            "U2" to "Ú",
            "U3" to "Ù",
            "U5" to "Û",
            "U6" to "Ǔ",
            "U7" to "Ū",
            "U8" to "U̍",
            "U9" to "Ŭ",
            // n (tones 2-9)
            "n2" to "ń",
            "n3" to "ǹ",
            "n5" to "n̂",
            "n6" to "ň",
            "n7" to "n̄",
            "n8" to "n̍",
            "n9" to "n̋",
            "N2" to "Ń",
            "N3" to "Ǹ",
            "N5" to "N̂",
            "N6" to "Ň",
            "N7" to "N̄",
            "N8" to "N̍",
            "N9" to "N̋",
            // ng (tones 2-9)
            "ng2" to "ńg",
            "ng3" to "ǹg",
            "ng5" to "n̂g",
            "ng6" to "ňg",
            "ng7" to "n̄g",
            "ng8" to "n̍g",
            "ng9" to "n̋g",
            "Ng2" to "Ńg",
            "Ng3" to "Ǹg",
            "Ng5" to "N̂g",
            "Ng6" to "Ňg",
            "Ng7" to "N̄g",
            "Ng8" to "N̍g",
            "Ng9" to "N̋g",
            // m (tones 2-9)
            "m2" to "ḿ",
            "m3" to "m̀",
            "m5" to "m̂",
            "m6" to "m̌",
            "m7" to "m̄",
            "m8" to "m̍",
            "m9" to "m̋",
            "M2" to "Ḿ",
            "M3" to "M̀",
            "M5" to "M̂",
            "M6" to "M̌",
            "M7" to "M̄",
            "M8" to "M̍",
            "M9" to "M̋",
        )

    /**
     * TL number to tone mapping: converts base character + tone number to tone marked character
     */
    val tlNumberToToneMapping =
        mapOf(
            // a (tones 2-9)
            "a2" to "á",
            "a3" to "à",
            "a5" to "â",
            "a6" to "ǎ",
            "a7" to "ā",
            "a8" to "a̍",
            "a9" to "a̋",
            "A2" to "Á",
            "A3" to "À",
            "A5" to "Â",
            "A6" to "Ǎ",
            "A7" to "Ā",
            "A8" to "A̍",
            "A9" to "A̋",
            // e (tones 2-9)
            "e2" to "é",
            "e3" to "è",
            "e5" to "ê",
            "e6" to "ě",
            "e7" to "ē",
            "e8" to "e̍",
            "e9" to "e̋",
            "E2" to "É",
            "E3" to "È",
            "E5" to "Ê",
            "E6" to "Ě",
            "E7" to "Ē",
            "E8" to "E̍",
            "E9" to "E̋",
            // i (tones 2-9)
            "i2" to "í",
            "i3" to "ì",
            "i5" to "î",
            "i6" to "ǐ",
            "i7" to "ī",
            "i8" to "i̍",
            "i9" to "i̋",
            "I2" to "Í",
            "I3" to "Ì",
            "I5" to "Î",
            "I6" to "Ǐ",
            "I7" to "Ī",
            "I8" to "I̍",
            "I9" to "I̋",
            // o (tones 2-9)
            "o2" to "ó",
            "o3" to "ò",
            "o5" to "ô",
            "o6" to "ǒ",
            "o7" to "ō",
            "o8" to "o̍",
            "o9" to "ő",
            "O2" to "Ó",
            "O3" to "Ò",
            "O5" to "Ô",
            "O6" to "Ǒ",
            "O7" to "Ō",
            "O8" to "O̍",
            "O9" to "Ő",
            // oo (tones 2-9)
            "oo2" to "óo",
            "oo3" to "òo",
            "oo5" to "ôo",
            "oo6" to "ǒo",
            "oo7" to "ōo",
            "oo8" to "o̍o",
            "oo9" to "őo",
            "Oo2" to "Óo",
            "Oo3" to "Òo",
            "Oo5" to "Ôo",
            "Oo6" to "Ǒo",
            "Oo7" to "Ōo",
            "Oo8" to "O̍o",
            "Oo9" to "Őo",
            // u (tones 2-9)
            "u2" to "ú",
            "u3" to "ù",
            "u5" to "û",
            "u6" to "ǔ",
            "u7" to "ū",
            "u8" to "u̍",
            "u9" to "ű",
            "U2" to "Ú",
            "U3" to "Ù",
            "U5" to "Û",
            "U6" to "Ǔ",
            "U7" to "Ū",
            "U8" to "U̍",
            "U9" to "Ű",
            // n (tones 2-9)
            "n2" to "ń",
            "n3" to "ǹ",
            "n5" to "n̂",
            "n6" to "ň",
            "n7" to "n̄",
            "n8" to "n̍",
            "n9" to "n̋",
            "N2" to "Ń",
            "N3" to "Ǹ",
            "N5" to "N̂",
            "N6" to "Ň",
            "N7" to "N̄",
            "N8" to "N̍",
            "N9" to "N̋",
            // ng (tones 2-9)
            "ng2" to "ńg",
            "ng3" to "ǹg",
            "ng5" to "n̂g",
            "ng6" to "ňg",
            "ng7" to "n̄g",
            "ng8" to "n̍g",
            "ng9" to "n̋g",
            "Ng2" to "Ńg",
            "Ng3" to "Ǹg",
            "Ng5" to "N̂g",
            "Ng6" to "Ňg",
            "Ng7" to "N̄g",
            "Ng8" to "N̍g",
            "Ng9" to "N̋g",
            // m (tones 2-9)
            "m2" to "ḿ",
            "m3" to "m̀",
            "m5" to "m̂",
            "m6" to "m̌",
            "m7" to "m̄",
            "m8" to "m̍",
            "m9" to "m̋",
            "M2" to "Ḿ",
            "M3" to "M̀",
            "M5" to "M̂",
            "M6" to "M̌",
            "M7" to "M̄",
            "M8" to "M̍",
            "M9" to "M̋",
        )

    /**
     * POJ lowercase to uppercase tone letter mapping
     */
    val pojLowercaseToUppercaseMapping =
        mapOf(
            // a
            "á" to "Á",
            "à" to "À",
            "â" to "Â",
            "ǎ" to "Ǎ",
            "ā" to "Ā",
            "a̍" to "A̍",
            "ă" to "Ă",
            // e
            "é" to "É",
            "è" to "È",
            "ê" to "Ê",
            "ě" to "Ě",
            "ē" to "Ē",
            "e̍" to "E̍",
            "ĕ" to "Ĕ",
            // i
            "í" to "Í",
            "ì" to "Ì",
            "î" to "Î",
            "ǐ" to "Ǐ",
            "ī" to "Ī",
            "i̍" to "I̍",
            "ĭ" to "Ĭ",
            // o
            "ó" to "Ó",
            "ò" to "Ò",
            "ô" to "Ô",
            "ǒ" to "Ǒ",
            "ō" to "Ō",
            "o̍" to "O̍",
            "ŏ" to "Ŏ",
            // o͘
            "ó͘" to "Ó͘",
            "ò͘" to "Ò͘",
            "ô͘" to "Ô͘",
            "ǒ͘" to "Ǒ͘",
            "ō͘" to "Ō͘",
            "o̍͘" to "O̍͘",
            "ŏ͘" to "Ŏ͘",
            // u
            "ú" to "Ú",
            "ù" to "Ù",
            "û" to "Û",
            "ǔ" to "Ǔ",
            "ū" to "Ū",
            "u̍" to "U̍",
            "ŭ" to "Ŭ",
            // n
            "ń" to "Ń",
            "ǹ" to "Ǹ",
            "n̂" to "N̂",
            "ň" to "Ň",
            "n̄" to "N̄",
            "n̍" to "N̍",
            "n̋" to "N̋",
            // m
            "ḿ" to "Ḿ",
            "m̀" to "M̀",
            "m̂" to "M̂",
            "m̌" to "M̌",
            "m̄" to "M̄",
            "m̍" to "M̍",
            "m̋" to "M̋",
        )

    /**
     * TL lowercase to uppercase tone letter mapping
     */
    val tlLowercaseToUppercaseMapping =
        mapOf(
            // a
            "á" to "Á",
            "à" to "À",
            "â" to "Â",
            "ǎ" to "Ǎ",
            "ā" to "Ā",
            "a̍" to "A̍",
            "a̋" to "A̋",
            // e
            "é" to "É",
            "è" to "È",
            "ê" to "Ê",
            "ě" to "Ě",
            "ē" to "Ē",
            "e̍" to "E̍",
            "e̋" to "E̋",
            // i
            "í" to "Í",
            "ì" to "Ì",
            "î" to "Î",
            "ǐ" to "Ǐ",
            "ī" to "Ī",
            "i̍" to "I̍",
            "i̋" to "I̋",
            // o
            "ó" to "Ó",
            "ò" to "Ò",
            "ô" to "Ô",
            "ǒ" to "Ǒ",
            "ō" to "Ō",
            "o̍" to "O̍",
            "ő" to "Ő",
            // oo
            "óo" to "Óo",
            "òo" to "Òo",
            "ôo" to "Ôo",
            "ǒo" to "Ǒo",
            "ōo" to "Ōo",
            "o̍o" to "O̍o",
            "őo" to "Őo",
            // u
            "ú" to "Ú",
            "ù" to "Ù",
            "û" to "Û",
            "ǔ" to "Ǔ",
            "ū" to "Ū",
            "u̍" to "U̍",
            "ű" to "Ű",
            // n
            "ń" to "Ń",
            "ǹ" to "Ǹ",
            "n̂" to "N̂",
            "ň" to "Ň",
            "n̄" to "N̄",
            "n̍" to "N̍",
            "n̋" to "N̋",
            // m
            "ḿ" to "Ḿ",
            "m̀" to "M̀",
            "m̂" to "M̂",
            "m̌" to "M̌",
            "m̄" to "M̄",
            "m̍" to "M̍",
            "m̋" to "M̋",
        )

    /**
     * POJ uppercase to lowercase tone letter mapping (auto-generated)
     */
    val pojUppercaseToLowercaseMapping: Map<String, String> by lazy {
        pojLowercaseToUppercaseMapping.entries.associate { (k, v) -> v to k }
    }

    /**
     * TL uppercase to lowercase tone letter mapping (auto-generated)
     */
    val tlUppercaseToLowercaseMapping: Map<String, String> by lazy {
        tlLowercaseToUppercaseMapping.entries.associate { (k, v) -> v to k }
    }

    /**
     * Check if the input contains Chinese characters (Hanzi)
     */
    fun isHanzi(input: String): Boolean {
        var i = 0
        while (i < input.length) {
            val codePoint = Character.codePointAt(input, i)
            if (codePoint in 0x4E00..0x9FFF || // CJK Unified Ideographs
                codePoint in 0x3400..0x4DBF || // CJK Extension A
                codePoint in 0x20000..0x2A6DF || // CJK Extension B
                codePoint in 0x2A700..0x2B73F || // CJK Extension C
                codePoint in 0x2B740..0x2B81F || // CJK Extension D
                codePoint in 0x2B820..0x2CEAF // CJK Extension E
            ) {
                return true
            }
            i += Character.charCount(codePoint)
        }
        return false
    }
}
