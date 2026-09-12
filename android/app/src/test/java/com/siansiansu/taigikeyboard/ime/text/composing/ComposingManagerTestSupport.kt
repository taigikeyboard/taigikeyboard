// Shared JVM-test scaffolding for ComposingManager: no engine dispatch, no settings reads.

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider

/** The manager never reads settings on the paths JVM tests exercise (no engine dispatch). */
internal object UnusedSettingsProvider : EngineSettingsProvider {
    override val current: EngineSettings
        get() = error("JVM test path must not read settings")
}

internal fun composingManagerForTest(delegate: ComposingDelegate = DefaultComposingDelegate) =
    ComposingManager(settingsProvider = UnusedSettingsProvider, delegate = delegate)
