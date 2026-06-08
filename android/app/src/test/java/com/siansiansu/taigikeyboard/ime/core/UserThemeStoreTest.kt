package com.siansiansu.taigikeyboard.ime.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.UUID

/**
 * Tests for [UserThemeStore] — the JSON persistence for user-created themes,
 * exercised against an in-memory string holder. Mirrors iOS UserThemeStoreTests.
 */
class UserThemeStoreTest {
    private class MemoryStore {
        var json = "[]"

        fun store(): UserThemeStore = UserThemeStore(read = { json }, write = { json = it })
    }

    private fun theme(name: String = "T", shadow: Float = 0f): UserTheme {
        val colors = KeyboardColorSettings(backgroundColor = 0xFF0000FF.toInt())
        val appearance = ThemeAppearance.DEFAULT.copy(colors = colors, keyShadowIntensity = shadow)
        return UserTheme(UUID.randomUUID().toString(), name, appearance, createdAt = 0L, updatedAt = 0L)
    }

    @Test
    fun load_emptyStore_returnsEmpty() {
        assertEquals(emptyList<UserTheme>(), MemoryStore().store().load())
    }

    @Test
    fun addAndLoad_roundTrip() {
        val store = MemoryStore().store()
        val t = theme("Mine", shadow = 0.3f)
        assertTrue(store.add(t))
        assertEquals(listOf(t), store.load())
    }

    @Test
    fun add_atCap_rejectsSixth() {
        val store = MemoryStore().store()
        repeat(UserThemeStore.MAX_USER_THEMES) { assertTrue(store.add(theme("T$it"))) }
        assertFalse(store.add(theme("overflow")))
        assertEquals(UserThemeStore.MAX_USER_THEMES, store.load().size)
    }

    @Test
    fun update_replacesById() {
        val store = MemoryStore().store()
        val t = theme("Before")
        store.add(t)
        store.update(t.copy(name = "After"))
        assertEquals(listOf("After"), store.load().map { it.name })
    }

    @Test
    fun update_absentId_noOps() {
        val store = MemoryStore().store()
        store.add(theme("Keep"))
        store.update(theme("Ghost"))
        assertEquals(listOf("Keep"), store.load().map { it.name })
    }

    @Test
    fun delete_removesById() {
        val store = MemoryStore().store()
        val keep = theme("Keep")
        val drop = theme("Drop")
        store.add(keep)
        store.add(drop)
        store.delete(drop.id)
        assertEquals(listOf("Keep"), store.load().map { it.name })
    }

    @Test
    fun load_corruptJson_returnsEmpty() {
        val store = UserThemeStore(read = { "not json" }, write = {})
        assertEquals(emptyList<UserTheme>(), store.load())
    }

    @Test
    fun addAndLoad_fullAppearance_roundTrips() {
        val store = MemoryStore().store()
        val appearance = ThemeAppearance(
            colors = KeyboardColorSettings(backgroundColor = 0xFF00FF00.toInt()),
            keyShadowIntensity = 0.4f,
            keyHeightScale = 1.1f,
            keyFontSizeScale = 0.9f,
            candidateTextSizeScale = 1.05f,
            keyCornerRadius = 12f,
            keyBorderWidth = 1.5f,
        )
        val t = UserTheme(UUID.randomUUID().toString(), "Full", appearance, createdAt = 0L, updatedAt = 0L)
        assertTrue(store.add(t))
        assertEquals(listOf(t), store.load())
    }
}
