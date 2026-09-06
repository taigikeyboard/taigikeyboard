// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only (java.net.URLEncoder is JDK stdlib,
// accepted per the InputNormalizer / java.text.Normalizer precedent).
// Eligible for cross-platform extraction.
// endregion

package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.engine.nfdPreprocessForLookup
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.stripTone
import java.net.URLEncoder

/**
 * Builds MOE / Chhoe Taigi external dictionary lookup URLs from TL display form.
 * Parallels iOS `ExternalLookupURLBuilder.swift`. Tone-digit conversion semantics
 * follow the external dictionaries' URL format (tone 1 / tone 4 omitted).
 *
 * Platform-owned by design — URL conventions (which tones to drop, which
 * query parameters each dictionary takes, percent-encoding rules) are
 * not phonetics; they belong with the platform that knows about
 * `URLEncoder`. The phonetic prep step delegates to
 * `RustEngineBridge.nfdPreprocessForLookup` and tone-strip to
 * `RustEngineBridge.stripTone`.
 */
object ExternalLookupURLBuilder {
    /** Build Chhoe Taigi lookup URL. Returns null if the TL string produces an empty digit form. */
    fun chhoeURL(tl: String): String? {
        val tlDigit = toTLDigit(tl)
        if (tlDigit.isEmpty()) return null
        val encoded = URLEncoder.encode(tlDigit, "UTF-8")
        return "https://chhoe.taigi.info/s?s=su&f=e&lmjf=ki&lmj=$encoded"
    }

    /** Build MOE Dictionary lookup URL. Returns null if the TL string produces an empty digit form. */
    fun moeURL(tl: String): String? {
        val tlDigit = toTLDigit(tl)
        if (tlDigit.isEmpty()) return null
        val encoded = URLEncoder.encode(tlDigit, "UTF-8")
        return "https://sutian.moe.edu.tw/zh-hant/tshiau/?lui=tai_su&tsha=$encoded"
    }

    /** Convert TL display form (with diacritics) to TL digit form for URL. */
    fun toTLDigit(tl: String): String {
        val syllables = tl.lowercase().split("-")
        return syllables.joinToString("-") { normalizeSyllableToDigit(it) }
    }

    private fun normalizeSyllableToDigit(syllable: String): String {
        if (syllable.isEmpty()) return ""
        // Quick path: nasal-only substitution suffices to detect already-digit
        // input (e.g. "ho2", "saⁿ1"). Full POJ preprocessing (`o͘` → `oo`) is
        // only needed for the diacritic branch below.
        val withNasalConverted = syllable.replace("ⁿ", "nn").replace("ᴺ", "nn")
        if (withNasalConverted.last().isDigit()) {
            val tone = withNasalConverted.last().toString()
            if (tone == "1" || tone == "4") return withNasalConverted.dropLast(1)
            return withNasalConverted
        }
        // Diacritic path: apply full Taigi preprocessing (nasal + `o͘` → `oo`)
        // before tone stripping — matches iOS `ExternalLookupURLBuilder` and
        // ensures custom-dict entries containing POJ `o͘` produce canonical
        // TL `hoo`/`hoo2` URLs instead of `ho͘`/`ho͘2`.
        val preprocessed = RustEngineBridge.nfdPreprocessForLookup(syllable)
        val stripped = RustEngineBridge.stripTone(preprocessed)
        if (stripped.tone.isEmpty() || stripped.tone == "1" || stripped.tone == "4") {
            return stripped.bare
        }
        return stripped.bare + stripped.tone
    }
}
