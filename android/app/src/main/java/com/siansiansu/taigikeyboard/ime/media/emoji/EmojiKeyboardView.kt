package com.siansiansu.taigikeyboard.ime.media.emoji

import android.content.Context
import android.util.AttributeSet
import android.widget.FrameLayout
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
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
                TaigiKeyboardTheme {
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

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()

        mainScope.cancel()
        composeView = null
    }
}
