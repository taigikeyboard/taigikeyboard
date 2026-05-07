package com.siansiansu.taigikeyboard.ime.popup

import android.content.res.Configuration
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.PopupWindow
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.lifecycle.setViewTreeViewModelStoreOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.media.emoji.EmojiKeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyView
import com.siansiansu.taigikeyboard.ime.text.keyboard.AnchorSide
import com.siansiansu.taigikeyboard.ime.text.keyboard.ExtendedPopupGeometryInput
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.ime.text.keyboard.PopupDimensionsInput
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

class KeyPopupManager<T_KBD : View, T_KV : View>(private val keyboardView: T_KBD) {
    private var anchorSide: AnchorSide = AnchorSide.LEFT
    private var anchorOffset: Int = 0
    private var activeExtIndex: Int? = null
    private val exceptionsForKeyCodes = listOf(
        KeyCode.ENTER,
        KeyCode.LANGUAGE_SWITCH,
        KeyCode.SWITCH_TO_TEXT_CONTEXT,
        KeyCode.SWITCH_TO_MEDIA_CONTEXT,
    )
    private var keyPopupWidth: Int
    private var keyPopupHeight: Int
    private var keyPopupDiffX: Int = 0
    private var row0count: Int = 0
    private var row1count: Int = 0

    private val composeView: ComposeView
    private val composeViewExt: ComposeView
    private val window: PopupWindow
    private val windowExt: PopupWindow

    private val _previewState = MutableStateFlow<PreviewState>(PreviewState.Hidden)
    private val _extendedState = MutableStateFlow<ExtendedState>(ExtendedState.Hidden)
    val previewState = _previewState.asStateFlow()
    val extendedState = _extendedState.asStateFlow()

    private var ownersInstalled = false

    /** Resolved once per show() and reused by the immediately-following extend()
     *  during a long-press; cleared on hide() so a fresh touch-down picks up
     *  any theme/font change between popups. */
    private var cachedDisplay: PopupDisplayParams? = null

    /** True while the preview popup composable is rendering a Visible state. */
    val isShowingPopup: Boolean
        get() = _previewState.value is PreviewState.Visible

    /** True while the extended popup window is on screen. Tracks the window
     *  rather than [_extendedState] because dismiss() detaches the ComposeView
     *  before the state mutation can be observed. */
    val isShowingExtendedPopup: Boolean
        get() = windowExt.isShowing

    init {
        keyPopupWidth = keyboardView.resources.getDimension(R.dimen.key_width).toInt()
        keyPopupHeight = keyboardView.resources.getDimension(R.dimen.key_height).toInt()

        composeView = popupComposeView(_previewState) { state ->
            if (state is PreviewState.Visible) KeyPopupBox(state)
        }
        composeViewExt = popupComposeView(_extendedState) { state ->
            if (state is ExtendedState.Visible) KeyPopupExtendedBox(state)
        }

        window = createPopupWindow(composeView)
        windowExt = createPopupWindow(composeViewExt)
    }

    private fun <S> popupComposeView(
        state: StateFlow<S>,
        content: @Composable (S) -> Unit,
    ): ComposeView {
        return ComposeView(keyboardView.context).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            )
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                TaigiKeyboardTheme {
                    val s by state.collectAsStateWithLifecycle()
                    content(s)
                }
            }
        }
    }

    private fun createPopupWindow(view: View): PopupWindow {
        return PopupWindow(keyboardView.context).apply {
            animationStyle = 0
            contentView = view
            enterTransition = null
            exitTransition = null
            isClippingEnabled = false
            isFocusable = false
            isTouchable = false
            setBackgroundDrawable(null)
        }
    }

    /**
     * Wire the popup [ComposeView]s to the IME service so they can compose
     * inside their host [PopupWindow]. Two pieces are required and neither
     * substitutes for the other:
     *
     * 1. `setParentCompositionContext(ime.popupRecomposer)` — shortcuts
     *    [androidx.compose.ui.platform.AbstractComposeView.resolveParentCompositionContext]
     *    so it does not call `findOrCreateWindowRecomposer` on the popup
     *    window's `PopupDecorView`. That default lookup fails because
     *    PopupDecorView has no `ViewTreeLifecycleOwner` tag (the IME's
     *    [com.siansiansu.taigikeyboard.ime.lifecycle.LifecycleInputMethodService.installViewTreeOwners]
     *    only tags the IME decor view, not the popup window's separate
     *    decor view) — historic crash signature
     *    `IllegalStateException: ViewTreeLifecycleOwner not found from
     *     android.widget.PopupWindow$PopupDecorView`.
     *
     * 2. `setViewTree*Owner` — `AndroidComposeView` still consumes
     *    `LocalLifecycleOwner` / `LocalSavedStateRegistryOwner` /
     *    `LocalViewModelStoreOwner` from the view tree at composition
     *    time, including for `collectAsStateWithLifecycle`. Removing
     *    these tags would replace the `PopupDecorView` crash with a
     *    different one inside `AndroidComposeView`.
     *
     * Idempotent + lazy: settings preview path (KeyboardPreviewPanel)
     * leaves `taigikeyboard` null and never triggers show()/extend(),
     * so this is a no-op there.
     */
    private fun installPopupViewTreeOwnersIfNeeded() {
        if (ownersInstalled) return
        val ime = (keyboardView as? KeyboardView)?.taigikeyboard ?: return

        composeView.setParentCompositionContext(ime.popupRecomposer)
        composeViewExt.setParentCompositionContext(ime.popupRecomposer)

        composeView.setViewTreeLifecycleOwner(ime)
        composeView.setViewTreeViewModelStoreOwner(ime)
        composeView.setViewTreeSavedStateRegistryOwner(ime)
        composeViewExt.setViewTreeLifecycleOwner(ime)
        composeViewExt.setViewTreeViewModelStoreOwner(ime)
        composeViewExt.setViewTreeSavedStateRegistryOwner(ime)
        ownersInstalled = true
    }

    private fun resolveDisplayParams(): PopupDisplayParams {
        val ctx = keyboardView.context
        val res = ctx.resources
        val density = res.displayMetrics.density
        val prefs = (keyboardView as? KeyboardView)?.prefs ?: PrefHelper(ctx)
        return PopupDisplayParams(
            fgColorArgb = getColorFromAttr(ctx, R.attr.key_popup_fgColor),
            bgColorArgb = getColorFromAttr(ctx, R.attr.key_popup_bgColor),
            extBgColorArgb = getColorFromAttr(ctx, R.attr.key_popup_extended_bgColor),
            extBgColorActiveArgb = getColorFromAttr(ctx, R.attr.key_popup_extended_bgColorActive),
            shadowColorArgb = getColorFromAttr(ctx, R.attr.key_popup_extended_shadowColor),
            cornerRadiusPx = res.getDimension(R.dimen.key_borderRadius),
            keyHeightPx = res.getDimension(R.dimen.key_height).toInt(),
            popupTextSizePx = res.getDimension(R.dimen.key_popup_textSize),
            threeDotsSizePx = (16f * density).toInt(),
            ringWidthPx = (1f * density).toInt().coerceAtLeast(1),
            iconPaddingFraction = 0.2f,
            typeface = TypefaceLoader.getTypefaceByType(prefs.fontType, ctx),
        )
    }

    private fun freshDisplayParams(): PopupDisplayParams =
        resolveDisplayParams().also { cachedDisplay = it }

    private fun reuseDisplayParams(): PopupDisplayParams =
        cachedDisplay ?: freshDisplayParams()

    private fun buildPopupCell(keyView: KeyView, k: Int): PopupCell {
        val popupKeyData = keyView.data.popup[k]
        return when (popupKeyData.code) {
            KeyCode.SETTINGS ->
                PopupCell(
                    label = null,
                    icon = PopupIcon.Settings,
                    textScale = 1.0f,
                    useCustomTypeface = false,
                )
            KeyCode.SWITCH_TO_TEXT_CONTEXT ->
                // Legacy KeyPopupExtendedSingleView did NOT apply the custom
                // typeface on this branch — preserve that behavior.
                PopupCell(
                    label = keyView.resources.getString(R.string.key__view_characters),
                    icon = null,
                    textScale = 1.0f,
                    useCustomTypeface = false,
                )
            KeyCode.SWITCH_TO_MEDIA_CONTEXT ->
                PopupCell(
                    label = null,
                    icon = PopupIcon.SentimentSatisfied,
                    textScale = 1.0f,
                    useCustomTypeface = false,
                )
            else -> {
                val textScale =
                    if (popupKeyData.code == KeyCode.URI_COMPONENT_TLD) 0.6f else 1.0f
                PopupCell(
                    label = keyView.getComputedLetter(popupKeyData),
                    icon = null,
                    textScale = textScale,
                    useCustomTypeface = true,
                )
            }
        }
    }

    /**
     * Shows a preview popup for the passed [keyView]. Ignores show requests for key views which
     * key code is equal to or less than [KeyCode.SPACE]. KeyViews with a code defined in
     * [exceptionsForKeyCodes] will only shadow-calculating the size of the key popup, as these
     * sizes are needed for the extended popup. No popup will be shown to the user in this case.
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     */
    fun show(keyView: T_KV) {
        if (keyView is KeyView && keyView.data.code <= KeyCode.SPACE
            && !exceptionsForKeyCodes.contains(keyView.data.code)
            && keyView.data.popup.isEmpty()
        ) {
            return
        }

        if (keyboardView is KeyboardView) {
            val popupDims = KeyboardLayoutSolver.solvePopupDimensions(
                PopupDimensionsInput(
                    desiredKeyWidth = keyboardView.desiredKeyWidth,
                    desiredKeyHeight = keyboardView.desiredKeyHeight,
                    keyViewMeasuredWidth = keyView.measuredWidth,
                    isLandscape =
                        keyboardView.resources.configuration.orientation ==
                            Configuration.ORIENTATION_LANDSCAPE,
                ),
            )
            keyPopupWidth = popupDims.popupWidth
            keyPopupHeight = popupDims.popupHeight
            keyPopupDiffX = popupDims.popupDiffX
        } else {
            // EmojiKeyboardView fallback (Compose path no longer routes here): keep
            // existing keyPopupWidth/keyPopupHeight, only recompute the diff.
            keyPopupDiffX = (keyView.measuredWidth - keyPopupWidth) / 2
        }
        // Calculating is done, so exit show() here if this key view is a special one.
        if (keyView is KeyView && exceptionsForKeyCodes.contains(keyView.data.code)) {
            return
        }

        installPopupViewTreeOwnersIfNeeded()

        val display = freshDisplayParams()
        val label = (keyView as? KeyView)?.getComputedLetter().orEmpty()
        val showThreeDots = (keyView as? KeyView)?.data?.popup?.isNotEmpty() ?: false

        _previewState.value = PreviewState.Visible(
            label = label,
            showThreeDots = showThreeDots,
            popupWidthPx = keyPopupWidth,
            popupHeightPx = keyPopupHeight,
            display = display,
        )

        val keyPopupX = keyPopupDiffX
        val keyPopupY = -keyPopupHeight
        if (window.isShowing) {
            window.update(keyView, keyPopupX, keyPopupY, keyPopupWidth, keyPopupHeight)
        } else {
            window.width = keyPopupWidth
            window.height = keyPopupHeight
            window.showAsDropDown(keyView, keyPopupX, keyPopupY, Gravity.NO_GRAVITY)
        }
    }

    /**
     * Extends the currently showing key preview popup if there are popup keys defined in the
     * key data of the passed [keyView]. Ignores extend requests for key views which key code
     * is equal to or less than [KeyCode.SPACE]. An exception is made for the codes defined in
     * [exceptionsForKeyCodes], as they most likely have special keys bound to them.
     *
     * Layout shape (anchorSide / row0count / row1count) is owned by
     * [KeyboardLayoutSolver.solveExtendedPopupGeometry]; this method consumes
     * its outputs without re-deriving thresholds.
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     */
    fun extend(keyView: T_KV) {
        if (keyView is KeyView && keyView.data.code <= KeyCode.SPACE
            && !exceptionsForKeyCodes.contains(keyView.data.code)
            && keyView.data.popup.isEmpty()
        ) {
            return
        }

        // EmojiKeyView is no longer used (Compose implementation), so the
        // popup count for the non-KeyView branch defaults to 0.
        val popupCount = if (keyView is KeyView) keyView.data.popup.size else 0
        val geometry = KeyboardLayoutSolver.solveExtendedPopupGeometry(
            ExtendedPopupGeometryInput(
                popupCount = popupCount,
                keyViewX = keyView.x,
                keyboardViewMeasuredWidth = keyboardView.measuredWidth,
                keyViewMeasuredWidth = keyView.measuredWidth,
                keyViewMeasuredHeight = keyView.measuredHeight,
                keyPopupWidth = keyPopupWidth,
                keyPopupHeight = keyPopupHeight,
            ),
        )
        anchorSide = geometry.anchorSide
        row0count = geometry.row0count
        row1count = geometry.row1count
        anchorOffset = geometry.anchorOffset

        installPopupViewTreeOwnersIfNeeded()

        val display = reuseDisplayParams()
        val cells: List<PopupCell> =
            if (keyView is KeyView) {
                keyView.data.popup.indices.map { k -> buildPopupCell(keyView, k) }
            } else {
                emptyList()
            }

        var initialActive = -1
        cells.indices.forEach { idx ->
            val isInitActive =
                (anchorSide == AnchorSide.LEFT && (idx - row1count == anchorOffset)) ||
                    (anchorSide == AnchorSide.RIGHT && (idx - row1count == row0count - 1 - anchorOffset))
            if (isInitActive) initialActive = idx
        }
        activeExtIndex = if (initialActive >= 0) initialActive else null

        _extendedState.value = ExtendedState.Visible(
            cells = cells,
            activeIndex = initialActive,
            anchorSide = geometry.anchorSide,
            row0count = geometry.row0count,
            row1count = geometry.row1count,
            cellWidthPx = keyPopupWidth,
            cellHeightPx = keyView.measuredHeight,
            totalWidthPx = geometry.extWidth,
            totalHeightPx = geometry.extHeight,
            display = display,
        )

        // Hide the three-dots indicator on the preview popup while the extended
        // popup is showing — preview text content stays visible.
        val previewVisible = _previewState.value as? PreviewState.Visible
        if (previewVisible != null) {
            _previewState.value = previewVisible.copy(showThreeDots = false)
        }

        // Position and show popup window
        if (windowExt.isShowing) {
            windowExt.update(keyView, geometry.popupX, geometry.popupY, geometry.extWidth, geometry.extHeight)
        } else {
            windowExt.width = geometry.extWidth
            windowExt.height = geometry.extHeight
            windowExt.showAsDropDown(keyView, geometry.popupX, geometry.popupY, Gravity.NO_GRAVITY)
        }
    }

    /**
     * Updates the current selected key in extended popup according to the passed [event].
     * This function does nothing if the extended popup is not showing and will return false.
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     * @param event The [MotionEvent] passed from the parent keyboard view's onTouch event.
     * @return True if the pointer movement is within the elements bounds, false otherwise.
     */
    fun propagateMotionEvent(keyView: T_KV, event: MotionEvent): Boolean {
        if (!isShowingExtendedPopup) {
            return false
        }

        val kX: Float = event.x / keyPopupWidth.toFloat()

        // Check if out of boundary on y-axis
        if (event.y < -keyPopupHeight || event.y > 0.9f * keyPopupHeight) {
            return false
        }

        val newActiveIndex = when (anchorSide) {
            AnchorSide.LEFT -> when {
                // check if out of boundary on x-axis
                event.x < keyPopupDiffX - (anchorOffset + 1) * keyPopupWidth ||
                    event.x > (keyPopupDiffX + (row0count + 1 - anchorOffset) * keyPopupWidth) -> {
                    return false
                }

                // row 1
                event.y < 0 && row1count > 0 -> {
                    when {
                        kX >= row1count - anchorOffset -> row1count - 1
                        kX < -anchorOffset -> 0
                        kX < 0 -> kX.toInt() - 1 + anchorOffset
                        else -> kX.toInt() + anchorOffset
                    }
                }

                // row 0
                else -> {
                    when {
                        kX >= row0count - anchorOffset -> row1count + row0count - 1
                        kX < -anchorOffset -> row1count
                        kX < 0 -> row1count + kX.toInt() - 1 + anchorOffset
                        else -> row1count + kX.toInt() + anchorOffset
                    }
                }
            }

            AnchorSide.RIGHT -> when {
                // check if out of boundary on x-axis
                event.x > keyView.measuredWidth - keyPopupDiffX + (anchorOffset + 1) * keyPopupWidth ||
                    event.x < (
                        keyView.measuredWidth -
                            keyPopupDiffX - (row0count + 1 - anchorOffset) * keyPopupWidth
                    ) -> {
                    return false
                }

                // row 1
                event.y < 0 && row1count > 0 -> {
                    when {
                        kX >= anchorOffset -> row1count - 1
                        kX < -(row1count - 1 - anchorOffset) -> 0
                        kX < 0 -> row1count - 2 + kX.toInt() - anchorOffset
                        else -> row1count - 1 + kX.toInt() - anchorOffset
                    }
                }

                // row 0
                else -> {
                    when {
                        kX >= anchorOffset -> row1count + row0count - 1
                        kX < -(row0count - 1 - anchorOffset) -> row1count
                        kX < 0 -> row1count + row0count - 2 + kX.toInt() - anchorOffset
                        else -> row1count + row0count - 1 + kX.toInt() - anchorOffset
                    }
                }
            }
        }

        if (newActiveIndex != activeExtIndex) {
            activeExtIndex = newActiveIndex
            val current = _extendedState.value as? ExtendedState.Visible
            if (current != null) {
                _extendedState.value = current.copy(activeIndex = newActiveIndex)
            }
        }

        return true
    }

    /**
     * Gets the [KeyData] of the currently active key. May be either the key of the popup preview
     * or one of the keys in extended popup, if shown. Returns null if type parameter [T_KV]
     * is not [KeyView].
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     * @return The [KeyData] object of the currently active key or null.
     */
    fun getActiveKeyData(keyView: T_KV): KeyData? {
        return if (keyView is KeyView) {
            keyView.data.popup.getOrNull(activeExtIndex ?: -1) ?: keyView.data
        } else {
            null
        }
    }

    /**
     * Gets the [EmojiKeyData] of the currently active key. EmojiKeyView is no
     * longer routed through this manager (the emoji palette is fully Compose),
     * so this always returns null.
     */
    fun getActiveEmojiKeyData(keyView: T_KV): EmojiKeyData? {
        return null
    }

    /**
     * Hides the key preview popup as well as the extended popup.
     */
    fun hide() {
        _previewState.value = PreviewState.Hidden
        _extendedState.value = ExtendedState.Hidden
        if (windowExt.isShowing) {
            windowExt.dismiss()
        }
        activeExtIndex = null
        cachedDisplay = null
    }

    /**
     * Dismisses all currently shown popups. Should be called by the parent keyboard view when it
     * is closing.
     */
    fun dismissAllPopups() {
        _previewState.value = PreviewState.Hidden
        _extendedState.value = ExtendedState.Hidden
        if (window.isShowing) {
            window.dismiss()
        }
        if (windowExt.isShowing) {
            windowExt.dismiss()
        }
        activeExtIndex = null
        cachedDisplay = null
    }
}
