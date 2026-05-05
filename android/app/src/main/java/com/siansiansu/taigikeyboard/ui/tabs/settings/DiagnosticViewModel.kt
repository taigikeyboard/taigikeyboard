package com.siansiansu.taigikeyboard.ui.tabs.settings

import android.app.Application
import androidx.lifecycle.AndroidViewModel

// ViewModel for the diagnostic section of InputSettingsScreen.
// Gather is synchronous; the VM exists so the Composable owns no service call.
class DiagnosticViewModel(
    application: Application,
) : AndroidViewModel(application) {
    fun gather(): DiagnosticInfo = DiagnosticService.gather(getApplication())
}
