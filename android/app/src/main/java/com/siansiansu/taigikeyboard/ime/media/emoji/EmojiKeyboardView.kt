package com.siansiansu.taigikeyboard.ime.media.emoji

import android.content.Context
import android.util.AttributeSet
import android.widget.FrameLayout
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearanceCache
import com.siansiansu.taigikeyboard.ime.core.isKeyboardNightMode
import com.siansiansu.taigikeyboard.ime.theme.KeyboardMaterialTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class EmojiKeyboardView : FrameLayout {
    // A7: `EmojiKeyboardView` is instantiated by `MediaInputManager` with the
    // IME service context (`taigikeyboard.context`), so casting is safe.
    private val taigikeyboard: TaigiKeyboard
        get() = context as TaigiKeyboard
    private val mainScope = MainScope()
    private var composeView: ComposeView? = null

    private lateinit var preferencesManager: EmojiPreferences

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(
        context,
        attrs,
        defStyleAttr,
    ) {
        // Initialization happens in onAttachedToWindow().
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()

        preferencesManager = EmojiPreferences(context)

        composeView =
            ComposeView(context).apply {
                setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
            }

        addView(
            composeView,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT,
            ),
        )

        mainScope.launch {
            val layouts =
                mainScope
                    .async(Dispatchers.IO) {
                        loadEmojiLayoutData(context)
                    }.await()

            composeView?.setContent {
                KeyboardMaterialTheme {
                    ThemedEmojiColors(themeColors()) {
                        val preferredSkinTone by preferencesManager.getPreferredSkinTone().collectAsState(
                            initial = com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone.DEFAULT,
                        )

                        EmojiPaletteView(
                            fullEmojiMappings = layouts,
                            preferredSkinTone = preferredSkinTone,
                            onEmojiClick = { emojiKeyData ->
                                taigikeyboard.mediaInputManager.sendEmojiKeyPress(emojiKeyData)
                            },
                            onSkinToneSelected = { skinTone ->
                                mainScope.launch {
                                    preferencesManager.setPreferredSkinTone(skinTone)
                                }
                            },
                            modifier = Modifier,
                        )
                    }
                }
            }
        }
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()

        mainScope.cancel()
        composeView = null
    }

    /** The active theme's colors, resolved on attach (the panel is recomposed per attach). */
    private fun themeColors(): KeyboardColorSettings = ThemeAppearanceCache(taigikeyboard.prefs).resolve(isKeyboardNightMode(context)).colors
}

/**
 * Applies the keyboard theme to the emoji palette: a themed keyboard (custom surface, painted
 * behind the media panel by [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardThemeSurfaceController])
 * gets a transparent palette container, key-text glyph / tab / indicator colors and the key fill
 * for the variants popup. The adaptive default keeps the Material3 scheme unchanged.
 */
@Composable
private fun ThemedEmojiColors(
    colors: KeyboardColorSettings,
    content: @Composable () -> Unit,
) {
    if (colors.surface == null) return content()
    val base = MaterialTheme.colorScheme
    val foreground = colors.keyTextColor?.let { Color(it) }
    MaterialTheme(
        colorScheme =
            base.copy(
                surface = Color.Transparent,
                onSurface = foreground ?: base.onSurface,
                onSurfaceVariant = foreground ?: base.onSurfaceVariant,
                primary = foreground ?: base.primary,
                surfaceVariant = colors.fixedKeyFill?.let { Color(it) } ?: base.surfaceVariant,
            ),
        typography = MaterialTheme.typography,
        content = content,
    )
}
