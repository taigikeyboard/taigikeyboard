package com.siansiansu.taigikeyboard.settings

import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

// Resets settings and user-owned data stores.
//
// Two surfaces, intentionally separate, mirroring
// ios/Sources/TaigiKeyboard/Settings/SettingsResetCoordinator.swift:
// - resetAll(prefs): pure settings reset.
// - resetAllUserData(root): destructive — wipes user frequency and next-word DBs.
//
// Android divergence from iOS (deferred parity): iOS wraps each destructive
// deletion in its own do/try/catch with per-op logging, so a partial failure
// still clears what it can. Android preserves pre-PR single try/catch at the
// caller boundary — the inner services already log and swallow their own DB
// exceptions, so only an exception that escapes (e.g. DataStore I/O from
// resetAll) skips subsequent operations.
object SettingsResetCoordinator {
    suspend fun resetAll(prefs: PrefHelper) {
        prefs.resetToDefaults()
    }

    suspend fun resetAllUserData(root: CompositionRoot) {
        root.userFreq.deleteDatabase()
        root.nextWord.clearAllAssociations()
    }
}
