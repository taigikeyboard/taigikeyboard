// Layout assets read from src/main/assets for JVM tests (Gradle runs unit tests with the module
// directory as the working directory), and helpers over the merged letter keyboard.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.keyVisible
import java.io.File

internal object LayoutTestAssets {
    val assets = LayoutAssets { path -> File("src/main/assets/$path").takeIf { it.exists() }?.readText() }

    val tables by lazy { CustomLayoutTables(assets) }

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
