// Unit tests for custom long-press lists: the record's edits and JSON, the built-in lists and tray
// characters CustomLayoutTables offers, and the lists the keyboard shows. Mirrors the long-press part
// of iOS CustomLayoutTests.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.substituted
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.tables
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.unchanged
import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CustomLayoutCalloutsTest {
    private val qwerty = unchanged("qwerty", LayoutScript.TL)

    private fun CustomLayout.withList(
        label: String,
        entries: List<String>,
    ) = tables.withCallouts(this, label, entries)

    private fun ComputedLayoutDataArrangement.key(label: String): KeyData = drop(1).flatten().first { it.label == label }

    @Test
    fun `a list round-trips through JSON and an old record has none`() {
        val edited = qwerty.withList(",", listOf("{", "}")).withList("a", emptyList())
        assertEquals(listOf(edited), CustomLayout.decodeList(CustomLayout.encodeList(listOf(edited)), tables))

        val old = JSONArray(CustomLayout.encodeList(listOf(qwerty))).apply { getJSONObject(0).remove("callouts") }.toString()
        assertEquals(listOf(qwerty), CustomLayout.decodeList(old, tables))
    }

    @Test
    fun `entries are trimmed, deduplicated and capped`() {
        val edited = qwerty.withList(",", listOf(" { ", "", "}", "{") + (1..10).map(Int::toString))
        assertEquals(listOf("{", "}", "1", "2", "3", "4", "5", "6"), edited.callouts[","])
    }

    @Test
    fun `the built-in list is not stored and an empty list is`() {
        // trace: taigi_tl.json `a` = á à â ǎ ā a̍ a̋; qwerty_tl `a` has no popup of its own.
        val builtIn = tables.defaultCallouts(qwerty, "a")
        assertEquals(listOf("á", "à", "â", "ǎ", "ā", "a̍", "a̋"), builtIn)
        assertTrue(qwerty.withList("a", builtIn).callouts.isEmpty())
        assertEquals(emptyList<String>(), qwerty.withList("a", emptyList()).callouts["a"])
        assertTrue(
            qwerty
                .withList("a", emptyList())
                .withDefaultCallouts("a")
                .callouts
                .isEmpty(),
        )
        assertFalse(tables.isUnchanged(qwerty.withList("a", emptyList())))
    }

    @Test
    fun `built-in lists come from the base's own key`() {
        // trace: qwerty_moe1 `,` popup = ， 、 ； ：; default_halfwidth `-` popup = · – — ﹏.
        assertEquals(listOf("，", "、", "；", "："), tables.defaultCallouts(unchanged("moe1", LayoutScript.TL), ","))
        assertEquals(listOf("·", "–", "—", "﹏"), tables.defaultCallouts(qwerty, "-"))
    }

    @Test
    fun `a key leaving the board drops its list`() {
        val edited = qwerty.withList("a", listOf("x"))
        val index = qwerty.keys.indexOf("a")
        assertTrue(edited.removed(LayoutPlace.Key(index)).callouts.isEmpty())
        assertTrue(edited.replaced(LayoutPlace.Key(index), "ee").callouts.isEmpty())
    }

    @Test
    fun `the keyboard shows a key's own list in place of the built-in one`() {
        val table = tables.baseTable("qwerty", LayoutScript.TL)!!
        val swapped = qwerty.swapped(LayoutPlace.Key(qwerty.keys.indexOf(",")), LayoutPlace.Key(0), table)
        val edited = swapped.withList(",", listOf("{", "an")).withList("a", emptyList())
        val rows = substituted(edited)
        rows.key("a").popup.add(KeyData(225, "á"))
        tables.applyCallouts(rows, edited)

        // `,` moved to the first place and kept its list; one code point types itself, `an` has code 0.
        assertEquals(",", rows[1][0].label)
        assertEquals(listOf(KeyData('{'.code, "{"), KeyData(0, "an")), rows.key(",").popup)
        assertEquals(emptyList<KeyData>(), rows.key("a").popup)
        // A key without a list keeps the built-in one.
        assertEquals(listOf("·", "–", "—", "﹏"), rows.key("-").popup.map { it.label })
    }

    @Test
    fun `the full-width key shows the half-width key's list, with a key removed`() {
        val edited = qwerty.removed(LayoutPlace.Key(0)).withList(",", listOf("{")).withList("w", listOf("x"))
        val rows = substituted(edited, isFullWidthPunctuation = true)
        tables.applyCallouts(rows, edited)
        assertEquals(listOf("{"), rows.key("，").popup.map { it.label })
        assertEquals(listOf("x"), rows.key("w").popup.map { it.label })
        assertEquals(emptyList<KeyData>(), rows.key("e").popup)
    }

    @Test
    fun `tone rows and symbols for the tray`() {
        // trace: row a = TL á à â ǎ ā a̍ a̋, then POJ-only ă; ⁿ leaves the n row.
        assertEquals(listOf("á", "à", "â", "ǎ", "ā", "a̍", "a̋", "ă"), tables.toneRows.first())
        assertEquals(listOf("a", "e", "i", "o", "oo", "o͘", "u", "m", "n", "ng").size, tables.toneRows.size)
        assertFalse(tables.toneRows.flatten().contains("ⁿ"))

        val symbols = tables.calloutSymbols
        assertTrue(symbols.containsAll(listOf("{", "}", "「", "」", "：")))
        assertTrue(symbols.indexOf("{") < symbols.indexOf("「"))
        assertFalse(symbols.contains("▶"))
        assertFalse(symbols.any { it in tables.trayLetters })
        assertFalse(tables.isPlaceable("『"))
    }
}
