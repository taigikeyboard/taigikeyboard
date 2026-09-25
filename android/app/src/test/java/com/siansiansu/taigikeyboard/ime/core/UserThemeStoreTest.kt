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

    // A fully seeded theme (what the editor saves), with a blue solid background so it
    // differs from the seed and survives load()'s seeding untouched.
    private fun theme(
        name: String = "T",
        shadow: Float = 0f,
    ): UserTheme {
        val colors = UserThemeSeed.colors.copy(background = ThemeBackground.Solid(0xFF0000FF.toInt()))
        val appearance = ThemeAppearance.USER_THEME_SEED.copy(colors = colors, keyShadowIntensity = shadow)
        return UserTheme(UUID.randomUUID().toString(), name, appearance, createdAt = 0L, updatedAt = 0L)
    }

    // Every persisted mutation hands the new list to the hook (the photo sweep); a rejected add does not.
    @Test
    fun mutations_runHookWithNewList() {
        val memory = MemoryStore()
        val seen = mutableListOf<List<UserTheme>>()
        val store = UserThemeStore(read = { memory.json }, write = { memory.json = it }, onMutated = { seen += it })
        val t = theme("A")
        store.add(t)
        store.update(t.copy(name = "B"))
        store.delete(t.id)
        assertEquals(listOf(listOf("A"), listOf("B"), emptyList()), seen.map { list -> list.map { it.name } })
        repeat(UserThemeStore.MAX_USER_THEMES) { store.add(theme("T$it")) }
        val before = seen.size
        store.add(theme("overflow"))
        assertEquals("a rejected add persists nothing", before, seen.size)
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
            colors = UserThemeSeed.colors.copy(background = ThemeBackground.Solid(0xFF00FF00.toInt())),
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

    // A theme saved with null roles (before the seed existed) decodes with every null role
    // filled from UserThemeSeed and the set roles untouched -> the theme no longer follows
    // light / dark (USER 2026-09-19); nothing is written back. Mirrors iOS
    // testLoad_seedsNilRoles_keepsSetRoles (Android seeds in UserTheme.fromJson, so every
    // read path — store, PrefHelper.loadUserThemes, observeUserThemes — is covered).
    @Test
    fun load_seedsNullRoles_keepsSetRoles() {
        val partial = ThemeAppearance.DEFAULT.copy(colors = KeyboardColorSettings(keyTextColor = 0xFF112233.toInt()))
        val t = UserTheme(UUID.randomUUID().toString(), "Partial", partial, createdAt = 0L, updatedAt = 0L)
        val memory = MemoryStore().apply { json = UserTheme.encodeList(listOf(t)) }

        val loaded = memory.store().load().single()

        assertEquals(0xFF112233.toInt(), loaded.appearance.colors.keyTextColor)
        assertEquals(UserThemeSeed.BACKGROUND, loaded.appearance.colors.background)
        assertEquals(UserThemeSeed.KEY_FILL, loaded.appearance.colors.normalKeyFillColor)
        assertEquals(UserThemeSeed.KEY_FILL, loaded.appearance.colors.specialKeyFillColor)
        assertEquals(UserThemeSeed.CANDIDATE_TEXT, loaded.appearance.colors.candidateTextColor)
        assertEquals("load never writes back", UserTheme.encodeList(listOf(t)), memory.json)
    }
}
