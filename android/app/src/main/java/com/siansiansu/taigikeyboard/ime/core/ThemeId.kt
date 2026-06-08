// 中文: 主題身分 — "default" = 既有全域外觀 buffer;UUID 字串 = 自訂主題;其餘非 UUID 字串 = 內建主題 id。

package com.siansiansu.taigikeyboard.ime.core

import java.util.UUID

/**
 * Theme identity helpers. `"default"` = the legacy free-pick buffer
 * (PrefHelper.colorSettings + scalars); UUID strings = user themes; other
 * non-UUID strings = built-in theme ids. Mirrors iOS ThemeId.
 */
object ThemeId {
    /** The legacy free-pick buffer sentinel. */
    const val DEFAULT = "default"

    /**
     * Whether [id] is a user theme. User-theme ids are UUID strings; the
     * `default` buffer and built-in ids are not.
     */
    fun isUserTheme(id: String): Boolean = runCatching { UUID.fromString(id) }.isSuccess
}
