// Unit tests for CustomLayoutTables — base tables, validation and the keyboard substitution — over
// the real layout assets. Mirrors iOS CustomLayoutTests (table and rows parts).

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.codes
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.labels
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.merged
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.substituted
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.tables
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.unchanged
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.visible
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.withoutGlobe
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CustomLayoutTablesTest {
    private val bases = KeyboardLayoutOptions.romanization.map { it.key }

    private val allFunctionKeys =
        listOf(
            FunctionKey.SHIFT,
            FunctionKey.BACKSPACE,
            FunctionKey.NUMERIC,
            FunctionKey.EMOJI,
            FunctionKey.SPACE,
            FunctionKey.TRANSLATE,
            FunctionKey.RETURN,
        )

    @Test
    fun `character places per base and script`() {
        // trace: main rows below the digit row + bottom-row characters of the mod —
        // Lohankha TL 10+9+7 + `,` `-` = 28, POJ adds o͘ = 29; PhahTaigi / MOE1 10+10+7 = 27;
        // MOE2 10+10+7 + `,` `.` = 29. Same counts as the iOS tables.
        val expected =
            mapOf(
                ("phahTaigi" to LayoutScript.TL) to 27,
                ("phahTaigi" to LayoutScript.POJ) to 27,
                ("qwerty" to LayoutScript.TL) to 28,
                ("qwerty" to LayoutScript.POJ) to 29,
                ("moe1" to LayoutScript.TL) to 27,
                ("moe1" to LayoutScript.POJ) to 27,
                ("moe2" to LayoutScript.TL) to 29,
                ("moe2" to LayoutScript.POJ) to 29,
            )
        for ((key, count) in expected) {
            val table = tables.baseTable(key.first, key.second)!!
            assertEquals("$key", count, table.labels.size)
            // shift + delete wrap the last letter row (3); the bottom row (4) carries the rest.
            assertEquals("$key", allFunctionKeys, table.functionKeys)
            assertEquals("$key", listOf(3, 3, 4, 4, 4, 4, 4), table.functionRows)
        }
        assertEquals(listOf(",", "-"), tables.baseTable("qwerty", LayoutScript.TL)!!.labels.takeLast(2))
        assertEquals(listOf(",", "."), tables.baseTable("moe2", LayoutScript.TL)!!.labels.takeLast(2))
        assertEquals("ph", tables.baseTable("moe2", LayoutScript.TL)!!.labels[1])
        assertNull(tables.baseTable("tps", LayoutScript.TL))
    }

    @Test
    fun `an unchanged layout shows the built-in keys`() {
        for (base in bases) {
            for (script in LayoutScript.entries) {
                for (isFullWidth in listOf(false, true)) {
                    val name = "$base ${script.inputMode} fullWidth=$isFullWidth"
                    val custom = substituted(unchanged(base, script), isFullWidth)
                    // The globe comes back in LayoutManager.
                    val builtIn = withoutGlobe(merged(base, script, isFullWidth))
                    for (variation in listOf(KeyVariation.NORMAL, KeyVariation.PASSWORD)) {
                        assertEquals(name, visible(builtIn, variation), visible(custom, variation))
                    }
                    val email = visible(custom, KeyVariation.EMAIL_ADDRESS)
                    if (base == "qwerty" || base == "moe2") {
                        // `@` / `/` take the bottom-row comma's place, as built in.
                        assertEquals(name, visible(builtIn, KeyVariation.EMAIL_ADDRESS), email)
                        assertEquals(name, visible(builtIn, KeyVariation.URI), visible(custom, KeyVariation.URI))
                    } else {
                        // No comma on the bottom row: right before space (iOS applyInputTypeKey); built-in
                        // PhahTaigi / MOE1 put it before emoji.
                        assertEquals(
                            name,
                            listOf(KeyCode.VIEW_SYMBOLS, KeyCode.SWITCH_TO_MEDIA_CONTEXT, '@'.code, KeyCode.SPACE, KeyCode.TRANSLATE, KeyCode.ENTER),
                            codes(email.last()),
                        )
                    }
                }
            }
        }
    }

    @Test
    fun `a swap moves the whole key with its inline popup`() {
        // trace: MOE2 TL place 0 = p (row 1), place 19 = `-` (row 2, last) with a 9-key popup.
        val layout = unchanged("moe2", LayoutScript.TL).let { it.copy(keys = it.keys.swapped(0, 19)) }
        val rows = substituted(layout)
        assertEquals("-", rows[1][0].label)
        assertEquals(listOf("-", "·", "_", "~", "'", "^", "@", "#", "\""), labels(rows[1][0].popup))
        assertEquals("p", rows[2].last().label)
    }

    @Test
    fun `a base's own key wins over the catalog`() {
        // trace: MOE1 `-` popup (9 keys) differs from the symbol page's (· – — ﹏), which leads the catalog.
        val swapped = unchanged("moe1", LayoutScript.TL).let { it.copy(keys = it.keys.swapped(0, 19)) }
        assertEquals(listOf("-", "·", "_", "~", "'", "^", "@", "#", "\""), labels(substituted(swapped)[1][0].popup))
        // MOE1's `(` stays half-width in the full-width table; the symbol page would give （.
        val moe1Row3 = substituted(unchanged("moe1", LayoutScript.TL), isFullWidthPunctuation = true)[3]
        assertTrue(labels(moe1Row3).containsAll(listOf("(", ")")))
    }

    @Test
    fun `a spare from the catalog takes the current width`() {
        // trace: q (Lohankha place 0) → `?`, from the symbol page: `?` 63 half-width, `？` 65311 full-width.
        val layout = unchanged("qwerty", LayoutScript.TL).let { it.copy(keys = listOf("?") + it.keys.drop(1)) }
        assertEquals("?" to 63, substituted(layout)[1][0].let { it.label to it.code })
        assertEquals("？" to 65311, substituted(layout, isFullWidthPunctuation = true)[1][0].let { it.label to it.code })
        val paren = unchanged("qwerty", LayoutScript.TL).let { it.copy(keys = listOf("(") + it.keys.drop(1)) }
        assertEquals("（", substituted(paren, isFullWidthPunctuation = true)[1][0].label)
    }

    @Test
    fun `a final from the tray is a plain key in both widths`() {
        // trace: q (Lohankha place 0) → `ang`, a catalog-only key: code 0, no popup, same label full-width.
        val layout = unchanged("qwerty", LayoutScript.TL).let { it.copy(keys = listOf("ang") + it.keys.drop(1)) }
        assertTrue(tables.isValid(layout))
        for (isFullWidthPunctuation in listOf(false, true)) {
            val key = substituted(layout, isFullWidthPunctuation)[1][0]
            assertEquals(Triple("ang", 0, 0), Triple(key.label, key.code, key.popup.size))
        }
    }

    @Test
    fun `the nasal marker from the tray is a plain key in both widths`() {
        // trace: q (Lohankha place 0) → ⁿ U+207F, a catalog-only key: code 0, no popup, same label full-width.
        val layout = unchanged("qwerty", LayoutScript.POJ).let { it.copy(keys = listOf("ⁿ") + it.keys.drop(1)) }
        assertTrue(tables.isValid(layout))
        for (isFullWidthPunctuation in listOf(false, true)) {
            val key = substituted(layout, isFullWidthPunctuation)[1][0]
            assertEquals(Triple("ⁿ", 0, 0), Triple(key.label, key.code, key.popup.size))
        }
    }

    @Test
    fun `removed keys leave their row and an empty row leaves the keyboard`() {
        val base = unchanged("qwerty", LayoutScript.TL)
        val withoutA = substituted(base.copy(keys = base.keys.toMutableList().also { it[10] = null }))
        assertEquals(listOf("s", "d", "f", "g", "h", "j", "k", "l"), labels(withoutA[2]))

        val withoutTopRow = substituted(base.copy(keys = List<String?>(10) { null } + base.keys.drop(10)))
        assertEquals(merged("qwerty", LayoutScript.TL, false).size - 1, withoutTopRow.size)
        assertEquals("a", withoutTopRow[1][0].label)
    }

    @Test
    fun `a removed emoji key leaves the keyboard`() {
        val rows = substituted(unchanged("phahTaigi", LayoutScript.TL).copy(hasEmojiKey = false))
        assertTrue(rows.flatten().none { it.code == KeyCode.SWITCH_TO_MEDIA_CONTEXT })
    }

    @Test
    fun `the email key goes before space once the comma leaves the bottom row`() {
        // trace: Lohankha place 26 = `,` (bottom row), place 0 = q.
        val layout = unchanged("qwerty", LayoutScript.TL).let { it.copy(keys = it.keys.swapped(0, 26)) }
        val rows = substituted(layout)
        assertEquals(KeyVariation.ALL, rows[1][0].variation)
        assertEquals(
            listOf(KeyCode.VIEW_SYMBOLS, 'q'.code, KeyCode.SWITCH_TO_MEDIA_CONTEXT, '@'.code, KeyCode.SPACE, '-'.code, KeyCode.TRANSLATE, KeyCode.ENTER),
            codes(visible(rows, KeyVariation.EMAIL_ADDRESS).last()),
        )
        assertEquals(
            listOf(KeyCode.VIEW_SYMBOLS, 'q'.code, KeyCode.SWITCH_TO_MEDIA_CONTEXT, KeyCode.SPACE, '-'.code, KeyCode.TRANSLATE, KeyCode.ENTER),
            codes(visible(rows, KeyVariation.NORMAL).last()),
        )
    }

    @Test
    fun `function keys swap within their row, the translate copies as one`() {
        // trace: function places 2 = 123, 6 = return (bottom row).
        val layout = unchanged("qwerty", LayoutScript.TL).let { it.copy(functionKeys = it.functionKeys.swapped(2, 6)) }
        val bottomRow = substituted(layout).last()
        assertEquals(KeyCode.ENTER, bottomRow.first().code)
        assertEquals(KeyCode.VIEW_SYMBOLS, bottomRow.last().code)
        assertEquals(3, bottomRow.count { it.code == KeyCode.TRANSLATE })
    }

    @Test
    fun `validation`() {
        val layout = unchanged("qwerty", LayoutScript.TL)
        assertTrue(tables.isValid(layout))
        assertTrue(tables.isValid(layout.copy(keys = layout.keys.swapped(0, 26), functionKeys = layout.functionKeys.swapped(2, 6))))
        assertTrue(tables.isValid(layout.copy(keys = listOf("oo") + layout.keys.drop(1))))
        assertTrue(tables.isValid(layout.copy(keys = listOf(null) + layout.keys.drop(1))))
        // A TL layout keeps 28 places whatever the live mode; the POJ table has 29.
        assertFalse(tables.isValid(layout.copy(script = LayoutScript.POJ)))
        assertFalse("count", tables.isValid(layout.copy(keys = layout.keys.drop(1))))
        assertFalse("unknown", tables.isValid(layout.copy(keys = listOf("xyz") + layout.keys.drop(1))))
        assertFalse("duplicate", tables.isValid(layout.copy(keys = listOf("w") + layout.keys.drop(1))))
        assertFalse("across rows", tables.isValid(layout.copy(functionKeys = layout.functionKeys.swapped(0, 4))))
        assertFalse("function count", tables.isValid(layout.copy(functionKeys = layout.functionKeys.drop(1))))
        assertFalse("tps", tables.isValid(layout.copy(base = "tps")))
    }

    private fun <T> List<T>.swapped(
        first: Int,
        second: Int,
    ): List<T> = toMutableList().also { java.util.Collections.swap(it, first, second) }
}
