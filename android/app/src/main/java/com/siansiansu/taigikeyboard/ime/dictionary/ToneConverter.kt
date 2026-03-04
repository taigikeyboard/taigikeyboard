package com.siansiansu.taigikeyboard.ime.dictionary

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode

/**
 * Tone converter for Taiwanese romanization systems (POJ and TL)
 *
 * Coordinates POJ and TL tone conversion through TaigiPhonetics engine.
 * POJ preprocessing (oo→o͘, nn→ⁿ) is handled here before delegating to TaigiPhonetics.
 */
object ToneConverter {

    private const val TAG = "ToneConverter"

    /**
     * Convert input with tone numbers to tone marks
     * @param input Input string with tone numbers
     * @param mode POJ or TL mode
     * @param enableDoubleTapOO Whether oo→o͘ conversion is enabled (user setting)
     * @param enableDoubleTapNN Whether nn→ⁿ conversion is enabled (user setting)
     * @return String with tone marks applied
     */
    fun convertToToneMarks(
        input: String,
        mode: InputMode,
        enableDoubleTapOO: Boolean = true,
        enableDoubleTapNN: Boolean = true,
    ): String {
        val result = when (mode) {
            InputMode.POJ -> {
                val preprocessed = preprocessPojInput(input, enableDoubleTapOO, enableDoubleTapNN)
                TaigiPhonetics.convertToToneMarks(preprocessed, InputMode.POJ)
            }
            InputMode.TL -> TaigiPhonetics.convertToToneMarks(input, InputMode.TL)
            InputMode.ENGLISH -> input
        }

        val adjusted = ToneUtilities.adjustNasalMarkerCase(result)

        if (BuildConfig.DEBUG && input != adjusted) {
            Log.d(TAG, "[TONE] input='$input' mode=$mode -> '$adjusted'")
        }

        return adjusted
    }

    // MARK: - POJ Preprocessing

    /**
     * Preprocess POJ input: apply oo→o͘ and nn→ⁿ conversions based on user settings.
     * These flags control the keyboard input shortcut; when disabled, the
     * literal "oo" / "nn" keystrokes are preserved as-is.
     */
    private fun preprocessPojInput(
        input: String,
        enableDoubleTapOO: Boolean,
        enableDoubleTapNN: Boolean,
    ): String {
        var result = input

        if (enableDoubleTapOO) {
            result = result.replace("oo", "o͘")
            result = result.replace("Oo", "O͘")
            result = result.replace("OO", "O͘")
        }

        if (enableDoubleTapNN) {
            result = convertNasalDoubleN(result)
        }

        return result
    }

    /**
     * Convert vowel + nn pattern to vowel + ⁿ
     */
    private fun convertNasalDoubleN(input: String): String {
        val vowels = "aeiouAEIOU"
        val result = StringBuilder()
        var i = 0

        while (i < input.length) {
            when {
                i + 2 < input.length &&
                vowels.contains(input[i]) &&
                input[i + 1].lowercaseChar() == 'n' &&
                input[i + 2].lowercaseChar() == 'n' -> {
                    result.append(input[i]).append("ⁿ")
                    i += 3
                }
                else -> {
                    result.append(input[i])
                    i++
                }
            }
        }

        return result.toString()
    }
}
