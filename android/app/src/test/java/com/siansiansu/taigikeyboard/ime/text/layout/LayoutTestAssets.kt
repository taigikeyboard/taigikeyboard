// Layout assets read from src/main/assets for JVM tests (Gradle runs unit tests with the module
// directory as the working directory), and helpers over the merged letter keyboard.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.keyVisible
import java.io.File

internal object LayoutTestAssets {
    /**
     * A stand-in for the engine's tone table (the JVM has no native engine): the full `a` lists, one
     * entry for every other base the editor tray leads with. The engine pins the real lists
     * (`engine/phonetics/src/tone_variations.rs` tests); `RustEngineBridgeTest` checks the decode.
     */
    val toneVariations =
        RustEngineBridge.ToneVariations(
            // trace: TL tones 2 3 5 6 7 8 9 on a = á à â ǎ ā a̍ a̋ (tone 9 = U+030B).
            tl = mapOf(
                "a" to listOf("á", "à", "â", "ǎ", "ā", "a̍", "a̋"),
                "e" to listOf("é"),
                "i" to listOf("í"),
                "o" to listOf("ó"),
                "oo" to listOf("óo"),
                "u" to listOf("ú"),
                "m" to listOf("ḿ"),
                "n" to listOf("ń", "ⁿ"),
                "ng" to listOf("ńg"),
            ),
            // trace: POJ tone 9 = breve U+0306, so a̋ → ă.
            poj = mapOf(
                "a" to listOf("á", "à", "â", "ǎ", "ā", "a̍", "ă"),
                "e" to listOf("é"),
                "i" to listOf("í"),
                "o" to listOf("ó"),
                "o͘" to listOf("ó͘"),
                "u" to listOf("ú"),
                "m" to listOf("ḿ"),
                "n" to listOf("ń", "ⁿ"),
                "ng" to listOf("ńg"),
            ),
            // trace: the engine interleaves the TL and POJ lists entry by entry, each entry once: `a`
            // keeps a̋ then ă; one-script keys (`oo`, `o͘`) keep their own list.
            universal = mapOf(
                "a" to listOf("á", "à", "â", "ǎ", "ā", "a̍", "a̋", "ă"),
                "e" to listOf("é"),
                "i" to listOf("í"),
                "o" to listOf("ó"),
                "oo" to listOf("óo"),
                "o͘" to listOf("ó͘"),
                "u" to listOf("ú"),
                "m" to listOf("ḿ"),
                "n" to listOf("ń", "ⁿ"),
                "ng" to listOf("ńg"),
            ),
        )

    val assets = LayoutAssets { path -> File("src/main/assets/$path").takeIf { it.exists() }?.readText() }

    val tables by lazy { CustomLayoutTables(assets) { toneVariations } }

    /** The merged built-in letter keyboard, digit row first, as LayoutManager merges it (before popups). */
    fun merged(
        base: String,
        script: LayoutScript,
        isFullWidthPunctuation: Boolean,
    ): ComputedLayoutDataArrangement = assets.letterKeyboard(base, script.inputMode, isFullWidthPunctuation)

    /** The base table, unchanged, as a custom layout. */
    fun unchanged(
        base: String,
        script: LayoutScript,
        id: String = "id-$base-${script.inputMode}",
    ): CustomLayout = tables.fresh(id, "Name", base, script)!!

    fun substituted(
        layout: CustomLayout,
        isFullWidthPunctuation: Boolean = false,
    ): ComputedLayoutDataArrangement = tables.substitute(merged(layout.base, layout.script, isFullWidthPunctuation), layout, isFullWidthPunctuation)

    /** The keys a field of [variation] shows. */
    fun visible(
        rows: List<List<KeyData>>,
        variation: KeyVariation,
    ): List<List<KeyData>> = rows.map { row -> row.filter { keyVisible(it, variation) } }

    fun labels(row: List<KeyData>): List<String> = row.map { it.label }

    fun codes(row: List<KeyData>): List<Int> = row.map { it.code }

    fun withoutGlobe(rows: ComputedLayoutDataArrangement): List<List<KeyData>> = rows.map { row -> row.filter { it.code != KeyCode.LANGUAGE_SWITCH } }
}
