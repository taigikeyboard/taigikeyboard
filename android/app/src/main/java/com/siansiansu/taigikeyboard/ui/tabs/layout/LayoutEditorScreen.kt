package com.siansiansu.taigikeyboard.ui.tabs.layout

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.LocalStringResolver
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.i18n.stringRes
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayout
import com.siansiansu.taigikeyboard.ime.text.layout.CustomLayoutTables
import com.siansiansu.taigikeyboard.ime.text.layout.FunctionKey
import com.siansiansu.taigikeyboard.ime.text.layout.GridKey
import com.siansiansu.taigikeyboard.ime.text.layout.KeyboardLayoutOptions
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutKeyGrid
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutKeyGridInteraction
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutPlace
import com.siansiansu.taigikeyboard.ime.text.layout.accessibilityName
import com.siansiansu.taigikeyboard.ui.components.GALLERY_PREVIEW_ASPECT
import com.siansiansu.taigikeyboard.ui.components.NameDialog
import java.util.Collections

// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling), all by taps.
// Tap two keys to swap them (function keys within their row), a key then a spare to replace it, a key
// then the trash to remove it. The selected character key's long-press list sits in LayoutCalloutStrip
// below the board: tap two slots to swap or move, a slot then a tray character to fill it, a slot then
// the trash to empty it. Nothing is stored until Save, which names the layout and applies it; Back
// discards the draft. Mirrors iOS LayoutEditorView.

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LayoutEditorScreen(
    tables: CustomLayoutTables,
    initial: CustomLayout,
    isEditing: Boolean,
    onSave: (CustomLayout) -> Unit,
    onNavigateBack: () -> Unit,
) {
    // A saved draft that no longer decodes restores nothing, so the draft starts again from [initial].
    val draftSaver =
        remember(tables) {
            Saver<MutableState<CustomLayout>, String>(
                save = { CustomLayout.encodeList(listOf(it.value)) },
                restore = { saved -> CustomLayout.decodeList(saved, tables).firstOrNull()?.let(::mutableStateOf) },
            )
        }
    var draft by rememberSaveable(saver = draftSaver) { mutableStateOf(initial) }
    var selected by remember { mutableStateOf<LayoutPlace?>(null) }
    // The selected slot of the selected key's long-press list (past its end = an empty slot).
    var selectedCallout by remember { mutableStateOf<Int?>(null) }
    var selectedKeyCenterX by remember { mutableStateOf<Float?>(null) }
    var showNameDialog by rememberSaveable { mutableStateOf(false) }
    val table = checkNotNull(tables.baseTable(draft.base, draft.script)) { "no ${draft.base} table" }
    val isDark = isSystemInDarkTheme()
    val view = LocalView.current
    // Resolver captured for the name-dialog callback (runs outside composition), so the blank-name
    // fallback resolves under the live display language at save time.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)

    // The label of the selected character key; null for a function key, a removed key or none. The
    // handlers below call these instead of reading values captured at composition, so two taps before
    // a redraw both see the current draft.
    fun selectedLabel(): String? = (selected as? LayoutPlace.Key)?.let { draft.keys[it.index] }

    // The selected key's long-press list as shown; empty while no character key is selected.
    fun selectedCallouts(): List<String> = selectedLabel()?.let { tables.callouts(draft, it) }.orEmpty()

    fun changeDraft(next: CustomLayout) {
        if (next != draft) {
            draft = next
            view.performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
        }
    }

    fun select(place: LayoutPlace?) {
        selected = place
        selectedCallout = null
        // The new key reports its centre once laid out; until then the bubble shows no tail.
        selectedKeyCenterX = null
    }

    fun edit(next: CustomLayout) {
        changeDraft(next)
        select(null)
    }

    // A second key swaps with the selected one where it can (CustomLayout.canSwap: character keys
    // anywhere, function keys within their row); otherwise the tap moves or clears the selection.
    fun tap(place: LayoutPlace) {
        val current = selected
        if (current != null && draft.canSwap(current, place, table)) {
            edit(draft.swapped(current, place, table))
        } else {
            select(if (current == place) null else place)
        }
    }

    // Applies [change] to the selected key's list and clears the slot selection; the key stays selected.
    fun editCallouts(change: (MutableList<String>) -> Unit) {
        val label = selectedLabel() ?: return
        changeDraft(tables.withCallouts(draft, label, selectedCallouts().toMutableList().also(change)))
        selectedCallout = null
    }

    fun resetCallouts() {
        selectedLabel()?.let { changeDraft(draft.withDefaultCallouts(it)) }
        selectedCallout = null
    }

    // A second slot swaps with the selected one when both hold entries; an entry tapped with an empty
    // slot moves to the list's end (entries stay packed at the front). Tapping the selected slot clears it.
    fun tapCallout(index: Int) {
        val current = selectedCallout
        if (current == null || current == index) {
            selectedCallout = if (current == index) null else index
            return
        }
        val filled = selectedCallouts().indices
        when {
            current in filled && index in filled -> editCallouts { Collections.swap(it, current, index) }
            current in filled || index in filled -> editCallouts { it.add(it.removeAt(if (current in filled) current else index)) }
            else -> selectedCallout = index
        }
    }

    // Puts [spare] in slot [index]: replaces an entry, or adds one in an empty slot and moves the
    // selection to the next empty slot, so several characters go in one tap each.
    fun fillCallout(
        index: Int,
        spare: String,
    ) {
        val isAdding = index !in selectedCallouts().indices
        editCallouts { if (isAdding) it.add(spare) else it[index] = spare }
        // Read from the edited draft: a repeat is dropped, so the list may not have grown.
        val count = selectedCallouts().size
        if (isAdding && count < CustomLayout.MAX_CALLOUTS) selectedCallout = count
    }

    // A spare fills the selected long-press slot, else replaces the selected key (long-press-only
    // symbols cannot go on the board).
    fun replaceSelected(spare: String) {
        val slot = selectedCallout
        when {
            slot != null -> fillCallout(slot, spare)
            tables.isPlaceable(spare) -> selected?.let { edit(draft.replaced(it, spare)) }
        }
    }

    fun removeSelected() {
        val slot = selectedCallout
        if (slot != null) {
            if (slot in selectedCallouts().indices) editCallouts { it.removeAt(slot) }
            return
        }
        selected?.let { edit(draft.removed(it)) }
    }

    fun restart(base: String) {
        tables.fresh(draft.id, draft.name, base, draft.script)?.let { draft = it }
        select(null)
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.surfaceContainer,
        topBar = {
            TopAppBar(
                title = { Text(if (isEditing) draft.name else L10n.layoutEditorTitleNew) },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = L10n.commonBack)
                    }
                },
                actions = {
                    TextButton(onClick = { showNameDialog = true }) { Text(L10n.commonSave) }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
            )
        },
    ) { innerPadding ->
        BoxWithConstraints(
            Modifier
                .fillMaxSize()
                .padding(innerPadding),
        ) {
            // At most half the height, so the tray keeps room in landscape.
            val boardWidth = min(maxWidth - 40.dp, maxHeight * 0.5f * GALLERY_PREVIEW_ASPECT)
            val label = selectedLabel()
            val callouts = selectedCallouts()
            Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                BaseChips(selectedBase = draft.base, onSelect = ::restart)

                LayoutKeyGrid(
                    rows = remember(draft) { tables.editorRows(draft) },
                    base = draft.base,
                    isDark = isDark,
                    interaction =
                        LayoutKeyGridInteraction(
                            selected = selected,
                            removedPlaceLabel = L10n.settingsSyllableSeparatorNone,
                            onTap = ::tap,
                            onSelectedKeyCenterX = { selectedKeyCenterX = it },
                        ),
                    modifier =
                        Modifier
                            .align(Alignment.CenterHorizontally)
                            .width(boardWidth)
                            .aspectRatio(GALLERY_PREVIEW_ASPECT)
                            .clip(RoundedCornerShape(10.dp)),
                )

                // Always in place, so the tray never moves: empty slots until a character key is selected.
                LayoutCalloutStrip(
                    entries = if (label == null) null else callouts,
                    selectedSlot = selectedCallout,
                    canReset = label != null && label in draft.callouts,
                    tailX = selectedKeyCenterX.takeIf { selected != null },
                    onTapSlot = ::tapCallout,
                    onReset = ::resetCallouts,
                    modifier = Modifier.padding(start = 20.dp, end = 8.dp),
                )

                Column(
                    Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .padding(horizontal = 10.dp)
                        .verticalScroll(rememberScrollState())
                        .padding(horizontal = 10.dp, vertical = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    val canRemove = selectedCallout?.let { it in callouts.indices } ?: (selected?.let(draft::canRemove) == true)
                    // A character the selected key's list already holds stays in place, dimmed, so the
                    // rows keep their columns.
                    val held = remember(callouts) { tables.trayForms(callouts) }
                    SpareFlow(remember(draft) { draft.spares(tables.trayLetters) }, onTap = ::replaceSelected) {
                        TrayChip(onClick = ::removeSelected, enabled = canRemove, description = L10n.commonDelete) {
                            Icon(Icons.Outlined.Delete, contentDescription = null, tint = MaterialTheme.colorScheme.error)
                        }
                        if (!draft.hasEmojiKey) {
                            // Tap restores the emoji key at once.
                            TrayChip(
                                onClick = { edit(draft.restoreEmojiKey()) },
                                description = accessibilityName(GridKey.Face.Function(FunctionKey.EMOJI)),
                            ) {
                                Icon(painterResource(R.drawable.ic_sentiment_satisfied), contentDescription = null)
                            }
                        }
                    }
                    ToneGrid(tables.toneRows, held = held, onTap = ::replaceSelected)
                    SpareFlow(remember(draft) { tables.traySymbols(draft) }, onTap = ::replaceSelected, held = held)
                    TextButton(
                        onClick = { restart(draft.base) },
                        enabled = !remember(draft) { tables.isUnchanged(draft) },
                        colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                        modifier = Modifier.align(Alignment.CenterHorizontally),
                    ) {
                        Text(L10n.themeEditorResetAll)
                    }
                }
            }
        }

        if (showNameDialog) {
            NameDialog(
                title = L10n.layoutNameHeader,
                placeholder = L10n.layoutEditorTitleNew,
                initialName = draft.name,
                onConfirm = { entered ->
                    showNameDialog = false
                    val name = entered.trim().ifEmpty { stringResolver.resolve(StringKey.LAYOUT_EDITOR_TITLE_NEW) }
                    draft = draft.copy(name = name)
                    onSave(draft)
                },
                onDismiss = { showNameDialog = false },
            )
        }
    }
}

/** The four romanization bases; picking one restarts the draft from it. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BaseChips(
    selectedBase: String,
    onSelect: (String) -> Unit,
) {
    Row(
        modifier =
            Modifier
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = 20.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        KeyboardLayoutOptions.romanization.forEach { option ->
            FilterChip(
                selected = option.key == selectedBase,
                onClick = { onSelect(option.key) },
                label = { Text(stringRes(option.labelKey)) },
                // The accent fill of the iOS base chips.
                colors =
                    FilterChipDefaults.filterChipColors(
                        selectedContainerColor = MaterialTheme.colorScheme.primary,
                        selectedLabelColor = MaterialTheme.colorScheme.onPrimary,
                    ),
            )
        }
    }
}

/**
 * A run of tray chips, [leading] first, then one chip per spare (tap = put it on the selected key or
 * long-press slot); [held] chips are dimmed and inert.
 */
@Composable
private fun SpareFlow(
    spares: List<String>,
    onTap: (String) -> Unit,
    held: Set<String> = emptySet(),
    leading: @Composable () -> Unit = {},
) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        leading()
        spares.forEach { spare -> SpareChip(spare, onTap, spare in held) }
    }
}

/** Tone-marked letters, one row per base, columns aligned. */
@Composable
private fun ToneGrid(
    rows: List<List<String>>,
    held: Set<String>,
    onTap: (String) -> Unit,
) {
    val columns = rows.maxOfOrNull { it.size } ?: 0
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        rows.forEach { row ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                repeat(columns) { column ->
                    val char = row.getOrNull(column)
                    if (char == null) {
                        Spacer(Modifier.weight(1f))
                    } else {
                        SpareChip(char, onTap, char in held, Modifier.weight(1f))
                    }
                }
            }
        }
    }
}

@Composable
private fun SpareChip(
    spare: String,
    onTap: (String) -> Unit,
    isHeld: Boolean,
    modifier: Modifier = TRAY_CHIP_WIDTH,
) {
    TrayChip(onClick = { onTap(spare) }, description = spare, enabled = !isHeld, disabledAlpha = 0.3f, modifier = modifier) {
        Text(spare, style = MaterialTheme.typography.bodyLarge, maxLines = 1)
    }
}

@Composable
private fun TrayChip(
    onClick: () -> Unit,
    description: String,
    enabled: Boolean = true,
    disabledAlpha: Float = 0.4f,
    modifier: Modifier = TRAY_CHIP_WIDTH,
    content: @Composable () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = MaterialTheme.colorScheme.surface,
        modifier =
            modifier
                .height(44.dp)
                .alpha(if (enabled) 1f else disabledAlpha)
                .clickable(enabled = enabled, onClick = onClick)
                .semantics { contentDescription = description },
    ) {
        Box(Modifier.padding(horizontal = 10.dp), contentAlignment = Alignment.Center) { content() }
    }
}

private val TRAY_CHIP_WIDTH = Modifier.widthIn(min = 48.dp)
