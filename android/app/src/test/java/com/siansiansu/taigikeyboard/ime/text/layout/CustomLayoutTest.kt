// Unit tests for the CustomLayout record: JSON (shared shape with iOS), active selection, the list cap.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.tables
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.unchanged
import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CustomLayoutTest {
    @Test
    fun `picking a script keeps every base valid both ways`() {
        KeyboardLayoutOptions.romanization.forEach { option ->
            val poj = tables.withUse(unchanged(option.key, LayoutScript.TL), LayoutUse.POJ)
            assertEquals(LayoutScript.POJ, poj.script)
            assertTrue("${option.key} TL → POJ", tables.isValid(poj))
            assertTrue("${option.key} POJ → TL", tables.isValid(tables.withUse(poj, LayoutUse.TL)))
        }
    }

    @Test
    fun `picking the other script keeps the keys where the places match`() {
        val layout = unchanged("moe2", LayoutScript.TL).let { it.swapped(LayoutPlace.Key(0), LayoutPlace.Key(1), tables.baseTable("moe2", LayoutScript.TL)!!) }
        assertEquals(layout.keys, tables.withUse(layout, LayoutUse.POJ).keys)
    }

    @Test
    fun `QWERTY gets a removed key for the extra POJ place and drops it going back`() {
        // trace: QWERTY TL rows below digits hold 10 / 9 / 7 / 2 character keys, POJ 10 / 10 / 7 / 2
        // (o͘ ends row 2, slot 19).
        val tl = unchanged("qwerty", LayoutScript.TL)
        assertEquals(tl.keys.take(19) + null + tl.keys.drop(19), tables.withUse(tl, LayoutUse.POJ).keys)
        val poj =
            tables.withCallouts(tables.withCallouts(unchanged("qwerty", LayoutScript.POJ), "o\u0358", listOf("1")), "a", listOf("2"))
        assertEquals("o\u0358", poj.keys[19])
        val back = tables.withUse(poj, LayoutUse.TL)
        assertEquals(poj.keys.take(19) + poj.keys.drop(20), back.keys)
        assertEquals(mapOf("a" to listOf("2")), back.callouts)
    }

    @Test
    fun `the use defaults to the base table and old records decode the same`() {
        assertEquals(LayoutUse.UNIVERSAL, unchanged("phahTaigi", LayoutScript.POJ).use)
        assertEquals(LayoutUse.POJ, unchanged("moe1", LayoutScript.POJ).use)
        assertEquals(LayoutUse.UNIVERSAL, tables.withUse(unchanged("moe1", LayoutScript.POJ), LayoutUse.UNIVERSAL).use)
        listOf(unchanged("phahTaigi", LayoutScript.TL), unchanged("moe1", LayoutScript.TL)).forEach { layout ->
            val record = JSONArray(CustomLayout.encodeList(listOf(layout))).getJSONObject(0)
            assertTrue(record.has("isUniversal"))
            record.remove("isUniversal")
            assertEquals(listOf(layout), CustomLayout.decodeList(JSONArray().put(record).toString(), tables))
        }
    }

    @Test
    fun `a list round-trips through JSON`() {
        val edited =
            unchanged("moe2", LayoutScript.POJ, id = "b").let {
                it.copy(keys = listOf(null) + it.keys.drop(1), hasEmojiKey = false)
            }
        val layouts = listOf(unchanged("qwerty", LayoutScript.TL, id = "a"), edited)
        assertEquals(layouts, CustomLayout.decodeList(CustomLayout.encodeList(layouts), tables))
    }

    @Test
    fun `a record without function keys or the emoji flag takes the base's`() {
        val json =
            CustomLayout.encodeList(listOf(unchanged("phahTaigi", LayoutScript.TL))).let { encoded ->
                val record = JSONArray(encoded).getJSONObject(0)
                record.remove("functionKeys")
                record.remove("hasEmojiKey")
                JSONArray().put(record).toString()
            }
        assertEquals(listOf(unchanged("phahTaigi", LayoutScript.TL)), CustomLayout.decodeList(json, tables))
    }

    @Test
    fun `a bad record is dropped and the others stay`() {
        val good = unchanged("qwerty", LayoutScript.TL, id = "good")
        val array = JSONArray(CustomLayout.encodeList(listOf(good)))
        array.put(JSONArray(CustomLayout.encodeList(listOf(good.copy(id = "unknown key", keys = listOf("xyz") + good.keys.drop(1))))).get(0))
        array.put(JSONArray(CustomLayout.encodeList(listOf(good.copy(id = "bad script")))).getJSONObject(0).put("script", "tps"))
        array.put(JSONArray(CustomLayout.encodeList(listOf(good.copy(id = "bad key type")))).getJSONObject(0).put("keys", JSONArray().put(1)))
        array.put("not an object")
        array.put(JSONArray(CustomLayout.encodeList(listOf(good.copy(id = "last")))).get(0))
        assertEquals(listOf("good", "last"), CustomLayout.decodeList(array.toString(), tables).map { it.id })
        assertEquals(emptyList<CustomLayout>(), CustomLayout.decodeList("{corrupt", tables))
    }

    @Test
    fun `the active layout is the selected one on its base`() {
        val layouts = listOf(unchanged("qwerty", LayoutScript.TL, id = "a"), unchanged("moe1", LayoutScript.TL, id = "b"))
        assertEquals("b", CustomLayout.active(layouts, "b", "moe1")?.id)
        // TPS entered through the input mode stores the TPS layout: the selection waits.
        assertNull(CustomLayout.active(layouts, "b", "tps"))
        assertNull(CustomLayout.active(layouts, "b", "qwerty"))
        assertNull(CustomLayout.active(layouts, "", "moe1"))
    }

    @Test
    fun `saving replaces a saved layout and stops at the cap`() {
        val layouts = (1..CustomLayout.MAX_CUSTOM_LAYOUTS).map { unchanged("qwerty", LayoutScript.TL, id = "$it") }
        assertNull(CustomLayout.upserted(layouts, unchanged("moe2", LayoutScript.TL, id = "new")))
        val renamed = layouts[2].copy(name = "Renamed")
        assertEquals(layouts.toMutableList().also { it[2] = renamed }, CustomLayout.upserted(layouts, renamed))
        assertEquals(listOf("1", "new"), CustomLayout.upserted(layouts.take(1), unchanged("moe2", LayoutScript.TL, id = "new"))?.map { it.id })
    }
}
