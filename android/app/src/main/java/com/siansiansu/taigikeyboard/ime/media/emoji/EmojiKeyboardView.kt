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

/**
 * Emoji 鍵盤 View（整合 Compose）
 */
class EmojiKeyboardView : FrameLayout {
    // A7: `EmojiKeyboardView` is instantiated by `MediaInputManager` with the
    // IME service context (`taigikeyboard.context`), so casting is safe.
    private val taigikeyboard: TaigiKeyboard
        get() = context as TaigiKeyboard
    private val mainScope = MainScope()
    private var composeView: ComposeView? = null

    // 膚色偏好管理器
    private lateinit var preferencesManager: EmojiPreferences

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(
        context,
        attrs,
        defStyleAttr,
    ) {
        // 初始化將在 onAttachedToWindow 執行
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()

        // 初始化管理器
        preferencesManager = EmojiPreferences(context)

        // 建立 ComposeView（會自動從 view tree 找到 LifecycleOwner）
        composeView =
            ComposeView(context).apply {
                // 設定 composition strategy
                setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
            }

        addView(
            composeView,
            LayoutParams(
                LayoutParams.MATCH_PARENT,
                LayoutParams.MATCH_PARENT,
            ),
        )

        // 異步載入 emoji 資料並設定 content
        mainScope.launch {
            val layouts =
                mainScope
                    .async(Dispatchers.IO) {
                        loadEmojiLayoutData(context)
                    }.await()

            // 資料載入完成後設定 Compose content
            composeView?.setContent {
                TaigiKeyboardTheme {
                    // 觀察膚色偏好
                    val preferredSkinTone by preferencesManager.getPreferredSkinTone().collectAsState(
                        initial = com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone.DEFAULT,
                    )

                    EmojiPaletteView(
                        fullEmojiMappings = layouts,
                        preferredSkinTone = preferredSkinTone,
                        onEmojiClick = { emojiKeyData ->
                            // 點擊 emoji 時送出文字
                            taigikeyboard.mediaInputManager.sendEmojiKeyPress(emojiKeyData)
                        },
                        onSkinToneSelected = { skinTone ->
                            // 記錄使用者選擇的膚色偏好
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
