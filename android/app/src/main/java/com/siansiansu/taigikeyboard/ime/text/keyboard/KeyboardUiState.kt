package com.siansiansu.taigikeyboard.ime.text.keyboard

import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation

/**
 * Immutable snapshot of the keyboard body state consumed by [KeyboardImeRoot].
 *
 * Container-width-dependent values (per-key dimensions) are derived inside
 * the Composable from `BoxWithConstraints` so the IME-side push path stays
 * decoupled from layout-pass timing. Theme + caps + composing snapshot is
 * pre-resolved on the IME side and surfaced through [appearance].
 *
 * Recomposition is driven entirely by `data class` equality: `TextInputManager`
 * skips publishing when the next state equals the current one, so no-op
 * appearance refreshes do not invalidate the ~30 `KeyContent` slots.
 *
 * `layouts` is published as a fresh [Map] copy on every mutation so that
 * snapshot equality drives recomposition correctly — never mutate the map
 * in place after a [KeyboardUiState] is published.
 */
data class KeyboardUiState(
    val activeMode: KeyboardMode,
    val layouts: Map<KeyboardMode, KeyboardLayoutData>,
    val keyVariation: KeyVariation,
    /** Theme + caps + composing snapshot. `null` until the first push from
     *  `TextInputManager` — the Composable skips rendering on null. */
    val appearance: KeyboardAppearance?,
) {
    companion object {
        val EMPTY = KeyboardUiState(
            activeMode = KeyboardMode.CHARACTERS,
            layouts = emptyMap(),
            keyVariation = KeyVariation.NORMAL,
            appearance = null,
        )
    }
}
