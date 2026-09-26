package com.siansiansu.taigikeyboard.settings

import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper

// Resets settings and user-owned data stores.
//
// Two surfaces, intentionally separate, mirroring
// ios/Sources/TaigiKeyboard/Settings/SettingsResetCoordinator.swift:
// - resetAll(prefs): pure settings reset.
// - resetAllUserData(root): destructive — wipes user frequency, next-word and
//   learned-phrase data.
//
// The engine owns the stores (roadmap P8b) and attempts every one, so a
// partial failure still clears what it can; one that could not be emptied
// surfaces as an exception, reported by the caller's single try/catch.
object SettingsResetCoordinator {
    suspend fun resetAll(prefs: PrefHelper) {
        prefs.resetToDefaults()
    }

    suspend fun resetAllUserData(root: CompositionRoot) {
        root.userData.clearLearningRecords()
    }
}
