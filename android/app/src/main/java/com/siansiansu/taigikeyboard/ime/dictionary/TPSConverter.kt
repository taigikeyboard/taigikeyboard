package com.siansiansu.taigikeyboard.ime.dictionary

/**
 * 方音符號（TPS）轉換器
 *
 * Bidirectional conversion:
 * - TPS → TL: for Trie search
 * - TL → TPS: for candidate display
 *
 * Reference: https://github.com/leechunhoe/Tailo-TPS-Converter
 */
object TPSConverter {

    // ========================================
    // MARK: - TPS → TL mapping tables
    // ========================================

    /** Consonant mapping (TPS → TL). Compound consonants must come first for greedy match. */
    private val consonants: List<Pair<String, String>> = listOf(
        // Compound consonants (match first)
        "ㄑㄧ" to "tshi",
        "ㄐㄧ" to "tsi",
        "ㄒㄧ" to "si",
        "ㆢㄧ" to "ji",
        // Standalone palatalized consonants (for nasalized vowel combos like ㄐㆪ)
        "ㄐ" to "ts",
        "ㄑ" to "tsh",
        "ㄒ" to "s",
        "ㆢ" to "j",
        // Single consonants
        "ㄅ" to "p",
        "ㄆ" to "ph",
        "ㄇ" to "m",
        "ㆠ" to "b",
        "ㄉ" to "t",
        "ㄊ" to "th",
        "ㄋ" to "n",
        "ㄌ" to "l",
        "ㄍ" to "k",
        "ㄎ" to "kh",
        "ㄫ" to "ng",
        "ㆣ" to "g",
        "ㄏ" to "h",
        "ㄗ" to "ts",
        "ㄘ" to "tsh",
        "ㄙ" to "s",
        "ㆡ" to "j",
    )

    /** Vowel mapping (TPS → TL). Compound vowels must come first for greedy match. */
    private val vowels: List<Pair<String, String>> = listOf(
        // Nasalized vowels (match first)
        "ㆮ" to "ainn",
        "ㆯ" to "aunn",
        "ㆩ" to "ann",
        "ㆥ" to "enn",
        "ㆪ" to "inn",
        "ㆧ" to "onn",
        "ㆫ" to "unn",
        // Compound vowels
        "ㄤ" to "ang",
        "ㆲ" to "ong",
        "ㆦ" to "oo",
        "ㄝ" to "ee",
        "ㄜ" to "er",
        "ㆨ" to "ir",
        "ㄞ" to "ai",
        "ㄠ" to "au",
        "ㆰ" to "am",
        "ㆱ" to "om",
        "ㄢ" to "an",
        "ㆭ" to "ng",
        // Single vowels
        "ㄚ" to "a",
        "ㆤ" to "e",
        "ㄧ" to "i",
        "ㄛ" to "o",
        "ㄨ" to "u",
        "ㄣ" to "n",
        "ㄥ" to "ng",
        "ㆬ" to "m",
    )

    /** Checked tone finals (TPS → TL). Dotted variants (tone 8) must come before plain (tone 4). */
    private val tones: List<Triple<String, String, String>> = listOf(
        Triple("ㆴ˙", "p", "8"),  // p + tone 8
        Triple("ㆵ˙", "t", "8"),  // t + tone 8
        Triple("ㆻ˙", "k", "8"),  // k + tone 8
        Triple("ㆷ˙", "h", "8"),  // h + tone 8
        Triple("ㆴ", "p", "4"),   // p + tone 4
        Triple("ㆵ", "t", "4"),   // t + tone 4
        Triple("ㆻ", "k", "4"),   // k + tone 4
        Triple("ㆷ", "h", "4"),   // h + tone 4
    )

    /** Non-checked tone marks */
    private val toneMarks: List<Pair<String, String>> = listOf(
        "ˋ" to "2",   // tone 2
        "˪" to "3",   // tone 3
        "ˊ" to "5",   // tone 5
        "ˇ" to "6",   // tone 6
        "˫" to "7",   // tone 7
        "ˆ" to "9",   // tone 9
        "˙" to "8",   // tone 8 (non-stop), U+02D9 DOT ABOVE
        // tone 1: no mark
    )

    // ========================================
    // MARK: - TL → TPS mapping tables
    // ========================================

    /** TL → TPS consonant mapping (longest match first) */
    private val tlToTPSConsonants: List<Pair<String, String>> = listOf(
        "tshi" to "ㄑㄧ",
        "tsi" to "ㄐㄧ",
        "tsh" to "ㄘ",
        "ts" to "ㄗ",
        "ph" to "ㄆ",
        "th" to "ㄊ",
        "kh" to "ㄎ",
        "ng" to "ㄫ",
        "si" to "ㄒㄧ",
        "ji" to "ㆢㄧ",
        "p" to "ㄅ",
        "m" to "ㄇ",
        "b" to "ㆠ",
        "t" to "ㄉ",
        "n" to "ㄋ",
        "l" to "ㄌ",
        "k" to "ㄍ",
        "g" to "ㆣ",
        "h" to "ㄏ",
        "s" to "ㄙ",
        "j" to "ㆡ",
    )

    /** TL → TPS vowel mapping (longest match first) */
    private val tlToTPSVowels: List<Pair<String, String>> = listOf(
        "ainn" to "ㆮ",
        "aunn" to "ㆯ",
        "ann" to "ㆩ",
        "enn" to "ㆥ",
        "inn" to "ㆪ",
        "onn" to "ㆧ",
        "unn" to "ㆫ",
        "ang" to "ㄤ",
        "ong" to "ㆲ",
        "oo" to "ㆦ",
        "ee" to "ㄝ",
        "er" to "ㄜ",
        "ir" to "ㆨ",
        "or" to "ㄛ",
        "ai" to "ㄞ",
        "au" to "ㄠ",
        "am" to "ㆰ",
        "om" to "ㆱ",
        "an" to "ㄢ",
        "ng" to "ㆭ",
        "a" to "ㄚ",
        "e" to "ㆤ",
        "i" to "ㄧ",
        "o" to "ㄛ",
        "u" to "ㄨ",
        "m" to "ㆬ",
        "n" to "ㄣ",
    )

    /** TL → TPS tone mapping */
    private val tlToTPSTones: List<Pair<String, String>> = listOf(
        "2" to "ˋ",
        "3" to "˪",
        "5" to "ˊ",
        "6" to "ˇ",
        "7" to "˫",
        "8" to "\u02D9",  // Non-stop tone 8 (U+02D9 DOT ABOVE)
        "9" to "ˆ",
        // 1, 4: no mark
    )

    /** TL → TPS checked tone finals */
    private val tlToTPSCheckedTones: List<Pair<String, String>> = listOf(
        "p8" to "ㆴ˙",
        "t8" to "ㆵ˙",
        "k8" to "ㆻ˙",
        "h8" to "ㆷ˙",
        "p4" to "ㆴ",
        "t4" to "ㆵ",
        "k4" to "ㆻ",
        "h4" to "ㆷ",
    )

    /** Stop consonants excluded from vowel matching (already consumed as checked tone finals) */
    private val stopConsonants = setOf("p", "t", "k", "h")

    // ========================================
    // MARK: - TPS character set (for detection)
    // ========================================

    /** All TPS characters for fast detection */
    private val tpsCharacters: Set<Char> by lazy {
        val chars = mutableSetOf<Char>()
        consonants.forEach { (tps, _) -> chars.addAll(tps.toList()) }
        vowels.forEach { (tps, _) -> chars.addAll(tps.toList()) }
        tones.forEach { (tps, _, _) -> chars.addAll(tps.toList()) }
        toneMarks.forEach { (tps, _) -> chars.addAll(tps.toList()) }
        chars
    }

    // ========================================
    // MARK: - TPS Initial Key Auto-Selection
    // ========================================

    /** Characters that indicate the start of a new syllable (tone marks and checked tone finals) */
    private val syllableBoundaryChars: Set<Char> by lazy {
        val chars = mutableSetOf<Char>()
        // Tone marks: ˋ ˪ ˊ ˇ ˫ ˙ ˆ
        toneMarks.forEach { (tps, _) -> chars.addAll(tps.toList()) }
        // Checked tone finals: ㆴ ㆵ ㆻ ㆷ
        chars.addAll(listOf('ㆴ', 'ㆵ', 'ㆻ', 'ㆷ'))
        chars
    }

    /**
     * Returns context-adjusted TPS character for ㄇ/ㄫ keys.
     * Only called when layoutType == "tps".
     *
     * At syllable start (empty, after tone mark, after checked final, after space) → initial form.
     * After ㄧ: ㄇ→ㆬ, ㄫ→ㄥ (ing special case).
     * Other positions: ㄇ→ㆬ, ㄫ→ㆭ.
     */
    fun adjustTPSInitialKey(char: String, afterRawInput: String): String {
        if (char != "ㄇ" && char != "ㄫ") return char

        // At syllable start → keep initial form
        if (afterRawInput.isEmpty()) return char
        val lastChar = afterRawInput.last()
        if (lastChar == ' ' || lastChar in syllableBoundaryChars) return char

        // Not at syllable start → final/syllabic form
        if (char == "ㄇ") return "ㆬ"
        // char == "ㄫ": after ㄧ → ㄥ (ing), otherwise → ㆭ
        return if (lastChar == 'ㄧ') "ㄥ" else "ㆭ"
    }

    // ========================================
    // MARK: - Public API
    // ========================================

    /** Check if string contains any TPS characters */
    fun containsTPS(input: String): Boolean {
        return input.any { it in tpsCharacters }
    }

    /** Return TPS display form when layout is "tps", otherwise return roman as-is. */
    fun displayRoman(roman: String, layoutType: String, orMapsToER: Boolean = false): String =
        if (layoutType == "tps") toTPS(roman, orMapsToER) else roman

    /**
     * Convert TPS to TL.
     *
     * Processing order (greedy, longest match first):
     * 1. Checked tone finals (affects both ending and tone)
     * 2. Tone marks
     * 3. Consonants (compound first)
     * 4. Vowels (compound first)
     * 5. Unmatched chars preserved
     *
     * @param tps TPS string (e.g., "ㄉㄧㄠˊ")
     * @return TL string (e.g., "tiau5")
     */
    fun toTL(tps: String): String {
        if (tps.isEmpty()) return ""

        var remaining = tps
        val result = StringBuilder()

        while (remaining.isNotEmpty()) {
            var matched = false

            // 1. Try checked tone finals
            for ((tpsPattern, tlEnding, tone) in tones) {
                if (remaining.startsWith(tpsPattern)) {
                    result.append(tlEnding).append(tone)
                    remaining = remaining.substring(tpsPattern.length)
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 2. Try tone marks
            for ((tpsPattern, tone) in toneMarks) {
                if (remaining.startsWith(tpsPattern)) {
                    result.append(tone)
                    remaining = remaining.substring(tpsPattern.length)
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 3. Try consonants (compound first, table is pre-sorted)
            for ((tpsPattern, tl) in consonants) {
                if (remaining.startsWith(tpsPattern)) {
                    result.append(tl)
                    remaining = remaining.substring(tpsPattern.length)
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 4. Try vowels (compound first, table is pre-sorted)
            for ((tpsPattern, tl) in vowels) {
                if (remaining.startsWith(tpsPattern)) {
                    result.append(tl)
                    remaining = remaining.substring(tpsPattern.length)
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 5. Unmatched char: preserve as-is (spaces, punctuation, etc.)
            result.append(remaining[0])
            remaining = remaining.substring(1)
        }

        // Post-process: oo before stop tone → o (e.g., "ook4" → "ok4", "oot8" → "ot8")
        // Reference: taigi-converter fromZhuyin rule
        return result.toString().replace(Regex("oo([ptk][48])"), "o$1")
    }

    /**
     * Convert multi-syllable TPS to TL (space-separated).
     *
     * @param tps TPS string (e.g., "ㄉㄧㄠˊ ㄙㄨˊ")
     * @return TL string (e.g., "tiau5 su5")
     */
    fun toTLMultiSyllable(tps: String): String {
        return tps.split(" ")
            .map { toTL(it) }
            .joinToString(" ")
    }

    /**
     * Convert TL to TPS.
     *
     * Hyphen "-" becomes space separator.
     *
     * @param tl TL string (e.g., "gua2-gua2")
     * @param orMapsToER true: or → ㄜ (dialect variant), false: or → ㄛ (default)
     * @return TPS string (e.g., "ㄍㄨㄚˋ ㄍㄨㄚˋ")
     */
    fun toTPS(tl: String, orMapsToER: Boolean = false): String {
        if (tl.isEmpty()) return ""

        val syllables = tl.split("-")
        return syllables.joinToString(" ") { convertSyllableToTPS(it, orMapsToER) }
    }

    /**
     * Convert a single TL syllable to TPS.
     *
     * Steps:
     * 1. Match consonant (longest first)
     * 2. Extract checked tone suffix (p4/t4/k4/h4/p8/t8/k8/h8)
     * 3. Extract tone digit if no checked tone
     * 4. Match vowels (greedy, longest first)
     * 5. Post-processing: standalone m/ng → syllabic form
     * 6. Post-processing: palatalized + ㄣㄣ → nasalized ㆪ
     */
    private fun convertSyllableToTPS(syllable: String, orMapsToER: Boolean = false): String {
        if (syllable.isEmpty()) return ""

        var remaining = syllable.lowercase()
        var consonant = ""
        var vowel = ""
        var tone = ""

        // 1. Match consonant (longest match first, table is pre-sorted)
        for ((tl, tps) in tlToTPSConsonants) {
            if (remaining.startsWith(tl)) {
                consonant = tps
                remaining = remaining.substring(tl.length)
                break
            }
        }

        // 2. Extract checked tone suffix
        for ((tl, tps) in tlToTPSCheckedTones) {
            if (remaining.endsWith(tl)) {
                remaining = remaining.substring(0, remaining.length - tl.length)
                tone = tps
                break
            }
        }

        // 3. Extract general tone digit if no checked tone was found
        if (tone.isEmpty() && remaining.isNotEmpty() && remaining.last().isDigit()) {
            val toneDigit = remaining.last().toString()
            remaining = remaining.substring(0, remaining.length - 1)
            for ((tlTone, tpsTone) in tlToTPSTones) {
                if (toneDigit == tlTone) {
                    tone = tpsTone
                    break
                }
            }
        }

        // 4. Match vowels from remaining (greedy, longest match first)
        while (remaining.isNotEmpty()) {
            var matched = false
            for ((tl, tps) in tlToTPSVowels) {
                if (remaining.startsWith(tl) && tl !in stopConsonants) {
                    val effectiveTPS = if (orMapsToER && tl == "or") "ㄜ" else tps
                    vowel += effectiveTPS
                    remaining = remaining.substring(tl.length)
                    matched = true
                    break
                }
            }
            if (!matched) {
                vowel += remaining[0]
                remaining = remaining.substring(1)
            }
        }

        // 5. Post-processing: standalone m/ng → syllabic form
        if (vowel.isEmpty()) {
            when (consonant) {
                "ㄇ" -> { consonant = ""; vowel = "ㆬ" }
                "ㄫ" -> { consonant = ""; vowel = "ㆭ" }
            }
        }

        // 5b. Post-processing: ing special case — ㄧㆭ → ㄧㄥ
        if (vowel == "ㄧㆭ") { vowel = "ㄧㄥ" }

        // 6. Post-processing: palatalized initial + ㄣㄣ → nasalized ㆪ
        if (consonant.endsWith("ㄧ") && vowel == "ㄣㄣ") {
            consonant = consonant.dropLast(1)
            vowel = "ㆪ"
        }

        // 7. Post-processing: o → oo before stop tone (e.g., "ok4" → ㆦㆻ not ㄛㆻ)
        // Reference: taigi-converter toZhuyin — if vowel contains ㄛ and tone is p/t/k checked,
        // replace ㄛ with ㆦ (the "oo" form is used before stop consonants)
        if ("ㄛ" in vowel && tone.isNotEmpty()) {
            val first = tone[0]
            // ㆴ = U+31B4 (p), ㆵ = U+31B5 (t), ㆻ = U+31BB (k)
            if (first == '\u31B4' || first == '\u31B5' || first == '\u31BB') {
                vowel = vowel.replace("ㄛ", "ㆦ")
            }
        }

        return consonant + vowel + tone
    }
}
