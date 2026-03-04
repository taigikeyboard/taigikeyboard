package com.siansiansu.taigikeyboard.ime.dictionary

import java.text.Normalizer

/**
 * Tone restoration
 *
 * Restores tone-marked characters to their base form for backspace operations.
 * Uses NFD decomposition instead of lookup tables.
 */
object ToneRestoration {

    /**
     * Attempt to restore tone marks in text
     * @param text Text to restore
     * @param mode Input mode (POJ/TL) — reserved for future use
     * @return Restored text, or null if no tone marks found
     */
    fun restore(text: String, mode: ToneConverterModels.InputMode): String? {
        if (text.isEmpty()) return null

        // NFD decompose to expose combining marks
        val nfd = Normalizer.normalize(text, Normalizer.Form.NFD)
        val codePoints = nfd.codePoints().toArray()

        // Scan from end, find last combining tone mark
        for (i in codePoints.indices.reversed()) {
            if (TaigiPhonetics.combiningToToneNum.containsKey(codePoints[i])) {
                // Remove the combining mark
                val restored = buildString {
                    for (j in codePoints.indices) {
                        if (j != i) {
                            appendCodePoint(codePoints[j])
                        }
                    }
                }
                // NFC recompose
                return Normalizer.normalize(restored, Normalizer.Form.NFC)
            }
        }

        return null
    }
}
