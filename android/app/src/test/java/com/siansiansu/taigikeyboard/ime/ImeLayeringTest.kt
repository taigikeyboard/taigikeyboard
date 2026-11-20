package com.siansiansu.taigikeyboard.ime

import com.siansiansu.taigikeyboard.locateMainSourceRoot
import com.siansiansu.taigikeyboard.mainKotlinSources
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.io.File

/**
 * Pins the IME → app layering rule (`docs/contributing/android-guidelines.md` §3): code under
 * `ime/` may use the shared look (`ui/theme`, `ui/components`) but never an app tab screen
 * (`ui/tabs`). Something both need moves out of `ui/tabs` first, as the option lists did
 * (`ime/settings/SettingOptions.kt`).
 *
 * A source walk rather than reflection, so a fully qualified reference counts as well as an import.
 */
class ImeLayeringTest {
    @Test
    fun ime_never_references_app_tab_screens() {
        val imeRoot = File(locateMainSourceRoot(), "com/siansiansu/taigikeyboard/ime")
        val sources = mainKotlinSources(imeRoot).toList()
        assertTrue("source walk must find ime/ files (path stale?)", sources.isNotEmpty())

        val violations =
            sources.flatMap { file ->
                file.readLines().withIndex().filter { (_, line) -> FORBIDDEN in line }.map { (index, line) ->
                    "${file.relativeTo(imeRoot)}:${index + 1}  ${line.trim()}"
                }
            }
        if (violations.isNotEmpty()) {
            fail("ime/ must not reference app tab screens ($FORBIDDEN*):\n" + violations.joinToString("\n"))
        }
    }

    private companion object {
        const val FORBIDDEN = "com.siansiansu.taigikeyboard.ui.tabs."
    }
}
