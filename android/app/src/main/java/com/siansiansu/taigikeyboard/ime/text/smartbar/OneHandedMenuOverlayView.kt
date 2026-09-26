package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.util.AttributeSet
import android.widget.FrameLayout
import androidx.annotation.DrawableRes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.compose.ui.res.dimensionResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.ProvideDisplayLanguage
import com.siansiansu.taigikeyboard.i18n.generated.L10n
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.settings.KeyboardToolbarAction
import com.siansiansu.taigikeyboard.ime.core.settings.OneHandedMode
import com.siansiansu.taigikeyboard.ime.theme.KeyboardMaterialTheme

/**
 * Long-press callout of the toolbar keyboard button, drawn over the whole input view
 * (transparent; a tap outside the callout closes it). Same FrameLayout + ComposeView
 * shell as [SettingsSelectionOverlayView]; [ToolbarManager] shows / hides it.
 */
class OneHandedMenuOverlayView : FrameLayout {
    private var composeView: ComposeView? = null

    // Refreshed on each show(): the highlight follows the live mode, the colours a live theme change.
    private val currentMode = mutableStateOf(OneHandedMode.OFF)
    private val refreshTrigger = mutableIntStateOf(0)

    var onSelectMode: ((OneHandedMode) -> Unit)? = null
    var onSelectDismiss: (() -> Unit)? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    init {
        visibility = GONE
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        composeView =
            ComposeView(context).apply {
                setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
            }
        addView(composeView, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))

        // IME-only overlay: `context` is the `TaigiKeyboard` service (same as the settings overlay).
        val prefs = (context as TaigiKeyboard).prefs
        composeView?.setContent {
            KeyboardMaterialTheme {
                val displayLanguageFlow = remember(prefs) { prefs.observeDisplayLanguage() }
                val displayLanguageTag by displayLanguageFlow.collectAsState(initial = prefs.displayLanguageTag)
                ProvideDisplayLanguage(DisplayLanguage.fromTag(displayLanguageTag)) {
                    val mode by currentMode
                    val trigger by refreshTrigger
                    OneHandedMenuContent(
                        currentMode = mode,
                        appearance = rememberKeyboardOverlayAppearance(prefs, trigger),
                        onSelectMode = { onSelectMode?.invoke(it) },
                        onSelectDismiss = { onSelectDismiss?.invoke() },
                        onClose = { hide() },
                    )
                }
            }
        }
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        composeView = null
    }

    fun show(mode: OneHandedMode) {
        currentMode.value = mode
        refreshTrigger.intValue++
        visibility = VISIBLE
    }

    fun hide() {
        visibility = GONE
    }
}

/**
 * Dismiss / Left / Normal / Right. The current one-handed mode is highlighted; Dismiss never is
 * (it is an action, not a mode). Mirrors iOS `OneHandedModeCallout`.
 */
@Composable
private fun OneHandedMenuContent(
    currentMode: OneHandedMode,
    appearance: KeyboardOverlayAppearance,
    onSelectMode: (OneHandedMode) -> Unit,
    onSelectDismiss: () -> Unit,
    onClose: () -> Unit,
) {
    val shape = RoundedCornerShape(12.dp)
    Box(
        modifier = Modifier
            .fillMaxSize()
            .pointerInput(Unit) { detectTapGestures { onClose() } },
    ) {
        // Drops down over the keys, under the toolbar row, near the keyboard button.
        Row(
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(top = dimensionResource(R.dimen.smartbar_height), end = 8.dp)
                .shadow(6.dp, shape)
                .background(appearance.solidBackground, shape)
                .padding(6.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            MenuCell(KeyboardToolbarAction.DISMISS.iconRes, L10n.keyboardDismissKeyboard, appearance, isSelected = false, onSelectDismiss)
            ModeCell(OneHandedMode.LEFT, KeyboardToolbarAction.LEFT.iconRes, L10n.keyboardOneHandedLeft, currentMode, appearance, onSelectMode)
            ModeCell(OneHandedMode.OFF, R.drawable.ic_keyboard, L10n.keyboardOneHandedOff, currentMode, appearance, onSelectMode)
            ModeCell(OneHandedMode.RIGHT, KeyboardToolbarAction.RIGHT.iconRes, L10n.keyboardOneHandedRight, currentMode, appearance, onSelectMode)
        }
    }
}

@Composable
private fun ModeCell(
    mode: OneHandedMode,
    @DrawableRes iconRes: Int,
    label: String,
    currentMode: OneHandedMode,
    appearance: KeyboardOverlayAppearance,
    onSelectMode: (OneHandedMode) -> Unit,
) = MenuCell(iconRes, label, appearance, isSelected = currentMode == mode) { onSelectMode(mode) }

@Composable
private fun MenuCell(
    @DrawableRes iconRes: Int,
    label: String,
    appearance: KeyboardOverlayAppearance,
    isSelected: Boolean,
    onClick: () -> Unit,
) {
    // Same selected-chip look as the toolbar mode buttons: accent fill, white content.
    val contentColor = if (isSelected) Color.White else appearance.foreground
    Column(
        modifier = Modifier
            .defaultMinSize(minWidth = 56.dp, minHeight = 56.dp)
            .background(if (isSelected) appearance.accent else Color.Transparent, RoundedCornerShape(8.dp))
            .clickable(onClick = onClick)
            .semantics { selected = isSelected }
            .padding(horizontal = 4.dp, vertical = 6.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp, Alignment.CenterVertically),
    ) {
        Icon(painter = painterResource(iconRes), contentDescription = null, tint = contentColor, modifier = Modifier.size(24.dp))
        Text(text = label, color = contentColor, style = MaterialTheme.typography.labelSmall, maxLines = 1)
    }
}
