// Unit tests for custom layouts over the TPS table: the table without a digit row, the TPS tray,
// placement, validation, the keyboard substitution and long-press lists. Mirrors the TPS section of
// iOS CustomLayoutTests.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.labels
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.merged
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.substituted
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.tables
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.unchanged
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.visible
import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CustomLayoutTpsTest {
    private val tps = unchanged("tps", LayoutScript.TL, id = "t")

    @Test
    fun `a fresh TPS layout is the TPS table with every row editable`() {
        val table = tables.baseTable("tps", LayoutScript.TL)!!
        assertEquals(TPS_KEYS, table.labels)
        assertEquals(TPS_KEYS, tps.keys)
        // trace: tps.json row 3 ends in delete; mod row 1 = 123, ㄌㆡㄙㆨㆤ, space, emoji, enter.
        assertEquals(
            listOf(FunctionKey.BACKSPACE, FunctionKey.NUMERIC, FunctionKey.SPACE, FunctionKey.EMOJI, FunctionKey.RETURN),
            table.functionKeys,
        )
        assertEquals(listOf(3, 4, 4, 4, 4), table.functionRows)
        assertEquals(emptyList<String>(), table.digits)
        // No digit row: row 0's ten glyphs are slots 0–9; row 3's delete is function 0.
        val rows = tables.editorRows(tps)
        assertEquals((0 until 10).map(LayoutPlace::Key), rows[0].map { it.place })
        assertEquals(LayoutPlace.Function(0), rows[3][9].place)
        assertEquals(LayoutUse.TPS, tps.use)
        assertFalse(tps.isUniversal)
        assertTrue(tables.isValid(tps))
        // TPS has one table, whatever the script.
        assertEquals(table, tables.baseTable("tps", LayoutScript.POJ))
    }

    @Test
    fun `an unchanged TPS layout shows the built-in keys`() {
        val built = merged("tps", LayoutScript.TL, isFullWidthPunctuation = true)
        assertEquals(built.map { labels(it) }, substituted(tps, isFullWidthPunctuation = true).map { labels(it) })
    }

    @Test
    fun `the tray offers the variants as letters and the board glyphs with digits for long-press`() {
        assertEquals(TPS_VARIANTS, tables.trayLetters(tps))
        // trace: every board glyph (TPS_KEYS minus the symbol-page `,`), then 1…9, 0.
        assertEquals(TPS_KEYS.filter { it != "," } + "1234567890".map(Char::toString), tables.trayTpsCallouts(tps))
        assertEquals(emptyList<String>(), tables.trayTpsCallouts(unchanged("phahTaigi", LayoutScript.TL)))
    }

    @Test
    fun `placement keeps TPS glyphs and Latin letters apart`() {
        val key = LayoutPlace.Key(0)
        assertTrue("variant", tables.canReplace(tps, key, "ㆴ"))
        assertTrue("symbol page", tables.canReplace(tps, key, "~"))
        assertTrue("second symbol page", tables.canReplace(tps, key, "{"))
        assertFalse("Latin letter", tables.canReplace(tps, key, "a"))
        assertFalse("digit", tables.canReplace(tps, key, "1"))
        assertFalse("already on the board", tables.canReplace(tps, key, "ㄅ"))
        val romanization = unchanged("phahTaigi", LayoutScript.TL)
        assertFalse("TPS glyph on romanization", tables.canReplace(romanization, key, "ㆴ"))
        assertTrue(tables.canReplace(romanization, key, "an"))

        val edited = tps.replaced(LayoutPlace.Key(10), "ㆴ")
        assertTrue(tables.isValid(edited))
        assertTrue("ㄅ" in tables.trayLetters(edited))
        // trace: no digit row, so slot 10 opens arrangement row 1; a variant is a plain code-0 glyph key.
        val variant = substituted(edited, isFullWidthPunctuation = true)[1][0]
        assertEquals("ㆴ", variant.label)
        assertEquals(0, variant.code)
    }

    @Test
    fun `stored TPS records are validated per base and never universal`() {
        fun decode(
            layout: CustomLayout,
            isUniversal: Boolean? = null,
        ): CustomLayout? {
            val record = layout.toJson().apply { if (isUniversal != null) put("isUniversal", isUniversal) else remove("isUniversal") }
            return CustomLayout.decodeList(JSONArray().put(record).toString(), tables).firstOrNull()
        }
        assertEquals(tps, decode(tps))
        assertFalse(decode(tps, isUniversal = true)!!.isUniversal)
        assertNull("Latin letter on TPS", decode(tps.copy(keys = listOf("a") + tps.keys.drop(1))))
        assertNull("romanization keys on TPS", decode(tps.copy(keys = unchanged("phahTaigi", LayoutScript.TL).keys)))
        val romanization = unchanged("phahTaigi", LayoutScript.TL)
        assertNull("TPS glyph on romanization", decode(romanization.copy(keys = listOf("ㆴ") + romanization.keys.drop(1))))
    }

    @Test
    fun `use changes keep a TPS layout TPS and a romanization one off TPS`() {
        assertEquals(tps, tables.withUse(tps, LayoutUse.UNIVERSAL))
        val romanization = unchanged("moe1", LayoutScript.TL)
        assertEquals(romanization, tables.withUse(romanization, LayoutUse.TPS))
        assertEquals(listOf(LayoutUse.TPS), tps.editorUses)
        assertEquals(KeyboardLayoutOptions.phonetic, tps.editorBases)
    }

    @Test
    fun `a moved comma keeps its full-width form and shows in email fields`() {
        // trace: `,` is slot 38 (row 3's last character); slot 39 is ㄌ, the bottom row's first glyph.
        val swapped = tps.swapped(LayoutPlace.Key(38), LayoutPlace.Key(39), tables.baseTable("tps", LayoutScript.TL)!!)
        val rows = substituted(swapped, isFullWidthPunctuation = true)
        assertTrue("，" in labels(rows.last()))
        assertTrue("，" in labels(visible(rows, KeyVariation.EMAIL_ADDRESS).last()))
        assertTrue("，" in labels(visible(rows, KeyVariation.URI).last()))
        // A tray symbol takes its full-width form under TPS.
        val question = substituted(tps.replaced(LayoutPlace.Key(0), "?"), isFullWidthPunctuation = true)
        assertEquals("？", question[0][0].label)
    }

    @Test
    fun `a TPS glyph's built-in long-press lists itself first and follows the key`() {
        // trace: tps.json ㄅ popup [ㆴ], ㆠ popup [1]; the keyboard prepends the glyph (tpsPopupWithBaseGlyph).
        assertEquals(listOf("ㄅ", "ㆴ"), tables.callouts(tps, "ㄅ"))
        assertEquals(listOf("ㆠ", "1"), tables.callouts(tps, "ㆠ"))
        assertEquals(emptyList<String>(), tables.callouts(tps, "ㄧ"))
        val swapped = tps.swapped(LayoutPlace.Key(0), LayoutPlace.Key(10), tables.baseTable("tps", LayoutScript.TL)!!)
        assertEquals(listOf("ㆠ", "1"), tables.callouts(swapped, "ㆠ"))
    }

    @Test
    fun `a custom list replaces a TPS key's popup and names both comma forms`() {
        val custom = tables.withCallouts(tables.withCallouts(tps, "ㄅ", listOf("ㄅ", "ㆴ", "1")), ",", listOf("。", "、"))
        val rows = substituted(custom, isFullWidthPunctuation = true)
        tables.applyCallouts(rows, custom, isFullWidthPunctuation = true)
        assertEquals(listOf("ㄅ", "ㆴ", "1"), rows[1][0].popup.map { it.label })
        assertEquals(listOf("。", "、"), rows[3].first { it.label == "，" }.popup.map { it.label })
        assertEquals(setOf("ㄅ", ",", "，"), tables.customCalloutLabels(custom))
    }

    @Test
    fun `the active layout reads the TPS slot while the layout is TPS`() {
        val romanization = unchanged("moe1", LayoutScript.TL, id = "r")
        val layouts = listOf(romanization, tps)
        assertEquals(tps, CustomLayout.active(layouts, romanizationId = "r", tpsId = "t", layoutType = "tps"))
        assertEquals(romanization, CustomLayout.active(layouts, romanizationId = "r", tpsId = "t", layoutType = "moe1"))
        assertNull(CustomLayout.active(layouts, romanizationId = "r", tpsId = "", layoutType = "tps"))
        // Shown only where its table is live; never in English.
        assertEquals(tps, CustomLayout.shown(tps, inputMode = "tps", layoutType = "tps"))
        assertNull(CustomLayout.shown(tps, inputMode = "tl", layoutType = "moe1"))
        assertNull(CustomLayout.shown(romanization, inputMode = "tps", layoutType = "moe1"))
        assertNull(CustomLayout.shown(romanization, inputMode = "english", layoutType = "moe1"))
        // The mode and layout prefs briefly disagree mid-cascade: either one on TPS reads as TPS.
        assertEquals(tps, CustomLayout.shown(tps, inputMode = "tl", layoutType = "tps"))
        assertNull(CustomLayout.shown(romanization, inputMode = "tl", layoutType = "tps"))
    }

    private companion object {
        // trace: characters/tps.json rows 0–3 (row 3's `，` read as its half label `,`), then mod/tps_halfwidth.json row 1.
        val TPS_KEYS =
            listOf(
                "ㆠ",
                "ˋ",
                "˪",
                "ㆣ",
                "ˊ",
                "ˇ",
                "˫",
                "˙",
                "ㆪ",
                "ㆩ",
                "ㄅ",
                "ㄉ",
                "ㄍ",
                "ㄏ",
                "ㄧ",
                "ㄚ",
                "ㄞ",
                "ㄤ",
                "ㆫ",
                "ㆧ",
                "ㄆ",
                "ㄊ",
                "ㄎ",
                "ㄗ",
                "ㄨ",
                "ㄛ",
                "ㄠ",
                "ㆲ",
                "ㆥ",
                "ㆮ",
                "ㄇ",
                "ㄋ",
                "ㄫ",
                "ㄘ",
                "ㄜ",
                "ㆦ",
                "ㄢ",
                "ㆰ",
                ",",
                "ㄌ",
                "ㆡ",
                "ㄙ",
                "ㆨ",
                "ㆤ",
            )

        // trace: each glyph's non-ASCII popup entries in table order: ㆪ ㆳ · ㄅ ㆴ · ㄉ ㆵ · ㄍ ㆻ · ㄏ ㆷ ·
        // ㄗ ㄐ · ㆮ ㆯ · ㄇ ㆬ · ㄋ ㄣ · ㄫ ㆭ ㄥ · ㄘ ㄑ · ㆰ ㆱ · ㆡ ㆢ · ㄙ ㄒ · ㆤ ㄝ (iOS tpsVariants).
        val TPS_VARIANTS = listOf("ㆳ", "ㆴ", "ㆵ", "ㆻ", "ㆷ", "ㄐ", "ㆯ", "ㆬ", "ㄣ", "ㆭ", "ㄥ", "ㄑ", "ㆱ", "ㆢ", "ㄒ", "ㄝ")
    }
}
