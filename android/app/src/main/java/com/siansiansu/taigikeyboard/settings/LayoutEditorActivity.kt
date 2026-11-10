package com.siansiansu.taigikeyboard.settings

import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.settings.PrefHelper
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayoutTables
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutScript
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutUse
import com.siansiansu.taigikeyboard.ui.setTaigiContent
import com.siansiansu.taigikeyboard.ui.tabs.layout.LayoutEditorScreen
import java.util.UUID

// Hosts the custom-layout editor (create or edit), launched from the Layout tab; a full-screen
// Activity like ThemeEditorActivity, so the board and tray are not squeezed by the tab bar. Save
// stores and applies the layout (PrefHelper.saveCustomLayout); the Layout tab and the keyboard follow
// through their DataStore flows.
class LayoutEditorActivity : ComponentActivity() {
    companion object {
        private const val EXTRA_LAYOUT_ID = "layout_id"
        private const val EXTRA_NAME = "name"
        private const val EXTRA_USE = "use"

        /** Edits the layout [layoutId]. */
        fun editIntent(
            context: Context,
            layoutId: String,
        ): Intent = Intent(context, LayoutEditorActivity::class.java).putExtra(EXTRA_LAYOUT_ID, layoutId)

        /** A new layout named [name], shown where [use] says — both chosen in the Layout tab's create dialog. */
        fun createIntent(
            context: Context,
            name: String,
            use: LayoutUse,
        ): Intent =
            Intent(context, LayoutEditorActivity::class.java)
                .putExtra(EXTRA_NAME, name)
                .putExtra(EXTRA_USE, use.name)

        /**
         * A new layout starts from the layout in use — its base table in the current script — or from
         * PhahTaigi while the TPS layout is on. Its use is the create dialog's default. Mirrors iOS
         * `LayoutEditorView.newDraft`.
         */
        fun newDraft(
            prefs: PrefHelper,
            tables: CustomLayoutTables,
        ): CustomLayout {
            val active = prefs.activeCustomLayout
            val stored = prefs.keyboardLayoutType
            val base = active?.base ?: stored.takeIf { layout -> KeyboardLayoutOptions.romanization.any { it.key == layout } } ?: "phahTaigi"
            val script = active?.script ?: LayoutScript.fromStorage(prefs.romanizationInputMode) ?: LayoutScript.TL
            return checkNotNull(tables.fresh(UUID.randomUUID().toString(), "", base, script)) { "no $base table" }
        }

        /**
         * A new layout named [name]: [newDraft] in the script [use] names (a universal one keeps the
         * current script's table), shown where [use] says. Mirrors iOS `LayoutEditorView.newDraft(named:use:)`.
         */
        private fun newDraft(
            prefs: PrefHelper,
            tables: CustomLayoutTables,
            name: String,
            use: LayoutUse,
        ): CustomLayout {
            val start = newDraft(prefs, tables)
            val script = use.script ?: start.script
            val fresh = checkNotNull(tables.fresh(start.id, name, start.base, script)) { "no ${start.base} table" }
            // The script already matches, so this only sets the universal flag.
            return tables.withUse(fresh, use)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val prefs = PrefHelper(this)
        prefs.warmUp()
        val tables = CompositionRoot.shared(this).customLayoutTables

        // An edit request whose layout no longer exists (deleted before the editor opened, or across
        // recreation) closes the editor; it never becomes a new layout.
        val requestedEditId = intent.getStringExtra(EXTRA_LAYOUT_ID)
        val editing = requestedEditId?.let { id -> prefs.loadCustomLayouts().firstOrNull { it.id == id } }
        if (requestedEditId != null && editing == null) {
            finish()
            return
        }

        val initial =
            editing ?: newDraft(
                prefs,
                tables,
                name = intent.getStringExtra(EXTRA_NAME).orEmpty(),
                use = LayoutUse.valueOf(checkNotNull(intent.getStringExtra(EXTRA_USE)) { "a new layout needs its use" }),
            )
        setTaigiContent(prefs) {
            LayoutEditorScreen(
                tables = tables,
                initial = initial,
                isEditing = editing != null,
                // A failed save (a new layout past the cap) keeps the editor open with the draft.
                onSave = { layout -> if (prefs.saveCustomLayout(layout)) finish() },
                onNavigateBack = { onBackPressedDispatcher.onBackPressed() },
            )
        }
    }
}
