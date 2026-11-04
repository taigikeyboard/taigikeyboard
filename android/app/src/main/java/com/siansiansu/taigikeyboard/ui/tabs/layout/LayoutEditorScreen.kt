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
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
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
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutUse
import com.siansiansu.taigikeyboard.ime.text.layout.accessibilityName
import com.siansiansu.taigikeyboard.ime.text.layout.isFullWidthSymbol
import com.siansiansu.taigikeyboard.ui.components.GALLERY_PREVIEW_ASPECT
import com.siansiansu.taigikeyboard.ui.components.NameDialog
import com.siansiansu.taigikeyboard.ui.theme.AppStyle
import java.util.Collections

// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling), all by taps.
// Tap a key to select it, then a spare to replace it or the trash to remove it. Tap a second key to
// mark both, then the swap button below the board to swap them (function keys within their row). The selected character key's
// long-press list sits in LayoutCalloutStrip below the board: tap two slots and the swap button to
// swap or move, a slot then a tray character to fill it, a slot then the trash to empty it. The undo
// button steps back one edit at a time. Nothing is stored until Save,
// whose dialog names the layout and picks where it shows (LayoutUse), then applies it; Back discards
// the draft. Mirrors iOS LayoutEditorView.

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
    // The key tapped before [selected], marked with it for the swap button; null unless the two can swap.
    var swapPartner by remember { mutableStateOf<LayoutPlace?>(null) }
    // The slot tapped before [selectedCallout], marked with it for the swap button.
    var calloutPartner by remember { mutableStateOf<Int?>(null) }
    // The draft before each edit, oldest first.
    val history = remember { mutableStateListOf<CustomLayout>() }
    var selectedKeyCenterX by remember { mutableStateOf<Float?>(null) }
    var showNameDialog by rememberSaveable { mutableStateOf(false) }
    // Where the layout shows, picked in the name dialog; starts from the draft's each time it opens.
    var pendingUse by rememberSaveable { mutableStateOf(LayoutUse.UNIVERSAL) }
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
            history.add(draft)
            draft = next
            swapPartner = null
            calloutPartner = null
            view.performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
        }
    }

    fun select(place: LayoutPlace?) {
        selected = place
        selectedCallout = null
        swapPartner = null
        calloutPartner = null
        // The new key reports its centre once laid out; until then the bubble shows no tail.
        selectedKeyCenterX = null
    }

    fun edit(next: CustomLayout) {
        changeDraft(next)
        select(null)
    }

    // A tap never edits, so looking at another key's long-press list changes nothing. A second key that
    // can swap with the selected one (CustomLayout.canSwap: character keys anywhere, function keys within
    // their row) marks the pair for the swap button; a tap after a pair starts over.
    fun tap(place: LayoutPlace) {
        val current = selected
        val isPairing = swapPartner == null && current != null && draft.canSwap(current, place, table)
        select(if (swapPartner == null && current == place) null else place)
        if (isPairing) swapPartner = current
    }

    fun undo() {
        draft = history.removeLastOrNull() ?: return
        select(null)
    }

    // Applies [change] to the selected key's list, then selects slot [nextSlot] (clamped to the first
    // empty slot, or the last one once the list is full), or no slot when null. Filling and removing
    // keep a slot selected: with none, the next tray tap would replace the key itself and drop its list.
    fun editCallouts(
        nextSlot: Int?,
        change: (MutableList<String>) -> Unit,
    ) {
        val label = selectedLabel() ?: return
        changeDraft(tables.withCallouts(draft, label, selectedCallouts().toMutableList().also(change)))
        // Read from the edited draft: a dropped repeat can leave the list unchanged or shorter.
        selectedCallout = nextSlot?.let { minOf(it, selectedCallouts().size, CustomLayout.MAX_CALLOUTS - 1) }
    }

    // A second slot marks the pair for the swap button when either holds an entry, and a tap after a
    // pair starts over. Tapping the selected slot clears it.
    fun tapCallout(index: Int) {
        swapPartner = null
        val current = selectedCallout
        val filled = selectedCallouts().indices
        val isPairing = calloutPartner == null && current != null && current != index && (current in filled || index in filled)
        selectedCallout = if (calloutPartner == null && current == index) null else index
        calloutPartner = if (isPairing) current else null
    }

    // Two entries swap; an entry paired with an empty slot moves to the list's end (entries stay packed
    // at the front).
    fun swapCallouts(
        first: Int,
        second: Int,
    ) {
        // Cleared here too: an entry already last leaves the draft unchanged.
        calloutPartner = null
        val filled = selectedCallouts().indices
        editCallouts(null) {
            if (first in filled && second in filled) {
                Collections.swap(it, first, second)
            } else {
                it.add(it.removeAt(if (first in filled) first else second))
            }
        }
    }

    // Swaps the marked long-press slots, else the marked keys.
    fun swapMarked() {
        val firstSlot = calloutPartner
        val secondSlot = selectedCallout
        if (firstSlot != null && secondSlot != null) {
            swapCallouts(firstSlot, secondSlot)
            return
        }
        val first = swapPartner ?: return
        val second = selected ?: return
        edit(draft.swapped(first, second, table))
    }

    // Puts [spare] in slot [index]: replaces an entry (which stays selected), or adds one in an empty
    // slot and moves the selection to the next empty slot, so several characters go in one tap each.
    fun fillCallout(
        index: Int,
        spare: String,
    ) {
        val isAdding = index !in selectedCallouts().indices
        editCallouts(if (isAdding) CustomLayout.MAX_CALLOUTS else index) { if (isAdding) it.add(spare) else it[index] = spare }
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
            if (slot in selectedCallouts().indices) editCallouts(slot) { it.removeAt(slot) }
            return
        }
        selected?.let { edit(draft.removed(it)) }
    }

    fun restart(base: String) {
        tables.fresh(draft.id, draft.name, base, draft.script)?.let { changeDraft(it) }
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
                    TextButton(onClick = {
                        pendingUse = draft.use
                        showNameDialog = true
                    }) { Text(L10n.commonSave) }
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
            // At most half the height, so a short (landscape) screen still shows a usable board.
            val boardWidth = min(maxWidth - 40.dp, maxHeight * 0.5f * GALLERY_PREVIEW_ASPECT)
            // Below Material 3's compact-height breakpoint (480dp, a phone in landscape) the board and
            // bubble leave the tray no height, so the whole editor scrolls instead of the tray alone.
            val scrollsWhole = maxHeight < 480.dp
            val scrollState = rememberScrollState()
            val label = selectedLabel()
            val callouts = selectedCallouts()
            Column(
                Modifier
                    .fillMaxSize()
                    .then(if (scrollsWhole) Modifier.verticalScroll(scrollState) else Modifier),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                BaseChips(selectedBase = draft.base, onSelect = ::restart)

                LayoutKeyGrid(
                    rows = remember(draft) { tables.editorRows(draft) },
                    base = draft.base,
                    isDark = isDark,
                    interaction =
                        LayoutKeyGridInteraction(
                            selected = selected,
                            // Only what the next action works on is lit: the bubble's tail still names the key.
                            isSelectionMuted = selectedCallout != null,
                            swapPartner = swapPartner,
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
                    partnerSlot = calloutPartner,
                    tailX = selectedKeyCenterX.takeIf { selected != null },
                    onTapSlot = ::tapCallout,
                    modifier = Modifier.padding(horizontal = 20.dp).fillMaxWidth(),
                )

                // Fixed between the bubble and the tray, in thumb reach and always in view.
                Row(
                    Modifier
                        .align(Alignment.CenterHorizontally)
                        .width(boardWidth),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Button(
                        onClick = ::swapMarked,
                        enabled = swapPartner != null || calloutPartner != null,
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(painterResource(R.drawable.ic_swap), contentDescription = L10n.themePhotoHintMove)
                    }
                    FilledTonalButton(
                        onClick = ::undo,
                        enabled = history.isNotEmpty(),
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(painterResource(R.drawable.ic_undo), contentDescription = L10n.desktopMenuUndo)
                    }
                    FilledTonalButton(
                        onClick = ::removeSelected,
                        enabled = selectedCallout?.let { it in callouts.indices } ?: (selected?.let(draft::canRemove) == true),
                        colors = ButtonDefaults.filledTonalButtonColors(contentColor = MaterialTheme.colorScheme.error),
                        modifier = Modifier.weight(1f),
                    ) {
                        Icon(Icons.Outlined.Delete, contentDescription = L10n.commonDelete)
                    }
                }

                Column(
                    (if (scrollsWhole) Modifier else Modifier.weight(1f))
                        .fillMaxWidth()
                        .padding(horizontal = 10.dp)
                        .then(if (scrollsWhole) Modifier else Modifier.verticalScroll(scrollState))
                        .padding(horizontal = 10.dp, vertical = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    // A character the selected key's list already holds stays in place, dimmed, so the
                    // rows keep their columns.
                    val held = remember(callouts) { tables.trayForms(callouts) }
                    val letters = remember(draft) { draft.spares(tables.trayLetters) }
                    val (halfWidth, fullWidth) = remember(draft) { tables.traySymbols(draft).partition { !isFullWidthSymbol(it) } }
                    if (!draft.hasEmojiKey || letters.isNotEmpty()) {
                        TraySection(L10n.layoutTrayLetters) {
                            SpareFlow(letters, onTap = ::replaceSelected, held = held) {
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
                        }
                    }
                    TraySection(L10n.layoutTrayTones) { ToneGrid(tables.toneRows, held = held, onTap = ::replaceSelected) }
                    TraySection(L10n.symbolHalfWidth) { SpareFlow(halfWidth, onTap = ::replaceSelected, held = held) }
                    TraySection(L10n.symbolFullWidth) { SpareFlow(fullWidth, onTap = ::replaceSelected, held = held) }
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
                    // A copy, so a failed save leaves the draft as it was.
                    onSave(tables.withUse(draft.copy(name = name), pendingUse))
                },
                onDismiss = { showNameDialog = false },
            ) {
                UseMenu(selected = pendingUse, onSelect = { pendingUse = it })
            }
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

/** Where the layout shows (Universal / Tâi-lô / Pe̍h-ōe-jī): a dropdown under the name field. Mirrors the iOS save sheet's menu picker. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun UseMenu(
    selected: LayoutUse,
    onSelect: (LayoutUse) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
        OutlinedTextField(
            value = stringRes(selected.labelKey),
            onValueChange = {},
            readOnly = true,
            singleLine = true,
            label = { Text(L10n.settingsInputMode) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            LayoutUse.entries.forEach { use ->
                DropdownMenuItem(
                    text = { Text(stringRes(use.labelKey)) },
                    onClick = {
                        onSelect(use)
                        expanded = false
                    },
                )
            }
        }
    }
}

@Composable
private fun TraySection(
    title: String,
    content: @Composable () -> Unit,
) {
    Column {
        Text(
            title,
            modifier = Modifier.padding(bottom = AppStyle.sectionHeaderBottomPadding),
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.titleMedium,
        )
        content()
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
    held: Set<String>,
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
