package com.siansiansu.taigikeyboard.ui.tabs.tab4

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.settings.SettingsResetCoordinator
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

// ViewModel for the "Reset all settings" flow in InputSettingsScreen.
// Owns the coroutine launch + resetCounter signal that InputSettingsScreen
// observes via remember(resetCounter) to re-read prefs after a reset.
//
// Activity supplies its warmed PrefHelper via the method param (matches
// DataManagementViewModel.exportBackup shape) and an onResult callback so the
// Toast stays at Activity scope. All-or-nothing semantics preserved verbatim
// from pre-VM SettingsMainActivity.resetAllSettings.
class SettingsResetViewModel(
    application: Application,
) : AndroidViewModel(application) {
    private val _resetCounter = MutableStateFlow(0)
    val resetCounter: StateFlow<Int> = _resetCounter.asStateFlow()

    fun resetAllSettings(
        prefs: PrefHelper,
        onResult: (success: Boolean) -> Unit,
    ) {
        viewModelScope.launch {
            val success =
                try {
                    SettingsResetCoordinator.resetAll(prefs)
                    val root = CompositionRoot.shared(getApplication())
                    SettingsResetCoordinator.resetAllUserData(root)
                    _resetCounter.value++
                    true
                } catch (_: Exception) {
                    false
                }
            onResult(success)
        }
    }
}
