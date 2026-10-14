// Unit tests for the CustomLayout record: JSON (shared shape with iOS), active selection, the list cap.

package com.siansiansu.taigikeyboard.ime.text.layout

import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.tables
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutTestAssets.unchanged
import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CustomLayoutTest {
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
