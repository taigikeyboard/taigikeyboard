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
    private val consonants: List<Pair<String, String>> =
        listOf(
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
    private val vowels: List<Pair<String, String>> =
        listOf(
            // Nasalized vowels (match first)
            "ㆮ" to "ainn",
            "ㆯ" to "aunn",
            "ㆩ" to "ann",
            "ㆥ" to "enn",
            "ㆪ" to "inn",
            "ㆳ" to "inn", // Vertical glyph variant of ㆪ (same symbol, duplicate Unicode encoding)
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
    private val tones: List<Triple<String, String, String>> =
        listOf(
            Triple("ㆴ˙", "p", "8"), // p + tone 8
            Triple("ㆵ˙", "t", "8"), // t + tone 8
            Triple("ㆻ˙", "k", "8"), // k + tone 8
            Triple("ㆷ˙", "h", "8"), // h + tone 8
            Triple("ㆴ", "p", "4"), // p + tone 4
            Triple("ㆵ", "t", "4"), // t + tone 4
            Triple("ㆻ", "k", "4"), // k + tone 4
            Triple("ㆷ", "h", "4"), // h + tone 4
        )

    /** Non-checked tone marks */
    private val toneMarks: List<Pair<String, String>> =
        listOf(
            "ˋ" to "2", // tone 2
            "˪" to "3", // tone 3
            "ˊ" to "5", // tone 5
            "ˇ" to "6", // tone 6
            "˫" to "7", // tone 7
            "ˆ" to "9", // tone 9
            "˙" to "8", // tone 8 (non-stop), U+02D9 DOT ABOVE
            // tone 1: no mark
        )

    // ========================================
    // MARK: - TL → TPS mapping tables
    // ========================================

    /** TL → TPS consonant mapping (longest match first) */
    private val tlToTPSConsonants: List<Pair<String, String>> =
        listOf(
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
    private val tlToTPSVowels: List<Pair<String, String>> =
        listOf(
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
    private val tlToTPSTones: List<Pair<String, String>> =
        listOf(
            "2" to "ˋ",
            "3" to "˪",
            "5" to "ˊ",
            "6" to "ˇ",
            "7" to "˫",
            "8" to "\u02D9", // Non-stop tone 8 (U+02D9 DOT ABOVE)
            "9" to "ˆ",
            // 1, 4: no mark
        )

    /** TL → TPS checked tone finals */
    private val tlToTPSCheckedTones: List<Pair<String, String>> =
        listOf(
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

    /**
     * Non-palatalized affricates that cannot be followed by ㄧ.
     * In TPS, [ts/tsh/s/j] + [i] must use the palatalized compound initials
     * ㄐㄧ/ㄑㄧ/ㄒㄧ/ㆢㄧ, not the non-palatalized forms ㄗ/ㄘ/ㄙ/ㆡ + ㄧ.
     */
    private val nonPalatalizedAffricates = setOf("ㄗ", "ㄘ", "ㄙ", "ㆡ")

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
    // MARK: - Tone Mark Detection
    // ========================================

    /** Non-entering tone mark characters only (ˋ ˪ ˊ ˇ ˫ ˙ ˆ).
     *  Does NOT include entering tone finals (ㆴ ㆵ ㆻ ㆷ).
     *  Used to detect whether a syllable already has an explicit tone specified. */
    private val toneMarkCharSet: Set<Char> by lazy {
        toneMarks.flatMap { (tps, _) -> tps.toList() }.toSet()
    }

    /** Returns true if the character is a non-entering TPS tone mark (ˋ ˪ ˊ ˇ ˫ ˙ ˆ). */
    fun isTPSToneMark(char: Char): Boolean = char in toneMarkCharSet

    // ========================================
    // MARK: - TPS Initial Key Auto-Selection
    // ========================================

    /** Characters that indicate the start of a new syllable (tone marks, checked tone finals, nasal finals) */
    private val syllableBoundaryChars: Set<Char> by lazy {
        val chars = mutableSetOf<Char>()
        // Tone marks: ˋ ˪ ˊ ˇ ˫ ˙ ˆ
        toneMarks.forEach { (tps, _) -> chars.addAll(tps.toList()) }
        // Checked tone finals: ㆴ ㆵ ㆻ ㆷ
        chars.addAll(listOf('ㆴ', 'ㆵ', 'ㆻ', 'ㆷ'))
        // Nasal finals: ㆬ ㄣ ㆭ ㄥ
        chars.addAll(listOf('ㆬ', 'ㄣ', 'ㆭ', 'ㄥ'))
        chars
    }

    // ========================================
    // Palatalization auto-correct
    // ========================================

    /** Palatalization mapping: non-palatalized → palatalized affricate. */
    private val palatalizationMap =
        mapOf(
            'ㄗ' to "ㄐ",
            'ㄘ' to "ㄑ",
            'ㄙ' to "ㄒ",
            'ㆡ' to "ㆢ",
        )

    /** Characters that trigger palatalization of the preceding affricate. */
    private val palatalizationTriggers = setOf('ㄧ', 'ㆪ')

    /**
     * Returns the palatalized replacement for the last character of rawInput,
     * or null if no replacement is needed.
     *
     * When user types ㄧ or ㆪ after a non-palatalized affricate (ㄗ/ㄘ/ㄙ/ㆡ),
     * the affricate should be auto-corrected to its palatalized form (ㄐ/ㄑ/ㄒ/ㆢ).
     */
    fun palatalizationReplacement(
        incoming: String,
        lastRawChar: Char?,
    ): String? {
        if (lastRawChar == null) return null
        val first = incoming.firstOrNull() ?: return null
        if (first !in palatalizationTriggers) return null
        return palatalizationMap[lastRawChar]
    }

    // ========================================
    // Syllabic nasal tone-triggered correction
    // ========================================

    /**
     * Returns the syllabic replacement for the last character of rawInput,
     * or null if no replacement is needed.
     *
     * When user types a tone mark after a bare ㄇ or ㄫ at syllable start,
     * the consonant should be retroactively corrected to its syllabic form:
     * ㄇ → ㆬ (syllabic m), ㄫ → ㆭ (syllabic ng).
     */
    fun syllabicNasalReplacement(
        incoming: String,
        lastRawChar: Char?,
    ): String? {
        if (lastRawChar == null) return null
        val first = incoming.firstOrNull() ?: return null
        if (!isTPSToneMark(first)) return null
        return when (lastRawChar) {
            'ㄇ' -> "ㆬ"
            'ㄫ' -> "ㆭ"
            else -> null
        }
    }

    // ========================================
    // Nasal/stop positional auto-select
    // ========================================

    /**
     * Returns context-adjusted TPS character for keys with initial/final dual forms.
     * Only called when layoutType == "tps".
     *
     * At syllable start (empty, after tone mark, after checked final, after space) → initial form.
     * Not at syllable start → final form:
     *   Nasals: ㄇ→ㆬ, ㄋ→ㄣ, ㄫ→ㄥ (after ㄧ) / ㆭ (otherwise).
     *   Stops:  ㄅ→ㆴ, ㄉ→ㆵ, ㄍ→ㆻ, ㄏ→ㆷ (entering tone coda, tone 4 default).
     */
    fun adjustTPSInitialKey(
        char: String,
        afterRawInput: String,
    ): String {
        if (char != "ㄇ" && char != "ㄋ" && char != "ㄫ" &&
            char != "ㄅ" && char != "ㄉ" && char != "ㄍ" && char != "ㄏ"
        ) {
            return char
        }

        // At syllable start → keep initial form
        if (afterRawInput.isEmpty()) return char
        val lastChar = afterRawInput.last()
        if (lastChar == ' ' || lastChar in syllableBoundaryChars) return char

        // Not at syllable start → final form
        if (char == "ㄇ") return "ㆬ"
        if (char == "ㄋ") return "ㄣ"
        if (char == "ㄅ") return "ㆴ"
        if (char == "ㄉ") return "ㆵ"
        if (char == "ㄍ") return "ㆻ"
        if (char == "ㄏ") return "ㆷ"
        // char == "ㄫ": after ㄧ → ㄥ (ing), otherwise → ㆭ
        return if (lastChar == 'ㄧ') "ㄥ" else "ㆭ"
    }

    // ========================================
    // ㆮ/ㆯ auto-correct
    // ========================================

    /**
     * Auto-correct ㆮ (ainn) → ㆯ (aunn) when preceded by ㄧ.
     * "iainn" is not a valid Taiwanese final; only "iaunn" exists.
     * Only called when layoutType == "tps".
     */
    fun adjustTPSNasalizedVowelKey(
        char: String,
        afterRawInput: String,
    ): String {
        if (char != "ㆮ") return char
        val lastChar = afterRawInput.lastOrNull() ?: return char
        return if (lastChar == 'ㄧ') "ㆯ" else char
    }

    // ========================================
    // MARK: - Public API
    // ========================================

    /** Check if string contains any TPS characters */
    fun containsTPS(input: String): Boolean = input.any { it in tpsCharacters }

    /** Return TPS display form when layout is "tps", otherwise return roman as-is. */
    fun displayRoman(
        roman: String,
        layoutType: String,
        orMapsToER: Boolean = false,
    ): String = if (layoutType == "tps") toTPS(roman, orMapsToER) else roman

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
        // Track syllable state with separate consonant/vowel flags.
        // In TPS, consonant codas are encoded in entering tone symbols (ㆴㆵㆻㆷ),
        // and syllabic nasals use vowel-table characters (ㆬ, ㆭ).
        // So: consonant mid-syllable = new syllable; tone after consonant-only = invalid syllable.
        var hasConsonant = false
        var hasVowel = false
        var lastConsonantTPS = ""
        // Tone/entering-tone ends a syllable; the next consonant or vowel needs a space.
        var needsSpace = false

        while (remaining.isNotEmpty()) {
            var matched = false

            // 1. Try checked tone finals
            for ((tpsPattern, tlEnding, tone) in tones) {
                if (remaining.startsWith(tpsPattern)) {
                    result.append(tlEnding).append(tone)
                    remaining = remaining.substring(tpsPattern.length)
                    hasConsonant = false
                    hasVowel = false
                    lastConsonantTPS = ""
                    needsSpace = true
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 2. Try tone marks
            // If consonant-only (no vowel), insert space — initial + tone is not a valid syllable.
            // e.g. ㄫˊ → "ng 5" (not "ng5"), but ㆭˊ → "ng5" (ㆭ is a vowel).
            for ((tpsPattern, tone) in toneMarks) {
                if (remaining.startsWith(tpsPattern)) {
                    if (hasConsonant && !hasVowel) {
                        result.append(' ')
                    }
                    result.append(tone)
                    remaining = remaining.substring(tpsPattern.length)
                    hasConsonant = false
                    hasVowel = false
                    lastConsonantTPS = ""
                    needsSpace = true
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 3. Try consonants (compound first, table is pre-sorted)
            // If mid-syllable, insert space boundary first — consonant starts a new syllable.
            for ((tpsPattern, tl) in consonants) {
                if (remaining.startsWith(tpsPattern)) {
                    if (needsSpace || hasConsonant || hasVowel) {
                        result.append(' ')
                        needsSpace = false
                    }
                    result.append(tl)
                    remaining = remaining.substring(tpsPattern.length)
                    hasConsonant = true
                    // Compound consonants with ㄧ (ㄑㄧ→tshi etc.) include a vowel component
                    hasVowel = tpsPattern.endsWith("ㄧ")
                    lastConsonantTPS = tpsPattern
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 4. Try vowels (compound first, table is pre-sorted)
            for ((tpsPattern, tl) in vowels) {
                if (remaining.startsWith(tpsPattern)) {
                    if (needsSpace) {
                        result.append(' ')
                        needsSpace = false
                    }
                    // Non-palatalized affricates (ㄗ/ㄘ/ㄙ/ㆡ) + ㄧ is invalid TPS.
                    // Must use compound initials ㄐㄧ/ㄑㄧ/ㄒㄧ/ㆢㄧ instead.
                    if (tpsPattern == "ㄧ" && !hasVowel && lastConsonantTPS in nonPalatalizedAffricates) {
                        result.append(' ')
                        hasConsonant = false
                        lastConsonantTPS = ""
                    }
                    result.append(tl)
                    remaining = remaining.substring(tpsPattern.length)
                    hasVowel = true
                    matched = true
                    break
                }
            }
            if (matched) continue

            // 5. Unmatched char: preserve as-is (spaces, punctuation, etc.)
            result.append(remaining[0])
            remaining = remaining.substring(1)
            hasConsonant = false
            hasVowel = false
            lastConsonantTPS = ""
            needsSpace = false
        }

        // Post-process: oo before stop tone → o (e.g., "ook4" → "ok4", "oot8" → "ot8")
        // Reference: taigi-converter fromZhuyin rule
        return result.toString().replace(Regex("oo([ptk][48])"), "o$1")
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
    fun toTPS(
        tl: String,
        orMapsToER: Boolean = false,
    ): String {
        if (tl.isEmpty()) return ""

        val syllables = tl.split("-")
        return syllables.joinToString(" ") { convertSyllableToTPS(it, orMapsToER) }
    }

    /**
     * Convert display-form romanization (with diacritical tone marks) to TPS.
     *
     * The dictionary stores romanization with diacritics (e.g., "n̂g", "guá").
     * This method first normalizes to numeric TL (e.g., "ng5", "gua2"),
     * then converts to TPS.
     *
     * @param displayRoman Display-form romanization with diacritical tone marks
     * @param orMapsToER true: or → ㄜ, false: or → ㄛ
     * @return TPS string
     */
    fun toTPSFromDisplay(
        displayRoman: String,
        orMapsToER: Boolean = false,
    ): String {
        if (displayRoman.isEmpty()) return ""

        val numericTL =
            displayRoman.split("-").joinToString("-") { syllable ->
                val (bare, tone) = TaigiPhonetics.stripToneMark(syllable)
                val normalized = TaigiPhonetics.normalizeToTL(bare.lowercase())
                normalized + tone
            }

        return toTPS(numericTL, orMapsToER)
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
    private fun convertSyllableToTPS(
        syllable: String,
        orMapsToER: Boolean = false,
    ): String {
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
                "ㄇ" -> {
                    consonant = ""
                    vowel = "ㆬ"
                }

                "ㄫ" -> {
                    consonant = ""
                    vowel = "ㆭ"
                }
            }
        }

        // 5b. Post-processing: ing special case — ㄧㆭ → ㄧㄥ
        if (vowel == "ㄧㆭ") {
            vowel = "ㄧㄥ"
        }

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
