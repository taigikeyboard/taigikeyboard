// TextInputManager 的 UI-state + layout-reload 協作器 — 擁有 _keyboardUi 寫入端、
// layoutReloadJob 取消鏈、Dispatchers.IO fetchComputedLayout 包裝;TIM 透過 forwarder 維持公開 API。

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.view.View
import android.view.ViewGroup
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.popup.KeyPopupManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Owns the [KeyboardUiState] write-side ([_keyboardUi]), the [layoutReloadJob]
 * cancellation chain, and the [Dispatchers.IO] [LayoutManager.fetchComputedLayout]
 * wrappers. Borrows TIM's [CoroutineScope] (TIM's `onDestroy` calls `cancel()`
 * which cancels all child Jobs). Hands appearance-refresh back to TIM via the
 * [onLayoutChanged] callback so the [com.siansiansu.taigikeyboard.ime.text.KeyboardAppearanceResolver]
 * snapshot ownership stays on TIM.
 */
internal class KeyboardUiCoordinator(
    private val scope: CoroutineScope,
    layoutManagerFactory: () -> LayoutManager,
    private val activeSubtypeProvider: () -> Subtype,
    private val translateSwappedProvider: () -> Boolean,
    private val onLayoutChanged: () -> Unit,
    private val onActiveModeChanged: () -> Unit,
) {
    private val layoutManager: LayoutManager by lazy(layoutManagerFactory)

    private val _keyboardUi = MutableStateFlow(KeyboardUiState.EMPTY)
    val keyboardUi: StateFlow<KeyboardUiState> = _keyboardUi.asStateFlow()

    var activeKeyboardMode: KeyboardMode = KeyboardMode.CHARACTERS
        private set

    private var layoutReloadJob: Job? = null

    fun publishKeyVariation(keyVariation: KeyVariation) {
        _keyboardUi.update { current ->
            if (current.keyVariation == keyVariation) current else current.copy(keyVariation = keyVariation)
        }
    }

    fun publishAppearance(appearance: KeyboardAppearance) {
        _keyboardUi.update { current ->
            if (current.appearance == appearance) current else current.copy(appearance = appearance)
        }
    }

    /**
     * Adds a single ComposeView under the smartbar inside `text_input_content`,
     * closing over [_keyboardUi] so write-side ownership stays private to the
     * coordinator. Returns the mounted ComposeView so TIM can stash it as the
     * popup-anchor + dispatcher `composeHost` reference. `null` when the
     * `text_input_content` container is missing.
     */
    fun mountKeyboardComposeView(
        inputView: InputView,
        coordinator: KeyTouchCoordinator,
        popupHost: KeyPopupManager,
        onHeightFactorChanged: (Float) -> Unit,
    ): View? {
        val container = inputView.findViewById<ViewGroup>(R.id.text_input_content) ?: return null
        val placeholder = inputView.findViewById<View>(R.id.keyboard_compose_host)
        val composeView = ComposeView(inputView.context).apply {
            id = R.id.keyboard_compose_host
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            )
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                KeyboardImeRoot(
                    uiStateFlow = _keyboardUi,
                    coordinator = coordinator,
                    popupHost = popupHost,
                    onHeightFactorChanged = onHeightFactorChanged,
                )
            }
        }
        if (placeholder != null) {
            val index = container.indexOfChild(placeholder)
            container.removeView(placeholder)
            container.addView(composeView, index)
        } else {
            container.addView(composeView)
        }
        return composeView
    }

    /** CLIPBOARD carries no layout to fetch; an already-cached mode needs no
     *  recompute. Shared skip guard for both the async [ensureLayoutLoaded] and
     *  the synchronous [ensureLayoutLoadedNow] so the two stay in lockstep. */
    private fun isLayoutLoadNeeded(mode: KeyboardMode): Boolean =
        mode != KeyboardMode.CLIPBOARD && !_keyboardUi.value.layouts.containsKey(mode)

    /**
     * Loads [KeyboardLayoutData] for [mode] off the main thread, then publishes
     * it into [_keyboardUi] so the Composable can render. Skipped when [mode]
     * is [KeyboardMode.CLIPBOARD] or already cached.
     */
    suspend fun ensureLayoutLoaded(mode: KeyboardMode) {
        if (!isLayoutLoadNeeded(mode)) return
        val computed = withContext(Dispatchers.IO) {
            layoutManager.fetchComputedLayout(mode, activeSubtypeProvider())
        }
        publishLayout(mode, KeyboardLayoutData.from(computed))
    }

    /**
     * Synchronous sibling of [ensureLayoutLoaded] for the boot / first-show
     * seam. Computes + publishes the layout on the CALLING (main) thread so the
     * keyboard body holds a non-empty layout in [_keyboardUi] BEFORE the IME
     * window's first measure. Without it the cold-open Compose body renders
     * empty (no `layout`/`appearance`), the `wrap_content` input view is pinned
     * to smartbar-only height, and the async [ensureLayoutLoaded] publish lands
     * too late to reliably re-grow the IME window. The first call per mode does
     * one bounded synchronous layout compute (assets read + parse) on the calling
     * thread; warm reopens cache-hit and return without I/O.
     */
    fun ensureLayoutLoadedNow(mode: KeyboardMode) {
        if (!isLayoutLoadNeeded(mode)) return
        val computed = layoutManager.fetchComputedLayout(mode, activeSubtypeProvider())
        publishLayout(mode, KeyboardLayoutData.from(computed))
    }

    /**
     * Coerces [KeyboardMode.CLIPBOARD] to [KeyboardMode.CHARACTERS], updates
     * [activeKeyboardMode], and flips [KeyboardUiState.activeMode]. Cache-hit
     * branch stays SYNCHRONOUS — async-ifying it would re-introduce the
     * `mainViewFlipper.indexOfChild(null) == -1` race-class via a different
     * door. [onActiveModeChanged] fires once after [setActiveMode] succeeds
     * (sync cache-hit OR async load-then-Main path).
     */
    fun setActiveKeyboardMode(mode: KeyboardMode) {
        val actualMode = if (mode == KeyboardMode.CLIPBOARD) KeyboardMode.CHARACTERS else mode
        activeKeyboardMode = actualMode
        if (_keyboardUi.value.layouts.containsKey(actualMode)) {
            publishActiveMode(actualMode)
            onActiveModeChanged()
        } else {
            scope.launch(Dispatchers.Default) {
                ensureLayoutLoaded(actualMode)
                withContext(Dispatchers.Main) {
                    publishActiveMode(actualMode)
                    onActiveModeChanged()
                }
            }
        }
    }

    /** Flips [KeyboardUiState.activeMode] without triggering the smartbar /
     *  measure side effects. Used by the [onRegisterInputView] bootstrap path
     *  where smartbar wiring already ran ahead of this call. */
    fun publishActiveMode(mode: KeyboardMode) {
        _keyboardUi.update { current ->
            if (current.activeMode == mode) current else current.copy(activeMode = mode)
        }
    }

    fun reloadCurrentLayout() {
        val currentMode = activeKeyboardMode
        layoutReloadJob?.cancel()
        layoutReloadJob = scope.launch {
            val isTranslateSwapped = translateSwappedProvider()
            val computed = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(currentMode, activeSubtypeProvider(), isTranslateSwapped)
            }
            publishLayout(currentMode, KeyboardLayoutData.from(computed))
            onLayoutChanged()
        }
    }

    /**
     * Reloads every cached mode except [activeKeyboardMode] off the main
     * thread. Deliberately does NOT touch [layoutReloadJob] — runs concurrently
     * with normal reloads.
     */
    fun reloadAllLayoutsInBackground() {
        scope.launch {
            val isTranslateSwapped = translateSwappedProvider()
            val modes = _keyboardUi.value.layouts.keys.toList()
            for (mode in modes) {
                if (mode != activeKeyboardMode) {
                    val computed = withContext(Dispatchers.IO) {
                        layoutManager.fetchComputedLayout(mode, activeSubtypeProvider(), isTranslateSwapped)
                    }
                    withContext(Dispatchers.Main) {
                        publishLayout(mode, KeyboardLayoutData.from(computed))
                    }
                }
            }
        }
    }

    fun reloadForSubtype(newSubtype: Subtype) {
        layoutReloadJob?.cancel()
        layoutReloadJob = scope.launch {
            val computed = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, newSubtype)
            }
            publishLayout(KeyboardMode.CHARACTERS, KeyboardLayoutData.from(computed))
            onLayoutChanged()
        }
    }

    fun reloadForInputMode(overrideInputMode: String) {
        layoutReloadJob?.cancel()
        layoutReloadJob = scope.launch {
            val computed = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(
                    KeyboardMode.CHARACTERS,
                    activeSubtypeProvider(),
                    overrideInputMode = overrideInputMode,
                )
            }
            publishLayout(KeyboardMode.CHARACTERS, KeyboardLayoutData.from(computed))
            onLayoutChanged()
        }
    }

    fun reloadForLayoutType() {
        layoutReloadJob?.cancel()
        layoutReloadJob = scope.launch {
            val computed = withContext(Dispatchers.IO) {
                layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, activeSubtypeProvider())
            }
            publishLayout(KeyboardMode.CHARACTERS, KeyboardLayoutData.from(computed))
            onLayoutChanged()
        }
    }

    private fun publishLayout(mode: KeyboardMode, data: KeyboardLayoutData) {
        _keyboardUi.update { current ->
            if (current.layouts[mode] == data) {
                current
            } else {
                current.copy(layouts = current.layouts + (mode to data))
            }
        }
    }
}
