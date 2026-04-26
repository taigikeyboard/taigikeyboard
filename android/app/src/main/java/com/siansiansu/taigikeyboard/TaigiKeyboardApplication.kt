package com.siansiansu.taigikeyboard

import android.app.Application
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

// Application root — owns the process-wide service graph + prefs.
class TaigiKeyboardApplication : Application() {
    lateinit var prefs: PrefHelper
        private set

    lateinit var compositionRoot: CompositionRoot
        private set

    // Process-lifetime scope — never cancelled. Used only for fire-and-forget
    // work that must NOT be tied to an IME service lifecycle (prefs migration
    // survives cold IME starts; custom-dict seed runs at most once).
    val applicationScope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onCreate() {
        super.onCreate()

        prefs = PrefHelper(this)
        prefs.warmUp()
        compositionRoot = CompositionRoot.shared(this)

        // D9.2 — register the Rust shared-core logger sink so any
        // `log::warn!` etc. from `librust_taigi.so` reaches the same
        // `LoggerBackend` the rest of the app uses.
        RustEngineBridge.install(compositionRoot.logger)

        // Deferred boot work — kept off the Application.onCreate main thread
        // per `android-state-audit.md` §A7 (keep Application.onCreate cheap).
        // Services are idempotent, so a duplicate call from a legacy caller
        // would be harmless during the A7 migration window.
        applicationScope.launch {
            prefs.migrateFromSharedPreferences()
        }
        applicationScope.launch {
            compositionRoot.customDict.seedDefaultEntryIfEmpty()
        }
    }
}
