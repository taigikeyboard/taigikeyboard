package com.siansiansu.taigikeyboard.ui.tabs.theme

import com.siansiansu.taigikeyboard.ime.core.ThemeId
import org.junit.Assert.assertEquals
import org.junit.Test

// Orphan-guard for deleting a custom theme: the selection only falls back to the
// default when the deleted theme was the active one.
class ThemePickerSelectionTest {
    @Test
    fun selectionAfterDelete_deletingActiveTheme_fallsBackToDefault() {
        val activeId = "11111111-1111-1111-1111-111111111111"
        assertEquals(ThemeId.DEFAULT, selectionAfterDelete(deletedId = activeId, currentSelection = activeId))
    }

    @Test
    fun selectionAfterDelete_deletingInactiveTheme_keepsSelection() {
        val activeId = "11111111-1111-1111-1111-111111111111"
        val otherId = "22222222-2222-2222-2222-222222222222"
        assertEquals(activeId, selectionAfterDelete(deletedId = otherId, currentSelection = activeId))
    }

    @Test
    fun selectionAfterDelete_whileOnBuiltIn_keepsBuiltIn() {
        val deletedId = "11111111-1111-1111-1111-111111111111"
        assertEquals("standardBlue", selectionAfterDelete(deletedId = deletedId, currentSelection = "standardBlue"))
    }
}
