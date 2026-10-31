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

// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling), all by taps.
// Tap two keys to swap them (function keys within their row), a key then a spare to replace it, a key
// then the trash to remove it. Nothing is stored until Save, which names the layout and applies it;
// Back discards the draft. Mirrors iOS LayoutEditorView.

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
    var showNameDialog by rememberSaveable { mutableStateOf(false) }
    val table = checkNotNull(tables.baseTable(draft.base, draft.script)) { "no ${draft.base} table" }
    val isDark = isSystemInDarkTheme()
    val view = LocalView.current
    // Resolver captured for the name-dialog callback (runs outside composition), so the blank-name
    // fallback resolves under the live display language at save time.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)

    fun edit(next: CustomLayout) {
        if (next != draft) {
            draft = next
            view.performHapticFeedback(HapticFeedbackConstants.CONTEXT_CLICK)
        }
        selected = null
    }

    // A second key swaps with the selected one where it can (CustomLayout.canSwap: character keys
    // anywhere, function keys within their row); otherwise the tap moves or clears the selection.
    fun tap(place: LayoutPlace) {
        val current = selected
        if (current != null && draft.canSwap(current, place, table)) {
            edit(draft.swapped(current, place, table))
        } else {
            selected = if (current == place) null else place
        }
    }

    fun replaceSelected(spare: String) {
        selected?.let { edit(draft.replaced(it, spare)) }
    }

    fun removeSelected() {
        selected?.let { edit(draft.removed(it)) }
    }

    fun restart(base: String) {
        tables.fresh(draft.id, draft.name, base, draft.script)?.let { draft = it }
        selected = null
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
                        ),
                    modifier =
                        Modifier
                            .align(Alignment.CenterHorizontally)
                            .width(boardWidth)
                            .aspectRatio(GALLERY_PREVIEW_ASPECT)
                            .clip(RoundedCornerShape(10.dp)),
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
                    val canRemove = selected?.let(draft::canRemove) == true
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
                    SpareFlow(remember(draft) { draft.spares(tables.traySymbols) }, onTap = ::replaceSelected)
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

/** A run of tray chips, [leading] first, then one chip per spare (tap = put it on the selected key). */
@Composable
private fun SpareFlow(
    spares: List<String>,
    onTap: (String) -> Unit,
    leading: @Composable () -> Unit = {},
) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        leading()
        spares.forEach { spare ->
            TrayChip(onClick = { onTap(spare) }, description = spare) {
                Text(spare, style = MaterialTheme.typography.bodyLarge)
            }
        }
    }
}

@Composable
private fun TrayChip(
    onClick: () -> Unit,
    description: String,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = MaterialTheme.colorScheme.surface,
        modifier =
            Modifier
                .widthIn(min = 48.dp)
                .height(44.dp)
                .alpha(if (enabled) 1f else 0.4f)
                .clickable(enabled = enabled, onClick = onClick)
                .semantics { contentDescription = description },
    ) {
        Box(Modifier.padding(horizontal = 10.dp), contentAlignment = Alignment.Center) { content() }
    }
}
