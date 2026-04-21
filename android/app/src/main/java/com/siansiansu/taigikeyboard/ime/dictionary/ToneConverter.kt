// region Shared-Core Candidate
// Pure logic, Kotlin stdlib only. Eligible for cross-platform extraction.
// endregion
package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles
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
     * Convert input with tone numbers to tone marks.
     *
     * @param input Input string with tone numbers
     * @param mode POJ or TL mode
     * @param toggles POJ preprocessing toggles (oo→o͘, nn→ⁿ) — live-read
     *   value type carried from `EngineSettingsProvider.current.toneToggles`
     *   so engine-layer callers (including `ComposingState`) don't touch
     *   Android settings APIs.
     * @param logger Diagnostic sink. Defaults to [NullLoggerBackend] so
     *   shared-core callers (e.g. `ComposingState`) stay Kotlin-stdlib-pure.
     *   A platform wrapper may pass an `AndroidLoggerBackend` to surface the
     *   `[TONE] input=… -> adjusted=…` trace in dev builds.
     * @return String with tone marks applied
     */
    fun convertToToneMarks(
        input: String,
        mode: InputMode,
        toggles: ToneToggles = ToneToggles(isDoubleTapOOEnabled = true, isDoubleTapNNEnabled = true),
        logger: LoggerBackend = NullLoggerBackend,
    ): String {
        val result =
            when (mode) {
                InputMode.POJ -> {
                    val preprocessed =
                        preprocessPojInput(
                            input,
                            toggles.isDoubleTapOOEnabled,
                            toggles.isDoubleTapNNEnabled,
                        )
                    TaigiPhonetics.convertToToneMarks(preprocessed, InputMode.POJ)
                }

                InputMode.TL -> {
                    TaigiPhonetics.convertToToneMarks(input, InputMode.TL)
                }

                InputMode.ENGLISH -> {
                    input
                }
            }

        val adjusted = ToneUtilities.adjustNasalMarkerCase(result)

        if (input != adjusted) {
            logger.debug(TAG) { "[TONE] input='$input' mode=$mode -> '$adjusted'" }
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
    private val nasalVowels = setOf('a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U')

    private fun convertNasalDoubleN(input: String): String {
        val result = StringBuilder()
        var i = 0

        while (i < input.length) {
            when {
                i + 2 < input.length &&
                    input[i] in nasalVowels &&
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
