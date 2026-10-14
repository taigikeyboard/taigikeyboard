package com.siansiansu.taigikeyboard.ui.tabs.layout

import android.view.HapticFeedbackConstants
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
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
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
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
import androidx.compose.runtime.State
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
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import androidx.compose.ui.unit.sp
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
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutKeyFace
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutKeyGrid
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutKeyGridInteraction
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutPlace
import com.siansiansu.taigikeyboard.ime.text.layout.WindowAnchor
import com.siansiansu.taigikeyboard.ime.text.layout.accessibilityName
import com.siansiansu.taigikeyboard.ime.text.layout.layoutKeyColor
import com.siansiansu.taigikeyboard.ime.text.layout.layoutKeyTextColor
import com.siansiansu.taigikeyboard.ui.components.GALLERY_PREVIEW_ASPECT
import com.siansiansu.taigikeyboard.ui.components.NameDialog
import kotlin.math.roundToInt

// Custom-layout editor: base chips, the board (fixed) and the spare tray (scrolling). Drag a key onto
// another to swap them (function keys within their row), a spare onto a key to replace it (press and
// hold the spare first, so the tray still scrolls), a key onto the tray to remove it. Taps do the
// same: tap two keys, or a key then a spare or the trash. Nothing is stored until Save, which names
// the layout and applies it; Back discards the draft. Mirrors iOS LayoutEditorView.

/** What a drag carries: a board key, a spare from the tray, or the tray's emoji chip. */
private sealed interface DragSource {
    data class Board(
        val place: LayoutPlace,
    ) : DragSource

    data class Spare(
        val label: String,
    ) : DragSource

    data object Emoji : DragSource
}

/**
 * A drag and what releasing it now would do. The finger's position is kept apart (read only where the
 * lifted key is placed), so moving the finger redraws the screen only when the target changes.
 */
private data class EditorDrag(
    val source: DragSource,
    val target: LayoutPlace? = null,
    val isOverTray: Boolean = false,
)

/** The editor's measured geometry, read by drag handlers only (kept out of composition state). */
private class EditorGeometry {
    val keyBounds = HashMap<LayoutPlace, Rect>()
    var trayBounds = Rect.Zero
    var root: LayoutCoordinates? = null
}

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
    var drag by remember { mutableStateOf<EditorDrag?>(null) }
    // The finger under a drag, in window coordinates.
    val pointer = remember { mutableStateOf(Offset.Zero) }
    var showNameDialog by rememberSaveable { mutableStateOf(false) }
    val geometry = remember { EditorGeometry() }
    val table = checkNotNull(tables.baseTable(draft.base, draft.script)) { "no ${draft.base} table" }
    val isDark = isSystemInDarkTheme()
    val view = LocalView.current
    // Resolver captured for the name-dialog callback (runs outside composition), so the blank-name
    // fallback resolves under the live display language at save time.
    val stringResolver by rememberUpdatedState(LocalStringResolver.current)

    // A tick when a key lifts and when it reaches a new target; a click when a drop changed the board.
    fun tick() = view.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)

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
        drag = null
    }

    fun dragged(
        source: DragSource,
        point: Offset,
    ): EditorDrag {
        val place = geometry.keyBounds.entries
            .firstOrNull { it.value.contains(point) }
            ?.key
        return when (source) {
            is DragSource.Board ->
                if (place != null && draft.canSwap(source.place, place, table)) {
                    EditorDrag(source, target = place)
                } else {
                    EditorDrag(source, isOverTray = geometry.trayBounds.contains(point) && draft.canRemove(source.place))
                }
            is DragSource.Spare -> EditorDrag(source, target = place as? LayoutPlace.Key)
            DragSource.Emoji -> EditorDrag(source, target = place?.takeIf { it == draft.emojiPlace })
        }
    }

    // One drag at a time: a second finger's drag is ignored, and only the drag's own gesture moves,
    // drops or cancels it.
    fun startDrag(
        source: DragSource,
        point: Offset,
    ) {
        if (drag != null) return
        selected = null
        pointer.value = point
        drag = dragged(source, point)
        tick()
    }

    fun moveDrag(
        source: DragSource,
        point: Offset,
    ) {
        val current = drag?.takeIf { it.source == source } ?: return
        pointer.value = point
        val next = dragged(source, point)
        if (next != current) {
            if (next.target != null) tick()
            drag = next
        }
    }

    fun cancelDrag(source: DragSource) {
        if (drag?.source == source) drag = null
    }

    // Ends a drag where the finger lifted: swap, remove, replace, or nothing (back on its own place).
    fun endDrag(
        source: DragSource,
        point: Offset,
    ) {
        val current = drag?.takeIf { it.source == source }?.let { dragged(source, point) } ?: return
        drag = null
        val target = current.target
        when (val source = current.source) {
            is DragSource.Board ->
                when {
                    target != null -> edit(draft.swapped(source.place, target, table))
                    current.isOverTray -> edit(draft.removed(source.place))
                }
            is DragSource.Spare -> if (target != null) edit(draft.replaced(target, source.label))
            DragSource.Emoji -> if (target != null) edit(draft.restoreEmojiKey())
        }
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
                .padding(innerPadding)
                .onGloballyPositioned { geometry.root = it },
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
                            lifted = (drag?.source as? DragSource.Board)?.place,
                            highlighted = drag?.target,
                            removedPlaceLabel = L10n.settingsSyllableSeparatorNone,
                            onTap = { place -> if (drag == null) tap(place) },
                            onDragStart = { place, point -> startDrag(DragSource.Board(place), point) },
                            onDragMove = { place, point -> moveDrag(DragSource.Board(place), point) },
                            onDragEnd = { place, point -> endDrag(DragSource.Board(place), point) },
                            onDragCancel = { place -> cancelDrag(DragSource.Board(place)) },
                            onKeyBounds = { place, bounds ->
                                if (bounds == null) geometry.keyBounds.remove(place) else geometry.keyBounds[place] = bounds
                            },
                        ),
                    modifier =
                        Modifier
                            .align(Alignment.CenterHorizontally)
                            .width(boardWidth)
                            .aspectRatio(GALLERY_PREVIEW_ASPECT)
                            .clip(RoundedCornerShape(10.dp)),
                )

                val isOverTray = drag?.isOverTray == true
                Column(
                    Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        // The visible tray, not its scrolled content: a key dropped inside it is removed.
                        .onGloballyPositioned { geometry.trayBounds = it.boundsInWindow() }
                        .padding(horizontal = 10.dp)
                        .border(BorderStroke(2.dp, MaterialTheme.colorScheme.error.copy(alpha = if (isOverTray) 1f else 0f)), RoundedCornerShape(10.dp))
                        .verticalScroll(rememberScrollState())
                        .padding(horizontal = 10.dp, vertical = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    val canRemove = selected?.let(draft::canRemove) == true
                    // Remembered per base table (which the drag handlers read), so the chips skip redraws.
                    val spareDrag =
                        remember(table) {
                            SpareDragCallbacks(
                                onStart = ::startDrag,
                                onMove = ::moveDrag,
                                onEnd = { source -> endDrag(source, pointer.value) },
                                onCancel = ::cancelDrag,
                            )
                        }
                    SpareFlow(remember(draft) { draft.spares(tables.trayLetters) }, spareDrag, onTap = ::replaceSelected) {
                        TrayChip(onClick = ::removeSelected, enabled = canRemove, description = L10n.commonDelete) {
                            Icon(Icons.Outlined.Delete, contentDescription = null, tint = MaterialTheme.colorScheme.error)
                        }
                        if (!draft.hasEmojiKey) {
                            // Tap restores the emoji key at once; dragged, it only lands on its own place.
                            TrayChip(
                                onClick = { edit(draft.restoreEmojiKey()) },
                                description = accessibilityName(GridKey.Face.Function(FunctionKey.EMOJI)),
                                modifier = Modifier.spareDrag(DragSource.Emoji, spareDrag),
                            ) {
                                Icon(painterResource(R.drawable.ic_sentiment_satisfied), contentDescription = null)
                            }
                        }
                    }
                    SpareFlow(remember(draft) { draft.spares(tables.traySymbols) }, spareDrag, onTap = ::replaceSelected)
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

            drag?.let { current -> LiftedKey(current, draft, geometry, pointer, isDark) }
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
    spareDrag: SpareDragCallbacks,
    onTap: (String) -> Unit,
    leading: @Composable () -> Unit = {},
) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        leading()
        spares.forEach { spare ->
            TrayChip(onClick = { onTap(spare) }, description = spare, modifier = Modifier.spareDrag(DragSource.Spare(spare), spareDrag)) {
                Text(spare, style = MaterialTheme.typography.bodyLarge)
            }
        }
    }
}

@Composable
private fun TrayChip(
    onClick: () -> Unit,
    description: String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    content: @Composable () -> Unit,
) {
    Surface(
        shape = RoundedCornerShape(8.dp),
        color = MaterialTheme.colorScheme.surface,
        modifier =
            modifier
                .widthIn(min = 48.dp)
                .height(44.dp)
                .alpha(if (enabled) 1f else 0.4f)
                .clickable(enabled = enabled, onClick = onClick)
                .semantics { contentDescription = description },
    ) {
        Box(Modifier.padding(horizontal = 10.dp), contentAlignment = Alignment.Center) { content() }
    }
}

/** The drag callbacks a tray chip reports to. */
private class SpareDragCallbacks(
    val onStart: (DragSource, Offset) -> Unit,
    val onMove: (DragSource, Offset) -> Unit,
    /** The finger lifted; the drop lands where the last move left the drag. */
    val onEnd: (DragSource) -> Unit,
    val onCancel: (DragSource) -> Unit,
)

/**
 * Press and hold a tray chip to lift it (so a swipe still scrolls the tray); the chip stays in the
 * tray until the drop changes the draft. A tap goes to the chip's click. Keyed on the source only:
 * the latest callbacks are read through [rememberUpdatedState], so the redraws a drag causes never
 * restart it.
 */
@Composable
private fun Modifier.spareDrag(
    source: DragSource,
    callbacks: SpareDragCallbacks,
): Modifier {
    val current by rememberUpdatedState(callbacks)
    val anchor = remember { WindowAnchor() }
    return onGloballyPositioned { anchor.coordinates = it }
        .pointerInput(source) {
            detectDragGesturesAfterLongPress(
                onDragStart = { current.onStart(source, anchor.toWindow(it)) },
                onDrag = { change, _ ->
                    change.consume()
                    current.onMove(source, anchor.toWindow(change.position))
                },
                onDragEnd = { current.onEnd(source) },
                onDragCancel = { current.onCancel(source) },
            )
        }
}

/** The dragged key, lifted above the finger. */
@Composable
private fun LiftedKey(
    drag: EditorDrag,
    draft: CustomLayout,
    geometry: EditorGeometry,
    pointer: State<Offset>,
    isDark: Boolean,
) {
    val face =
        when (val source = drag.source) {
            is DragSource.Board -> draft.face(source.place)
            is DragSource.Spare -> GridKey.Face.Character(source.label)
            DragSource.Emoji -> GridKey.Face.Function(FunctionKey.EMOJI)
        }
    LayoutKeyFace(
        face = face,
        color = layoutKeyTextColor(isDark),
        fontSize = 26.sp,
        modifier =
            Modifier
                // Read at placement, so following the finger never recomposes.
                .offset {
                    val local = geometry.root?.windowToLocal(pointer.value) ?: Offset.Zero
                    IntOffset((local.x - 28.dp.toPx()).roundToInt(), (local.y - 66.dp.toPx()).roundToInt())
                }.size(width = 56.dp, height = 60.dp)
                .shadow(8.dp, RoundedCornerShape(10.dp))
                .background(layoutKeyColor(isDark), RoundedCornerShape(10.dp)),
    )
}
