package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.theme.BuiltInThemes
import com.siansiansu.taigikeyboard.ime.theme.ThemeAppearance
import com.siansiansu.taigikeyboard.ime.theme.UserTheme
import com.siansiansu.taigikeyboard.ime.theme.UserThemeStore
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
        private const val EXTRA_BASE_THEME_ID = "base_theme_id"

        /**
         * [themeId] null = create a new theme; a UUID string = edit that theme. [baseThemeId] (create
         * only) = customize a new theme from that built-in.
         */
        fun createIntent(
            context: Context,
            themeId: String?,
            baseThemeId: String? = null,
        ): Intent =
            Intent(context, ThemeEditorActivity::class.java).apply {
                themeId?.let { putExtra(EXTRA_THEME_ID, it) }
                baseThemeId?.let { putExtra(EXTRA_BASE_THEME_ID, it) }
            }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        prefs.warmUp()
        val store = CompositionRoot.shared(this).userThemeStore(prefs)

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
                base = intent.getStringExtra(EXTRA_BASE_THEME_ID)?.let { BuiltInThemes.theme(it) },
                canSaveNew = { store.load().size < UserThemeStore.MAX_USER_THEMES },
                onSave = { name, appearance, baseThemeId -> saveTheme(prefs, store, editing, name, appearance, baseThemeId) },
                onNavigateBack = { onBackPressedDispatcher.onBackPressed() },
            )
        }
    }

    // A photo picked in the editor is only kept once its theme is saved: when the editor
    // really closes (not a rotation — the draft survives that with its photo) drop every
    // photo no saved theme references. Mutations sweep through the store's own hook.
    override fun onDestroy() {
        super.onDestroy()
        if (isFinishing) {
            val root = CompositionRoot.shared(this)
            root.themeImages.sweep(root.userThemeStore(PrefHelper(this)).load())
        }
    }

    // Returns false only when adding a NEW theme loses a cap race (caller re-shows
    // the cap dialog).
    private fun saveTheme(
        prefs: PrefHelper,
        store: UserThemeStore,
        editing: UserTheme?,
        name: String,
        appearance: ThemeAppearance,
        baseThemeId: String?,
    ): Boolean {
        val now = System.currentTimeMillis()
        if (editing == null) {
            val theme = UserTheme(UUID.randomUUID().toString(), name, appearance, createdAt = now, updatedAt = now, baseThemeId = baseThemeId)
            if (!store.add(theme)) return false
            prefs.selectedThemeId = theme.id
        } else {
            // Only apply if the theme still exists (a concurrent delete must not
            // resurrect an orphan selection); update() is a no-op for a missing id.
            if (store.load().none { it.id == editing.id }) {
                finish()
                return true
            }
            val theme = editing.copy(name = name, appearance = appearance, updatedAt = now)
            store.update(theme)
            prefs.selectedThemeId = theme.id
        }
        finish()
        return true
    }
}
