// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import java.text.Normalizer

// / Unified Taigi phonetics engine
// /
// / Ported from `references/taigi-converter/src/` (tables.js, phonetics.js, tl.js, poj.js).
// / Replaces ToneMappings, VowelAnalyzer, POJToneConverter, TLToneConverter with
// / a single NFD/NFC-based pipeline: parse syllable -> place combining mark -> normalize.
object TaigiPhonetics {
    // MARK: - Data Tables

    val tlInitials: Set<String> =
        setOf(
            "p",
            "ph",
            "m",
            "b",
            "t",
            "th",
            "n",
            "l",
            "k",
            "kh",
            "ng",
            "g",
            "ts",
            "tsh",
            "s",
            "j",
            "h",
            "",
        )

    val tlFinals: Set<String> =
        setOf(
            "a",
            "ah",
            "ap",
            "at",
            "ak",
            "ann",
            "annh",
            "am",
            "an",
            "ang",
            "e",
            "eh",
            "enn",
            "ennh",
            "i",
            "ih",
            "ip",
            "it",
            "ik",
            "inn",
            "innh",
            "im",
            "in",
            "ing",
            "o",
            "oh",
            "oo",
            "ooh",
            "op",
            "ok",
            "om",
            "ong",
            "onn",
            "onnh",
            "u",
            "uh",
            "ut",
            "un",
            "ai",
            "aih",
            "ainn",
            "ainnh",
            "au",
            "auh",
            "aunn",
            "aunnh",
            "ia",
            "iah",
            "iap",
            "iat",
            "iak",
            "iam",
            "ian",
            "iang",
            "iann",
            "iannh",
            "io",
            "ioh",
            "iok",
            "iong",
            "ionn",
            "iu",
            "iuh",
            "iut",
            "iunn",
            "iunnh",
            "ua",
            "uah",
            "uat",
            "uak",
            "uan",
            "uann",
            "uannh",
            "ue",
            "ueh",
            "uenn",
            "uennh",
            "ui",
            "uih",
            "uinn",
            "uinnh",
            "iau",
            "iauh",
            "iaunn",
            "iaunnh",
            "uai",
            "uaih",
            "uainn",
            "uainnh",
            "m",
            "mh",
            "ng",
            "ngh",
            "ioo",
            "iooh",
            "iai",
            "iaih",
            "er",
            "erh",
            "erk",
            "erm",
            "ere",
            "ereh",
            "eng",
            "ir",
            "irh",
            "irp",
            "irt",
            "irk",
            "irm",
            "irn",
            "irng",
            "irinn",
            "iri",
            "ie",
            "or",
            "orh",
            "ior",
            "iorh",
            "uang",
            "oi",
            "oih",
            "ee",
            "eeh",
        )

    /** Tone number -> combining mark (NFD). Tones 1 and 4 have no mark. */
    val toneNumToCombining: Map<String, String> =
        mapOf(
            "1" to "",
            "2" to "\u0301",
            "3" to "\u0300",
            "4" to "",
            "5" to "\u0302",
            "6" to "\u030C",
            "7" to "\u0304",
            "8" to "\u030D",
            "9" to "\u0306",
        )

    /** TL tone 9 uses double acute accent (U+030B) instead of breve */
    val tlTone9Combining = "\u030B"

    /** Combining mark code point -> tone number (reverse of toneNumToCombining, plus POJ breve and TL double acute) */
    val combiningToToneNum: Map<Int, String> =
        mapOf(
            0x0301 to "2", // COMBINING ACUTE ACCENT
            0x0300 to "3", // COMBINING GRAVE ACCENT
            0x0302 to "5", // COMBINING CIRCUMFLEX ACCENT
            0x030C to "6", // COMBINING CARON
            0x0304 to "7", // COMBINING MACRON
            0x030D to "8", // COMBINING VERTICAL LINE ABOVE
            0x0306 to "9", // COMBINING BREVE (POJ tone 9)
            0x030B to "9", // COMBINING DOUBLE ACUTE ACCENT (TL tone 9)
        )

    /** All combining code points we recognize as tone marks */
    private val combiningCodePoints: Set<Int> = combiningToToneNum.keys

    /** Pre-compiled regex patterns for POJ tone mark placement */
    private val TWO_VOWEL_REGEX = Regex("[aeiou]{2}")
    private val SINGLE_VOWEL_REGEX = Regex("[aeiou]")

    /** TL initial -> POJ initial */
    val pojInitialFromTL: Map<String, String> = mapOf("ts" to "ch", "tsh" to "chh")

    /** TL final -> POJ final substitutions (order matters: nn before oo) */
    val pojFinalSubstitutions: List<Pair<String, String>> =
        listOf(
            "nn" to "\u207F", // nn -> ⁿ
            "oo" to "o\u0358", // oo -> o͘
            "ua" to "oa",
            "ue" to "oe",
            "ing" to "eng",
            "ik" to "ek",
        )

    // MARK: - Core Parsing (from phonetics.js)

    /** Strip tone mark from text, returning (bare NFC text, tone number string).
     *  Recognizes both combining diacritics and trailing digit. */
    fun stripToneMark(text: String): Pair<String, String> {
        val decomposed = Normalizer.normalize(text, Normalizer.Form.NFD)
        var foundMark: Int? = null

        for (codePoint in decomposed.codePoints().toArray()) {
            if (codePoint in combiningCodePoints) {
                foundMark = codePoint
                break
            }
        }

        if (foundMark != null) {
            val toneNum = combiningToToneNum[foundMark] ?: ""
            val bare =
                buildString {
                    for (cp in decomposed.codePoints().toArray()) {
                        if (cp != foundMark) {
                            appendCodePoint(cp)
                        }
                    }
                }
            val bareNfc = Normalizer.normalize(bare, Normalizer.Form.NFC)
            return Pair(bareNfc, toneNum)
        }

        // No combining mark — check trailing digit
        if (text.isNotEmpty()) {
            val last = text.last()
            val digit = last.digitToIntOrNull()
            if (digit != null && digit in 1..9) {
                val bare = text.dropLast(1)
                val bareNfc = Normalizer.normalize(bare, Normalizer.Form.NFC)
                return Pair(bareNfc, digit.toString())
            }
        }

        return Pair(Normalizer.normalize(text, Normalizer.Form.NFC), "")
    }

    /** Normalize text to TL spelling (lowercase). Replaces POJ conventions with TL equivalents. */
    fun normalizeToTL(text: String): String {
        var result = text
        result = result.replace("ch", "ts")
        result = result.replace("ou", "oo")
        result = result.replace("o\u0358", "oo")
        result = result.replace("\u207F", "nn")
        result = result.replace("\u1D3A", "nn") // uppercase nasal marker
        result = result.replace("oa", "ua")
        result = result.replace("oe", "ue")
        result = result.replace("eng", "ing")
        result = result.replace("ek", "ik")
        result = result.replace("oonn", "onn")
        return result
    }

    /** Check if a final ends in a stop consonant (p, t, k, h), ignoring trailing nn. */
    fun isStopTone(final_: String): Boolean {
        val cleaned = final_.lowercase().replace("nn", "")
        return cleaned.endsWith("p") || cleaned.endsWith("t") ||
            cleaned.endsWith("k") || cleaned.endsWith("h")
    }

    /** Split bare TL text into (initial, final) by iterating prefixes. */
    fun splitInitialFinal(text: String): Pair<String, String>? {
        for (i in 0..text.length) {
            val initial = text.substring(0, i)
            if (initial in tlInitials) {
                val final_ = text.substring(i)
                if (final_ in tlFinals) {
                    return Pair(initial, final_)
                }
            }
        }
        return null
    }

    /** Parse a syllable (with tone marks or trailing digit) into (initial, final, tone).
     *  Returns null if the syllable cannot be parsed. */
    fun parseSyllable(text: String): Triple<String, String, String>? {
        val (bare, tone) = stripToneMark(text)
        val normalized = normalizeToTL(bare.lowercase())
        val split = splitInitialFinal(normalized) ?: return null
        val (initial, final_) = split
        val finalTone = if (tone.isEmpty()) (if (isStopTone(final_)) "4" else "1") else tone
        return Triple(initial, final_, finalTone)
    }

    // MARK: - TL Assembly (from tl.js)

    /** Assemble a TL syllable from initial + final + tone number. */
    fun toTL(
        initial: String,
        final_: String,
        tone: String,
    ): String {
        var mark = toneNumToCombining[tone] ?: ""
        if (tone == "9") mark = tlTone9Combining
        val markedFinal = placeTLToneMark(final_, mark)
        return Normalizer.normalize(initial + markedFinal, Normalizer.Form.NFC)
    }

    /** Place tone mark on the correct vowel in a TL final.
     *  Rule priority: a > oo > ere > e > o > ui→i > iu→u > iri > i > u > ng > m */
    private fun placeTLToneMark(
        f: String,
        mark: String,
    ): String {
        if (mark.isEmpty()) return f
        if ("a" in f) return f.replaceFirst("a", "a$mark")
        if ("oo" in f) return f.replaceFirst("oo", "o${mark}o")
        if ("ere" in f) return f.replaceFirst("ere", "ere$mark")
        if ("e" in f) return f.replaceFirst("e", "e$mark")
        if ("o" in f) return f.replaceFirst("o", "o$mark")
        if ("ui" in f) return f.replaceFirst("i", "i$mark")
        if ("iu" in f) return f.replaceFirst("u", "u$mark")
        if ("iri" in f) return f.replaceFirst("iri", "iri$mark")
        if ("i" in f) return f.replaceFirst("i", "i$mark")
        if ("u" in f) return f.replaceFirst("u", "u$mark")
        if ("ng" in f) return f.replaceFirst("ng", "n${mark}g")
        if ("m" in f) return f.replaceFirst("m", "m$mark")
        return f
    }

    // MARK: - POJ Assembly (from poj.js)

    /** Assemble a POJ syllable from TL initial + final + tone number. */
    fun toPOJ(
        initial: String,
        final_: String,
        tone: String,
    ): String {
        val pojInitial = pojInitialFromTL[initial] ?: initial
        val pojFinal = tlFinalToPOJ(final_)
        var mark = toneNumToCombining[tone] ?: ""
        if (tone == "9") mark = "\u0306" // POJ uses breve for tone 9
        val markedFinal = placePOJToneMark(pojFinal, mark)
        return Normalizer.normalize(pojInitial + markedFinal, Normalizer.Form.NFC)
    }

    /** Convert TL final spelling to POJ final spelling. */
    fun tlFinalToPOJ(final_: String): String {
        var result = final_
        for ((tlPart, pojPart) in pojFinalSubstitutions) {
            result = result.replace(tlPart, pojPart)
        }
        return result
    }

    /** Place tone mark on the correct vowel in a POJ final. */
    private fun placePOJToneMark(
        f: String,
        mark: String,
    ): String {
        if (mark.isEmpty()) return f

        // o͘ (o + U+0358) gets mark between o and combining dot
        if ("o\u0358" in f) {
            return f.replaceFirst("o\u0358", "o${mark}\u0358")
        }

        // iau/oai -> mark on a
        if ("iau" in f || "oai" in f) {
            return f.replaceFirst("a", "a$mark")
        }

        // Two adjacent vowels
        val twoVowelMatch = TWO_VOWEL_REGEX.find(f)
        if (twoVowelMatch != null) {
            val matchStart = twoVowelMatch.range.first
            val first = f[matchStart]
            val second = f[matchStart + 1]
            val target: Char

            if (first == 'i') {
                target = second
            } else if (second == 'i') {
                target = first
            } else if (f.length == 2) {
                target = first
            } else if ((f.endsWith("\u207F") || f.endsWith("\u1D3A")) &&
                !f.endsWith("h\u207F") && !f.endsWith("h\u1D3A")
            ) {
                target = first
            } else {
                val afterSecondIdx = matchStart + 2
                if (afterSecondIdx < f.length) {
                    val suffix = f[afterSecondIdx]
                    target =
                        if ("nmgptkh\u207F\u1D3A".contains(suffix)) {
                            second
                        } else {
                            first
                        }
                } else {
                    target = first
                }
            }
            return f.replaceFirst(target.toString(), "$target$mark")
        }

        // Single vowel
        val singleVowelMatch = SINGLE_VOWEL_REGEX.find(f)
        if (singleVowelMatch != null) {
            val vowel = singleVowelMatch.value
            return f.replaceFirst(vowel, "$vowel$mark")
        }

        // Syllabic consonants: ng, m
        if ("ng" in f) return f.replaceFirst("n", "n$mark")
        if ("m" in f) return f.replaceFirst("m", "m$mark")
        return f
    }

    // MARK: - High-Level API

    /** Convert a single syllable (with trailing tone digit) to tone-marked form.
     *  Keyboard-specific: tones 1/4 keep the trailing digit (e.g. "gua1" stays "gua1"). */
    fun convertSyllable(
        syllable: String,
        mode: InputMode,
    ): String {
        if (syllable.isEmpty()) return syllable

        val last = syllable.last()
        val tone = last.digitToIntOrNull() ?: return syllable
        if (tone !in 1..9) return syllable

        val baseForm = syllable.dropLast(1)
        if (baseForm.isEmpty()) return syllable

        // Tone 1, 4: keep trailing digit for composing display
        if (tone == 1 || tone == 4) {
            return syllable
        }

        // Parse the base form to get initial/final
        val normalized = normalizeToTL(baseForm.lowercase())
        val split = splitInitialFinal(normalized) ?: return syllable
        val (initial, final_) = split
        val toneStr = tone.toString()

        // Assemble with tone marks, preserving original case
        val assembled =
            when (mode) {
                InputMode.POJ -> toPOJ(initial = initial, final_ = final_, tone = toneStr)
                InputMode.TL -> toTL(initial = initial, final_ = final_, tone = toneStr)
                InputMode.ENGLISH -> return syllable
            }

        // Restore case: if original starts uppercase, capitalize result
        if (baseForm.first().isUpperCase()) {
            return assembled.substring(0, 1).uppercase() + assembled.substring(1)
        }
        return assembled
    }

    /** Convert hyphen-separated input to tone marks.
     *  Splits by "-", converts each syllable, rejoins with "-". */
    fun convertToToneMarks(
        input: String,
        mode: InputMode,
    ): String {
        val syllables = input.split("-")
        val converted =
            syllables.map { syllable ->
                if (syllable.isEmpty()) "" else convertSyllable(syllable, mode)
            }
        return converted.joinToString("-")
    }

    /** Convert POJ display text (with diacritics) to TL display text.
     *  Splits by "-" and " " (word boundary), for each syllable: strip tone -> normalizeToTL -> toTL.
     *  Preserves original separators (space = word boundary, hyphen = syllable boundary). */
    fun pojDisplayToTLDisplay(text: String): String {
        if (text.isEmpty()) return ""

        // Split while preserving separators (space and hyphen)
        val tokens = mutableListOf<Pair<String, String>>() // (text, separator)
        val current = StringBuilder()
        for (char in text) {
            if (char == '-' || char == ' ') {
                tokens.add(current.toString() to char.toString())
                current.clear()
            } else {
                current.append(char)
            }
        }
        tokens.add(current.toString() to "")

        val result = StringBuilder()
        for ((i, token) in tokens.withIndex()) {
            val (s, separator) = token
            if (s.isEmpty()) {
                // Preserve separator (e.g., "--" for 輕聲)
                if (i < tokens.size - 1 || separator.isNotEmpty()) {
                    result.append(separator)
                }
                continue
            }

            val (bare, toneNum) = stripToneMark(s)
            val converted =
                if (bare.isEmpty()) {
                    s
                } else {
                    val normalized = normalizeToTL(bare.lowercase())
                    val split = splitInitialFinal(normalized)
                    if (split != null) {
                        val (initial, final_) = split
                        val tone = if (toneNum.isEmpty()) (if (isStopTone(final_)) "4" else "1") else toneNum
                        val tlResult = toTL(initial = initial, final_ = final_, tone = tone)
                        if (s.first().isUpperCase()) {
                            tlResult.substring(0, 1).uppercase() + tlResult.substring(1)
                        } else {
                            tlResult
                        }
                    } else {
                        s
                    }
                }

            result.append(converted)
            if (separator.isNotEmpty()) {
                result.append(separator)
            }
        }
        return result.toString()
    }

    /** Convert TL display text (with diacritics) to POJ display text.
     *  Splits by "-" and " " (word boundary), for each syllable: strip tone -> parse -> toPOJ.
     *  Preserves original separators (space = word boundary, hyphen = syllable boundary). */
    fun tlDisplayToPOJDisplay(text: String): String {
        if (text.isEmpty()) return ""

        // Split while preserving separators (space and hyphen)
        val tokens = mutableListOf<Pair<String, String>>() // (text, separator)
        val current = StringBuilder()
        for (char in text) {
            if (char == '-' || char == ' ') {
                tokens.add(current.toString() to char.toString())
                current.clear()
            } else {
                current.append(char)
            }
        }
        tokens.add(current.toString() to "")

        val result = StringBuilder()
        for ((i, token) in tokens.withIndex()) {
            val (s, separator) = token
            if (s.isEmpty()) {
                // Preserve separator (e.g., "--" for 輕聲)
                if (i < tokens.size - 1 || separator.isNotEmpty()) {
                    result.append(separator)
                }
                continue
            }

            val (bare, toneNum) = stripToneMark(s)
            val converted =
                if (bare.isEmpty()) {
                    s
                } else {
                    val normalized = normalizeToTL(bare.lowercase())
                    val split = splitInitialFinal(normalized)
                    if (split != null) {
                        val (initial, final_) = split
                        val tone = if (toneNum.isEmpty()) (if (isStopTone(final_)) "4" else "1") else toneNum
                        val pojResult = toPOJ(initial = initial, final_ = final_, tone = tone)
                        if (s.first().isUpperCase()) {
                            pojResult.substring(0, 1).uppercase() + pojResult.substring(1)
                        } else {
                            pojResult
                        }
                    } else {
                        s
                    }
                }

            result.append(converted)
            if (separator.isNotEmpty()) {
                result.append(separator)
            }
        }
        return result.toString()
    }
}
