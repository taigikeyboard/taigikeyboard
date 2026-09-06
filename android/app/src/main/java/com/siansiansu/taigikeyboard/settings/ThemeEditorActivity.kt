package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.core.UserTheme
import com.siansiansu.taigikeyboard.ime.core.UserThemeStore
import com.siansiansu.taigikeyboard.ui.setTaigiContent
import com.siansiansu.taigikeyboard.ui.tabs.theme.ThemeEditorScreen
import java.util.UUID

// Hosts the user-theme editor (create or edit). Launched from the theme picker;
// a full-screen Activity (not an intra-tab nav child) so the pinned preview is not
// squeezed by the bottom tab bar. Persists via UserThemeStore + auto-applies on save;
// the picker's shelf and selection refresh reactively via PrefHelper Flows.
class ThemeEditorActivity : ComponentActivity() {
    companion object {
        private const val EXTRA_THEME_ID = "theme_id"

        /** [themeId] null = create a new theme; a UUID string = edit that theme. */
        fun createIntent(
            context: Context,
            themeId: String?,
        ): Intent =
            Intent(context, ThemeEditorActivity::class.java).apply {
                themeId?.let { putExtra(EXTRA_THEME_ID, it) }
            }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        prefs.warmUp()
        val store = UserThemeStore(read = { prefs.userThemes }, write = { prefs.userThemes = it })

        // Create mode = no id; edit mode = a requested id loaded fresh from the store
        // (never a stale snapshot). An edit request whose theme no longer exists (e.g.
        // deleted before the editor opened, or across recreation) is a no-op — it must
        // NOT silently become a new-theme creation.
        val requestedEditId = intent.getStringExtra(EXTRA_THEME_ID)
        val editing = requestedEditId?.let { id -> store.load().firstOrNull { it.id == id } }
        if (requestedEditId != null && editing == null) {
            finish()
            return
        }

        // i18n live-switch root: a standalone Activity (launched from the picker), so it
        // provides its own resolver like SettingsMainActivity / the other detail Activities.
        setTaigiContent(prefs) {
            ThemeEditorScreen(
                prefs = prefs,
                editing = editing,
                canSaveNew = { store.load().size < UserThemeStore.MAX_USER_THEMES },
                onSave = { name, appearance -> saveTheme(prefs, store, editing, name, appearance) },
                onNavigateBack = { onBackPressedDispatcher.onBackPressed() },
            )
        }
    }

    // Returns false only when adding a NEW theme loses a cap race (caller re-shows
    // the cap dialog). Strips any background gradient — custom themes are flat;
    // gradients are a built-in-only feature the editor cannot author. (iOS omits
    // this strip — its draft type also can't hold a gradient; Android keeps an
    // explicit guard against future field-add drift.)
    private fun saveTheme(
        prefs: PrefHelper,
        store: UserThemeStore,
        editing: UserTheme?,
        name: String,
        appearance: ThemeAppearance,
    ): Boolean {
        val sanitized = appearance.copy(colors = appearance.colors.copy(backgroundGradient = null))
        val now = System.currentTimeMillis()
        if (editing == null) {
            val theme = UserTheme(UUID.randomUUID().toString(), name, sanitized, createdAt = now, updatedAt = now)
            if (!store.add(theme)) return false
            prefs.selectedThemeId = theme.id
        } else {
            // Only apply if the theme still exists (a concurrent delete must not
            // resurrect an orphan selection); update() is a no-op for a missing id.
            if (store.load().none { it.id == editing.id }) {
                finish()
                return true
            }
            val theme = editing.copy(name = name, appearance = sanitized, updatedAt = now)
            store.update(theme)
            prefs.selectedThemeId = theme.id
        }
        finish()
        return true
    }
}
